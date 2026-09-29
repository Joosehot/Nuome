//! Dividing powers of the same base subtracts the exponents:
//! x^5/x^2 = x^3, x^2/x^5 = 1/x^3, 6x^3y/(3xy^2) = 2x^2/y.

use super::powers::{base_exp, power_of};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};
use crate::q::Q;
use std::collections::BTreeMap;

pub struct PowerQuotient;

/// A monomial: (coefficient, letter -> whole-number exponent).
fn monomial(t: &Expr) -> Option<(Q, BTreeMap<String, i128>)> {
    let (c, rest) = coeff(t);
    let factors = match rest {
        Expr::Mul(v) => v,
        r if r.is_num(1) => vec![],
        r => vec![r],
    };
    let mut powers = BTreeMap::new();
    for f in factors {
        let (b, e) = base_exp(&f)?;
        let Expr::Var(v) = b else { return None };
        if !e.is_int() {
            return None;
        }
        *powers.entry(v).or_insert(0) += e.num();
    }
    Some((c, powers))
}

fn build(c: i128, powers: &BTreeMap<String, i128>) -> Expr {
    let fs: Vec<Expr> = powers.iter().filter(|(_, e)| **e > 0).map(|(v, e)| power_of(expr::var(v), Q::int(*e))).collect();
    with_coeff(Q::int(c), expr::mul(fs))
}

impl Rule for PowerQuotient {
    fn name(&self) -> &'static str {
        "power_quotient"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["subtract"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("power_quotient", m, |e, _| {
            let Expr::Div(a, b) = e else { return vec![] };
            let (Some((ca, pa)), Some((cb, pb))) = (monomial(a), monomial(b)) else { return vec![] };
            if !pa.keys().any(|v| pb.contains_key(v)) {
                return vec![];
            }
            let Some(c) = ca.div(&cb) else { return vec![] };
            let (mut top, mut bottom) = (BTreeMap::new(), BTreeMap::new());
            for v in pa.keys().chain(pb.keys()) {
                let k = pa.get(v).copied().unwrap_or(0) - pb.get(v).copied().unwrap_or(0);
                if k > 0 {
                    top.insert(v.clone(), k);
                } else if k < 0 {
                    bottom.insert(v.clone(), -k);
                }
            }
            let (t, d) = (build(c.num(), &top), build(c.den(), &bottom));
            let new = if d.is_num(1) { t } else { expr::div(t, d) };
            let says = Line::new().t("Same base: subtract the exponents, ").e(e).t(" = ").e(&new).t(".");
            vec![Rewrite { variant: "subtract", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn subtracts_exponents() {
        assert_eq!(test_moves(&PowerQuotient, "simplify x^5/x^2"), vec!["x^3"]);
        assert_eq!(test_moves(&PowerQuotient, "simplify 6x^3 y/(4x y^2)"), vec!["3x^2/(2y)"]);
        assert!(test_moves(&PowerQuotient, "simplify x^2/y").is_empty());
    }
}
