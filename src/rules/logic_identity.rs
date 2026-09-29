//! Identity laws (logic and sets, agent L): P and T is P, P or F is P;
//! X intersect U = X, X union (empty set) = X.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, B};

pub struct LogicIdentity;

impl Rule for LogicIdentity {
    fn name(&self) -> &'static str {
        "logic_identity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["and_true", "or_false"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("logic_identity", m, cx, |_, f, b| {
            // the constant that changes nothing: T in an and, F in an or
            let (v, unit) = match b {
                B::And(v) => (v, true),
                B::Or(v) => (v, false),
                _ => return vec![],
            };
            let is_unit = |x: &Expr| matches!(logic::view(x), Some((g, B::Const(c))) if g == f && c == unit);
            if !v.iter().any(is_unit) || v.iter().all(is_unit) {
                return vec![];
            }
            let rest: Vec<Expr> = v.iter().filter(|x| !is_unit(x)).cloned().collect();
            let p = logic::ph(f, 0);
            let (new, lhs) = if unit { (logic::and(f, rest), logic::and(f, vec![p.clone(), logic::konst(f, true)])) } else { (logic::or(f, rest), logic::or(f, vec![p.clone(), logic::konst(f, false)])) };
            let says = logic::law_says(f, "Identity law", lhs, p);
            vec![Rewrite { variant: if unit { "and_true" } else { "or_false" }, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn true_in_an_and_goes() {
        assert_eq!(test_moves(&LogicIdentity, "prove (p and T) -> p or q"), vec!["p -> (p or q)"]);
        assert!(test_moves(&LogicIdentity, "prove (p and q) -> p").is_empty());
    }
}
