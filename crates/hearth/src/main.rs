mod slop;

use std::{env, process::ExitCode};

const HELP: &str = "Hearth: shared terminal tools for developers and agents\n\nUsage: hearth [--help | --version]\n\nThis is the workspace skeleton. The TUI and tool runtime are not implemented yet.";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        [arg] if arg == "--version" || arg == "-V" => {
            println!("hearth {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        // Hidden bonus feature; intentionally left out of --help.
        [arg] if arg == "--slop" => match slop::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("Unsupported arguments. Run hearth --help for available options.");
            ExitCode::from(2)
        }
    }
}
