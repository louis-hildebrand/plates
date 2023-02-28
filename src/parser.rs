use anyhow::{anyhow, Error};

use crate::{
    lexer::{Lexer, Token},
    reader::Reader,
};

#[derive(Debug, Clone)]
pub enum Instruction {
    PushData(u32),
    PushFunction(String),
    PushCopy,
    PushRandom,
    Define(String, Vec<Instruction>),
    CallIf,
    Exit,
}

pub struct Parser<T>
where
    T: Reader,
{
    lexer: Lexer<T>,
    depth: usize,
}

impl<T> Parser<T>
where
    T: Reader,
{
    pub fn next_instruction(&mut self) -> Result<Option<Instruction>, Error> {
        match self.consume_instruction(false, "") {
            Err(e) => {
                // Reset the depth in case the error occurred in the middle
                // of a definition or something
                self.depth = 0;
                Err(e)
            }
            Ok(x) => Ok(x),
        }
    }
}

impl<T> Parser<T>
where
    T: Reader,
{
    pub fn new(reader: T) -> Self {
        let lexer = Lexer::new(reader);
        Parser { lexer, depth: 0 }
    }

    fn consume_instruction(
        &mut self,
        inside_defn: bool,
        func_name: &str,
    ) -> Result<Option<Instruction>, Error> {
        loop {
            match self.lexer.next_token(self.depth) {
                Ok(None) if inside_defn => {
                    return Err(anyhow!(
                        "Syntax error: reached end of file with unfinished definition for '{}'.",
                        func_name
                    ));
                }
                Ok(None) => return Ok(None),
                Ok(Some(Token::Push)) => return self.consume_push(),
                // TODO: Block nested DEFNs?
                Ok(Some(Token::Defn)) => return self.consume_defn(),
                Ok(Some(Token::CallIf)) => return Ok(Some(Instruction::CallIf)),
                Ok(Some(Token::Exit)) => return Ok(Some(Instruction::Exit)),
                Ok(Some(Token::Whitespace)) => continue,
                Ok(Some(Token::RightCurlyBracket)) if inside_defn => return Ok(None),
                Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
                Err(e) => return Err(e),
            }
        }
    }

    fn consume_push(&mut self) -> Result<Option<Instruction>, Error> {
        // Expect whitespace between PUSH and value
        match self.lexer.next_token(self.depth) {
            Ok(Some(Token::Whitespace)) => {}
            Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Ok(None) => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Err(e) => return Err(e),
        }

        // Increase the depth in case the whitespace was a newline
        self.depth += 1;

        // Get value
        let instruction = match self.next_non_whitespace_token() {
            Ok(None) => return Err(anyhow!("Syntax error: unexpected end of file")),
            Ok(Some(Token::Word(n))) => Instruction::PushData(n),
            Ok(Some(Token::FunctionName(f))) => Instruction::PushFunction(f),
            Ok(Some(Token::Caret)) => Instruction::PushCopy,
            Ok(Some(Token::Asterisk)) => Instruction::PushRandom,
            Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => return Err(e),
        };

        // Reset depth
        self.depth -= 1;

        // Expect end of file or space between this instruction and the next
        match self.lexer.next_token(self.depth) {
            Ok(None) | Ok(Some(Token::Whitespace)) => Ok(Some(instruction)),
            Ok(Some(t)) => Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => Err(e),
        }
    }

    fn consume_defn(&mut self) -> Result<Option<Instruction>, Error> {
        // Expect whitespace between DEFN and function name
        match self.lexer.next_token(self.depth) {
            Ok(None) => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Ok(Some(Token::Whitespace)) => {}
            Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => return Err(e),
        }

        // Increase depth by 1 in case the whitespace was a newline
        self.depth += 1;

        // Get function name
        let func_name = match self.next_non_whitespace_token() {
            Ok(None) => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Ok(Some(Token::FunctionName(f))) => f,
            Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => return Err(e),
        };
        if func_name.starts_with("__") {
            return Err(anyhow!("Syntax error: cannot define function '{}' because the prefix __ is reserved for built-in functions.", func_name));
        }

        // Expect curly bracket (with optional whitespace before it)
        match self.next_non_whitespace_token() {
            Ok(None) => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Ok(Some(Token::LeftCurlyBracket)) => {}
            Ok(Some(t)) => return Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => return Err(e),
        }

        // Get body
        let body = match self.consume_defn_body(&func_name) {
            Err(msg) => return Err(msg),
            Ok(instructions) => instructions,
        };
        let instruction = Instruction::Define(func_name, body);

        // Reset depth
        self.depth -= 1;

        // Expect end of file or space between this instruction and the next
        match self.lexer.next_token(self.depth) {
            Ok(None) | Ok(Some(Token::Whitespace)) => Ok(Some(instruction)),
            Ok(Some(t)) => Err(anyhow!("Syntax error: unexpected token {:#?}", t)),
            Err(e) => Err(e),
        }
    }

    fn consume_defn_body(&mut self, func_name: &str) -> Result<Vec<Instruction>, Error> {
        let mut body = Vec::new();
        loop {
            match self.consume_instruction(true, func_name) {
                Ok(None) => return Ok(body),
                Ok(Some(instruction)) => body.push(instruction),
                Err(msg) => return Err(msg),
            }
        }
    }

    fn next_non_whitespace_token(&mut self) -> Result<Option<Token>, Error> {
        loop {
            match self.lexer.next_token(self.depth) {
                Ok(Some(Token::Whitespace)) => continue,
                Ok(Some(t)) => return Ok(Some(t)),
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            }
        }
    }
}
