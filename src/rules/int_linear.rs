//! Linearity of the integral: the integral of a sum is the sum of the
//! integrals, and a constant factor stays outside:
//! int (3x^2 + cos x) dx = int 3x^2 dx + int cos(x) dx, int 5cos(x) dx = 5 int cos(x) dx.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};
use crate::q::Q;

pub struct IntLinear;

/// A term's number factor and the rest, also through a denominator:
/// 3x^2 -> (3, x^2), x/2 -> (1/2, x), -1/(2(x - 1)) -> (-1/2, 1/(x - 1)).
pub fn split_const(e: &Expr) -> (Q, Expr) {
    match e {
        Expr::Neg(a) => {
            let (c, r) = split_const(a);
            (c.neg(), r)
        }
        Expr::Div(a, b) if b.as_num().is_none() => {
            let (ca, ra) = split_const(a);
            let (cb, rb) = coeff(b);
            match ca.div(&cb) {
                Some(c) => (c, expr::div(ra, rb)),
                None => (Q::ONE, e.clone()),
            }
        }
        _ => coeff(e),
    }
}

/// A number times an expression, written as a textbook would: 3sin(x),
/// x^3/3, -cos(2x)/2, 1/(2x).
pub fn scaled(c: Q, body: Expr) -> Expr {
    if c.is_int() {
        return with_coeff(c, body);
    }
    if c.is_neg() {
        return expr::neg(scaled(c.neg(), body));
    }
    let top = |n: Expr| with_coeff(Q::int(c.num()), n);
    match body {
        Expr::Div(n, d) => expr::div(top(*n), with_coeff(Q::int(c.den()), *d)),
        b => expr::div(top(b), Expr::Num(Q::int(c.den()))),
    }
}

fn int(e: Expr, v: &str) -> Expr {
    Expr::Integral(Box::new(e), v.to_string())
}

impl Rule for IntLinear {
    fn name(&self) -> &'static str {
        "int_linear"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum", "constant"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_linear", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            match &**inner {
                Expr::Add(ts) => {
                    let new = Expr::Add(ts.iter().map(|t| int(t.clone(), v)).collect());
                    vec![Rewrite { variant: "sum", new, says: Line::new().t("Integrate term by term."), work: vec![] }]
                }
                Expr::Mul(fs) if fs.iter().any(|f| !f.has_var(v)) && fs.iter().any(|f| f.has_var(v)) => {
                    let mut c: Vec<Expr> = fs.iter().filter(|f| !f.has_var(v)).cloned().collect();
                    let rest = expr::mul(fs.iter().filter(|f| f.has_var(v)).cloned().collect());
                    let k = expr::mul(c.clone());
                    c.push(int(rest, v));
                    vec![Rewrite { variant: "constant", new: Expr::Mul(c), says: Line::new().t("The constant factor ").e(&k).t(" comes outside the integral."), work: vec![] }]
                }
                Expr::Neg(a) if a.has_var(v) => vec![Rewrite { variant: "constant", new: expr::neg(int((**a).clone(), v)), says: Line::new().t("The minus sign comes outside the integral."), work: vec![] }],
                Expr::Div(a, b) if !b.has_var(v) && a.has_var(v) => {
                    vec![Rewrite { variant: "constant", new: expr::div(int((**a).clone(), v), (**b).clone()), says: Line::new().t("Dividing by ").e(b).t(" comes outside the integral."), work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn splits_sums_and_constants() {
        assert_eq!(test_moves(&IntLinear, "integrate x^2 + cos x"), vec!["int x^2 dx + int cos(x) dx"]);
        assert_eq!(test_moves(&IntLinear, "integrate 5cos x"), vec!["5 * int cos(x) dx"]);
        assert!(test_moves(&IntLinear, "integrate sin x").is_empty());
        let cos = expr::func(crate::expr::Func::Cos, expr::var("x"));
        assert_eq!(crate::print::expr(&scaled(Q::new(-1, 2).unwrap(), cos), crate::print::Style::Ascii), "-cos(x)/2");
    }
}
