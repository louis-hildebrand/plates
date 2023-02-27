use crate::{lexer::{Token, Lexer}, reader::Reader};

#[derive(Debug)]
#[derive(Clone)]
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
where T: Reader
{
    lexer: Lexer<T>,
    depth: usize,
}

impl<T> Iterator for Parser<T>
where T: Reader
{
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.consume_instruction(false, "") {
                Err(msg) => {
                    // TODO: Terminate the program here? Seems like it should
                    // depend on the context (interactive session vs file)
                    println!("{}", msg);
                    // Reset the depth in case the error occurred in the middle
                    // of a definition or something
                    self.depth = 0;
                    continue;
                },
                Ok(None) => return None,
                Ok(instruction) => return instruction,
            }
        }
    }
}

impl<T> Parser<T>
where T: Reader
{
    pub fn new(reader: T) -> Self {
        let lexer = Lexer::new(reader);
        Parser { lexer, depth: 0 }
    }

    fn consume_instruction(&mut self, inside_defn: bool, func_name: &str) -> Result<Option<Instruction>, String> {
        loop {
            match self.lexer.next_token(self.depth) {
                None if !inside_defn => return Ok(None),
                None if inside_defn => {
                    return Err(
                        format!("Syntax error: reached end of file with unfinished definition for '{}'.", func_name)
                    );
                },
                Some(Token::Push) => return self.consume_push(),
                Some(Token::Defn) => return self.consume_defn(),
                Some(Token::CallIf) => return Ok(Some(Instruction::CallIf)),
                Some(Token::Exit) => return Ok(Some(Instruction::Exit)),
                Some(Token::Whitespace) => continue,
                Some(Token::RightCurlyBracket) if inside_defn => return Ok(None),
                t => return Err(format!("Syntax error: unexpected token {:#?}", t))
            }
        }
    }

    fn consume_push(&mut self) -> Result<Option<Instruction>, String> {
        // Expect whitespace between PUSH and value
        match self.lexer.next_token(self.depth) {
            Some(Token::Whitespace) => {},
            Some(t) => return Err(format!("Syntax error: unexpected token {:#?}", t)),
            None => return Err(String::from("Syntax error: unexpected end of file.")),
        }

        // Increase the depth in case the whitespace was a newline
        self.depth += 1;

        // Get value
        let instruction = match self.next_non_whitespace_token() {
            None => return Err(String::from("Syntax error: unexpected end of file")),
            Some(Token::Word(n)) => Instruction::PushData(n),
            Some(Token::FunctionName(f)) => Instruction::PushFunction(f),
            Some(Token::Caret) => Instruction::PushCopy,
            Some(Token::Asterisk) => Instruction::PushRandom,
            Some(t) => return Err(format!("Syntax error: unexpected token {:#?}", t)),
        };

        // Reset depth
        self.depth -= 1;

        // Expect end of file or space between this instruction and the next
        match self.lexer.next_token(self.depth) {
            None | Some(Token::Whitespace) => Ok(Some(instruction)),
            Some(t) => Err(format!("Syntax error: unexpected token {:#?}", t)),
        }
    }

    fn consume_defn(&mut self) -> Result<Option<Instruction>, String> {
        // Expect whitespace between DEFN and function name
        match self.lexer.next_token(self.depth) {
            None => return Err(String::from("Syntax error: unexpected end of file.")),
            Some(Token::Whitespace) => {},
            Some(t) => return Err(format!("Syntax error: unexpected token {:#?}", t)),
        }

        // Increase depth by 1 in case the whitespace was a newline
        self.depth += 1;

        // Get function name
        let func_name = match self.next_non_whitespace_token() {
            None => return Err(String::from("Syntax error: unexpected end of file.")),
            Some(Token::FunctionName(f)) => f,
            Some(t) => return Err(format!("Syntax error: unexpected token {:#?}", t)),
        };
        if func_name.starts_with("__") {
            return Err(format!("Syntax error: cannot define function '{}' because the prefix __ is reserved for built-in functions.", func_name));
        }

        // Expect curly bracket (with optional whitespace before it)
        match self.next_non_whitespace_token() {
            None => return Err(String::from("Syntax error: unexpected end of file.")),
            Some(Token::LeftCurlyBracket) => {},
            Some(t) => return Err(format!("Syntax error: unexpected token {:#?}", t)),
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
            None | Some(Token::Whitespace) => Ok(Some(instruction)),
            Some(t) => Err(format!("Syntax error: unexpected token {:#?}", t)),
        }
    }

    fn consume_defn_body(&mut self, func_name: &str) -> Result<Vec<Instruction>, String> {
        let mut body = Vec::new();
        loop {
            match self.consume_instruction(true, func_name) {
                Ok(None) => return Ok(body),
                Ok(Some(instruction)) => body.push(instruction),
                Err(msg) => return Err(msg),
            }
        }
    }

    fn next_non_whitespace_token(&mut self) -> Option<Token> {
        loop {
            match self.lexer.next_token(self.depth) {
                Some(Token::Whitespace) => continue,
                Some(t) => return Some(t),
                None => return None,
            }
        }
    }
}
