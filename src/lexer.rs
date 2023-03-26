use std::collections::VecDeque;

use anyhow::{anyhow, Context, Error};

use crate::reader::Reader;

#[derive(Debug, Eq, PartialEq)]
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
    LeftParen,
    RightParen,
    Argument(usize),
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

    /// Discards any remaining tokens that were already lexed.
    pub fn clear(&mut self) {
        self.tokens.clear();
    }

    /// Returns true if a new line has been read since the last time this
    /// function was called and all its tokens have been returned.
    pub fn full_line_consumed(&mut self) -> bool {
        self.tokens.is_empty()
    }

    pub fn next_token(&mut self, depth: usize) -> Result<Option<Token>, Error> {
        loop {
            if let Some(t) = self.tokens.pop_front() {
                return Ok(Some(t));
            }

            if !self.refill_tokens(depth)? {
                return Ok(None);
            }
        }
    }

    /// Gets a new line, lexes it, and adds the tokens to self.tokens. If the
    /// reader has no more lines, returns false. Otherwise, returns true.
    fn refill_tokens(&mut self, depth: usize) -> Result<bool, Error> {
        let line = match self.reader.next_line(depth) {
            None => return Ok(false),
            Some(x) => x,
        };

        let new_tokens = lex_line(&line)?;
        for nt in new_tokens {
            self.tokens.push_back(nt);
        }

        Ok(true)
    }
}

fn lex_line(source: &str) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut my_source = source;

    loop {
        match consume_token(my_source)? {
            (None, _) => {
                return Ok(tokens);
            }
            (Some(token), updated_source) => {
                tokens.push(token);
                my_source = updated_source;
            }
        }
    }
}

fn consume_token(source: &str) -> Result<(Option<Token>, &str), Error> {
    let mut source = source;
    loop {
        match source.chars().next() {
            None => return Ok((None, source)),
            Some('^') => return Ok((Some(Token::Caret), &source[1..])),
            Some('*') => return Ok((Some(Token::Asterisk), &source[1..])),
            Some('{') => return Ok((Some(Token::LeftCurlyBracket), &source[1..])),
            Some('}') => return Ok((Some(Token::RightCurlyBracket), &source[1..])),
            Some('(') => return Ok((Some(Token::LeftParen), &source[1..])),
            Some(')') => return Ok((Some(Token::RightParen), &source[1..])),
            Some('$') => return consume_argument(source),
            Some(c) if c.is_whitespace() => {
                source = consume_whitespace(source)?;
            }
            // TODO: support different types (hexadecimal, binary, octal, character)
            Some(c) if c.is_ascii_digit() => return consume_word(source),
            // Immediately return None because the comment extends all the way until
            // the end of the line
            _ if source.starts_with("//") => return Ok((None, source)),
            Some(c) if c.is_alphabetic() || c == '_' => return consume_symbol(source),
            Some(c) => return Err(anyhow!("Syntax error: unexpected character '{c}'.")),
        }
    }
}

fn consume_whitespace(source: &str) -> Result<&str, Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !c.is_whitespace() => break,
            _ => i += 1,
        }
    }
    // TODO: Handle Unicode characters properly
    Ok(&source[i..])
}

fn consume_word(source: &str) -> Result<(Option<Token>, &str), Error> {
    let (n, updated_source) = consume_base10_int(source)?;

    Ok((Some(Token::Word(n)), updated_source))
}

fn consume_base10_int(source: &str) -> Result<(u32, &str), Error> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !c.is_ascii_digit() => break,
            _ => i += 1,
        }
    }

    let n = source[..i]
        .parse::<u32>()
        .with_context(|| format!("Syntax error: invalid word '{}'.", &source[..i]))?;

    Ok((n, &source[i..]))
}

fn consume_symbol(source: &str) -> Result<(Option<Token>, &str), Error> {
    let (symbol, updated_source) = get_symbol(source);

    match symbol {
        "PUSH" => Ok((Some(Token::Push), updated_source)),
        "DEFN" => Ok((Some(Token::Defn), updated_source)),
        "CALLIF" => Ok((Some(Token::CallIf), updated_source)),
        "EXIT" => Ok((Some(Token::Exit), updated_source)),
        _ => Ok((Some(Token::FunctionName(symbol.to_owned())), updated_source)),
    }
}

fn get_symbol(source: &str) -> (&str, &str) {
    for (i, c) in source.chars().enumerate() {
        if !c.is_alphanumeric() && c != '_' {
            return (&source[..i], &source[i..]);
        }
    }
    (source, "")
}

fn consume_argument(source: &str) -> Result<(Option<Token>, &str), Error> {
    let (n, updated_source) = consume_base10_int(&source[1..])?;

    let n = usize::try_from(n)?;

    Ok((Some(Token::Argument(n)), updated_source))
}
