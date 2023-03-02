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
        loop {
            match self.lexer.next_token(self.depth)? {
                None if inside_defn => {
                    return Err(anyhow!(
                        "Syntax error: reached end of file with unfinished definition for '{}'.",
                        func_name
                    ));
                }
                None => return Ok(None),
                Some(Token::Push) => return self.consume_push(),
                // Block nested DEFNs
                Some(Token::Defn) if inside_defn => {
                    return Err(anyhow!("Syntax error: nested definitions are not allowed."))
                }
                Some(Token::Defn) => return self.consume_defn(),
                Some(Token::CallIf) => return self.consume_callif(),
                Some(Token::Exit) => return Ok(Some(Instruction::Exit)),
                Some(Token::Whitespace) => continue,
                Some(Token::RightCurlyBracket) if inside_defn => return Ok(None),
                Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
            }
        }
    }

    fn consume_push(&mut self) -> Result<Option<Instruction>, Error> {
        // Expect whitespace between PUSH and value
        match self.lexer.next_token(self.depth)? {
            Some(Token::Whitespace) => {}
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
        }

        // Increase the depth in case the whitespace was a newline
        self.depth += 1;

        // Get value
        let instruction = match self.next_non_whitespace_token()? {
            None => return Err(anyhow!("Syntax error: unexpected end of file")),
            Some(Token::Word(n)) => Instruction::PushData(n),
            Some(Token::FunctionName(f)) => Instruction::PushFunction(f),
            Some(Token::Caret) => Instruction::PushCopy,
            Some(Token::Asterisk) => Instruction::PushRandom,
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        };

        // Reset depth
        self.depth -= 1;

        // Expect end of file or whitespace between this instruction and the next
        match self.lexer.next_token(self.depth)? {
            None | Some(Token::Whitespace) => Ok(Some(instruction)),
            Some(t) => Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }
    }

    fn consume_defn(&mut self) -> Result<Option<Instruction>, Error> {
        // Expect whitespace between DEFN and function name
        match self.lexer.next_token(self.depth)? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(Token::Whitespace) => {}
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }

        // Increase depth by 1 in case the whitespace was a newline
        self.depth += 1;

        // Get function name
        let func_name = match self.next_non_whitespace_token()? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(Token::FunctionName(f)) => f,
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        };
        if func_name.starts_with("__") {
            return Err(anyhow!("Syntax error: cannot define function '{}' because the prefix __ is reserved for built-in functions.", func_name));
        }

        // Expect curly bracket (with optional whitespace before it)
        match self.next_non_whitespace_token()? {
            None => return Err(anyhow!("Syntax error: unexpected end of file.")),
            Some(Token::LeftCurlyBracket) => {}
            Some(t) => return Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }

        // Get body
        let body = self.consume_defn_body(&func_name)?;
        let instruction = Instruction::Define(func_name, body);

        // Reset depth
        self.depth -= 1;

        // Expect end of file or whitespace between this instruction and the next
        match self.lexer.next_token(self.depth)? {
            None | Some(Token::Whitespace) => Ok(Some(instruction)),
            Some(t) => Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }
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

    fn consume_callif(&mut self) -> Result<Option<Instruction>, Error> {
        // Expect end of file or whitespace between this instruction and the next
        match self.lexer.next_token(self.depth)? {
            None | Some(Token::Whitespace) => Ok(Some(Instruction::CallIf)),
            Some(t) => Err(anyhow!("Syntax error: unexpected token {:?}", t)),
        }
    }

    fn next_non_whitespace_token(&mut self) -> Result<Option<Token>, Error> {
        loop {
            match self.lexer.next_token(self.depth)? {
                Some(Token::Whitespace) => continue,
                x => return Ok(x),
            }
        }
    }
}
