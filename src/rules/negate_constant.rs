//! Negating a constant (logic and sets, agent L): ~T is F and ~F is T;
//! U' is empty and the complement of the empty set is U.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::Math;
use crate::logic::{self, B};

pub struct NegateConstant;

impl Rule for NegateConstant {
    fn name(&self) -> &'static str {
        "negate_constant"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["flip"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("negate_constant", m, cx, |_, f, b| {
            let B::Not(inner) = b else { return vec![] };
            match logic::view(inner) {
                Some((g, B::Const(c))) if g == f => {
                    let says = logic::law_says(f, "Negation of a constant", logic::not(f, logic::konst(f, c)), logic::konst(f, !c));
                    vec![Rewrite { variant: "flip", new: logic::konst(f, !c), says, work: vec![] }]
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
    fn not_false_is_true() {
        assert_eq!(test_moves(&NegateConstant, "prove ~F or p"), vec!["T or p"]);
        assert!(test_moves(&NegateConstant, "prove ~p or p").is_empty());
    }
}
