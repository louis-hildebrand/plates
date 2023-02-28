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

fn main() {
    let args = CliArgs::parse();

    if args.files.len() == 0 {
        run_interactive(args);
    } else {
        run_from_files(args);
    }
}

fn run_interactive(args: CliArgs) {
    println!("Welcome to the plates REPL!");

    let parser = InteractiveReader::read_instructions();
    let mut runtime = Runtime::new();

    for instruction in parser {
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
}

fn run_from_files(args: CliArgs) {
    let parser = match FileReader::read_instructions(args.files) {
        Err(msg) => {
            println!("{}", msg);
            return;
        }
        Ok(parser) => parser,
    };
    let mut runtime = Runtime::new();

    for instruction in parser {
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
}
