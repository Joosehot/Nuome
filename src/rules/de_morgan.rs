//! De Morgan's laws (logic and sets, agent L): ~(P and Q) is ~P or ~Q and
//! ~(P or Q) is ~P and ~Q; for sets, (X intersect Y)' = X' union Y' and
//! (X union Y)' = X' intersect Y'.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::Math;
use crate::logic::{self, B};

pub struct DeMorgan;

impl Rule for DeMorgan {
    fn name(&self) -> &'static str {
        "de_morgan"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["and", "or"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("de_morgan", m, cx, |_, f, b| {
            let B::Not(inner) = b else { return vec![] };
            let (p, q) = (logic::ph(f, 0), logic::ph(f, 1));
            let nots = |v: &[crate::expr::Expr]| v.iter().map(|x| logic::not(f, x.clone())).collect::<Vec<_>>();
            match logic::view(inner) {
                Some((g, B::And(v))) if g == f => {
                    let says = logic::law_says(f, "De Morgan's law", logic::not(f, logic::and(f, vec![p.clone(), q.clone()])), logic::or(f, nots(&[p, q])));
                    vec![Rewrite { variant: "and", new: logic::or(f, nots(v)), says, work: vec![] }]
                }
                Some((g, B::Or(v))) if g == f => {
                    let says = logic::law_says(f, "De Morgan's law", logic::not(f, logic::or(f, vec![p.clone(), q.clone()])), logic::and(f, nots(&[p, q])));
                    vec![Rewrite { variant: "or", new: logic::and(f, nots(v)), says, work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{test_expr, test_moves, test_moves_from};

    #[test]
    fn negation_moves_inside() {
        // a law doesn't prove itself: that is the truth table's job
        assert!(test_moves(&DeMorgan, "prove not (p and q) is equivalent to not p or not q").is_empty());
        assert_eq!(test_moves(&DeMorgan, "prove ~(p or q) -> ~p"), vec!["(~p and ~q) -> ~p"]);
        assert_eq!(test_moves_from(&DeMorgan, "prove (A ∪ B)' = A' ∩ B'", Math::Eq(test_expr("(A ∪ B)' ∩ C"), test_expr("C"))), vec!["A' intersect B' intersect C = C"]);
        assert!(test_moves(&DeMorgan, "prove p or ~p is a tautology").is_empty());
    }
}
