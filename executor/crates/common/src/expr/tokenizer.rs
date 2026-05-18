use std::fmt;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Pow;

use super::value::ParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    Ident(String),
    Num(BigRational),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    Let,
    Eq,
    In,
    If,
    Then,
    Else,
    Backslash,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Dot,
    /// A string literal, already split into literal/interpolation segments.
    Str(Vec<StrPart>),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StrPart {
    Lit(String),
    /// Tokens of an `\(expr)` interpolation (without a trailing `Eof`).
    Interp(Vec<Token>),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "ident `{s}`"),
            Token::Num(n) => write!(f, "number `{n}`"),
            Token::Plus => write!(f, "`+`"),
            Token::Minus => write!(f, "`-`"),
            Token::Star => write!(f, "`*`"),
            Token::Slash => write!(f, "`/`"),
            Token::LParen => write!(f, "`(`"),
            Token::RParen => write!(f, "`)`"),
            Token::Lt => write!(f, "`<`"),
            Token::Gt => write!(f, "`>`"),
            Token::Le => write!(f, "`<=`"),
            Token::Ge => write!(f, "`>=`"),
            Token::EqEq => write!(f, "`==`"),
            Token::Ne => write!(f, "`!=`"),
            Token::Let => write!(f, "`let`"),
            Token::Eq => write!(f, "`=`"),
            Token::In => write!(f, "`in`"),
            Token::If => write!(f, "`if`"),
            Token::Then => write!(f, "`then`"),
            Token::Else => write!(f, "`else`"),
            Token::Backslash => write!(f, r"`\`"),
            Token::LBracket => write!(f, "`[`"),
            Token::RBracket => write!(f, "`]`"),
            Token::LBrace => write!(f, "`{{`"),
            Token::RBrace => write!(f, "`}}`"),
            Token::Comma => write!(f, "`,`"),
            Token::Semicolon => write!(f, "`;`"),
            Token::Dot => write!(f, "`.`"),
            Token::Str(_) => write!(f, "string"),
            Token::Eof => write!(f, "end of input"),
        }
    }
}

pub(crate) struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    /// Skips whitespace and `#` line comments. A `#` starts a comment only
    /// when followed by a space, a newline, or end of input; it runs to the
    /// next newline (or EOF). Otherwise `#` is left for `next_token` to reject.
    fn skip_trivia(&mut self) {
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() || self.input[self.pos] != b'#' {
                break;
            }
            match self.input.get(self.pos + 1) {
                None | Some(b' ') | Some(b'\n') => {}
                Some(_) => break,
            }
            while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
                self.pos += 1;
            }
        }
    }

    fn read_digits(&mut self) -> String {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        std::str::from_utf8(&self.input[start..self.pos])
            .unwrap()
            .to_owned()
    }

    fn parse_bigint(&self, s: &str) -> Result<BigInt, ParseError> {
        s.parse::<BigInt>()
            .map_err(|_| ParseError::InvalidNumber(s.to_owned()))
    }

    fn read_number(&mut self) -> Result<BigRational, ParseError> {
        let integer_part = self.read_digits();

        let mut value = BigRational::from(self.parse_bigint(&integer_part)?);

        if self.pos < self.input.len() && self.input[self.pos] == b'.' {
            self.pos += 1;
            let frac_start = self.pos;
            let frac_digits = self.read_digits();
            if self.pos == frac_start {
                return Err(ParseError::ExpectedDigits("after decimal point"));
            }
            let scale = BigInt::from(10).pow(frac_digits.len() as u32);
            let frac = BigRational::new(self.parse_bigint(&frac_digits)?, scale);
            value = value + frac;
        }

        if self.pos < self.input.len()
            && (self.input[self.pos] == b'e' || self.input[self.pos] == b'E')
        {
            self.pos += 1;
            let neg = if self.pos < self.input.len() && self.input[self.pos] == b'-' {
                self.pos += 1;
                true
            } else {
                if self.pos < self.input.len() && self.input[self.pos] == b'+' {
                    self.pos += 1;
                }
                false
            };
            let exp_start = self.pos;
            let exp_digits = self.read_digits();
            if self.pos == exp_start {
                return Err(ParseError::ExpectedDigits("after exponent"));
            }
            let exp: u32 = exp_digits
                .parse()
                .map_err(|_| ParseError::InvalidNumber(exp_digits))?;
            let factor = BigRational::from(BigInt::from(10).pow(exp));
            value = if neg { value / factor } else { value * factor };
        }

        Ok(value)
    }

    /// Reads `"..."` with `\(expr)` interpolation and `\\ \" \n \t \r` escapes.
    /// Assumes the opening `"` is at `self.pos`.
    fn read_string(&mut self) -> Result<Token, ParseError> {
        self.pos += 1; // opening quote
        let mut parts: Vec<StrPart> = Vec::new();
        let mut lit: Vec<u8> = Vec::new();

        let flush = |lit: &mut Vec<u8>, parts: &mut Vec<StrPart>| -> Result<(), ParseError> {
            if !lit.is_empty() {
                let s = String::from_utf8(std::mem::take(lit))
                    .map_err(|_| ParseError::UnterminatedString)?;
                parts.push(StrPart::Lit(s));
            }
            Ok(())
        };

        loop {
            let Some(&c) = self.input.get(self.pos) else {
                return Err(ParseError::UnterminatedString);
            };
            match c {
                b'"' => {
                    self.pos += 1;
                    break;
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(&e) = self.input.get(self.pos) else {
                        return Err(ParseError::UnterminatedString);
                    };
                    self.pos += 1;
                    match e {
                        b'(' => {
                            flush(&mut lit, &mut parts)?;
                            let toks = self.read_interpolation()?;
                            parts.push(StrPart::Interp(toks));
                        }
                        b'n' => lit.push(b'\n'),
                        b't' => lit.push(b'\t'),
                        b'r' => lit.push(b'\r'),
                        b'\\' => lit.push(b'\\'),
                        b'"' => lit.push(b'"'),
                        other => return Err(ParseError::UnexpectedChar(other as char)),
                    }
                }
                _ => {
                    lit.push(c);
                    self.pos += 1;
                }
            }
        }

        flush(&mut lit, &mut parts)?;
        Ok(Token::Str(parts))
    }

    /// Reads tokens of an interpolation up to its matching `)`. The opening
    /// `(` of `\(` has already been consumed; the closing `)` is consumed but
    /// not returned. Nested strings are handled by recursing through
    /// `next_token`, so their inner parens do not affect the depth count.
    fn read_interpolation(&mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        let mut depth = 1usize;
        loop {
            let tok = self.next_token()?;
            match tok {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(tokens);
                    }
                }
                Token::Eof => return Err(ParseError::UnterminatedString),
                _ => {}
            }
            tokens.push(tok);
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_trivia();

        if self.pos >= self.input.len() {
            return Ok(Token::Eof);
        }

        let ch = self.input[self.pos];

        if ch == b'"' {
            return self.read_string();
        }

        if ch.is_ascii_digit()
            || (ch == b'.'
                && self.pos + 1 < self.input.len()
                && self.input[self.pos + 1].is_ascii_digit())
        {
            return Ok(Token::Num(self.read_number()?));
        }

        if ch.is_ascii_alphabetic() || ch == b'_' {
            let start = self.pos;
            while self.pos < self.input.len()
                && (self.input[self.pos].is_ascii_alphanumeric() || self.input[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap();
            return Ok(match s {
                "let" => Token::Let,
                "in" => Token::In,
                "if" => Token::If,
                "then" => Token::Then,
                "else" => Token::Else,
                _ => Token::Ident(s.to_owned()),
            });
        }

        self.pos += 1;
        match ch {
            b'+' => Ok(Token::Plus),
            b'-' => Ok(Token::Minus),
            b'*' => Ok(Token::Star),
            b'/' => Ok(Token::Slash),
            b'(' => Ok(Token::LParen),
            b')' => Ok(Token::RParen),
            b'\\' => Ok(Token::Backslash),
            b'[' => Ok(Token::LBracket),
            b']' => Ok(Token::RBracket),
            b'{' => Ok(Token::LBrace),
            b'}' => Ok(Token::RBrace),
            b',' => Ok(Token::Comma),
            b';' => Ok(Token::Semicolon),
            b'.' => Ok(Token::Dot),
            b'=' => {
                if self.pos < self.input.len() && self.input[self.pos] == b'=' {
                    self.pos += 1;
                    Ok(Token::EqEq)
                } else {
                    Ok(Token::Eq)
                }
            }
            b'!' => {
                if self.pos < self.input.len() && self.input[self.pos] == b'=' {
                    self.pos += 1;
                    Ok(Token::Ne)
                } else {
                    Err(ParseError::UnexpectedChar('!'))
                }
            }
            b'<' => {
                if self.pos < self.input.len() && self.input[self.pos] == b'=' {
                    self.pos += 1;
                    Ok(Token::Le)
                } else {
                    Ok(Token::Lt)
                }
            }
            b'>' => {
                if self.pos < self.input.len() && self.input[self.pos] == b'=' {
                    self.pos += 1;
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }
            _ => Err(ParseError::UnexpectedChar(ch as char)),
        }
    }

    pub(crate) fn tokenize(mut self) -> Result<Vec<Token>, ParseError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}
