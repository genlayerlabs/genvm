use mlua::prelude::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct LuaRat(pub BigRational);

impl LuaRat {
    fn from_args(vm: &Lua, args: LuaMultiValue) -> LuaResult<Self> {
        let mut iter = args.into_iter();
        let first = iter
            .next()
            .ok_or_else(|| LuaError::external("expected at least one argument"))?;

        match first {
            LuaValue::String(s) => {
                let s = s.to_str()?;
                let rat = BigRational::from_str(&s).map_err(|e| {
                    LuaError::external(format!("failed to parse rational from string: {e}"))
                })?;
                Ok(LuaRat(rat))
            }
            LuaValue::Integer(n) => {
                let denom = match iter.next() {
                    Some(LuaValue::Integer(d)) => BigInt::from(d),
                    Some(LuaValue::String(s)) => {
                        let s = s.to_str()?;
                        BigInt::from_str(&s).map_err(|e| {
                            LuaError::external(format!("failed to parse denominator: {e}"))
                        })?
                    }
                    Some(v) => {
                        return Err(LuaError::external(format!(
                            "expected integer or string for denominator, got {}",
                            v.type_name()
                        )));
                    }
                    None => BigInt::one(),
                };
                if denom.is_zero() {
                    return Err(LuaError::external("denominator cannot be zero"));
                }
                Ok(LuaRat(BigRational::new(BigInt::from(n), denom)))
            }
            LuaValue::Number(n) => {
                let denom = match iter.next() {
                    Some(LuaValue::Integer(d)) => BigInt::from(d),
                    Some(LuaValue::Number(d)) => BigInt::from(d as i64),
                    Some(LuaValue::String(s)) => {
                        let s = s.to_str()?;
                        BigInt::from_str(&s).map_err(|e| {
                            LuaError::external(format!("failed to parse denominator: {e}"))
                        })?
                    }
                    Some(v) => {
                        return Err(LuaError::external(format!(
                            "expected number or string for denominator, got {}",
                            v.type_name()
                        )));
                    }
                    None => BigInt::one(),
                };
                if denom.is_zero() {
                    return Err(LuaError::external("denominator cannot be zero"));
                }
                Ok(LuaRat(BigRational::new(BigInt::from(n as i64), denom)))
            }
            LuaValue::UserData(ud) => {
                let r: LuaUserDataRef<LuaRat> = ud
                    .borrow()
                    .map_err(|_| LuaError::external("expected Rat userdata"))?;
                Ok(r.clone())
            }
            _ => Err(LuaError::external(format!(
                "Rat: expected string, number, or Rat, got {}",
                first.type_name()
            ))),
        }
    }

    fn extract_other(val: &LuaValue) -> LuaResult<BigRational> {
        match val {
            LuaValue::UserData(ud) => {
                let r: LuaUserDataRef<LuaRat> = ud
                    .borrow()
                    .map_err(|_| LuaError::external("expected Rat userdata"))?;
                Ok(r.0.clone())
            }
            LuaValue::Integer(n) => Ok(BigRational::from(BigInt::from(*n))),
            LuaValue::Number(n) => Ok(BigRational::from(BigInt::from(*n as i64))),
            _ => Err(LuaError::external(format!(
                "expected Rat or number, got {}",
                val.type_name()
            ))),
        }
    }
}

impl LuaUserData for LuaRat {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(LuaRat(&this.0 + &other))
        });

        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(LuaRat(&this.0 - &other))
        });

        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(LuaRat(&this.0 * &other))
        });

        methods.add_meta_method(LuaMetaMethod::Div, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            if other.is_zero() {
                return Err(LuaError::external("division by zero"));
            }
            Ok(LuaRat(&this.0 / &other))
        });

        methods.add_meta_method(LuaMetaMethod::Unm, |_, this, _: ()| {
            Ok(LuaRat(-this.0.clone()))
        });

        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(this.0 == other)
        });

        methods.add_meta_method(LuaMetaMethod::Lt, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(this.0 < other)
        });

        methods.add_meta_method(LuaMetaMethod::Le, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            Ok(this.0 <= other)
        });

        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            Ok(this.0.to_string())
        });

        methods.add_meta_method(LuaMetaMethod::Mod, |_, this, other: LuaValue| {
            let other = LuaRat::extract_other(&other)?;
            if other.is_zero() {
                return Err(LuaError::external("modulo by zero"));
            }
            Ok(LuaRat(&this.0 % &other))
        });

        methods.add_method("numer", |_, this, _: ()| Ok(this.0.numer().to_string()));

        methods.add_method("denom", |_, this, _: ()| Ok(this.0.denom().to_string()));

        methods.add_method("to_float", |_, this, _: ()| {
            let numer = this.0.numer();
            let denom = this.0.denom();
            let f = numer.to_string().parse::<f64>().unwrap_or(f64::INFINITY)
                / denom.to_string().parse::<f64>().unwrap_or(f64::INFINITY);
            Ok(f)
        });

        methods.add_method("abs", |_, this, _: ()| Ok(LuaRat(this.0.abs())));

        methods.add_method("sign", |_, this, _: ()| {
            Ok(this
                .0
                .signum()
                .numer()
                .to_string()
                .parse::<i64>()
                .unwrap_or(0))
        });

        methods.add_method("is_zero", |_, this, _: ()| Ok(this.0.is_zero()));

        methods.add_method("is_integer", |_, this, _: ()| Ok(this.0.is_integer()));

        methods.add_method("floor", |_, this, _: ()| {
            Ok(LuaRat(BigRational::from(this.0.floor().to_integer())))
        });

        methods.add_method("ceil", |_, this, _: ()| {
            Ok(LuaRat(BigRational::from(this.0.ceil().to_integer())))
        });

        methods.add_method("trunc", |_, this, _: ()| {
            Ok(LuaRat(BigRational::from(this.0.trunc().to_integer())))
        });

        methods.add_method("pow", |_, this, exp: i64| {
            if this.0.is_zero() && exp <= 0 {
                return Err(LuaError::external("zero raised to a non-positive power"));
            }
            let abs_exp = exp.unsigned_abs();
            let numer = num_traits::Pow::pow(this.0.numer(), abs_exp);
            let denom = num_traits::Pow::pow(this.0.denom(), abs_exp);
            if exp >= 0 {
                Ok(LuaRat(BigRational::new(numer, denom)))
            } else {
                Ok(LuaRat(BigRational::new(denom, numer)))
            }
        });
    }
}

pub fn lua_rat_to_u256(table: &mlua::Table) -> anyhow::Result<primitive_types::U256> {
    let val: LuaValue = table.get("consumed_gen")?;
    match val {
        LuaValue::UserData(ud) => {
            let r: mlua::UserDataRef<LuaRat> = ud.borrow()?;
            if !r.0.is_integer() {
                anyhow::bail!("consumed_gen must be an integer rational");
            }
            let numer = r.0.numer();
            if numer.sign() == num_bigint::Sign::Minus {
                anyhow::bail!("consumed_gen must be non-negative");
            }
            let (_, bytes) = numer.to_bytes_be();
            if bytes.len() > 32 {
                anyhow::bail!("consumed_gen exceeds U256 range");
            }
            let mut buf = [0u8; 32];
            buf[32 - bytes.len()..].copy_from_slice(&bytes);
            Ok(primitive_types::U256::from_big_endian(&buf))
        }
        LuaValue::Integer(n) => Ok(primitive_types::U256::from(n.max(0) as u64)),
        LuaValue::Number(n) => Ok(primitive_types::U256::from(n.max(0.0) as u64)),
        LuaValue::Nil => Ok(primitive_types::U256::zero()),
        _ => anyhow::bail!("consumed_gen: expected Rat, number, or nil"),
    }
}

pub fn register_rat_global(vm: &Lua) -> anyhow::Result<()> {
    vm.globals()
        .set("rat", create_rat_lib(vm)?)
        .map_err(Into::into)
}

pub fn create_rat_lib(vm: &Lua) -> LuaResult<LuaValue> {
    let rat_table = vm.create_table()?;

    rat_table.set(
        "new",
        vm.create_function(|vm: &Lua, args: LuaMultiValue| LuaRat::from_args(vm, args))?,
    )?;

    rat_table.set(
        "is_rat",
        vm.create_function(|_: &Lua, val: LuaValue| {
            Ok(matches!(
                val,
                LuaValue::UserData(ref ud) if ud.borrow::<LuaRat>().is_ok()
            ))
        })?,
    )?;

    rat_table.set(
        "zero",
        LuaRat(BigRational::new(BigInt::from(0), BigInt::from(1))),
    )?;

    Ok(LuaValue::Table(rat_table))
}
