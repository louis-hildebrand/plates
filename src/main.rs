use crate::{reader::InteractiveReader, runtime::Runtime};

mod lexer;
mod parser;
mod reader;
mod runtime;

fn main() {
    println!("Welcome to the plates REPL!");

    let parser = InteractiveReader::read_instructions();
    let mut runtime = Runtime::new();

    for instruction in parser {
        match runtime.run(instruction) {
            Err(msg) => println!("{}", msg),
            Ok(true) => break,
            Ok(false) => {},
        }
    }

    println!("Program completed successfully.");
}
