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
    Comma,
    Eof,
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
            Token::Comma => write!(f, "`,`"),
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

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(Token::Eof);
        }

        let ch = self.input[self.pos];

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
            b',' => Ok(Token::Comma),
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
