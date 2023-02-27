use crate::reader::InteractiveReader;

mod lexer;
mod parser;
mod reader;

fn main() {
    println!("Welcome to the plates REPL!");
    
    let parser = InteractiveReader::read_instructions();
    for instruction in parser {
        println!("{:#?}", instruction);
    }
}
