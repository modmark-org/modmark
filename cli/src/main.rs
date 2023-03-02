mod error;
mod package;

use clap::Parser;
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal, ExecutableCommand,
};
use error::CliError;
use modmark_core::{context::CompilationState, OutputFormat};
use modmark_core::{eval, Context};
use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use once_cell::sync::Lazy;
use std::io::{stdout, Write};
use std::{env, fs, fs::File, path::Path, path::PathBuf, sync::Mutex, time::Duration};

use parser::{parse, Ast};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(index = 1, help = "Path to input file")]
    input: PathBuf,

    #[arg(index = 2, help = "Path to output file")]
    output: PathBuf,

    #[arg(short = 'f', long = "format", help = "The output format of the file")]
    format: Option<String>,

    #[arg(
        short = 'w',
        long = "watch",
        help = "Watches file and compiles changes"
    )]
    watch: bool,

    #[arg(short = 'd', long = "dev", help = "Prints the tree")]
    dev: bool,
}

static CTX: Lazy<Mutex<Context>> = Lazy::new(|| Mutex::new(Context::default()));

// Infer the output format based on the file extension of the output format
fn infer_output_format(output: &Path) -> Option<OutputFormat> {
    output.extension().and_then(|ext| match ext.to_str() {
        Some("tex") => Some(OutputFormat::new("latex")),
        Some("html") => Some(OutputFormat::new("html")),
        Some("htm") => Some(OutputFormat::new("html")),
        _ => None,
    })
}

fn compile_file(args: &Args) -> Result<(Ast, CompilationState), CliError> {
    let source = fs::read_to_string(&args.input)?;

    let Some(format) = args.format
        .as_ref()
        .map(|s| OutputFormat::new(s))
        .or_else(|| infer_output_format(&args.output)) else {
        return Err(CliError::UnknownOutputFormat);
    };

    let (output, state) = eval(&source, &mut CTX.lock().unwrap(), &format)?;

    let mut output_file = File::create(&args.output)?;
    output_file.write_all(output.as_bytes())?;

    // Also return the Element tree for debug purposes
    Ok((parse(&source)?, state))
}

fn print_tree(tree: &Ast) {
    println!("{}", tree.tree_string());
}

fn watch(args: &Args, target: &String) -> Result<(), CliError> {
    fn watch_compile(
        event: notify::Result<Event>,
        args: &Args,
        target: &String,
    ) -> Result<(), CliError> {
        let mut stdout = stdout();

        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        stdout.execute(cursor::MoveTo(0, 0))?;
        stdout.execute(style::PrintStyledContent("Recompiling...".yellow()))?;

        let (tree, state) = match event {
            Ok(_) => compile_file(args),
            Err(e) => return Err(CliError::Notify(e)),
        }?;

        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        stdout.execute(cursor::MoveTo(0, 0))?;
        stdout.execute(style::PrintStyledContent(
            "File successfully compiled!\n\n".green(),
        ))?;

        if !state.warnings.is_empty() {
            stdout.execute(style::PrintStyledContent("Warnings:\n".yellow()))?;
            for warning in state.warnings {
                stdout.execute(style::PrintStyledContent(format!("{warning}\n").yellow()))?;
            }
        }

        if !state.errors.is_empty() {
            stdout.execute(style::PrintStyledContent("Errors:\n".red()))?;
            for error in state.errors {
                stdout.execute(style::PrintStyledContent(format!("{error}\n").red()))?;
            }
        }

        let mut location = Path::new(&target).to_path_buf();
        location.push(&args.output);

        println!(
            "Your file can be found at {}\nSave your file to recompile changes.",
            location.display()
        );

        if args.dev {
            print_tree(&tree);
        }

        stdout.flush()?;

        Ok(())
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        let config = Config::default().with_poll_interval(Duration::from_secs(10));
        Box::new(PollWatcher::new(tx, config).unwrap())
    } else {
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };

    watcher.watch(Path::new(&args.input), RecursiveMode::Recursive)?;

    watch_compile(Ok(Event::default()), args, target)?;

    for event in rx {
        watch_compile(event, args, target)?;
    }

    Ok(())
}

fn main() -> Result<(), CliError> {
    let args = Args::parse();
    let current_path = env::current_dir()?;
    let target = current_path.into_os_string().into_string().unwrap();

    if args.watch {
        watch(&args, &target)?;
    } else {
        match compile_file(&args) {
            Ok((tree, state)) => {
                let mut stdout = stdout();

                stdout.execute(terminal::Clear(terminal::ClearType::All))?;
                stdout.execute(cursor::MoveTo(0, 0))?;
                stdout.execute(style::PrintStyledContent(
                    "File successfully compiled!\n\n".green(),
                ))?;

                if !state.warnings.is_empty() {
                    stdout.execute(style::PrintStyledContent("Warnings:\n".yellow()))?;
                    for warning in state.warnings {
                        stdout.execute(style::PrintStyledContent(format!("{warning}").yellow()))?;
                    }
                }

                if !state.errors.is_empty() {
                    stdout.execute(style::PrintStyledContent("Errors:\n".red()))?;
                    for error in state.errors {
                        stdout.execute(style::PrintStyledContent(format!("{error}").red()))?;
                    }
                }

                let mut location = Path::new(&target).to_path_buf();
                location.push(&args.output);

                println!("Output file can be found at {}", location.display());

                if args.dev {
                    print_tree(&tree);
                }
                stdout.flush()?;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
