use std::io;
use std::io::Write;

use crate::lexer::Lexer;

pub trait Reader {
    /// depth starts at zero and increases by one for each unfinished DEFN.
    fn next_line(&mut self, depth: usize) -> Option<String>;
}

pub struct InteractiveReader {}

impl Reader for InteractiveReader {
    fn next_line(&mut self, depth: usize) -> Option<String> {
        print!("{} ", ">".repeat(depth + 1));
        io::stdout().flush().expect("Failed to flush stdout");

        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .expect("Failed to read from stdin");

        Some(line)
    }
}

pub fn read_instructions() -> Lexer<InteractiveReader> {
    let reader = InteractiveReader { };
    Lexer::new(reader)
}
