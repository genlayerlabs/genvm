use std::sync::Arc;

use super::tokenizer::{Token, Tokenizer};
use super::value::{BinOp, Expr, ParseError};

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
            Token::Backslash => self.parse_lambda(),
            _ => self.parse_comparison(),
        }
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        self.advance();
        let param = match self.advance() {
            Token::Ident(s) => s,
            tok => {
                return Err(ParseError::ExpectedToken {
                    expected: "identifier".to_owned(),
                    got: tok.to_string(),
                })
            }
        };
        self.expect(&Token::Eq)?;
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            param: Arc::from(param),
            body: Arc::new(body),
        })
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
        self.parse_application()
    }

    fn is_primary_start(&self) -> bool {
        matches!(self.peek(), Token::Num(_) | Token::Ident(_) | Token::LParen)
    }

    fn parse_application(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        while self.is_primary_start() {
            let arg = self.parse_primary()?;
            expr = Expr::Apply {
                func: Box::new(expr),
                arg: Box::new(arg),
            };
        }
        Ok(expr)
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
}
