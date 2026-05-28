use genvm_modules::scripting::rat;
use mlua::prelude::*;

fn make_vm() -> Lua {
    let vm = Lua::new();
    rat::register_rat_global(&vm).unwrap();
    vm
}

fn eval<T: mlua::FromLua>(vm: &Lua, code: &str) -> T {
    vm.load(code).eval::<T>().unwrap()
}

#[test]
fn test_new_from_string() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('3/4'))");
    assert_eq!(s, "3/4");
}

#[test]
fn test_new_from_integers() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new(7, 3))");
    assert_eq!(s, "7/3");
}

#[test]
fn test_new_from_single_integer() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new(5))");
    assert_eq!(s, "5");
}

#[test]
fn test_addition() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('1/3') + rat.new('1/6'))");
    assert_eq!(s, "1/2");
}

#[test]
fn test_subtraction() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('1/2') - rat.new('1/3'))");
    assert_eq!(s, "1/6");
}

#[test]
fn test_multiplication() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('2/3') * rat.new('3/4'))");
    assert_eq!(s, "1/2");
}

#[test]
fn test_division() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('2/3') / rat.new('4/5'))");
    assert_eq!(s, "5/6");
}

#[test]
fn test_unary_minus() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(-rat.new('3/4'))");
    assert_eq!(s, "-3/4");
}

#[test]
fn test_equality() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new('2/4') == rat.new('1/2')");
    assert!(b);
}

#[test]
fn test_less_than() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new('1/3') < rat.new('1/2')");
    assert!(b);
}

#[test]
fn test_less_equal() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new('1/2') <= rat.new('1/2')");
    assert!(b);
}

#[test]
fn test_greater_than() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new('2/3') > rat.new('1/3')");
    assert!(b);
}

#[test]
fn test_numer_denom() {
    let vm = make_vm();
    let n: String = eval(&vm, "return rat.new('6/4'):numer()");
    assert_eq!(n, "3");
    let d: String = eval(&vm, "return rat.new('6/4'):denom()");
    assert_eq!(d, "2");
}

#[test]
fn test_to_float() {
    let vm = make_vm();
    let f: f64 = eval(&vm, "return rat.new('1/4'):to_float()");
    assert!((f - 0.25).abs() < 1e-10);
}

#[test]
fn test_abs() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('-3/4'):abs())");
    assert_eq!(s, "3/4");
}

#[test]
fn test_sign() {
    let vm = make_vm();
    let s: i64 = eval(&vm, "return rat.new('-3/4'):sign()");
    assert_eq!(s, -1);
    let s: i64 = eval(&vm, "return rat.new('3/4'):sign()");
    assert_eq!(s, 1);
    let s: i64 = eval(&vm, "return rat.new(0):sign()");
    assert_eq!(s, 0);
}

#[test]
fn test_is_zero() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new(0):is_zero()");
    assert!(b);
    let b: bool = eval(&vm, "return rat.new('1/2'):is_zero()");
    assert!(!b);
}

#[test]
fn test_is_integer() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.new(5):is_integer()");
    assert!(b);
    let b: bool = eval(&vm, "return rat.new('3/2'):is_integer()");
    assert!(!b);
}

#[test]
fn test_floor_ceil_trunc() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('7/3'):floor())");
    assert_eq!(s, "2");
    let s: String = eval(&vm, "return tostring(rat.new('7/3'):ceil())");
    assert_eq!(s, "3");
    let s: String = eval(&vm, "return tostring(rat.new('-7/3'):trunc())");
    assert_eq!(s, "-2");
}

#[test]
fn test_is_rat() {
    let vm = make_vm();
    let b: bool = eval(&vm, "return rat.is_rat(rat.new(1))");
    assert!(b);
    let b: bool = eval(&vm, "return rat.is_rat(42)");
    assert!(!b);
}

#[test]
fn test_arithmetic_with_integer() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('1/2') + 1)");
    assert_eq!(s, "3/2");
}

#[test]
fn test_division_by_zero() {
    let vm = make_vm();
    let res = vm
        .load("return rat.new('1/2') / rat.new(0)")
        .eval::<LuaValue>();
    assert!(res.is_err());
}

#[test]
fn test_zero_denominator() {
    let vm = make_vm();
    let res = vm.load("return rat.new(1, 0)").eval::<LuaValue>();
    assert!(res.is_err());
}

#[test]
fn test_big_numbers() {
    let vm = make_vm();
    let s: String = eval(
        &vm,
        "return tostring(rat.new('999999999999999999999999999999/1000000000000000000000000000000'))",
    );
    assert_eq!(
        s,
        "999999999999999999999999999999/1000000000000000000000000000000"
    );
}

#[test]
fn test_modulo() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('7/2') % rat.new('3/2'))");
    assert_eq!(s, "1/2");
}

#[test]
fn test_pow_positive() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('2/3'):pow(3))");
    assert_eq!(s, "8/27");
}

#[test]
fn test_pow_zero() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('2/3'):pow(0))");
    assert_eq!(s, "1");
}

#[test]
fn test_pow_negative() {
    let vm = make_vm();
    let s: String = eval(&vm, "return tostring(rat.new('2/3'):pow(-2))");
    assert_eq!(s, "9/4");
}

#[test]
fn test_pow_negative_zero_base() {
    let vm = make_vm();
    let res = vm.load("return rat.new(0):pow(-1)").eval::<LuaValue>();
    assert!(res.is_err());
}

#[test]
fn test_pow_zero_to_zero() {
    let vm = make_vm();
    let res = vm.load("return rat.new(0):pow(0)").eval::<LuaValue>();
    assert!(res.is_err());
}
