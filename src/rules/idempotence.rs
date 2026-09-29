//! Idempotence (logic and sets, agent L): P and P is P, P or P is P;
//! X union X = X, X intersect X = X.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, B};

pub struct Idempotence;

impl Rule for Idempotence {
    fn name(&self) -> &'static str {
        "idempotence"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["and", "or"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("idempotence", m, cx, |_, f, b| {
            let (v, is_and) = match b {
                B::And(v) => (v, true),
                B::Or(v) => (v, false),
                _ => return vec![],
            };
            let mut kept: Vec<Expr> = Vec::new();
            for x in v {
                if !kept.iter().any(|y| logic::same(x, y)) {
                    kept.push(x.clone());
                }
            }
            if kept.len() == v.len() {
                return vec![];
            }
            let p = logic::ph(f, 0);
            let (new, lhs) = if is_and { (logic::and(f, kept), logic::and(f, vec![p.clone(), p.clone()])) } else { (logic::or(f, kept), logic::or(f, vec![p.clone(), p.clone()])) };
            let says = logic::law_says(f, "Idempotence", lhs, p);
            vec![Rewrite { variant: if is_and { "and" } else { "or" }, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn a_repeated_part_goes() {
        assert_eq!(test_moves(&Idempotence, "prove (p or q or p) -> (p or q)"), vec!["(p or q) -> (p or q)"]);
        assert!(test_moves(&Idempotence, "prove (p or q) -> (q or p)").is_empty());
    }
}
