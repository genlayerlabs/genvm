use std::rc::Rc;

use num_traits::Zero;

use super::value::{BinOp, EvalError, Expr, Value};

#[derive(Clone)]
struct EvalContext {
    get_var: Rc<dyn Fn(&str) -> Result<Value, EvalError>>,
    let_bindings: rpds::RedBlackTreeMap<String, Value, archery::RcK>,
}

fn eval(expr: &Expr, ctx: &EvalContext) -> Result<Value, EvalError> {
    match expr {
        Expr::Const(n) => Ok(Value::Rational(n.clone())),
        Expr::Ident(name) => {
            if let Some(val) = ctx.let_bindings.get(name.as_str()) {
                Ok(val.clone())
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
            let val = eval(value, ctx)?;
            let new_ctx = EvalContext {
                get_var: ctx.get_var.clone(),
                let_bindings: ctx.let_bindings.insert(name.clone(), val),
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
            let a = eval(arg, ctx)?;
            apply(&f, &a, ctx)
        }
    }
}

fn apply(f: &Value, arg: &Value, ctx: &EvalContext) -> Result<Value, EvalError> {
    match f {
        Value::HostFn(f) => f(arg),
        Value::GuestFn { name, body, env } => {
            let new_ctx = EvalContext {
                get_var: ctx.get_var.clone(),
                let_bindings: env.insert(name.to_string(), arg.clone()),
            };
            eval(body, &new_ctx)
        }
        other => Err(EvalError::TypeError {
            expected: "function",
            got: other.typ(),
        }),
    }
}

impl Expr {
    pub fn evaluate(&self) -> Result<Value, EvalError> {
        self.evaluate_with(&|name: &str| Err(EvalError::UndefinedVariable(name.to_owned())))
    }

    pub fn evaluate_with(
        &self,
        get_var: impl Fn(&str) -> Result<Value, EvalError> + 'static,
    ) -> Result<Value, EvalError> {
        eval(
            self,
            &EvalContext {
                get_var: Rc::new(get_var),
                let_bindings: Default::default(),
            },
        )
    }
}
