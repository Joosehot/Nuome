//! Double negation (logic and sets, agent L): ~~P is P; (X')' = X.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::Math;
use crate::logic::{self, B};

pub struct DoubleNegation;

impl Rule for DoubleNegation {
    fn name(&self) -> &'static str {
        "double_negation"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["twice"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("double_negation", m, cx, |_, f, b| {
            let B::Not(inner) = b else { return vec![] };
            match logic::view(inner) {
                Some((g, B::Not(x))) if g == f => {
                    let p = logic::ph(f, 0);
                    let says = logic::law_says(f, "Double negation", logic::not(f, logic::not(f, p.clone())), p);
                    vec![Rewrite { variant: "twice", new: x.clone(), says, work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn two_negations_cancel() {
        assert_eq!(test_moves(&DoubleNegation, "prove ~~p or q <-> (~p -> q)"), vec!["p or q <=> (~p -> q)"]);
        assert!(test_moves(&DoubleNegation, "prove ~p or p is a tautology").is_empty());
    }
}
