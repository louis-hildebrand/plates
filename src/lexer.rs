use std::collections::VecDeque;

use anyhow::{anyhow, Error};

use crate::reader::Reader;

#[derive(Debug)]
pub enum Token {
    Push,
    Defn,
    CallIf,
    Exit,
    Caret,
    Asterisk,
    LeftCurlyBracket,
    RightCurlyBracket,
    FunctionName(String),
    Word(u32),
    Whitespace,
}

pub struct Lexer<T>
where
    T: Reader,
{
    tokens: VecDeque<Token>,
    reader: T,
}

impl<T> Lexer<T>
where
    T: Reader,
{
    pub fn new(reader: T) -> Self
    where
        T: Reader,
    {
        Lexer {
            tokens: VecDeque::new(),
            reader,
        }
    }

    pub fn next_token(&mut self, depth: usize) -> Result<Option<Token>, Error> {
        loop {
            match self.tokens.pop_front() {
                Some(t) => return Ok(Some(t)),
                None => {}
            }

            match self.refill_tokens(depth) {
                Err(e) => return Err(e),
                Ok(false) => return Ok(None),
                Ok(true) => continue,
            }
        }
    }

    /// Gets a new line, lexes it, and adds the tokens to self.tokens. If the
    /// reader has no more lines, returns false. Otherwise, returns true.
    fn refill_tokens(&mut self, depth: usize) -> Result<bool, Error> {
        loop {
            let line = match self.reader.next_line(depth) {
                None => return Ok(false),
                Some(x) => x,
            };

            match lex_line(&line) {
                Err(e) => return Err(e),
                Ok(new_tokens) => {
                    for nt in new_tokens {
                        self.tokens.push_back(nt);
                    }
                    return Ok(true);
                }
            }
        }
    }
}

fn lex_line(source: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut my_source = source;

    loop {
        match consume_token(my_source) {
            Err(e) => return Err(e),
            Ok((None, _)) => {
                // Add whitespace at the end of the line in case the reader
                // trims newlines
                match tokens.last() {
                    Some(Token::Whitespace) => {}
                    _ => tokens.push(Token::Whitespace),
                };
                return Ok(tokens);
            }
            Ok((Some(Token::Whitespace), updated_source)) => {
                // Combine whitespace
                match tokens.last() {
                    Some(Token::Whitespace) => {}
                    _ => {
                        tokens.push(Token::Whitespace);
                    }
                }
                my_source = updated_source;
            }
            Ok((Some(token), updated_source)) => {
                tokens.push(token);
                my_source = updated_source;
            }
        }
    }
}

fn consume_token(source: &str) -> Result<(Option<Token>, &str), Error> {
    match source.chars().nth(0) {
        None => Ok((None, source)),
        _ if source.starts_with("PUSH") => Ok((Some(Token::Push), &source[4..])),
        _ if source.starts_with("DEFN") => Ok((Some(Token::Defn), &source[4..])),
        _ if source.starts_with("CALLIF") => Ok((Some(Token::CallIf), &source[6..])),
        _ if source.starts_with("EXIT") => Ok((Some(Token::Exit), &source[4..])),
        _ if source.starts_with("//") => consume_comment(source),
        Some('^') => Ok((Some(Token::Caret), &source[1..])),
        Some('*') => Ok((Some(Token::Asterisk), &source[1..])),
        Some('{') => Ok((Some(Token::LeftCurlyBracket), &source[1..])),
        Some('}') => Ok((Some(Token::RightCurlyBracket), &source[1..])),
        Some(c) if c.is_ascii_lowercase() || c == '_' => consume_function_name(source),
        // TODO: support different types (hexadecimal, binary, octal, character)
        Some(c) if c.is_ascii_digit() => consume_word(source),
        Some(c) if c.is_whitespace() => consume_whitespace(source),
        Some(c) => Err(anyhow!("Syntax error: unexpected character '{c}'.")),
    }
}

fn consume_comment(source: &str) -> Result<(Option<Token>, &str), Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            // Replace comment with whitespace
            None | Some('\n') => return Ok((Some(Token::Whitespace), &source[i + 1..])),
            _ => {
                i += 1;
            }
        }
    }
}

fn consume_whitespace(source: &str) -> Result<(Option<Token>, &str), Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !c.is_whitespace() => break,
            _ => i += 1,
        }
    }
    Ok((Some(Token::Whitespace), &source[i..]))
}

fn consume_word(source: &str) -> Result<(Option<Token>, &str), Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !c.is_ascii_digit() => break,
            _ => i += 1,
        }
    }

    // TODO: Try unwrapping with ? here
    let n = match source[..i].parse::<u32>() {
        Ok(m) => m,
        Err(_) => return Err(anyhow!("Syntax error: invalid word '{}'.", &source[..i])),
    };

    Ok((Some(Token::Word(n)), &source[i..]))
}

fn consume_function_name(source: &str) -> Result<(Option<Token>, &str), Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !(c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) => break,
            _ => i += 1,
        }
    }

    Ok((
        Some(Token::FunctionName(source[..i].to_string())),
        &source[i..],
    ))
}
