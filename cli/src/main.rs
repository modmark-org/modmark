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
};

use clap::{Parser, Subcommand};
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal, ExecutableCommand,
};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::OnceCell;
use package::cache_location;
use portpicker::Port;
use tokio::sync::{
    mpsc::{self, channel, Receiver},
    RwLock,
};
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::UnboundedReceiverStream;
use walkdir::WalkDir;
use warp::{
    ws::{Message, WebSocket},
    Filter, Rejection, Reply,
};

use error::CliError;
use modmark_core::{context::CompilationState, eval, Context, CoreError, OutputFormat};
use parser::{parse, Ast};

use crate::file_access::CliAccessManager;
use crate::package::PackageManager;

mod error;
mod file_access;
mod package;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
struct CompileArgs {
    #[arg(index = 1, help = "Path to input file")]
    input: PathBuf,

    #[arg(index = 2, help = "Path to output file")]
    output: Option<PathBuf>,

    #[arg(short = 'f', long = "format", help = "The output format of the file")]
    format: Option<String>,

    #[arg(long = "catalog", help = "A URL to the package catalog to use")]
    catalog: Option<String>,

    #[arg(
        short = 'w',
        long = "watch",
        help = "Watches file and compiles changes"
    )]
    watch: bool,

    #[arg(long = "verbose", help = "Display detailed error and warnings")]
    verbose: bool,

    #[arg(short = 'd', long = "dev", help = "Print the AST")]
    dev: bool,

    #[arg(long = "deny-read", help = "Deny read read privileges to 'assets'")]
    deny_read: bool,

    #[arg(long = "deny-write", help = "Deny read write privileges to 'assets'")]
    deny_write: bool,

    #[arg(long = "deny-create", help = "Deny read create privileges to 'assets'")]
    deny_create: bool,

    #[arg(
        long = "allow-every-module",
        short = 'A',
        help = "Allow every modules access the 'assets' directory"
    )]
    allow_every_module: bool,

    #[arg(
        long = "assets",
        help = "Specifies the relative path to the directory with external files"
    )]
    assets: Option<String>,
}

impl CompileArgs {
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
    fn use_html_preview(&self) -> Result<bool, CliError> {
        // If no output file was provided and the output format is "html" (or left unspecified)
        // we know that the user wants to use the live preview.
        if self.output.is_none() {
            return Ok(self.format.is_none()
                || self
                    .get_output_format()
                    .map(|format| format == OutputFormat::new("html"))
                    .unwrap_or(false));
        }

        // When using the --watch flag and the output format is html
        // we also start the live preview
        if self.watch {
            if let Ok(format) = self.get_output_format() {
                return Ok(format == OutputFormat::new("html"));
            }
        }

        // In all other cases, don't use start the live preview
        Ok(false)
    }
}

#[derive(Subcommand)]
enum Command {
    Compile(CompileArgs),
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
}

#[derive(Subcommand)]
enum CacheCommand {
    Clear,
    List,
    Location,
}

static DEFAULT_CATALOG: &str =
    "https://raw.githubusercontent.com/modmark-org/package-registry/main/package-registry.json";

static CTX: OnceCell<Mutex<Context<PackageManager, CliAccessManager>>> = OnceCell::new();
static RESOLVE_COMPLETE_RX: OnceCell<Mutex<Receiver<()>>> = OnceCell::new();
static PREVIEW_PORT: OnceCell<Option<Port>> = OnceCell::new();
static ABSOLUTE_OUTPUT_PATH: OnceCell<PathBuf> = OnceCell::new();
static CONNECTION_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
static MAX_COMPILATION_TRIES: usize = 3;

type PreviewConnections = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;
type PreviewDoc = Arc<Mutex<String>>;

type CompilationResult = Result<(String, CompilationState, Ast), Vec<CoreError>>;

/// Compile a file and return the transpiled content, compilation state and ast.
async fn compile_file(input_file: &Path, output_format: &OutputFormat) -> CompilationResult {
    let source = fs::read_to_string(input_file).map_err(|e| vec![e.into()])?;
    let ast = parse(&source).map_err(|e| vec![e.into()])?;

    for i in 1..=MAX_COMPILATION_TRIES {
        if let Some((output, state)) = eval(
            &source,
            &mut CTX.get().unwrap().lock().unwrap(),
            output_format,
        )? {
            return Ok((output, state, ast));
        }

        if i != MAX_COMPILATION_TRIES {
            spawn_blocking(|| {
                RESOLVE_COMPLETE_RX
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .blocking_recv();
            })
            .await
            .unwrap();
        }
    }

    Err(vec![])
}

fn print_compiling_message() -> Result<(), CliError> {
    let mut stdout = stdout();
    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.execute(style::PrintStyledContent("Compiling...".yellow()))?;

    Ok(())
}

fn print_result(result: &CompilationResult, args: &CompileArgs) -> Result<(), CliError> {
    let mut stdout = stdout();

    let (_, state, ast) = match result {
        Ok(result) => result,
        Err(errors) => {
            stdout.execute(terminal::Clear(terminal::ClearType::All))?;
            stdout.execute(cursor::MoveTo(0, 0))?;
            let num_errors = errors.len();
            if num_errors == 0 {
                stdout.execute(style::PrintStyledContent(
                    "No result retrieved from compiler\n".red(),
                ))?;
            } else if num_errors == 1 {
                let error = errors.first().unwrap();
                stdout.execute(style::PrintStyledContent(
                    format!("1 compilation error:\n{error}\n").red(),
                ))?;
            } else {
                stdout.execute(style::PrintStyledContent(
                    format!("{} compilation errors:\n", num_errors).red(),
                ))?;
                for error in errors {
                    stdout.execute(style::PrintStyledContent(format!("{error:?}\n").red()))?;
                }
            }
            return Ok(());
        }
    };

    stdout.execute(terminal::Clear(terminal::ClearType::All))?;
    stdout.execute(cursor::MoveTo(0, 0))?;
    stdout.execute(style::PrintStyledContent(
        "File successfully compiled!\n".green(),
    ))?;

    // Print the path to the live preview (if using one)
    if args.use_html_preview()? {
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
fn save_result(result: &CompilationResult, args: &CompileArgs) -> Result<(), CliError> {
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

    match &args.command {
        Command::Compile(compile_args) => match run_compile(compile_args).await {
            Ok(_) => (),
            Err(error) => {
                let mut stdout = stdout();
                stdout
                    .execute(style::PrintStyledContent(format!("{error}").red()))
                    .unwrap();
            }
        },
        Command::Cache { command } => match run_cache(command).await {
            Ok(_) => (),
            Err(error) => {
                let mut stdout = stdout();
                stdout
                    .execute(style::PrintStyledContent(format!("{error}").red()))
                    .unwrap();
            }
        },
    }
}

async fn run_compile(args: &CompileArgs) -> Result<(), CliError> {
    let current_path = env::current_dir()?;

    let catalog = args
        .catalog
        .as_deref()
        .unwrap_or(DEFAULT_CATALOG)
        .to_string();

    let (tx, rx) = channel::<()>(1);

    RESOLVE_COMPLETE_RX.set(Mutex::new(rx)).unwrap();
    CTX.set(Mutex::new({
        let mut context = Context::new(
            PackageManager {
                catalog,
                complete_tx: tx,
            },
            CliAccessManager::new(&args),
        )
        .map_err(|e| {
            eprintln!("Error creating Context: {e}");
            e
        })
        .unwrap();
        context.verbose = args.verbose;
        context
    }))
    .unwrap();

    // Using html output format and watch flag
    // (or if the user never provided a output file at all)
    if args.use_html_preview()? && get_port().is_ok() {
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
        let compilation_result = compile_file(&args.input, &args.get_output_format()?).await;
        save_result(&compilation_result, &args)?;
        print_result(&compilation_result, &args)?;
    } else {
        return Err(CliError::MissingOutputFile);
    }

    Ok(())
}

async fn run_cache(command: &CacheCommand) -> Result<(), CliError> {
    match command {
        CacheCommand::Clear => {
            for dir in fs::read_dir(cache_location()?)? {
                fs::remove_dir_all(dir?.path())?;
            }
        }
        CacheCommand::List => {
            let cache_path = cache_location()?;

            if cache_path.read_dir()?.next().is_none() {
                println!("No packages installed.");
                return Ok(());
            }

            println!("These are the currently installed packages:");

            let mut counter = 0;
            for file in WalkDir::new(cache_path)
                .into_iter()
                .filter_map(|file| file.ok())
            {
                if file.file_type().is_file() {
                    let Some(file_stem) = file.path().file_stem() else { return Err(CliError::Cache)};
                    counter += 1;
                    println!("  {}. \t{}", counter, file_stem.to_string_lossy())
                }
            }
        }
        CacheCommand::Location => println!(
            "Your packages can be found in: {}",
            cache_location()?.to_string_lossy()
        ),
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
    args: &CompileArgs,
    watch_dir: P,
) -> Result<(), CliError> {
    // Function to recompile the document:
    async fn compile(
        document: Option<&PreviewDoc>,
        connections: Option<&PreviewConnections>,
        args: &CompileArgs,
    ) -> Result<(), CliError> {
        print_compiling_message()?;

        let compilation_result = compile_file(
            &args.input,
            &args
                .get_output_format()
                .unwrap_or_else(|_| OutputFormat::new("html")),
        )
        .await;

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

    let (mut watcher, mut rx) = get_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(watch_dir.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                // If we also generate a output file, discard any changes from that file
                if let Some(output_path) = ABSOLUTE_OUTPUT_PATH.get() {
                    if event.paths.contains(output_path) {
                        continue;
                    }
                }
                // Take a quick nap to ensure that the file is closed before we start compiling
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                // We only care about changes from when files are created, removed or modified
                if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
                    compile(document.as_ref(), connections.as_ref(), args).await?;
                }
            }
            Err(e) => eprintln!("watch error: {:?}", e),
        }
    }

    Ok(())
}

/// Get the file watcher using a async api
fn get_watcher() -> notify::Result<(
    RecommendedWatcher,
    UnboundedReceiverStream<notify::Result<Event>>,
)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let rx = UnboundedReceiverStream::new(rx);

    let watcher = RecommendedWatcher::new(
        move |res| {
            tx.send(res).unwrap();
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}
