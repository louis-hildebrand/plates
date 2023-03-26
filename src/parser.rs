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
    PushArg(usize),
    Define(String, u32, Vec<Instruction>),
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
    pub fn new(reader: T) -> Self {
        let lexer = Lexer::new(reader);
        Parser { lexer, depth: 0 }
    }

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

    /// Clears the underlying lexer.
    pub fn clear(&mut self) {
        self.lexer.clear();
    }

    pub fn full_line_consumed(&mut self) -> bool {
        self.lexer.full_line_consumed()
    }

    fn consume_instruction(
        &mut self,
        inside_defn: bool,
        func_name: &str,
    ) -> Result<Option<Instruction>, Error> {
        match self.lexer.next_token(self.depth)? {
            None if inside_defn => Err(anyhow!(
                "Syntax error: reached end of file with unfinished definition for '{}'.",
                func_name
            )),
            None => Ok(None),
            Some(Token::Push) => self.consume_push(inside_defn),
            // Block nested DEFNs
            Some(Token::Defn) if inside_defn => {
                Err(anyhow!("Syntax error: nested definitions are not allowed."))
            }
            Some(Token::Defn) => self.consume_defn(),
            Some(Token::CallIf) => Ok(Some(Instruction::CallIf)),
            Some(Token::Exit) => Ok(Some(Instruction::Exit)),
            Some(Token::RightCurlyBracket) if inside_defn => Ok(None),
            Some(t) => Err(anyhow!("Syntax error: unexpected token {:?}.", t)),
        }
    }

    fn consume_push(&mut self, inside_defn: bool) -> Result<Option<Instruction>, Error> {
        // Increase the depth in case there was a newline between PUSH and the word
        self.depth += 1;

        let instruction = match self.lexer.next_token(self.depth)? {
            None => return Err(anyhow!("Syntax error: unexpected end of file")),
            Some(Token::Word(n)) => Instruction::PushData(n),
            Some(Token::FunctionName(f)) => Instruction::PushFunction(f),
            Some(Token::Caret) => Instruction::PushCopy,
            Some(Token::Asterisk) => Instruction::PushRandom,
            // Arguments are only allowed inside functions
            Some(Token::Argument(_)) if !inside_defn => {
                return Err(anyhow!(
                    "Syntax error: cannot use arguments outside functions."
                ))
            }
            Some(Token::Argument(n)) => Instruction::PushArg(n),
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        };

        self.depth -= 1;

        Ok(Some(instruction))
    }

    fn consume_defn(&mut self) -> Result<Option<Instruction>, Error> {
        // Increase depth in case there was a newline between DEFN and the function name
        self.depth += 1;

        // Get function name
        let func_name = match self.lexer.next_token(self.depth)? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(Token::FunctionName(f)) => f,
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        };
        if func_name.starts_with("__") {
            return Err(anyhow!("Syntax error: cannot define function '{}' because the prefix __ is reserved for built-in functions.", func_name));
        }

        // Get argument count
        self.expect(Token::LeftParen)?;
        let arg_count = match self.lexer.next_token(self.depth)? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(Token::Word(n)) => n,
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        };
        self.expect(Token::RightParen)?;

        self.expect(Token::LeftCurlyBracket)?;

        // Get body
        let body = self.consume_defn_body(&func_name)?;
        let instruction = Instruction::Define(func_name, arg_count, body);

        // Reset depth
        self.depth -= 1;

        Ok(Some(instruction))
    }

    fn consume_defn_body(&mut self, func_name: &str) -> Result<Vec<Instruction>, Error> {
        let mut body = Vec::new();
        loop {
            match self.consume_instruction(true, func_name)? {
                None => return Ok(body),
                Some(instruction) => body.push(instruction),
            }
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), Error> {
        match self.lexer.next_token(self.depth)? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(t) if t == token => Ok(()),
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }
    }
}
