use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use num_rational::BigRational;

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
    TypeError {
        expected: &'static str,
        got: &'static str,
    },
    Custom(String),
    Dyn(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::UndefinedVariable(name) => write!(f, "undefined variable `{name}`"),
            EvalError::DivisionByZero => write!(f, "division by zero"),
            EvalError::TypeError { expected, got } => {
                write!(f, "type error: expected {expected}, got {got}")
            }
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
    Lambda {
        param: Arc<str>,
        body: Arc<Expr>,
    },
    Apply {
        func: Box<Expr>,
        arg: Box<Expr>,
    },
    Array(Vec<Expr>),
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

pub type Env = rpds::RedBlackTreeMap<String, Value, archery::RcK>;

#[derive(Clone)]
pub enum Value {
    Rational(BigRational),
    Bool(bool),
    HostFn(Rc<dyn Fn(&Value) -> Result<Value, EvalError>>),
    GuestFn {
        name: Arc<str>,
        body: Arc<Expr>,
        env: Env,
    },
    Array(Rc<Vec<Value>>),
}

impl Value {
    pub fn typ(&self) -> &'static str {
        match self {
            Value::Rational(_) => "number",
            Value::Bool(_) => "bool",
            Value::HostFn(_) | Value::GuestFn { .. } => "function",
            Value::Array(_) => "array",
        }
    }

    pub fn into_rational(self) -> Result<BigRational, EvalError> {
        match self {
            Value::Rational(r) => Ok(r),
            other => Err(EvalError::TypeError {
                expected: "number",
                got: other.typ(),
            }),
        }
    }

    pub fn is_truthy(&self) -> Result<bool, EvalError> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(EvalError::TypeError {
                expected: "bool",
                got: other.typ(),
            }),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Rational(r) => f.debug_tuple("Rational").field(r).finish(),
            Value::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            Value::HostFn(_) => f.debug_tuple("HostFn").finish(),
            Value::GuestFn { name, .. } => f.debug_tuple("GuestFn").field(name).finish(),
            Value::Array(elems) => f.debug_tuple("Array").field(elems).finish(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Rational(r) => write!(f, "{r}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::HostFn(_) => write!(f, "<host-fn>"),
            Value::GuestFn { name, .. } => write!(f, "<fn {name}>"),
            Value::Array(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, "]")
            }
        }
    }
}
