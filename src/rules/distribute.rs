//! a(b + c) -> ab + ac, and -(a - b) -> -a + b.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Expr, Math};
use crate::rules::powers::{base_exp, power_of};

pub struct Distribute;

/// Multiply two terms the way a person writes the product: numbers
/// together, same bases merged: 2x * (-3x) -> -6x^2.
pub fn times(a: &Expr, b: &Expr) -> Expr {
    let (ca, ra) = coeff(a);
    let (cb, rb) = coeff(b);
    let Some(c) = ca.mul(&cb) else { return expr::mul(vec![a.clone(), b.clone()]) };
    let mut factors: Vec<Expr> = Vec::new();
    for f in [ra, rb].into_iter().flat_map(|r| match r {
        Expr::Mul(v) => v,
        r if r.is_num(1) => vec![],
        r => vec![r],
    }) {
        let merged = base_exp(&f).and_then(|(b, e)| {
            let k = factors.iter().position(|g| base_exp(g).is_some_and(|(gb, _)| gb == b))?;
            let (_, ge) = base_exp(&factors[k])?;
            Some((k, power_of(b, ge.add(&e)?)))
        });
        match merged {
            Some((k, p)) => factors[k] = p,
            None => factors.push(f),
        }
    }
    with_coeff(c, expr::mul(factors))
}

impl Rule for Distribute {
    fn name(&self) -> &'static str {
        "distribute"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["over_sum", "minus_sign"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        local("distribute", m, |e, at| match e {
            // denominators with letters stay factored
            _ if at.in_letter_denominator() && cx.task() != crate::model::Task::Expand => vec![],
            Expr::Mul(v) => {
                let sums: Vec<usize> = v.iter().enumerate().filter(|(_, f)| matches!(f, Expr::Add(_))).map(|(i, _)| i).collect();
                if sums.len() != 1 {
                    return vec![];
                }
                let k = sums[0];
                let outside = expr::mul(v.iter().enumerate().filter(|(i, _)| *i != k).map(|(_, f)| f.clone()).collect());
                let new = expr::add(terms(&v[k]).iter().map(|t| times(&outside, t)).collect());
                let says = Line::new().t("Multiply ").e(&outside).t(" into the brackets.");
                vec![Rewrite { variant: "over_sum", new, says, work: vec![] }]
            }
            Expr::Neg(a) if matches!(**a, Expr::Add(_)) => {
                let new = expr::add(terms(a).iter().map(|t| with_coeff(coeff(t).0.neg(), coeff(t).1)).collect());
                vec![Rewrite { variant: "minus_sign", new, says: Line::new().t("A minus sign before brackets flips every sign inside."), work: vec![] }]
            }
            _ => vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn multiplies_into_brackets() {
        assert_eq!(test_moves(&Distribute, "expand 3(x - 2)"), vec!["3x - 6"]);
        assert_eq!(test_moves(&Distribute, "expand -(x - 2)"), vec!["-x + 2"]);
        assert_eq!(test_moves(&Distribute, "expand 2x(x + 1)"), vec!["2x^2 + 2x"]);
        assert!(test_moves(&Distribute, "expand (x + 1)(x + 2)").is_empty());
    }
}
