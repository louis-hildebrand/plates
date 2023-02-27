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

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut my_source = source;

    loop {
        match consume_token(my_source) {
            Err(msg) => return Err(msg),
            Ok((None, _)) => return Ok(tokens),
            Ok((Some(Token::Whitespace), updated_source)) => {
                // Combine whitespace
                match tokens.last() {
                    Some(Token::Whitespace) => {},
                    _ => { tokens.push(Token::Whitespace); }
                }
                my_source = updated_source;
            },
            Ok((Some(token), updated_source)) => {
                tokens.push(token);
                my_source = updated_source;
            }
        }
    }
}

fn consume_token(source: &str) -> Result<(Option<Token>, &str), String> {
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
        Some(c) if c.is_ascii_digit() && c != '0' => consume_word(source),
        Some(c) if c.is_whitespace() => consume_whitespace(source),
        Some(c) => Err(format!("Syntax error: unexpected character '{c}'.")),
    }
}

fn consume_comment(source: &str) -> Result<(Option<Token>, &str), String> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            // Replace comment with whitespace
            None | Some('\n') => return Ok((Some(Token::Whitespace), &source[i+1..])),
            _ => { i += 1; }
        }
    }
}

fn consume_whitespace(source: &str) -> Result<(Option<Token>, &str), String> {
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

fn consume_word(source: &str) -> Result<(Option<Token>, &str), String> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !c.is_ascii_digit() => break,
            _ => i += 1
        }
    }
    let n = match source[..i].parse::<u32>() {
        Ok(m) => m,
        Err(_) => return Err(format!("Syntax error: invalid word '{}'.", &source[..i]))
    };
    Ok((Some(Token::Word(n)), &source[i..]))
}

fn consume_function_name(source: &str) -> Result<(Option<Token>, &str), String> {
    let mut i = 1;
    loop {
        match source.chars().nth(i) {
            None => break,
            Some(c) if !(c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) => break,
            _ => i += 1
        }
    }
    Ok((Some(Token::FunctionName(source[..i].to_string())), &source[i..]))
}
