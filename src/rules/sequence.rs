//! Arithmetic and geometric sequences from their first terms: the n-th
//! term, the sum of the first n terms, the sum to infinity. The kind is read
//! off the terms (a common difference or a common ratio); with fewer than
//! three terms, or neither, nothing is assumed.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, sequence_kind, Kind, Named};
use crate::expr::{self, Expr, Math};
use crate::q::Q;

pub struct Sequence;

fn q(v: Q) -> Expr {
    Expr::Num(v)
}

impl Rule for Sequence {
    fn name(&self) -> &'static str {
        "sequence"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["arithmetic", "geometric"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("sequence", m, |e, _| {
            let Expr::Call(f @ (Named::SeriesSum | Named::NthTerm | Named::InfiniteSum), args) = e else { return vec![] };
            let Some(vals) = nums(args) else { return vec![] };
            let (n, ts) = if *f == Named::InfiniteSum { (Q::ZERO, &vals[..]) } else { (vals[0], &vals[1..]) };
            if *f != Named::InfiniteSum && (!n.is_int() || n.num() < 1) {
                return vec![];
            }
            let Some(kind) = sequence_kind(ts) else { return vec![] };
            let a = ts[0];
            let n1 = n.sub(&Q::ONE).unwrap_or(Q::ZERO);
            let (variant, kind_line, new, formula) = match (kind, f) {
                (Kind::Arithmetic(d), Named::NthTerm) => ("arithmetic", format!("The terms go up by {d} each time: arithmetic, a = {a}, d = {d}."), expr::add(vec![q(a), Expr::Mul(vec![q(n1), q(d)])]), "a + (n - 1)d"),
                (Kind::Arithmetic(d), Named::SeriesSum) => (
                    "arithmetic",
                    format!("The terms go up by {d} each time: arithmetic, a = {a}, d = {d}."),
                    expr::div(Expr::Mul(vec![q(n), expr::add(vec![Expr::Mul(vec![expr::num(2), q(a)]), Expr::Mul(vec![q(n1), q(d)])])]), expr::num(2)),
                    "n(2a + (n - 1)d)/2",
                ),
                (Kind::Geometric(r), Named::NthTerm) => ("geometric", format!("Each term is {r} times the one before: geometric, a = {a}, r = {r}."), Expr::Mul(vec![q(a), expr::pow(q(r), q(n1))]), "a r^(n - 1)"),
                (Kind::Geometric(r), Named::SeriesSum) => (
                    "geometric",
                    format!("Each term is {r} times the one before: geometric, a = {a}, r = {r}."),
                    expr::div(Expr::Mul(vec![q(a), Expr::Add(vec![expr::num(1), Expr::Neg(Box::new(expr::pow(q(r), q(n))))])]), Expr::Add(vec![expr::num(1), q(r.neg())])),
                    "a(1 - r^n)/(1 - r)",
                ),
                (Kind::Geometric(r), Named::InfiniteSum) if r.abs() < Q::ONE => {
                    ("geometric", format!("Each term is {r} times the one before, and |r| < 1, so the sum settles: a = {a}, r = {r}."), expr::div(q(a), Expr::Add(vec![expr::num(1), q(r.neg())])), "a/(1 - r)")
                }
                _ => return vec![],
            };
            let says = Line::new().t(format!("Use {formula}."));
            vec![Rewrite { variant, new, says, work: vec![Line::new().t(kind_line)] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn sequences() {
        assert_eq!(test_moves(&Sequence, "the 20th term of 3, 7, 11"), vec!["3 + 19 * 4"]);
        assert_eq!(test_moves(&Sequence, "sum to infinity of 1, 1/2, 1/4"), vec!["1/(1 - 1/2)"]);
        assert!(test_moves(&Sequence, "the 5th term of 1, 2, 4, 7").is_empty());
    }
}
