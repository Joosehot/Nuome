//! The power rule d/dx[x^n] = n x^(n-1), with its two edge cases: d/dx[x] = 1
//! and the derivative of a constant is 0.

use super::powers::power_of;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};
use crate::q::Q;

pub struct DiffPower;

impl Rule for DiffPower {
    fn name(&self) -> &'static str {
        "diff_power"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["power", "constant_multiple", "constant"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_power", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            if inner.has_deriv() {
                return vec![]; // a higher derivative: the inner one first
            }
            let x = expr::var(v);
            if !inner.has_var(v) {
                return vec![Rewrite { variant: "constant", new: expr::num(0), says: Line::new().t("The derivative of a constant is 0."), work: vec![] }];
            }
            // c * x^n, with c = 1 and n = 1 as special cases
            let (c, rest) = coeff(inner);
            let n = match &rest {
                r if *r == x => Q::ONE,
                Expr::Pow(b, n) if **b == x => match n.as_num() {
                    Some(n) => n,
                    None => return vec![],
                },
                _ => return vec![],
            };
            let (Some(k), Some(n1)) = (c.mul(&n), n.sub(&Q::ONE)) else { return vec![] };
            let new = with_coeff(k, power_of(x, n1));
            let new = if n1.is_zero() { Expr::Num(k) } else { new };
            let variant = if c.is_one() { "power" } else { "constant_multiple" };
            vec![Rewrite { variant, new, says: Line::new().t(format!("Power rule: d/d{v}[{v}^n] = n {v}^(n-1).")), work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn power_rule() {
        assert_eq!(test_moves(&DiffPower, "differentiate x^3"), vec!["3x^2"]);
        assert_eq!(test_moves(&DiffPower, "differentiate 7"), vec!["0"]);
        assert_eq!(test_moves(&DiffPower, "differentiate -2x^2"), vec!["-4x"]);
        assert!(test_moves(&DiffPower, "differentiate sin(x)").is_empty());
    }
}
