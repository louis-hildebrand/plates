use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{anyhow, Error};

use crate::parser::Parser;

pub trait Reader {
    /// depth starts at zero and increases by one for each unfinished DEFN.
    fn next_line(&mut self, depth: usize) -> Option<String>;
}

pub struct InteractiveReader {}

impl InteractiveReader {
    pub fn read_instructions() -> Parser<Self> {
        let reader = InteractiveReader {};
        Parser::new(reader)
    }
}

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

pub struct FileReader {
    file_lines: Box<dyn Iterator<Item = String>>,
}

impl FileReader {
    pub fn read_instructions(files: Vec<PathBuf>) -> Result<Parser<Self>, Error> {
        let mut combined_file_contents = String::new();
        for file in files {
            let contents = match fs::read_to_string(file) {
                Err(e) => return Err(anyhow!(e).context("Failed to read file.")),
                Ok(s) => s,
            };
            combined_file_contents = combined_file_contents + "\n" + &contents;
        }

        // Eagerly convert each line into a String
        // TODO: make this lazy?
        let file_lines = combined_file_contents
            .lines()
            .map(|s| String::from(s))
            .collect::<Vec<_>>()
            .into_iter();

        let reader = FileReader {
            file_lines: Box::new(file_lines),
        };
        Ok(Parser::new(reader))
    }
}

impl Reader for FileReader {
    fn next_line(&mut self, _: usize) -> Option<String> {
        self.file_lines.next()
    }
}
