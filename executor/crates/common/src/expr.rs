use std::fmt;

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Pow, Zero};

#[derive(Debug)]
pub enum ParseError {
    UnexpectedChar(char),
    UnexpectedToken(String),
    ExpectedToken { expected: String, got: String },
    ExpectedDigits(&'static str),
    InvalidNumber(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedChar(c) => write!(f, "unexpected character `{c}`"),
            ParseError::UnexpectedToken(t) => write!(f, "unexpected token {t}"),
            ParseError::ExpectedToken { expected, got } => {
                write!(f, "expected {expected}, got {got}")
            }
            ParseError::ExpectedDigits(ctx) => write!(f, "expected digits {ctx}"),
            ParseError::InvalidNumber(s) => write!(f, "invalid number `{s}`"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
pub enum EvalError {
    UndefinedVariable(String),
    DivisionByZero,
    Custom(String),
    Dyn(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::UndefinedVariable(name) => write!(f, "undefined variable `{name}`"),
            EvalError::DivisionByZero => write!(f, "division by zero"),
            EvalError::Custom(msg) => write!(f, "{msg}"),
            EvalError::Dyn(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for EvalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvalError::Dyn(e) => Some(&**e),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
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
            Token::Eof => write!(f, "end of input"),
        }
    }
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
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

    fn tokenize(mut self) -> Result<Vec<Token>, ParseError> {
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

#[derive(Debug, Clone)]
pub enum Expr {
    Ident(String),
    Const(BigRational),
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryMinus(Box<Expr>),
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ge => write!(f, ">="),
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        let tok = self.advance();
        if &tok != expected {
            return Err(ParseError::ExpectedToken {
                expected: expected.to_string(),
                got: tok.to_string(),
            });
        }
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            _ => self.parse_comparison(),
        }
    }

    fn parse_let(&mut self) -> Result<Expr, ParseError> {
        self.advance();
        let name = match self.advance() {
            Token::Ident(s) => s,
            tok => {
                return Err(ParseError::ExpectedToken {
                    expected: "identifier".to_owned(),
                    got: tok.to_string(),
                })
            }
        };
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        self.expect(&Token::In)?;
        let body = self.parse_expr()?;
        Ok(Expr::Let {
            name,
            value: Box::new(value),
            body: Box::new(body),
        })
    }

    fn parse_if(&mut self) -> Result<Expr, ParseError> {
        self.advance();
        let cond = self.parse_expr()?;
        self.expect(&Token::Then)?;
        let then_branch = self.parse_expr()?;
        self.expect(&Token::Else)?;
        let else_branch = self.parse_expr()?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                Token::Ge => BinOp::Ge,
                Token::EqEq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_additive()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinOp {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if *self.peek() == Token::Minus {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::UnaryMinus(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.advance() {
            Token::Num(n) => Ok(Expr::Const(n)),
            Token::Ident(s) => Ok(Expr::Ident(s)),
            Token::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            tok => Err(ParseError::UnexpectedToken(tok.to_string())),
        }
    }
}

impl Expr {
    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let tokens = Tokenizer::new(input).tokenize()?;
        let mut parser = Parser::new(tokens);
        let expr = parser.parse_expr()?;
        if *parser.peek() != Token::Eof {
            return Err(ParseError::UnexpectedToken(parser.peek().to_string()));
        }
        Ok(expr)
    }

    pub fn evaluate(&self) -> Result<BigRational, EvalError> {
        self.evaluate_with(&|name: &str| Err(EvalError::UndefinedVariable(name.to_owned())))
    }

    pub fn evaluate_with<'a>(
        &'a self,
        get_var: impl Fn(&str) -> Result<BigRational, EvalError>,
    ) -> Result<BigRational, EvalError> {
        self.evaluate_with_inner(&get_var, &mut std::collections::BTreeMap::new())
    }

    fn evaluate_with_inner<'a, F>(
        &'a self,
        get_var: &F,
        let_bindings: &mut std::collections::BTreeMap<&'a str, BigRational>,
    ) -> Result<BigRational, EvalError>
    where
        F: Fn(&str) -> Result<BigRational, EvalError>,
    {
        match self {
            Expr::Const(n) => Ok(n.clone()),
            Expr::Ident(name) => {
                if let Some(val) = let_bindings.get(name.as_str()) {
                    Ok(val.clone())
                } else {
                    get_var(name)
                }
            }
            Expr::UnaryMinus(e) => Ok(-e.evaluate_with_inner::<F>(get_var, let_bindings)?),
            Expr::BinOp { op, lhs, rhs } => {
                let l = lhs.evaluate_with_inner::<F>(get_var, let_bindings)?;
                let r = rhs.evaluate_with_inner::<F>(get_var, let_bindings)?;
                match op {
                    BinOp::Add => Ok(l + r),
                    BinOp::Sub => Ok(l - r),
                    BinOp::Mul => Ok(l * r),
                    BinOp::Div => {
                        if r.is_zero() {
                            return Err(EvalError::DivisionByZero);
                        }
                        Ok(l / r)
                    }
                    BinOp::Lt => Ok(bool_to_rational(l < r)),
                    BinOp::Gt => Ok(bool_to_rational(l > r)),
                    BinOp::Le => Ok(bool_to_rational(l <= r)),
                    BinOp::Ge => Ok(bool_to_rational(l >= r)),
                    BinOp::Eq => Ok(bool_to_rational(l == r)),
                    BinOp::Ne => Ok(bool_to_rational(l != r)),
                }
            }
            Expr::Let { name, value, body } => {
                let val = value.evaluate_with_inner::<F>(get_var, let_bindings)?;
                let old = let_bindings.insert(name, val);
                let res = body.evaluate_with_inner::<F>(get_var, let_bindings);
                if let Some(old_val) = old {
                    let_bindings.insert(name, old_val);
                } else {
                    let_bindings.remove(name as &str);
                }

                res
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let c = cond.evaluate_with_inner::<F>(get_var, let_bindings)?;
                if !c.is_zero() {
                    then_branch.evaluate_with_inner::<F>(get_var, let_bindings)
                } else {
                    else_branch.evaluate_with_inner::<F>(get_var, let_bindings)
                }
            }
        }
    }
}

fn bool_to_rational(b: bool) -> BigRational {
    if b {
        BigRational::one()
    } else {
        BigRational::zero()
    }
}
