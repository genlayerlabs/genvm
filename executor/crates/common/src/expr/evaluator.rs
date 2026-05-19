use std::sync::Arc;

use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};

use super::value::{BinOp, EvalError, Expr, StrSeg, Thunk, Value};

// SAFETY: this evaluator has no recursion depth or fuel limits.
// It is only used for trusted, operator-supplied fee config expressions,
// never for contract-supplied or user-supplied input.

#[derive(Clone)]
struct EvalContext {
    get_var: Arc<dyn Fn(&str) -> Result<Value, EvalError> + Send + Sync>,
    let_bindings: rpds::RedBlackTreeMap<String, Thunk, archery::ArcK>,
}

/// Builds a call-by-need thunk that evaluates `expr` in `ctx` when first forced.
fn thunk_of(expr: &Expr, ctx: &EvalContext) -> Thunk {
    let expr = expr.clone();
    let ctx = ctx.clone();
    Thunk::deferred(move || eval(&expr, &ctx))
}

fn eval(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::Const(n) => Ok(Value::Rational(n.clone())),
        Expr::Ident(name) => {
            if let Some(thunk) = ctx.let_bindings.get(name.as_str()) {
                thunk.force()
            } else if let Some(builtin) = resolve_builtin(name) {
                Ok(builtin)
            } else {
                (ctx.get_var)(name)
            }
        }
        Expr::UnaryMinus(e) => {
            let val = eval(e, ctx)?.into_rational()?;
            Ok(Value::Rational(-val))
        }
        Expr::BinOp { op, lhs, rhs } => {
            let l = eval(lhs, ctx)?;
            let r = eval(rhs, ctx)?;
            match op {
                BinOp::Add => Ok(Value::Rational(l.into_rational()? + r.into_rational()?)),
                BinOp::Sub => Ok(Value::Rational(l.into_rational()? - r.into_rational()?)),
                BinOp::Mul => Ok(Value::Rational(l.into_rational()? * r.into_rational()?)),
                BinOp::Div => {
                    let r = r.into_rational()?;
                    if r.is_zero() {
                        return Err(EvalError::DivisionByZero);
                    }
                    Ok(Value::Rational(l.into_rational()? / r))
                }
                BinOp::Lt => Ok(Value::Bool(l.into_rational()? < r.into_rational()?)),
                BinOp::Gt => Ok(Value::Bool(l.into_rational()? > r.into_rational()?)),
                BinOp::Le => Ok(Value::Bool(l.into_rational()? <= r.into_rational()?)),
                BinOp::Ge => Ok(Value::Bool(l.into_rational()? >= r.into_rational()?)),
                BinOp::Eq => Ok(Value::Bool(l.into_rational()? == r.into_rational()?)),
                BinOp::Ne => Ok(Value::Bool(l.into_rational()? != r.into_rational()?)),
            }
        }
        Expr::Let { name, value, body } => {
            // Non-recursive: the binding's thunk closes over `ctx` *without*
            // itself, so `let` cannot observe its own value. Recursion is
            // expressed with the Y combinator instead.
            let thunk = thunk_of(value, ctx);
            let new_ctx = EvalContext {
                get_var: ctx.get_var.clone(),
                let_bindings: ctx.let_bindings.insert(name.clone(), thunk),
            };
            eval(body, &new_ctx)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let c = eval(cond, ctx)?;
            if c.is_truthy()? {
                eval(then_branch, ctx)
            } else {
                eval(else_branch, ctx)
            }
        }
        Expr::Lambda { param, body } => Ok(Value::GuestFn {
            name: param.clone(),
            body: body.clone(),
            env: ctx.let_bindings.clone(),
        }),
        Expr::Apply { func, arg } => {
            let f = eval(func, ctx)?;
            // Call-by-need: the argument is passed unevaluated as a thunk and
            // forced only if (and when) the callee actually demands it.
            let arg = thunk_of(arg, ctx);
            apply(&f, arg, ctx)
        }
        Expr::Array(elems) => {
            let vals: Result<Vec<Value>, _> = elems.iter().map(|e| eval(e, ctx)).collect();
            Ok(Value::Array(Arc::new(vals?)))
        }
        Expr::Str(segs) => {
            let mut out = String::new();
            for seg in segs {
                match seg {
                    StrSeg::Lit(s) => out.push_str(s),
                    StrSeg::Interp(e) => out.push_str(&stringify(&eval(e, ctx)?)),
                }
            }
            Ok(Value::Str(out.into()))
        }
        Expr::Object(fields) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in fields {
                map.insert(k.clone(), eval(v, ctx)?);
            }
            Ok(Value::Object(Arc::new(map)))
        }
        Expr::Field { obj, key } => {
            let obj = eval(obj, ctx)?;
            obj.as_object()?
                .get(key)
                .cloned()
                .ok_or_else(|| EvalError::Custom(format!("object has no key `{key}`")))
        }
    }
}

/// Renders a value to its string form (used by `toString` and `\(..)`).
fn stringify(v: &Value) -> String {
    match v {
        Value::Str(s) => s.to_string(),
        other => other.to_string(),
    }
}

fn apply(f: &Value, arg: Thunk, ctx: &EvalContext) -> Result<Value, EvalError> {
    match f {
        // Builtins are strict in their argument.
        Value::HostFn(host) => host(&arg.force()?),
        Value::GuestFn { name, body, env } => {
            let new_ctx = EvalContext {
                get_var: ctx.get_var.clone(),
                let_bindings: env.insert(name.to_string(), arg),
            };
            eval(body, &new_ctx)
        }
        other => Err(EvalError::TypeError {
            expected: "function",
            got: other.typ(),
        }),
    }
}

fn resolve_builtin(name: &str) -> Option<Value> {
    match name {
        "floor" => Some(Value::HostFn(Arc::new(|arg: &Value| {
            Ok(Value::Rational(arg.clone().into_rational()?.floor()))
        }))),
        "toString" => Some(Value::HostFn(Arc::new(|arg: &Value| {
            Ok(Value::Str(stringify(arg).into()))
        }))),
        "hasKey" => Some(Value::HostFn(Arc::new(|obj: &Value| {
            let obj = obj.as_object()?.clone();
            Ok(Value::HostFn(Arc::new(move |key: &Value| {
                Ok(Value::Bool(obj.contains_key(key.as_str()?)))
            })))
        }))),
        "arrayLen" => Some(Value::HostFn(Arc::new(|arg: &Value| match arg {
            Value::Array(elems) => Ok(Value::Rational(BigRational::from_integer(
                elems.len().into(),
            ))),
            other => Err(EvalError::TypeError {
                expected: "array",
                got: other.typ(),
            }),
        }))),
        "arrayGetElem" => Some(Value::HostFn(Arc::new(|arr: &Value| match arr {
            Value::Array(elems) => {
                let elems = elems.clone();
                Ok(Value::HostFn(Arc::new(move |idx: &Value| {
                    let i = idx
                        .clone()
                        .into_rational()?
                        .to_integer()
                        .to_usize()
                        .ok_or_else(|| {
                            EvalError::Custom(format!("array index out of range: {idx}"))
                        })?;
                    elems.get(i).cloned().ok_or_else(|| {
                        EvalError::Custom(format!(
                            "array index {i} out of bounds for length {}",
                            elems.len()
                        ))
                    })
                })))
            }
            other => Err(EvalError::TypeError {
                expected: "array",
                got: other.typ(),
            }),
        }))),
        _ => None,
    }
}

impl Value {
    /// Applies this value as a function to `arg`. Free variables in a guest
    /// function's body are resolved via `get_var` (e.g. host-provided `node`).
    pub fn apply_with(
        &self,
        arg: Value,
        get_var: impl Fn(&str) -> Result<Value, EvalError> + Send + Sync + 'static,
    ) -> Result<Value, EvalError> {
        let ctx = EvalContext {
            get_var: Arc::new(get_var),
            let_bindings: Default::default(),
        };
        apply(self, Thunk::forced(arg), &ctx)
    }
}

impl Expr {
    pub fn evaluate(&self) -> Result<Value, EvalError> {
        self.evaluate_with(&|name: &str| Err(EvalError::UndefinedVariable(name.to_owned())))
    }

    pub fn evaluate_with(
        &self,
        get_var: impl Fn(&str) -> Result<Value, EvalError> + Send + Sync + 'static,
    ) -> Result<Value, EvalError> {
        eval(
            self,
            &EvalContext {
                get_var: Arc::new(get_var),
                let_bindings: Default::default(),
            },
        )
    }
}
