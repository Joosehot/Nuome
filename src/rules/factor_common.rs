//! Take out the greatest common factor: 6x^2 + 9x -> 3x(2x + 3). A
//! negative leading term takes the sign out too: -x^2 + 4 -> -(x^2 - 4).

use super::powers::{base_exp, power_of};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};
use crate::q::{gcd, Q};
use std::collections::BTreeMap;

pub struct FactorCommon;

/// A term as (integer coefficient, letter -> exponent), when it is one.
fn monomial(t: &Expr) -> Option<(i128, BTreeMap<String, i128>)> {
    let (c, rest) = coeff(t);
    if !c.is_int() {
        return None;
    }
    let mut powers = BTreeMap::new();
    let factors = match rest {
        Expr::Mul(v) => v,
        r if r.is_num(1) => vec![],
        r => vec![r],
    };
    for f in factors {
        let (b, e) = base_exp(&f)?;
        let Expr::Var(v) = b else { return None };
        if !e.is_int() || e.is_neg() {
            return None;
        }
        *powers.entry(v).or_insert(0) += e.num();
    }
    Some((c.num(), powers))
}

fn build(c: i128, powers: &BTreeMap<String, i128>) -> Expr {
    let factors: Vec<Expr> = powers.iter().filter(|(_, e)| **e > 0).map(|(v, e)| power_of(expr::var(v), Q::int(*e))).collect();
    with_coeff(Q::int(c), expr::mul(factors))
}

impl Rule for FactorCommon {
    fn name(&self) -> &'static str {
        "factor_common"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["gcf"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        local("factor_common", m, |e, at| {
            // when solving, only "... = 0" wants factors
            if cx.task() == crate::model::Task::Solve && !at.is_side_against_zero() {
                return vec![];
            }
            let Expr::Add(ts) = e else { return vec![] };
            let Some(monos) = ts.iter().map(monomial).collect::<Option<Vec<_>>>() else { return vec![] };
            let mut g = monos.iter().fold(0, |g, (c, _)| gcd(g, *c));
            if monos[0].0 < 0 {
                g = -g;
            }
            let mut common: BTreeMap<String, i128> = monos[0].1.clone();
            for (_, p) in &monos[1..] {
                common = common.into_iter().filter_map(|(v, e)| p.get(&v).map(|pe| (v, e.min(*pe)))).filter(|(_, e)| *e > 0).collect();
            }
            if g == 1 && common.is_empty() {
                return vec![];
            }
            let inner: Vec<Expr> = monos
                .iter()
                .map(|(c, p)| {
                    let mut p = p.clone();
                    for (v, e) in &common {
                        *p.get_mut(v).expect("common letter") -= e;
                    }
                    build(c / g, &p)
                })
                .collect();
            let outside = build(g, &common);
            let new = if outside.is_num(-1) { expr::neg(Expr::Add(inner)) } else { expr::mul(vec![outside.clone(), Expr::Add(inner)]) };
            let says = Line::new().t("Take out the common factor ").e(&outside).t(".");
            vec![Rewrite { variant: "gcf", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn takes_out_common_factors() {
        assert_eq!(test_moves(&FactorCommon, "factor 6x^2 + 9x"), vec!["3x(2x + 3)"]);
        assert_eq!(test_moves(&FactorCommon, "factor -x^2 + 4"), vec!["-(x^2 - 4)"]);
        assert!(test_moves(&FactorCommon, "factor x^2 + 3x + 1").is_empty());
    }
}
