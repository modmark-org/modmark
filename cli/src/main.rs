use clap::Parser;
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal, ExecutableCommand,
};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use notify_debouncer_mini::{
    new_debouncer,
    notify::{self, RecommendedWatcher, RecursiveMode},
    DebounceEventResult, DebouncedEvent, Debouncer,
};
use once_cell::sync::OnceCell;
use portpicker::Port;
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{stdout, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{
    ws::{Message, WebSocket},
    Filter, Rejection, Reply,
};

use error::CliError;
use modmark_core::{context::CompilationState, eval, Context, CoreError, OutputFormat};
use parser::{parse, Ast};

use crate::package::PackageManager;

mod error;
mod package;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(index = 1, help = "Path to input file")]
    input: PathBuf,

    #[arg(index = 2, help = "Path to output file")]
    output: Option<PathBuf>,

    #[arg(short = 'f', long = "format", help = "The output format of the file")]
    format: Option<String>,

    #[arg(short = 'r', long = "registry", help = "A URL to the registry to use")]
    registry: Option<String>,

    #[arg(
        short = 'w',
        long = "watch",
        help = "Watches file and compiles changes"
    )]
    watch: bool,

    #[arg(short = 'd', long = "dev", help = "Print the AST")]
    dev: bool,

    #[arg(long = "deny-read", help = "Disables read access for packages")]
    deny_read: bool,

    #[arg(long = "deny-write", help = "Disables write access for packages")]
    deny_write: bool,

    #[arg(
        long = "no-prompts",
        help = "Disables prompts for packages requesting file access"
    )]
    no_prompts: bool,

    #[arg(
        short = 'a',
        long = "assets",
        help = "Specifies the relative path to the directory with external files"
    )]
    assets: Option<String>,
}

impl Args {
    /// Get the output format from the cli and, if need be,
    /// infer the format based on the file extension on the output file
    fn get_output_format(&self) -> Result<OutputFormat, CliError> {
        let infer_output_format = |output: &PathBuf| {
            output.extension().and_then(|ext| match ext.to_str() {
                Some("tex") => Some(OutputFormat::new("latex")),
                Some("html") => Some(OutputFormat::new("html")),
                Some("htm") => Some(OutputFormat::new("html")),
                _ => None,
            })
        };

        if let Some(format) = &self.format {
            Ok(OutputFormat::new(format))
        } else {
            self.output
                .as_ref()
                .and_then(infer_output_format)
                .ok_or_else(|| CliError::UnknownOutputFormat)
        }
    }

    /// Check if a html live preview should be used
    fn use_html_preview(&self) -> bool {
        // If no output file was provided and the output format is "html" (or left unspecified)
        // we know that the user wants to use the live preview.
        if self.output.is_none() {
            return self.format.is_none()
                || self
                    .get_output_format()
                    .map(|format| format == OutputFormat::new("html"))
                    .unwrap_or(false);
        }

        // When using the --watch flag and the output format is html
        // we also start the live preview
        if self.watch {
            if let Ok(format) = self.get_output_format() {
                return format == OutputFormat::new("html");
            }
        }

        // In all other cases, don't use start the live preview
        false
    }
}

static DEFAULT_REGISTRY: &str =
    "https://raw.githubusercontent.com/modmark-org/package-registry/main/package-registry.json";

static CTX: OnceCell<Mutex<Context<PackageManager>>> = OnceCell::new();
static PREVIEW_PORT: OnceCell<Option<Port>> = OnceCell::new();
static ABSOLUTE_OUTPUT_PATH: OnceCell<PathBuf> = OnceCell::new();
static CONNECTION_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

type PreviewConnections = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;
type PreviewDoc = Arc<Mutex<String>>;

type CompilationResult = Result<(String, CompilationState, Ast), CoreError>;

/// Compile a file and return the transpiled content, compilation state and ast.
fn compile_file(input_file: &Path, output_format: &OutputFormat) -> CompilationResult {
    let source = fs::read_to_string(input_file)?;
    let ast = parse(&source)?;
    let (output, state) = eval(
        &source,
        &mut CTX.get().unwrap().lock().unwrap(),
        output_format,
    )?;

    Ok((output, state, ast))
}

fn print_compiling_message() -> Result<(), CliError> {
    let mut stdout = stdout();
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.execute(style::PrintStyledContent("Compiling...".yellow()))?;

    Ok(())
}

fn print_result(result: &CompilationResult, args: &Args) -> Result<(), CliError> {
    let mut stdout = stdout();

    let (_, state, ast) = match result {
        Ok(result) => result,
        Err(error) => {
            stdout.execute(terminal::Clear(terminal::ClearType::All))?;
            stdout.execute(cursor::MoveTo(0, 0))?;
            stdout.execute(style::PrintStyledContent(
                format!("Compilation error:\n{error}\n\n").red(),
            ))?;
            return Ok(());
        }
    };

    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.execute(style::PrintStyledContent(
        "File successfully compiled!\n".green(),
    ))?;

    // Print the path to the live preview (if using one)
    if args.use_html_preview() {
        let port = get_port()?;
        println!("Live preview available at: http://localhost:{port}");
    }

    println!();

    if !state.warnings.is_empty() {
        stdout.execute(style::PrintStyledContent("Warnings:\n".yellow()))?;
        for warning in &state.warnings {
            stdout.execute(style::PrintStyledContent(format!("{warning}\n").yellow()))?;
        }
    }

    if !state.errors.is_empty() {
        stdout.execute(style::PrintStyledContent("Errors:\n".red()))?;
        for error in &state.errors {
            stdout.execute(style::PrintStyledContent(format!("{error}\n").red()))?;
        }
    }

    // Print the path to the output
    // (If we have already saved the file and have gotten the absolute path
    // print that otherwise we print the provided path from the cli)
    if let Some(output_path) = ABSOLUTE_OUTPUT_PATH.get() {
        println!("Your file can be found at {}.", output_path.display());
    } else if let Some(output_path) = args.output.as_ref() {
        println!("Your file can be found at {}.", output_path.display());
    }

    if args.dev {
        println!("{}", ast.tree_string());
    }

    stdout.flush()?;

    Ok(())
}

/// Write the result to a file
fn save_result(result: &CompilationResult, args: &Args) -> Result<(), CliError> {
    if let Some(output) = &args.output {
        if let Ok((document, _, _)) = result {
            let mut file = File::create(output)?;

            // Save the absolute path to the output file
            // now once we have created it.
            ABSOLUTE_OUTPUT_PATH.get_or_init(|| {
                output
                    .canonicalize()
                    .expect("Failed to find absolute path of output file")
            });

            file.write_all(document.as_bytes())?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match run_cli(args).await {
        Ok(_) => (),
        Err(error) => {
            let mut stdout = stdout();
            stdout
                .execute(style::PrintStyledContent(format!("{error}").red()))
                .unwrap();
        }
    }
}

async fn run_cli(args: Args) -> Result<(), CliError> {
    let current_path = env::current_dir()?;

    let registry = args
        .registry
        .as_deref()
        .unwrap_or(DEFAULT_REGISTRY)
        .to_string();

    CTX.set(Mutex::new(
        Context::new_with_resolver(PackageManager { registry }).unwrap(),
    ))
    .unwrap();

    CTX.get().unwrap().lock().unwrap().set_args(
        &args.assets,
        args.deny_read,
        args.deny_write,
        args.no_prompts,
    );

    // Using html output format and watch flag
    // (or if the user never provided a output file at all)
    if args.use_html_preview() && get_port().is_ok() {
        let connections = PreviewConnections::default();
        let document = PreviewDoc::default();
        let port = get_port()?;

        println!("started server and watching dir {current_path:?}");

        // Spawn a seperate task for the preview server
        {
            let doc = document.clone();
            let conn = connections.clone();
            let path = current_path.clone();
            tokio::task::spawn(async move {
                let routes = get_server_config(doc, conn, path);
                warp::serve(routes).run(([127, 0, 0, 1], port)).await
            });
        }

        watch_files(
            Some(document.clone()),
            Some(connections.clone()),
            &args,
            &current_path,
        )
        .await?;

        return Ok(());
    }

    // using the watch flag but with some other output format
    if args.watch && args.output.is_some() {
        // Just start the file watcher, but without html live preview
        // which means that we wil provide None instead of the document and connections
        return watch_files(None, None, &args, &current_path).await;
    }

    // Otherwise, if they are not using the watcher or live preview
    // just compile the file once, assuming that they actually provided a output file
    if args.output.is_some() {
        print_compiling_message()?;
        let compilation_result = compile_file(&args.input, &args.get_output_format()?);
        save_result(&compilation_result, &args)?;
        print_result(&compilation_result, &args)?;
    } else {
        return Err(CliError::MissingOutputFile);
    }

    Ok(())
}

/// Choose a free port for hosting the html live preview
fn get_port() -> Result<u16, CliError> {
    PREVIEW_PORT
        .get_or_init(portpicker::pick_unused_port)
        .ok_or(CliError::NoFreePorts)
}

fn get_server_config(
    document: PreviewDoc,
    connections: PreviewConnections,
    current_path: PathBuf,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + 'static {
    let connections = warp::any().map(move || connections.clone());
    let document = warp::any().map(move || document.clone());

    let websocket = warp::path("ws").and(warp::ws()).and(connections).map(
        |ws: warp::ws::Ws, connections: PreviewConnections| {
            ws.on_upgrade(move |socket| handle_connection(socket, connections))
        },
    );

    let preview = warp::path::end().and(document).map(|document: PreviewDoc| {
        // Inject a JS script after the end of the </body> tg
        let html = document.lock().unwrap();
        let modified_html = html.replace(
            "</body>",
            concat!("</body>\n", include_str!("./preview_injection.html")),
        );
        warp::reply::html(modified_html)
    });

    let working_directory = warp::fs::dir(current_path);

    working_directory.or(websocket).or(preview)
}

async fn handle_connection(socket: WebSocket, connections: PreviewConnections) {
    // Transmitter and receiver for the websocket
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Transmitter and receiver for communication with the rest of the program
    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    // Get a id and add this connection to HashMap of all connections
    let id = CONNECTION_ID_COUNTER.fetch_add(1, Ordering::Acquire);
    connections.write().await.insert(id, tx);

    // Relay any message to the websocket connection
    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            ws_tx
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("Websocket send error: {e}");
                })
                .await;
        }
    });

    // Await input from the client.
    // Note that we don't actually care about what they send us
    // but this will keep the connection alive until the client disconnects
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(_) => (),
            Err(e) => {
                eprintln!("websocket error: {e}");
                break;
            }
        };
    }

    // Remove the client from the list of connections once they disconnect
    connections.write().await.remove(&id);
}

/// Watch a path for file changes and send a reload message to all connected clients
async fn watch_files<P: AsRef<Path>>(
    document: Option<PreviewDoc>,
    connections: Option<PreviewConnections>,
    args: &Args,
    watch_dir: P,
) -> Result<(), CliError> {
    // Function to recompile the document:
    async fn compile(
        document: Option<&PreviewDoc>,
        connections: Option<&PreviewConnections>,
        args: &Args,
    ) -> Result<(), CliError> {
        print_compiling_message()?;

        let compilation_result = compile_file(
            &args.input,
            &args
                .get_output_format()
                .unwrap_or_else(|_| OutputFormat::new("html")),
        );

        // Write to the output file (if there was one)
        save_result(&compilation_result, args)?;

        // Print the result to the terminal
        print_result(&compilation_result, args)?;

        // Also save the result to the live preview document
        if let Some(document) = &document {
            if let Ok((content, _, _)) = compilation_result {
                *document.lock().unwrap() = content;
            }
        }

        // Also, send a "reload" message to every connected preview client
        if let Some(connections) = &connections {
            for (_, ws_tx) in connections.read().await.iter() {
                ws_tx.send(Message::text("reload")).unwrap();
            }
        }

        Ok(())
    }

    // Trigger a first compilation
    compile(document.as_ref(), connections.as_ref(), args).await?;

    let (mut debounce_watcher, mut rx) = get_debounce_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    debounce_watcher
        .watcher()
        .watch(watch_dir.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(events) => {
                // If we also generate a output file, discard any changes from that file
                if let Some(output_path) = ABSOLUTE_OUTPUT_PATH.get() {
                    if events.iter().any(|event| event.path == *output_path) {
                        continue;
                    }
                }

                compile(document.as_ref(), connections.as_ref(), args).await?;
            }
            Err(e) => eprintln!("watch error: {e:?}"),
        }
    }

    Ok(())
}

/// Get a debouncing file watcher using a async api
fn get_debounce_watcher() -> Result<
    (
        Debouncer<RecommendedWatcher>,
        UnboundedReceiverStream<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    ),
    notify::Error,
> {
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let watcher = new_debouncer(
        Duration::from_millis(50),
        None,
        move |res: DebounceEventResult| {
            tx.send(res).unwrap();
        },
    )?;

    Ok((watcher, rx))
}
