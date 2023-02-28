use clap::Parser;

use crate::{reader::InteractiveReader, runtime::Runtime};

mod lexer;
mod parser;
mod reader;
mod runtime;

#[derive(clap::Parser)]
struct Args {
    /// Print debug info (e.g., the state of the stack) after each instruction
    #[clap(short, long, action)]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    println!("Welcome to the plates REPL!");

    let parser = InteractiveReader::read_instructions();
    let mut runtime = Runtime::new();

    for instruction in parser {
        match runtime.run(instruction) {
            Err(msg) => println!("{}", msg),
            Ok(true) => break,
            Ok(false) => {},
        }

        if args.debug {
            runtime.show_stack();
        }
    }

    println!("Program completed successfully.");
}
