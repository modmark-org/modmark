use clap::Parser;
use core::eval;
use notify::{
    Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Result, Watcher, WatcherKind,
};
use parser::{parse, Element};
use std::env;
use std::{fs, path::Path};
use std::{fs::File, io::Write, time::Duration};
use termion::{color, style};

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(index = 1, help = "Path to input file")]
    input: String,

    #[arg(index = 2, help = "Path to output file")]
    output: String,

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

fn compile_file(args: &Args) -> Result<Element> {
    let source = fs::read_to_string(&args.input)?;
    let document = parse(&source);
    let output = eval(&document);

    let mut output_file = File::create(&args.output)?;
    output_file.write_all(output.as_bytes())?;
    Ok(document)
}

fn watch(args: &Args, target: &String) -> Result<()> {
    fn watch_compile(event: Result<Event>, args: &Args, target: &String) -> Result<()> {
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
        println!(
            "{}Recompiling...{}\n",
            color::Fg(color::Yellow),
            style::Reset
        );

        let tree = match event {
            Ok(_) => match compile_file(args) {
                Ok(t) => t,
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e),
        };

        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
        println!(
            "{}File successfully compiled!{}\n",
            color::Fg(color::Green),
            style::Reset
        );

        println!(
            "Your file can be found at {}/{}\nSave your file to recompile changes.",
            target, &args.output
        );

        if args.dev {
            print_tree(tree);
        }

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

fn main() -> Result<()> {
    let args = Args::parse();
    let current_path = env::current_dir()?;
    let target = current_path.into_os_string().into_string().unwrap();

    if args.watch {
        watch(&args, &target)?;
    } else {
        match compile_file(&args) {
            Ok(tree) => {
                print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
                println!("{}File successfully compiled!\n", color::Fg(color::Green));

                println!(
                    "{}Output file can be found at {}/{}",
                    color::Fg(color::Blue),
                    target,
                    &args.output
                );

                if args.dev {
                    print_tree(tree);
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
