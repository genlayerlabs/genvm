use genvm_common::expr::{EvalError, Expr};
use num_bigint::BigInt;
use num_rational::BigRational;

fn eval(input: &str) -> BigRational {
    Expr::parse(input).unwrap().evaluate().unwrap()
}

fn ratio(n: i64, d: i64) -> BigRational {
    BigRational::new(BigInt::from(n), BigInt::from(d))
}

#[test]
fn arithmetic() {
    assert_eq!(eval("2 + 3 * 4"), ratio(14, 1));
    assert_eq!(eval("(2 + 3) * 4"), ratio(20, 1));
    assert_eq!(eval("10 - 3 - 2"), ratio(5, 1));
}

#[test]
fn rational_division() {
    assert_eq!(eval("1 / 3 + 1 / 6"), ratio(1, 2));
}

#[test]
fn decimal_literal() {
    assert_eq!(eval("2 * 0.5"), ratio(1, 1));
}

#[test]
fn scientific_notation() {
    assert_eq!(eval("1e9"), ratio(1_000_000_000, 1));
}

#[test]
fn unary_minus() {
    assert_eq!(eval("-5 + 3"), ratio(-2, 1));
    assert_eq!(eval("-(2 + 3)"), ratio(-5, 1));
}

#[test]
fn comparisons() {
    assert_eq!(eval("1 < 2"), ratio(1, 1));
    assert_eq!(eval("2 < 1"), ratio(0, 1));
    assert_eq!(eval("3 == 3"), ratio(1, 1));
    assert_eq!(eval("3 == 4"), ratio(0, 1));
    assert_eq!(eval("3 != 4"), ratio(1, 1));
    assert_eq!(eval("3 != 3"), ratio(0, 1));
    assert_eq!(eval("5 >= 5"), ratio(1, 1));
    assert_eq!(eval("4 >= 5"), ratio(0, 1));
    assert_eq!(eval("3 <= 4"), ratio(1, 1));
    assert_eq!(eval("3 > 2"), ratio(1, 1));
}

#[test]
fn let_expr() {
    assert_eq!(eval("let x =10 in x + 5"), ratio(15, 1));
    assert_eq!(eval("let x =2 in let y =3 in x * y"), ratio(6, 1));
}

#[test]
fn let_nested() {
    assert_eq!(eval("let x =10 in (let x =3 in x) + x"), ratio(13, 1));
}

#[test]
fn if_expr() {
    assert_eq!(eval("if 1 then 10 else 20"), ratio(10, 1));
    assert_eq!(eval("if 0 then 10 else 20"), ratio(20, 1));
    assert_eq!(eval("if 3 < 5 then 100 else 200"), ratio(100, 1));
}

#[test]
fn combined() {
    assert_eq!(
        eval("let x =7 in if x > 5 then x * 2 else x + 1"),
        ratio(14, 1)
    );
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
