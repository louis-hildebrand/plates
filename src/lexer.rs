use std::collections::VecDeque;

use anyhow::{anyhow, Context, Error};

use crate::reader::Reader;

#[derive(Debug, Eq, PartialEq)]
pub enum Token {
    Push,
    Defn,
    CallIf,
    Exit,
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

    /// Returns true if there are no tokens left on the current line.
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
            Some(c) => return Err(anyhow!("Syntax error: Unexpected character '{c}'.")),
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
        .with_context(|| format!("Syntax error: Invalid word '{}'.", &source[..i]))?;

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

#[cfg(test)]
mod tests {
    use super::{Lexer, Token};
    use paste::paste;

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

    /// Generates a test case with the given name, which checks that lexing the given inputs yields the expected
    /// outputs and then `None`.
    macro_rules! generate_success_test_case {
        ( $($name:ident: ($inputs:expr, $outputs:expr)),* $(,)? ) => {
            $(
                #[test]
                fn $name() {
                    let mut lexer = Lexer::new($inputs.into_iter().map(|x| x.to_owned()));
                    for expected in $outputs {
                        assert_ok_and_eq!(lexer.next_token(0), Some(expected));
                    }
                    assert_ok_and_eq!(lexer.next_token(0), None);
                }
            )*
        };
    }

    // TODO: test with more complex whitespace (e.g., U+200B, U+00A0)
    const SPC: &str = " \t ";
    const CMT: &str = "// comment";

    macro_rules! test_lex_success {
        ( $($name:ident: ($code:expr, $token:expr)),* $(,)? ) => {
            $(
                paste! {
                    generate_success_test_case![
                        // Single token
                        $name: (vec![$code], vec![$token]),
                        [<$name _whitespace>]: (vec![format!("{}{SPC}", $code)], vec![$token]),
                        [<$name _comment>]: (vec![format!("{}{CMT}", $code)], vec![$token]),
                        [<$name _lf>]: (vec![format!("{}\n", $code)], vec![$token]),
                        [<$name _crlf>]: (vec![format!("{}\r\n", $code)], vec![$token]),
                        [<$name _whitespace_comment >]: (
                            vec![format!("{}{SPC}{CMT}", $code)],
                            vec![$token]
                        ),
                        // Token followed by PUSH
                        [<$name _push>]: (
                            vec![format!("{}{SPC}PUSH", $code)],
                            vec![$token, Token::Push]
                        ),
                        [<$name _newline_push>]: (
                            vec![$code, "PUSH"],
                            vec![$token, Token::Push]
                        ),
                        [<$name _newline_lf_push>]: (
                            vec![format!("{}\n", $code), "PUSH\n".to_owned()],
                            vec![$token, Token::Push]
                        ),
                        [<$name _newline_crlf_push>]: (
                            vec![format!("{}\r\n", $code), "PUSH\r\n".to_owned()],
                            vec![$token, Token::Push]
                        ),
                        [<$name _newline_whitespace_lf_push>]: (
                            vec![
                                format!("{SPC}{}{SPC}\n", $code),
                                format!("{SPC}PUSH{SPC}\n"),
                            ],
                            vec![$token, Token::Push]
                        ),
                        [<$name _newline_whitespace_crlf_push>]: (
                            vec![
                                format!("{SPC}{}{SPC}\r\n", $code),
                                format!("{SPC}PUSH{SPC}\r\n"),
                            ],
                            vec![$token, Token::Push]
                        ),
                        // Token preceded by PUSH
                        [<$name _after_push>]: (
                            vec![format!("PUSH{SPC}{}", $code)],
                            vec![Token::Push, $token]
                        ),
                        [<$name _after_push_newline>]: (
                            vec!["PUSH", $code],
                            vec![Token::Push, $token]
                        ),
                        [<$name _after_push_newline_lf>]: (
                            vec!["PUSH\n".to_owned(), format!("{}\n", $code)],
                            vec![Token::Push, $token]
                        ),
                        [<$name _after_push_newline_crlf>]: (
                            vec!["PUSH\r\n".to_owned(), format!("{}\r\n", $code)],
                            vec![Token::Push, $token]
                        ),
                        [<$name _after_push_newline_whitespace_lf>]: (
                            vec![
                                format!("{SPC}PUSH{SPC}\n"),
                                format!("{SPC}{}{SPC}\n", $code),
                            ],
                            vec![Token::Push, $token]
                        ),
                        [<$name _after_push_newline_whitespace_crlf>]: (
                            vec![
                                format!("{SPC}PUSH{SPC}\r\n"),
                                format!("{SPC}{}{SPC}\r\n", $code),
                            ],
                            vec![Token::Push, $token]
                        ),
                        // Token followed immediately by bracket
                        [<$name _left_curly>]: (vec![format!("{}{{", $code)], vec![$token, Token::LeftCurlyBracket]),
                        [<$name _right_curly>]: (vec![format!("{}}}", $code)], vec![$token, Token::RightCurlyBracket]),
                        [<$name _left_paren>]: (vec![format!("{}(", $code)], vec![$token, Token::LeftParen]),
                        [<$name _right_paren>]: (vec![format!("{})", $code)], vec![$token, Token::RightParen]),
                    ];
                }
            )*
        };
    }

    /// Generates a test case with the given name, which checks that lexing the given inputs immediately produces an
    /// error with the given message and then produces `None`.
    macro_rules! test_lex_failure {
        ( $($name:ident: ($inputs:expr, $msg:expr)),* $(,)? ) => {
            $(
                #[test]
                fn $name() {
                    let mut lexer = Lexer::new($inputs.into_iter().map(|x| x.to_owned()));
                    assert_err_with_msg!(lexer.next_token(0), $msg);
                    assert_ok_and_eq!(lexer.next_token(0), None);
                }
            )*
        };
    }

    test_lex_success![
        push: ("PUSH", Token::Push),
        defn: ("DEFN", Token::Defn),
        callif: ("CALLIF", Token::CallIf),
        exit: ("EXIT", Token::Exit),
        asterisk: ("*", Token::Asterisk),
        left_curly_bracket: ("{", Token::LeftCurlyBracket),
        right_curly_bracket: ("}", Token::RightCurlyBracket),
        left_paren: ("(", Token::LeftParen),
        right_paren: (")", Token::RightParen),
        argument0: ("$0", Token::Argument(0)),
        argument10: ("$10", Token::Argument(10)),
        word_min: ("0", Token::Word(0)),
        // 2^32 - 1
        word_max: ("4294967295", Token::Word(4294967295)),
        function_name:
            (
                "my_funcName",
                Token::FunctionName("my_funcName".to_owned())
            ),
    ];

    generate_success_test_case![
        push123:
            (
                vec!["PUSH123"],
                vec![Token::FunctionName("PUSH123".to_owned())]
            ),
        defn123:
            (
                vec!["DEFN123"],
                vec![Token::FunctionName("DEFN123".to_owned())]
            ),
        callif123:
            (
                vec!["CALLIF123"],
                vec![Token::FunctionName("CALLIF123".to_owned())]
            ),
        exit123:
            (
                vec!["EXIT123"],
                vec![Token::FunctionName("EXIT123".to_owned())]
            ),
    ];

    test_lex_failure![
        fail_on_massive_word: (vec!["9".repeat(1000)], format!("Syntax error: Invalid word '{}'.", "9".repeat(1000))),
        // 2^32
        fail_on_too_large_word: (vec!["4294967296"], "Syntax error: Invalid word '4294967296'."),
        fail_on_negative_word: (vec!["-1"], "Syntax error: Unexpected character '-'."),
        fail_on_hashtag: (vec!["#"], "Syntax error: Unexpected character '#'."),
    ];

    #[test]
    fn fail_and_discard_line() {
        let lines = vec!["% PUSH 123".to_owned(), "PUSH 456".to_owned()];
        let mut lexer = Lexer::new(lines.into_iter());

        // After error, first line should be cleared but second line should remain
        assert_err_with_msg!(
            lexer.next_token(0),
            "Syntax error: Unexpected character '%'."
        );
        assert_ok_and_eq!(lexer.next_token(0), Some(Token::Push));
        assert_ok_and_eq!(lexer.next_token(0), Some(Token::Word(456)));
        assert_ok_and_eq!(lexer.next_token(0), None);
    }

    #[test]
    fn clear() {
        let lines = vec!["PUSH 123 PUSH 456".to_owned(), "PUSH 789".to_owned()];
        let mut lexer = Lexer::new(lines.into_iter());

        assert_ok_and_eq!(lexer.next_token(0), Some(Token::Push));

        lexer.clear();

        assert_ok_and_eq!(lexer.next_token(0), Some(Token::Push));
        assert_ok_and_eq!(lexer.next_token(0), Some(Token::Word(789)));
        assert_ok_and_eq!(lexer.next_token(0), None);
    }

    #[test]
    fn full_line_consumed() {
        let lines = vec![
            "DEFN f (0) {".to_owned(),
            "    PUSH 123".to_owned(),
            "}".to_owned(),
        ];
        let mut lexer = Lexer::new(lines.into_iter());
        let expected = [
            (Token::Defn, false),
            (Token::FunctionName("f".to_owned()), false),
            (Token::LeftParen, false),
            (Token::Word(0), false),
            (Token::RightParen, false),
            (Token::LeftCurlyBracket, true),
            (Token::Push, false),
            (Token::Word(123), true),
            (Token::RightCurlyBracket, true),
        ];

        for (token, end_of_line) in expected {
            assert_ok_and_eq!(lexer.next_token(0), Some(token));
            assert_eq!(lexer.full_line_consumed(), end_of_line);
        }

        assert_ok_and_eq!(lexer.next_token(0), None);
        assert_eq!(lexer.full_line_consumed(), true);
    }
}
