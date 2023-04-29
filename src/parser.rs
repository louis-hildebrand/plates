use anyhow::{anyhow, Error};

use crate::lexer::{Token, TokenStream};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Instruction {
    PushData(u32),
    PushFunction(String),
    PushRandom,
    PushArg(usize),
    Define(String, u32, Vec<Instruction>),
    CallIf,
    Exit,
}

pub struct Parser<T>
where
    T: TokenStream,
{
    token_stream: T,
    depth: usize,
}

impl<T> Parser<T>
where
    T: TokenStream,
{
    pub fn new(token_stream: T) -> Self {
        Parser {
            token_stream,
            depth: 0,
        }
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
    pub fn clear_line(&mut self) {
        self.token_stream.clear_line();
    }

    pub fn full_line_consumed(&mut self) -> bool {
        self.token_stream.full_line_consumed()
    }

    fn consume_instruction(
        &mut self,
        inside_defn: bool,
        func_name: &str,
    ) -> Result<Option<Instruction>, Error> {
        match self.token_stream.next_token(self.depth)? {
            None if inside_defn => Err(anyhow!(
                "Syntax error: Unexpected end of file in body of function '{func_name}'."
            )),
            None => Ok(None),
            Some(Token::Push) => self.consume_push(inside_defn),
            // Block nested DEFNs
            Some(Token::Defn) if inside_defn => {
                Err(anyhow!("Syntax error: Nested definitions are not allowed."))
            }
            Some(Token::Defn) => self.consume_defn(),
            Some(Token::CallIf) => Ok(Some(Instruction::CallIf)),
            Some(Token::Exit) => Ok(Some(Instruction::Exit)),
            Some(Token::RightCurlyBracket) if inside_defn => Ok(None),
            Some(t) => Err(anyhow!("Syntax error: Unexpected token {:?}.", t)),
        }
    }

    fn consume_push(&mut self, inside_defn: bool) -> Result<Option<Instruction>, Error> {
        // Increase the depth in case there was a newline between PUSH and the word
        self.depth += 1;

        let instruction = match self.token_stream.next_token(self.depth)? {
            None => {
                return Err(anyhow!(
                    "Syntax error: Unexpected end of file after token {:?}.",
                    Token::Push
                ))
            }
            Some(Token::Word(n)) => Instruction::PushData(n),
            Some(Token::FunctionName(f)) => Instruction::PushFunction(f),
            Some(Token::Asterisk) => Instruction::PushRandom,
            // Arguments are only allowed inside functions
            Some(Token::Argument(_)) if !inside_defn => {
                return Err(anyhow!(
                    "Syntax error: Cannot use arguments outside functions."
                ))
            }
            Some(Token::Argument(n)) => Instruction::PushArg(n),
            Some(t) => return Err(anyhow!("Syntax error: Unexpected token {:?}.", t)),
        };

        self.depth -= 1;

        Ok(Some(instruction))
    }

    fn consume_defn(&mut self) -> Result<Option<Instruction>, Error> {
        // Increase depth in case there was a newline between DEFN and the function name
        self.depth += 1;

        // Get function name
        let func_name = match self.token_stream.next_token(self.depth)? {
            None => {
                return Err(anyhow!(
                    "Syntax error: Unexpected end of file after token {:?}.",
                    Token::Defn
                ))
            }
            Some(Token::FunctionName(f)) => f,
            Some(t) => return Err(anyhow!("Syntax error: Unexpected token {:?}.", t)),
        };
        if func_name.starts_with("__") {
            return Err(anyhow!("Syntax error: Cannot define function '{}' because the prefix '__' is reserved for built-in functions.", func_name));
        }

        // Get argument count
        self.expect(
            Token::LeftParen,
            format!("Syntax error: Unexpected end of file in signature of function '{func_name}'."),
        )?;
        let arg_count = match self.token_stream.next_token(self.depth)? {
            None => {
                return Err(anyhow!(
                    "Syntax error: Unexpected end of file in signature of function '{func_name}'."
                ))
            }
            Some(Token::Word(n)) => n,
            Some(t) => return Err(anyhow!("Syntax error: Unexpected token {:?}.", t)),
        };
        self.expect(
            Token::RightParen,
            format!("Syntax error: Unexpected end of file in signature of function '{func_name}'."),
        )?;

        self.expect(
            Token::LeftCurlyBracket,
            format!("Syntax error: Unexpected end of file in signature of function '{func_name}'."),
        )?;

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

    fn expect(&mut self, token: Token, eof_msg: String) -> Result<(), Error> {
        match self.token_stream.next_token(self.depth)? {
            None => Err(anyhow!(eof_msg)),
            Some(t) if t == token => Ok(()),
            Some(t) => Err(anyhow!("Syntax error: Unexpected token {:?}.", t)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        lexer::Token,
        parser::{Instruction, Parser},
    };

    macro_rules! assert_ok_and_eq {
        ( $actual:expr, $expected:expr ) => {
            let actual_val = $actual;
            assert!(actual_val.is_ok());
            let actual_val = actual_val.unwrap();
            assert_eq!($expected, actual_val);
        };
    }

    macro_rules! assert_err_with_msg {
        ( $value:expr, $msg:expr ) => {
            match $value {
                Ok(x) => panic!("Expected an error but received 'Ok({x:?})'."),
                Err(e) => assert_eq!($msg, format!("{e}")),
            };
        };
    }

    macro_rules! test_parse_success {
        ( $( $name:ident: ($tokens:expr, $instruction:expr) ),* $(,)? ) => {
            $(
                #[test]
                fn $name() {
                    let mut parser = Parser::new($tokens.into_iter());
                    assert_ok_and_eq!(parser.next_instruction(), Some($instruction));
                    assert_ok_and_eq!(parser.next_instruction(), None);
                }
            )*
        };
    }

    macro_rules! test_parse_failure {
        ( $( $name:ident: ($tokens:expr, $msg:expr) ),* $(,)? ) => {
            $(
                #[test]
                fn $name() {
                    let mut parser = Parser::new($tokens.into_iter());
                    assert_err_with_msg!(parser.next_instruction(), $msg);
                }
            )*
        };
    }

    test_parse_success![
        push_data: (vec![Token::Push, Token::Word(123)], Instruction::PushData(123)),
        push_function: (
            vec![Token::Push, Token::FunctionName("foo".to_owned())],
            Instruction::PushFunction("foo".to_owned())
        ),
        push_random: (vec![Token::Push, Token::Asterisk], Instruction::PushRandom),
        define_empty: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
                Token::LeftCurlyBracket,
                Token::RightCurlyBracket,
            ],
            Instruction::Define("foo".to_owned(), 0, vec![])
        ),
        define_with_args: (
            vec![
                Token::Defn,
                Token::FunctionName("swap".to_owned()),
                Token::LeftParen,
                Token::Word(2),
                Token::RightParen,
                Token::LeftCurlyBracket,
                Token::Push,
                Token::Argument(0),
                Token::Push,
                Token::Argument(1),
                Token::RightCurlyBracket,
            ],
            Instruction::Define("swap".to_owned(), 2, vec![Instruction::PushArg(0), Instruction::PushArg(1)])
        ),
        callif: (vec![Token::CallIf], Instruction::CallIf),
        exit: (vec![Token::Exit], Instruction::Exit),
    ];

    test_parse_failure![
        nested_define: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
                Token::LeftCurlyBracket,
                    Token::Defn,
                    Token::FunctionName("bar".to_owned()),
                    Token::LeftParen,
                    Token::Word(0),
                    Token::RightParen,
                    Token::LeftCurlyBracket,
                    Token::RightCurlyBracket,
                Token::RightCurlyBracket,
            ],
            "Syntax error: Nested definitions are not allowed."
        ),
        unexpected_token: (
            vec![Token::LeftCurlyBracket],
            "Syntax error: Unexpected token LeftCurlyBracket."
        ),
        unexpected_token_after_push: (
            vec![Token::Push, Token::RightParen],
            "Syntax error: Unexpected token RightParen."
        ),
        unexpected_token_after_define0: (
            vec![
                Token::Defn,
                Token::Word(42),
            ],
            "Syntax error: Unexpected token Word(42)."
        ),
        unexpected_token_after_define1: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::Asterisk,
            ],
            "Syntax error: Unexpected token Asterisk."
        ),
        unexpected_token_after_define2: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Push,
            ],
            "Syntax error: Unexpected token Push."
        ),
        unexpected_token_after_define3: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightCurlyBracket,
            ],
            "Syntax error: Unexpected token RightCurlyBracket."
        ),
        unexpected_token_after_define4: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
                Token::LeftParen
            ],
            "Syntax error: Unexpected token LeftParen."
        ),
        args_outside_function: (
            vec![Token::Push, Token::Argument(0)],
            "Syntax error: Cannot use arguments outside functions."
        ),
        reserved_function_name: (
            vec![
                Token::Defn,
                Token::FunctionName("__empty".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
                Token::LeftCurlyBracket,
                Token::RightCurlyBracket,
            ],
            "Syntax error: Cannot define function '__empty' because the prefix '__' is reserved for built-in functions."
        ),
        unexpected_eof_in_push: (
            vec![Token::Push],
            "Syntax error: Unexpected end of file after token Push."
        ),
        unexpected_eof_in_define0: (
            vec![
                Token::Defn,
            ],
            "Syntax error: Unexpected end of file after token Defn."
        ),
        unexpected_eof_in_define1: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
            ],
            "Syntax error: Unexpected end of file in signature of function 'foo'."
        ),
        unexpected_eof_in_define2: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
            ],
            "Syntax error: Unexpected end of file in signature of function 'foo'."
        ),
        unexpected_eof_in_define3: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
            ],
            "Syntax error: Unexpected end of file in signature of function 'foo'."
        ),
        unexpected_eof_in_define4: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
            ],
            "Syntax error: Unexpected end of file in signature of function 'foo'."
        ),
        unexpected_eof_in_define5: (
            vec![
                Token::Defn,
                Token::FunctionName("foo".to_owned()),
                Token::LeftParen,
                Token::Word(0),
                Token::RightParen,
                Token::LeftCurlyBracket,
            ],
            "Syntax error: Unexpected end of file in body of function 'foo'."
        ),
    ];
}
