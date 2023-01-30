use clap::Parser;
use core::eval;
use notify::{
    Config, PollWatcher, RecommendedWatcher, RecursiveMode, Result, Watcher, WatcherKind,
};
use parser::parse;
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
    w: bool,
}

fn compile_file(args: &Args) -> Result<()> {
    let source = fs::read_to_string(&args.input).expect("[ERROR] Failed to read file...");
    let document = parse(&source);
    let output = eval(&document);

    let mut output_file = File::create(&args.output)?;
    output_file.write_all(&output.as_bytes())?;
    Ok(())
}

fn watch(args: &Args, target: &String) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: Box<dyn Watcher> = if RecommendedWatcher::kind() == WatcherKind::PollWatcher {
        let config = Config::default().with_poll_interval(Duration::from_secs(10));
        Box::new(PollWatcher::new(tx, config).unwrap())
    } else {
        Box::new(RecommendedWatcher::new(tx, Config::default()).unwrap())
    };

    watcher.watch(Path::new(&args.input), RecursiveMode::Recursive)?;

    for event in rx {
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
        println!(
            "{}Recompiling...{}\n",
            color::Fg(color::Yellow),
            style::Reset
        );
        match event {
            Ok(_) => match compile_file(args) {
                Ok(_) => {}
                Err(_) => println!("[ERROR] File could not compile..."),
            },
            Err(_) => println!("[ERROR] Input file removed..."),
        }
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
        println!(
            "{}File successfully compiled!{}\n",
            color::Fg(color::Green),
            style::Reset
        );

        println!(
            "Your file can be found at {}/{}\n
            Save your file to recompile changes.",
            target, &args.output
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let current_path = env::current_dir()?;
    let target = current_path.into_os_string().into_string().unwrap();

    if args.w {
        match watch(&args, &target) {
            Ok(_) => (),
            Err(_) => println!("[ERROR] Watching file failed..."),
        };
    } else {
        match compile_file(&args) {
            Ok(_) => (),
            Err(_) => println!("[ERROR] File could not compile..."),
        }
        print!("{}{}", termion::clear::All, termion::cursor::Goto(1, 1));
        println!("{}File successfully compiled!\n", color::Fg(color::Green));

        println!(
            "{}Output file can be found at {}/{}",
            color::Fg(color::Blue),
            target,
            &args.output
        );
    }

    Ok(())
}
