use genvm_common::expr::{EvalError, Expr, Value};
use num_bigint::BigInt;
use num_rational::BigRational;

fn eval(input: &str) -> Value {
    Expr::parse(input).unwrap().evaluate().unwrap()
}

fn assert_rational(val: Value, n: i64, d: i64) {
    let r = val.into_rational().unwrap();
    assert_eq!(r, BigRational::new(BigInt::from(n), BigInt::from(d)));
}

fn assert_bool(val: Value, expected: bool) {
    assert_eq!(val.is_truthy().unwrap(), expected);
}

#[test]
fn arithmetic() {
    assert_rational(eval("2 + 3 * 4"), 14, 1);
    assert_rational(eval("(2 + 3) * 4"), 20, 1);
    assert_rational(eval("10 - 3 - 2"), 5, 1);
}

#[test]
fn rational_division() {
    assert_rational(eval("1 / 3 + 1 / 6"), 1, 2);
}

#[test]
fn decimal_literal() {
    assert_rational(eval("2 * 0.5"), 1, 1);
}

#[test]
fn scientific_notation() {
    assert_rational(eval("1e9"), 1_000_000_000, 1);
}

#[test]
fn unary_minus() {
    assert_rational(eval("-5 + 3"), -2, 1);
    assert_rational(eval("-(2 + 3)"), -5, 1);
}

#[test]
fn comparisons() {
    assert_bool(eval("1 < 2"), true);
    assert_bool(eval("2 < 1"), false);
    assert_bool(eval("3 == 3"), true);
    assert_bool(eval("3 == 4"), false);
    assert_bool(eval("3 != 4"), true);
    assert_bool(eval("3 != 3"), false);
    assert_bool(eval("5 >= 5"), true);
    assert_bool(eval("4 >= 5"), false);
    assert_bool(eval("3 <= 4"), true);
    assert_bool(eval("3 > 2"), true);
}

#[test]
fn let_expr() {
    assert_rational(eval("let x =10 in x + 5"), 15, 1);
    assert_rational(eval("let x =2 in let y =3 in x * y"), 6, 1);
}

#[test]
fn let_nested() {
    assert_rational(eval("let x =10 in (let x =3 in x) + x"), 13, 1);
}

#[test]
fn if_expr() {
    assert_rational(eval("if 1 < 2 then 10 else 20"), 10, 1);
    assert_rational(eval("if 2 < 1 then 10 else 20"), 20, 1);
    assert_rational(eval("if 3 < 5 then 100 else 200"), 100, 1);
}

#[test]
fn combined() {
    assert_rational(eval("let x =7 in if x > 5 then x * 2 else x + 1"), 14, 1);
}

#[test]
fn division_by_zero() {
    let expr = Expr::parse("1 / 0").unwrap();
    assert!(matches!(expr.evaluate(), Err(EvalError::DivisionByZero)));
}

#[test]
fn undefined_variable() {
    let expr = Expr::parse("x + 1").unwrap();
    assert!(matches!(
        expr.evaluate(),
        Err(EvalError::UndefinedVariable(_))
    ));
}

#[test]
fn parse_error() {
    assert!(Expr::parse("1 @").is_err());
}

#[test]
fn parse_error_paren_close() {
    assert!(Expr::parse("1)").is_err());
}

#[test]
fn parse_error_paren_open() {
    assert!(Expr::parse("(1").is_err());
}

#[test]
fn lambda_identity() {
    assert_rational(eval(r"(\x = x) 42"), 42, 1);
}

#[test]
fn lambda_closure() {
    assert_rational(eval(r"let add1 = \x = x + 1 in add1 5"), 6, 1);
}

#[test]
fn church_numerals() {
    assert_rational(
        eval(
            r"
            let zero = \f = \x = x in
            let succ = \n = \f = \x = f (n f x) in
            let to_int = \n = n (\x = x + 1) 0 in
            let one = succ zero in
            let two = succ one in
            let three = succ two in
            to_int three
        ",
        ),
        3,
        1,
    );
}

#[test]
fn church_add() {
    assert_rational(
        eval(
            r"
            let zero = \f = \x = x in
            let succ = \n = \f = \x = f (n f x) in
            let add = \m = \n = \f = \x = m f (n f x) in
            let to_int = \n = n (\x = x + 1) 0 in
            let two = succ (succ zero) in
            let three = succ (succ (succ zero)) in
            to_int (add two three)
        ",
        ),
        5,
        1,
    );
}

#[test]
fn church_mul() {
    assert_rational(
        eval(
            r"
            let zero = \f = \x = x in
            let succ = \n = \f = \x = f (n f x) in
            let mul = \m = \n = \f = m (n f) in
            let to_int = \n = n (\x = x + 1) 0 in
            let three = succ (succ (succ zero)) in
            let four = succ (succ (succ (succ zero))) in
            to_int (mul three four)
        ",
        ),
        12,
        1,
    );
}

#[test]
fn church_booleans() {
    assert_rational(
        eval(
            r"
            let tru = \a = \b = a in
            let fls = \a = \b = b in
            let and = \p = \q = p q p in
            let to_int = \b = b 1 0 in
            to_int (and tru tru)
        ",
        ),
        1,
        1,
    );
    assert_rational(
        eval(
            r"
            let tru = \a = \b = a in
            let fls = \a = \b = b in
            let and = \p = \q = p q p in
            let to_int = \b = b 1 0 in
            to_int (and tru fls)
        ",
        ),
        0,
        1,
    );
}

#[test]
fn church_pairs() {
    assert_rational(
        eval(
            r"
            let pair = \a = \b = \f = f a b in
            let fst = \p = p (\a = \b = a) in
            let snd = \p = p (\a = \b = b) in
            let p = pair 3 7 in
            fst p + snd p
        ",
        ),
        10,
        1,
    );
}
