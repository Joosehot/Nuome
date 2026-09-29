//! A nonzero number over something going to 0: the fraction grows without
//! bound. If the bottom keeps one sign on both sides (x^2 near 0), the
//! limit is infinity or -infinity; if it changes sign (x near 0), the two
//! sides disagree and there is no limit, so this rule offers nothing.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Konst, Math};
use crate::poly;

pub struct LimInfinite;

impl Rule for LimInfinite {
    fn name(&self) -> &'static str {
        "lim_infinite"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same_sign"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("lim_infinite", m, |e, _| {
            let Expr::Limit(f, v, p) = e else { return vec![] };
            let Some(a) = p.eval_q(&|_| None) else { return vec![] };
            let Some((n, d)) = poly::rational_from_expr(f, v) else { return vec![] };
            let (Some(top), Some(bottom)) = (n.eval(&a), d.eval(&a)) else { return vec![] };
            if top.is_zero() || !bottom.is_zero() || d.is_zero() {
                return vec![];
            }
            // bottom = (x - a)^k * rest, rest(a) != 0
            let (mut rest, mut k) = (d.clone(), 0);
            while let Some(r) = rest.deflate(&a) {
                (rest, k) = (r, k + 1);
            }
            if k % 2 == 1 {
                return vec![];
            }
            let Some(s) = rest.eval(&a) else { return vec![] };
            let up = top.is_neg() == s.is_neg();
            let new = if up { Expr::Const(Konst::Inf) } else { expr::neg(Expr::Const(Konst::Inf)) };
            let side = if s.is_neg() { "negative" } else { "positive" };
            let says = Line::new().t(format!("The top goes to {top} and the bottom to 0, staying {side} on both sides of {v} = {a}: the fraction grows without bound."));
            vec![Rewrite { variant: "same_sign", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn grows_without_bound() {
        assert_eq!(test_moves(&LimInfinite, "limit of 1/x^2 as x approaches 0"), vec!["infinity"]);
        assert_eq!(test_moves(&LimInfinite, "limit of -3/(x - 1)^2 as x approaches 1"), vec!["-infinity"]);
        assert!(test_moves(&LimInfinite, "limit of 1/x as x approaches 0").is_empty());
    }
}
