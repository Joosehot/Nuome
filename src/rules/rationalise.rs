//! Rationalise a denominator: no square roots under the line.
//! Variants: multiply top and bottom by the root (1/sqrt(2) = sqrt(2)/2),
//! or by the conjugate (1/(1 + sqrt(2)) = (1 - sqrt(2))/(1 - 2)).

use super::distribute::times;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Expr, Func, Math};
use crate::q::Q;

pub struct Rationalise;

/// k * sqrt(n) with a whole number n: (k, n).
fn root_term(t: &Expr) -> Option<(Q, Q)> {
    let (k, rest) = coeff(t);
    match rest {
        Expr::Func(Func::Sqrt, a) => {
            let n = a.as_num().filter(|n| n.is_int() && !n.is_neg())?;
            // an exact root is the roots rule's job
            n.sqrt().is_none().then_some((k, n))
        }
        _ => None,
    }
}

/// top/den with the sign of den moved up: (1 - sqrt(2))/(-1) = -1 + sqrt(2).
fn over(top: Expr, den: Q) -> Expr {
    let (top, den) = if den.is_neg() { (expr::add(terms(&top).iter().map(|t| with_coeff(coeff(t).0.neg(), coeff(t).1)).collect()), den.neg()) } else { (top, den) };
    if den.is_one() {
        top
    } else {
        expr::div(top, Expr::Num(den))
    }
}

impl Rule for Rationalise {
    fn name(&self) -> &'static str {
        "rationalise"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["root", "conjugate"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("rationalise", m, |e, _| {
            let Expr::Div(a, b) = e else { return vec![] };
            if !a.vars().is_empty() || !b.vars().is_empty() {
                return vec![];
            }
            // k sqrt(n): multiply by sqrt(n)
            if let Some((k, n)) = root_term(b) {
                let Some(den) = k.mul(&n) else { return vec![] };
                let r = expr::sqrt(Expr::Num(n));
                let top = expr::add(terms(a).iter().map(|t| times(t, &r)).collect());
                let says = Line::new().t("Multiply top and bottom by ").e(&r).t(".");
                let work = vec![Line::new().e(b).t(" * ").e(&r).t(format!(" = {den}"))];
                return vec![Rewrite { variant: "root", new: over(top, den), says, work }];
            }
            // p + q sqrt(n): multiply by p - q sqrt(n)
            let ts = terms(b);
            if ts.len() != 2 {
                return vec![];
            }
            let (Some(p), Some((q, n))) = (ts[0].as_num().map_or_else(|| ts[1].as_num(), Some), ts.iter().find_map(root_term)) else { return vec![] };
            let Some(den) = (|| p.mul(&p)?.sub(&q.mul(&q)?.mul(&n)?))() else { return vec![] };
            if den.is_zero() {
                return vec![];
            }
            let r = expr::sqrt(Expr::Num(n));
            let conj = expr::add(vec![Expr::Num(p), with_coeff(q.neg(), r)]);
            let top = if a.is_num(1) { conj.clone() } else { expr::add(terms(a).iter().flat_map(|t| terms(&conj).into_iter().map(move |c| times(t, &c))).collect()) };
            let says = Line::new().t("Multiply top and bottom by the conjugate ").e(&conj).t(".");
            // (p + q sqrt(n))(p - q sqrt(n)) = p^2 - q^2 n
            let Some(q2) = q.mul(&q) else { return vec![] };
            let q2n = if q2.is_one() { Expr::Num(n) } else { Expr::Mul(vec![Expr::Num(q2), Expr::Num(n)]) };
            let diff = expr::add(vec![expr::pow(Expr::Num(p), expr::num(2)), expr::neg(q2n)]);
            let work = vec![Line::new().t("(").e(b).t(")(").e(&conj).t(") = ").e(&diff).t(format!(" = {den}"))];
            vec![Rewrite { variant: "conjugate", new: over(top, den), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn clears_roots_from_denominators() {
        assert_eq!(test_moves(&Rationalise, "what is 1/sqrt(2)"), vec!["sqrt(2)/2"]);
        assert_eq!(test_moves(&Rationalise, "what is 6/(2sqrt(3))"), vec!["6sqrt(3)/6"]);
        assert_eq!(test_moves(&Rationalise, "what is 1/(1 + sqrt(2))"), vec!["-1 + sqrt(2)"]);
        assert!(test_moves(&Rationalise, "what is sqrt(2)/2").is_empty());
    }
}
