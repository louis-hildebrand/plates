use anyhow::Error;
use clap::Parser;

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

fn main() -> Result<(), Error> {
    let args = CliArgs::parse();

    if args.files.len() == 0 {
        run_interactive(args)
    } else {
        run_from_files(args)
    }
}

fn run_interactive(args: CliArgs) -> Result<(), Error> {
    println!("Welcome to the plates REPL!");

    let reader = InteractiveReader::new();
    let mut parser = parser::Parser::new(reader);
    let mut runtime = Runtime::new();

    loop {
        let instruction = match parser.next_instruction() {
            Ok(None) => break,
            Ok(Some(x)) => x,
            Err(e) => return Err(e),
        };

        match runtime.run(instruction) {
            Err(msg) => println!("{}", msg),
            Ok(true) => break,
            Ok(false) => {}
        }

        if args.debug {
            runtime.show_stack();
        }
    }

    println!("Program completed successfully.");
    Ok(())
}

fn run_from_files(args: CliArgs) -> Result<(), Error> {
    let reader = match FileReader::new(args.files) {
        Err(e) => return Err(e),
        Ok(parser) => parser,
    };
    let mut parser = parser::Parser::new(reader);
    let mut runtime = Runtime::new();

    loop {
        let instruction = match parser.next_instruction() {
            Ok(None) => break,
            Ok(Some(x)) => x,
            Err(e) => return Err(e),
        };

        match runtime.run(instruction) {
            Err(msg) => println!("{}", msg),
            Ok(true) => break,
            Ok(false) => {}
        }

        if args.debug {
            runtime.show_stack();
        }
    }

    println!("Program completed successfully.");
    Ok(())
}
