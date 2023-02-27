use std::io;
use std::io::Write;

mod lexer;

fn main() {
    println!("Welcome to the plates REPL!");
    
    loop {
        print!("> ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .expect("Failed to read from stdin");

        match lexer::lex(&line) {
            Err(msg) => println!("{}", msg),
            Ok(tokens) => println!("{:#?}", tokens)
        }
    }
}
