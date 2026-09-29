//! Partial fractions: a proper fraction whose denominator splits into
//! distinct linear factors is a sum of simple fractions, each with a
//! logarithm for its integral: 1/(x^2 - 1) = 1/(2(x - 1)) - 1/(2(x + 1)).
//! The numerators come from the cover-up rule: A = P(r)/Q'(r) at each root r.

use super::int_linear::split_const;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct IntPartialFractions;

/// The linear factor with root r = p/q, with whole numbers: qx - p.
fn linear_factor(r: &Q, v: &str) -> Expr {
    Poly(vec![r.num().checked_neg().map_or(Q::ZERO, Q::int), Q::int(r.den())]).to_expr(v)
}

impl Rule for IntPartialFractions {
    fn name(&self) -> &'static str {
        "int_partial_fractions"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["distinct_linear"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_partial_fractions", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let (c, h) = split_const(inner);
            let Expr::Div(..) = h else { return vec![] };
            let Some((n, d)) = poly::rational_from_expr(&h, v) else { return vec![] };
            let (Some(dn), Some(dd)) = (n.deg().or(Some(0)), d.deg()) else { return vec![] };
            if dd < 2 || dn >= dd || n.is_zero() {
                return vec![];
            }
            // largest root first: 1/(x - 1) before 1/(x + 1)
            let mut roots = d.rational_roots();
            roots.reverse();
            if roots.len() != dd {
                return vec![];
            }
            let dprime: Option<Vec<Q>> = d.0.iter().enumerate().skip(1).map(|(i, k)| k.mul(&Q::int(i as i128))).collect();
            let Some(dprime) = dprime.map(Poly) else { return vec![] };
            let mut terms = Vec::new();
            let mut cover = Vec::new();
            for r in &roots {
                // A/(x - r) = Aq/(qx - p)
                let Some(a) = n.eval(r).and_then(|top| top.div(&dprime.eval(r)?)) else { return vec![] };
                let Some(k) = a.mul(&Q::int(r.den())).and_then(|k| k.mul(&c)) else { return vec![] };
                cover.push(format!("{a} at {v} = {r}"));
                if k.is_zero() {
                    continue;
                }
                let t = expr::div(expr::num(k.num().abs()), with_coeff(Q::int(k.den()), linear_factor(r, v)));
                terms.push(if k.is_neg() { expr::neg(t) } else { t });
            }
            let sum = expr::add(terms);
            let new = Expr::Integral(Box::new(sum.clone()), v.clone());
            let says = Line::new().t("Split into partial fractions: the denominator has the distinct roots ").t(roots.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", ")).t(".");
            let work = vec![
                Line::new().t(format!("cover-up, A = P(r)/Q'(r) at each root r: {}", cover.join(", "))),
                Line::new().e(&expr::with_coeff(c, h.clone())).t(" = ").e(&sum),
            ];
            vec![Rewrite { variant: "distinct_linear", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn splits_into_partial_fractions() {
        assert_eq!(test_moves(&IntPartialFractions, "integrate 1/(x^2 - 1)"), vec!["int (1/(2(x - 1)) - 1/(2(x + 1))) dx"]);
        assert!(test_moves(&IntPartialFractions, "integrate 1/(x^2 + 1)").is_empty());
    }
}
