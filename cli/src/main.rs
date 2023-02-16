mod error;
mod package;

use clap::Parser;
use core::{eval, Context, OutputFormat};
use crossterm::{
    cursor,
    style::{self, Stylize},
    terminal, ExecutableCommand,
};
use error::CliError;
use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher, WatcherKind};
use parser::{parse, Element};
use std::env;
use std::io::{stdout, Write};
use std::{fs, path::Path};
use std::{fs::File, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(index = 1, help = "Path to input file")]
    input: String,

    #[arg(index = 2, help = "Path to output file")]
    output: String,

    #[arg(
        short = 'f',
        long = "format",
        help = "The output format of the file",
        default_value = "html"
    )]
    format: String,

    #[arg(
        short = 'w',
        long = "watch",
        help = "Watches file and compiles changes"
    )]
    watch: bool,

    #[arg(short = 'd', long = "dev", help = "Prints the tree")]
    dev: bool,
}

fn print_tree(tree: parser::Element) {
    println!("\n{}", tree.tree_string(false));
}

fn compile_file(args: &Args) -> Result<Element, CliError> {
    let source = fs::read_to_string(&args.input)?;
    let mut ctx = Context::default();
    let output =
        eval(&source, &mut ctx, &OutputFormat::new(&args.format)).expect("Failed to evaluate file");

    let mut output_file = File::create(&args.output)?;
    output_file.write_all(output.as_bytes())?;

    // Also return the Element tree for debug purposes
    Ok(parse(&source).unwrap())
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

        let tree = match event {
            Ok(_) => compile_file(args),
            Err(e) => return Err(CliError::Notify(e)),
        };

        stdout.execute(terminal::Clear(terminal::ClearType::All))?;
        stdout.execute(cursor::MoveTo(0, 0))?;
        stdout.execute(style::PrintStyledContent(
            "File successfully compiled!\n\n".green(),
        ))?;

        let mut location = Path::new(&target).to_path_buf();
        location.push(&args.output);

        println!(
            "Your file can be found at {}\nSave your file to recompile changes.",
            location.display()
        );

        if args.dev {
            print_tree(tree?);
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
            Ok(tree) => {
                let mut stdout = stdout();

                stdout.execute(terminal::Clear(terminal::ClearType::All))?;
                stdout.execute(cursor::MoveTo(0, 0))?;
                stdout.execute(style::PrintStyledContent(
                    "File successfully compiled!\n\n".green(),
                ))?;

                let mut location = Path::new(&target).to_path_buf();
                location.push(&args.output);

                println!("Output file can be found at {}", location.display());

                if args.dev {
                    print_tree(tree);
                }
                stdout.flush()?;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
