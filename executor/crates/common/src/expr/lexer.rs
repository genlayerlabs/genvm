use std::sync::Arc;

use super::tokenizer::{StrPart, Token, Tokenizer};
use super::value::{BinOp, Expr, ParseError, StrSeg};

/// A string token used as an attribute name or field key must be a single
/// plain literal (no interpolation). The empty string is allowed.
fn string_key(parts: Vec<StrPart>) -> Result<String, ParseError> {
    match parts.as_slice() {
        [] => Ok(String::new()),
        [StrPart::Lit(s)] => Ok(s.clone()),
        _ => Err(ParseError::UnexpectedToken(
            "interpolation in attribute name".to_owned(),
        )),
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
            Token::Backslash => self.parse_lambda(),
            _ => self.parse_comparison(),
        }
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        self.advance();

        // `\x y z = body` is sugar for `\x = \y = \z = body`.
        let mut params = Vec::new();
        loop {
            match self.advance() {
                Token::Ident(s) => params.push(s),
                tok => {
                    return Err(ParseError::ExpectedToken {
                        expected: "identifier".to_owned(),
                        got: tok.to_string(),
                    })
                }
            }
            if self.peek() == &Token::Eq {
                break;
            }
        }
        self.expect(&Token::Eq)?;

        let mut expr = self.parse_expr()?;
        for param in params.into_iter().rev() {
            expr = Expr::Lambda {
                param: Arc::from(param),
                body: Arc::new(expr),
            };
        }
        Ok(expr)
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
        matches!(
            self.peek(),
            Token::Num(_)
                | Token::Ident(_)
                | Token::LParen
                | Token::LBracket
                | Token::LBrace
                | Token::Str(_)
        )
    }

    fn parse_application(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_postfix()?;
        while self.is_primary_start() {
            let arg = self.parse_postfix()?;
            expr = Expr::Apply {
                func: Box::new(expr),
                arg: Box::new(arg),
            };
        }
        Ok(expr)
    }

    /// `obj.key` / `obj."key"` attribute selection. Binds tighter than
    /// application (so `f a.b` is `f (a.b)`), matching Nix.
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        while *self.peek() == Token::Dot {
            self.advance();
            let key = match self.advance() {
                Token::Ident(s) => s,
                Token::Str(parts) => string_key(parts)?,
                tok => {
                    return Err(ParseError::ExpectedToken {
                        expected: "field name".to_owned(),
                        got: tok.to_string(),
                    })
                }
            };
            expr = Expr::Field {
                obj: Box::new(expr),
                key,
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
            Token::LBracket => {
                let mut elems = Vec::new();
                if *self.peek() != Token::RBracket {
                    elems.push(self.parse_expr()?);
                    while *self.peek() == Token::Comma {
                        self.advance();
                        elems.push(self.parse_expr()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::Array(elems))
            }
            Token::Str(parts) => Ok(Expr::Str(Self::parse_str_parts(parts)?)),
            Token::LBrace => {
                // Nix-style attrset: `{ name = expr; ... }`, empty `{}` ok.
                let mut fields = Vec::new();
                while *self.peek() != Token::RBrace {
                    let key = match self.advance() {
                        Token::Ident(s) => s,
                        Token::Str(parts) => string_key(parts)?,
                        tok => {
                            return Err(ParseError::ExpectedToken {
                                expected: "attribute name".to_owned(),
                                got: tok.to_string(),
                            })
                        }
                    };
                    self.expect(&Token::Eq)?;
                    let value = self.parse_expr()?;
                    self.expect(&Token::Semicolon)?;
                    fields.push((key, value));
                }
                self.advance(); // closing `}`
                Ok(Expr::Object(fields))
            }
            tok => Err(ParseError::UnexpectedToken(tok.to_string())),
        }
    }

    fn parse_str_parts(parts: Vec<StrPart>) -> Result<Vec<StrSeg>, ParseError> {
        let mut segs = Vec::with_capacity(parts.len());
        for part in parts {
            match part {
                StrPart::Lit(s) => segs.push(StrSeg::Lit(s)),
                StrPart::Interp(mut tokens) => {
                    tokens.push(Token::Eof);
                    let mut sub = Parser::new(tokens);
                    let expr = sub.parse_expr()?;
                    if *sub.peek() != Token::Eof {
                        return Err(ParseError::UnexpectedToken(sub.peek().to_string()));
                    }
                    segs.push(StrSeg::Interp(Box::new(expr)));
                }
            }
        }
        Ok(segs)
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
