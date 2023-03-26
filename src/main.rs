use anyhow::Error;
use clap::Parser;
use colored::Colorize;

use crate::{
    reader::{FileReader, InteractiveReader},
    runtime::Runtime,
};

mod lexer;
mod parser;
mod reader;
mod runtime;

#[derive(clap::Parser)]
struct CliArgs {
    /// Files to run. If none are provided, the REPL will be launched instead.
    files: Vec<std::path::PathBuf>,

    /// Print debug info (e.g., the state of the stack) after each instruction
    #[clap(short, long, action)]
    debug: bool,
}

fn main() {
    let args = CliArgs::parse();

    if args.files.is_empty() {
        run_interactive(args);
    } else {
        run_from_files(args);
    }
}

fn run_interactive(args: CliArgs) {
    print_info("Welcome to the plates REPL!");

    let reader = InteractiveReader::new();
    let mut parser = parser::Parser::new(reader);
    let mut runtime = Runtime::new();

    loop {
        match parser.next_instruction() {
            Ok(None) => break,
            Ok(Some(instruction)) => match runtime.run(instruction) {
                Err(e) => print_error(&e),
                Ok(true) => break,
                Ok(false) => {}
            },
            Err(e) => {
                print_error(&e);
                parser.clear();
            }
        };

        // Only show stack once per line
        if args.debug && parser.full_line_consumed() {
            print_debug(&runtime.stack_to_string());
        }
    }

    print_info("Program completed successfully.");
}

fn run_from_files(args: CliArgs) {
    let reader = match FileReader::new(args.files) {
        Err(e) => {
            print_error(&e);
            return;
        }
        Ok(r) => r,
    };
    let mut parser = parser::Parser::new(reader);
    let mut runtime = Runtime::new();

    loop {
        let instruction = match parser.next_instruction() {
            Err(e) => {
                print_error(&e);
                return;
            }
            Ok(None) => break,
            Ok(Some(x)) => x,
        };

        let should_exit = match runtime.run(instruction) {
            Err(e) => {
                print_error(&e);
                return;
            }
            Ok(x) => x,
        };
        if should_exit {
            break;
        }

        if args.debug {
            print_debug(&runtime.stack_to_string());
        }
    }

    print_info("Program completed successfully.");
}

fn print_error(e: &Error) {
    let mut msg = format!("{e}");
    for cause in e.chain().skip(1) {
        msg += &format!("\n\nCaused by:\n    {cause}");
    }

    eprintln!("{}", msg.bold().red());
}

fn print_info(msg: &str) {
    println!("{}", msg.bold());
}

fn print_debug(msg: &str) {
    println!("{}", msg.italic().truecolor(128, 128, 128));
}
