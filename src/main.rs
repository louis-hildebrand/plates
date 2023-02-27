use crate::reader::read_instructions;

mod lexer;
mod reader;

fn main() {
    println!("Welcome to the plates REPL!");
    
    let mut lexer = read_instructions();
    loop {
        match lexer.next_token(1) {
            None => break,
            Some(t) => println!(" -- {:#?}", t)
        }
    }
}
