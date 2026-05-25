use std::fmt;
use std::sync::{Arc, Mutex};

use num_rational::BigRational;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedChar(char),
    UnexpectedToken(String),
    ExpectedToken { expected: String, got: String },
    ExpectedDigits(&'static str),
    InvalidNumber(String),
    UnterminatedString,
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
            ParseError::UnterminatedString => write!(f, "unterminated string literal"),
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
    /// String literal with `\(expr)` interpolation segments.
    Str(Vec<StrSeg>),
    /// Object/attrset literal. Keys are stored unsorted here; the evaluated
    /// [`Value::Object`] is sorted by key (it is a `BTreeMap`).
    Object(Vec<(String, Expr)>),
    /// `obj.key` attribute selection.
    Field {
        obj: Box<Expr>,
        key: String,
    },
}

#[derive(Debug, Clone)]
pub enum StrSeg {
    Lit(String),
    Interp(Box<Expr>),
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

pub type Env = rpds::RedBlackTreeMap<String, Thunk, archery::ArcK>;

pub type HostFnImpl = dyn Fn(&Value) -> Result<Value, EvalError> + Send + Sync;

#[derive(Clone)]
pub enum Value {
    Rational(BigRational),
    Bool(bool),
    HostFn(Arc<HostFnImpl>),
    GuestFn {
        name: Arc<str>,
        body: Arc<Expr>,
        env: Env,
    },
    Array(Arc<Vec<Value>>),
    Str(Arc<str>),
    /// Attribute set, kept sorted by key.
    Object(Arc<std::collections::BTreeMap<String, Value>>),
}

/// A memoized, lazily-evaluated value (call-by-need).
///
/// Holds either an already-computed [`Value`] or a deferred computation that
/// is run at most once, on the first [`Thunk::force`]. Re-entrant forcing of
/// the same thunk (which would only happen via genuine self-reference, since
/// `let` is non-recursive — recursion is expressed with the Y combinator)
/// is detected and reported instead of deadlocking or looping forever.
#[derive(Clone)]
pub struct Thunk(Arc<Mutex<ThunkState>>);

enum ThunkState {
    Forced(Value),
    Deferred(Box<dyn FnOnce() -> Result<Value, EvalError> + Send>),
    InProgress,
}

impl Thunk {
    pub fn forced(value: Value) -> Self {
        Thunk(Arc::new(Mutex::new(ThunkState::Forced(value))))
    }

    pub fn deferred(f: impl FnOnce() -> Result<Value, EvalError> + Send + 'static) -> Self {
        Thunk(Arc::new(Mutex::new(ThunkState::Deferred(Box::new(f)))))
    }

    pub fn force(&self) -> Result<Value, EvalError> {
        let deferred = {
            let mut state = self.0.lock().expect("thunk mutex poisoned");
            match std::mem::replace(&mut *state, ThunkState::InProgress) {
                ThunkState::Forced(v) => {
                    *state = ThunkState::Forced(v.clone());
                    return Ok(v);
                }
                ThunkState::InProgress => {
                    return Err(EvalError::Custom(
                        "infinite recursion while forcing a lazy value".to_owned(),
                    ));
                }
                ThunkState::Deferred(f) => f,
            }
        };

        // Lock is released here on purpose: a re-entrant `force` of this same
        // thunk now observes `InProgress` and errors instead of deadlocking.
        let result = deferred();

        let mut state = self.0.lock().expect("thunk mutex poisoned");
        match &result {
            Ok(v) => *state = ThunkState::Forced(v.clone()),
            // Leave it `InProgress`: a failed computation is not retried, and
            // any later force surfaces the recursion/error path consistently.
            Err(_) => {}
        }
        result
    }
}

impl fmt::Debug for Thunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0.try_lock().as_deref() {
            Ok(ThunkState::Forced(v)) => f.debug_tuple("Thunk").field(v).finish(),
            _ => f.write_str("Thunk(<unevaluated>)"),
        }
    }
}

impl Value {
    pub fn typ(&self) -> &'static str {
        match self {
            Value::Rational(_) => "number",
            Value::Bool(_) => "bool",
            Value::HostFn(_) | Value::GuestFn { .. } => "function",
            Value::Array(_) => "array",
            Value::Str(_) => "string",
            Value::Object(_) => "object",
        }
    }

    pub fn as_str(&self) -> Result<&str, EvalError> {
        match self {
            Value::Str(s) => Ok(s),
            other => Err(EvalError::TypeError {
                expected: "string",
                got: other.typ(),
            }),
        }
    }

    pub fn as_object(&self) -> Result<&std::collections::BTreeMap<String, Value>, EvalError> {
        match self {
            Value::Object(o) => Ok(o),
            other => Err(EvalError::TypeError {
                expected: "object",
                got: other.typ(),
            }),
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

impl From<BigRational> for Value {
    fn from(v: BigRational) -> Self {
        Value::Rational(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Self {
        Value::Rational(BigRational::from_integer(v.into()))
    }
}

impl From<usize> for Value {
    fn from(v: usize) -> Self {
        Value::Rational(BigRational::from_integer(v.into()))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Rational(BigRational::from_integer(v.into()))
    }
}

impl From<Arc<str>> for Value {
    fn from(v: Arc<str>) -> Self {
        Value::Str(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Str(v.into())
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
            Value::Str(s) => f.debug_tuple("Str").field(s).finish(),
            Value::Object(o) => f.debug_tuple("Object").field(o).finish(),
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
            Value::Str(s) => write!(f, "{s}"),
            Value::Object(o) => {
                write!(f, "{{ ")?;
                for (k, v) in o.iter() {
                    write!(f, "{k} = {v}; ")?;
                }
                write!(f, "}}")
            }
        }
    }
}
