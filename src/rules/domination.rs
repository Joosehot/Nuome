//! Domination laws (logic and sets, agent L): P or T is T, P and F is F;
//! X union U = U, X intersect (empty set) is empty.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, B};

pub struct Domination;

impl Rule for Domination {
    fn name(&self) -> &'static str {
        "domination"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["or_true", "and_false"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("domination", m, cx, |_, f, b| {
            // the constant that decides it: T in an or, F in an and
            let (v, zero) = match b {
                B::Or(v) => (v, true),
                B::And(v) => (v, false),
                _ => return vec![],
            };
            if !v.iter().any(|x: &Expr| matches!(logic::view(x), Some((g, B::Const(c))) if g == f && c == zero)) {
                return vec![];
            }
            let p = logic::ph(f, 0);
            let lhs = if zero { logic::or(f, vec![p, logic::konst(f, true)]) } else { logic::and(f, vec![p, logic::konst(f, false)]) };
            let says = logic::law_says(f, "Domination law", lhs, logic::konst(f, zero));
            vec![Rewrite { variant: if zero { "or_true" } else { "and_false" }, new: logic::konst(f, zero), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn true_decides_an_or() {
        assert_eq!(test_moves(&Domination, "prove (q or T) and (p or ~p)"), vec!["T and (p or ~p)"]);
        assert!(test_moves(&Domination, "prove (p and T) -> p").is_empty());
    }
}
