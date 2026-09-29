//! The power rule backwards: int x^n dx = x^(n+1)/(n+1) for n != -1, with
//! its edge cases: int 1/x dx = ln|x| and int k dx = kx.

use super::int_linear::{scaled, split_const};
use super::powers::power_of;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Func, Math};
use crate::q::Q;

pub struct IntPower;

/// The exponent n when h is u^n in any of its written forms: u, u^n,
/// sqrt(u), 1/u, 1/u^n, 1/sqrt(u).
pub fn power_form(h: &Expr, u: &Expr) -> Option<Q> {
    match h {
        x if x == u => Some(Q::ONE),
        Expr::Pow(b, n) if **b == *u => n.as_num(),
        Expr::Func(Func::Sqrt, b) if **b == *u => Q::new(1, 2),
        Expr::Div(one, d) if one.is_num(1) => power_form(d, u).map(|n| n.neg()),
        _ => None,
    }
}

/// ln|u|
pub fn log_abs(u: &Expr) -> Expr {
    expr::func(Func::Ln, expr::func(Func::Abs, u.clone()))
}

/// int c u^n du for n != -1, written as a textbook does: x^3/3,
/// 2x^(3/2)/3, -1/x, -1/(2x^2).
pub fn reverse_power(c: Q, u: &Expr, n: Q) -> Option<Expr> {
    let m = n.add(&Q::ONE)?;
    if m.is_zero() {
        return None;
    }
    let k = c.div(&m)?;
    if !m.is_neg() {
        return Some(scaled(k, power_of(u.clone(), m)));
    }
    // k u^m with m < 0: k / u^(-m)
    let den = with_coeff(Q::int(k.den()), power_of(u.clone(), m.neg()));
    let q = expr::div(expr::num(k.num().abs()), den);
    Some(if k.is_neg() { expr::neg(q) } else { q })
}

impl Rule for IntPower {
    fn name(&self) -> &'static str {
        "int_power"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["power", "constant_multiple", "constant", "reciprocal"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_power", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let x = expr::var(v);
            if !inner.has_var(v) {
                let new = match inner.as_num() {
                    Some(k) => with_coeff(k, x),
                    None => expr::mul(vec![(**inner).clone(), x]),
                };
                let says = Line::new().t("The integral of a constant: ").e(e).t(" = ").e(&new).t(".");
                return vec![Rewrite { variant: "constant", new, says, work: vec![] }];
            }
            let (c, h) = split_const(inner);
            let Some(n) = power_form(&h, &x) else { return vec![] };
            if n == Q::int(-1) {
                let new = scaled(c, log_abs(&x));
                let base = Expr::Integral(Box::new(expr::div(expr::num(1), x.clone())), v.clone());
                let says = Line::new().t("Standard integral: ").e(&base).t(" = ").e(&log_abs(&x)).t(".");
                return vec![Rewrite { variant: "reciprocal", new, says, work: vec![] }];
            }
            let Some(new) = reverse_power(c, &x, n) else { return vec![] };
            let variant = if c.is_one() { "power" } else { "constant_multiple" };
            let says = Line::new().t("Power rule for integrals: raise the power by 1 and divide by the new power.");
            vec![Rewrite { variant, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn reverse_power_rule() {
        assert_eq!(test_moves(&IntPower, "integrate x^2"), vec!["x^3/3"]);
        assert_eq!(test_moves(&IntPower, "integrate 6x^2"), vec!["2x^3"]);
        assert_eq!(test_moves(&IntPower, "integrate 1/x^2"), vec!["-1/x"]);
        assert_eq!(test_moves(&IntPower, "integrate sqrt x"), vec!["2x^(3/2)/3"]);
        assert_eq!(test_moves(&IntPower, "integrate 3/x"), vec!["3ln|x|"]);
        assert_eq!(test_moves(&IntPower, "integrate 5"), vec!["5x"]);
        assert!(test_moves(&IntPower, "integrate sin x").is_empty());
    }
}
