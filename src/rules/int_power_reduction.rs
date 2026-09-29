//! Power reduction for integrals: a square of sin or cos is not in the
//! table, but the double angle turns it into one that is:
//! cos^2 u = 1/2 + cos(2u)/2, sin^2 u = 1/2 - cos(2u)/2.

use super::int_linear::{scaled, split_const};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Math};
use crate::q::Q;

pub struct IntPowerReduction;

impl Rule for IntPowerReduction {
    fn name(&self) -> &'static str {
        "int_power_reduction"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["cos_squared", "sin_squared"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_power_reduction", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let (c, h) = split_const(inner);
            let Expr::Pow(b, n) = &h else { return vec![] };
            let Expr::Func(f @ (Func::Sin | Func::Cos), u) = &**b else { return vec![] };
            if !n.is_num(2) || !u.has_var(v) {
                return vec![];
            }
            let (Some(half), Some(k)) = (c.div(&Q::int(2)), Q::new(1, 2)) else { return vec![] };
            let cos2 = expr::func(Func::Cos, expr::mul(vec![expr::num(2), (**u).clone()]));
            let second = scaled(half, cos2);
            let (variant, new, rule) = match f {
                Func::Cos => ("cos_squared", expr::add(vec![Expr::Num(c.mul(&k).unwrap_or(k)), second]), "cos^2 u = 1/2 + cos(2u)/2"),
                _ => ("sin_squared", expr::add(vec![Expr::Num(c.mul(&k).unwrap_or(k)), expr::neg(second)]), "sin^2 u = 1/2 - cos(2u)/2"),
            };
            let says = Line::new().t(format!("Reduce the power with the double angle: {rule}."));
            vec![Rewrite { variant, new: Expr::Integral(Box::new(expr::tidy(new)), v.clone()), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn reduces_squares() {
        assert_eq!(test_moves(&IntPowerReduction, "integrate cos^2 x"), vec!["int (1/2 + cos(2x)/2) dx"]);
        assert_eq!(test_moves(&IntPowerReduction, "integrate sin^2 x"), vec!["int (1/2 - cos(2x)/2) dx"]);
        assert!(test_moves(&IntPowerReduction, "integrate cos x").is_empty());
    }
}
