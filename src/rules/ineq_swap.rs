//! Read an inequality from the other side to put the letter on the left:
//! 3 > x says the same as x < 3 (the sign turns round with it).

use super::{Cx, Line, Move, Rule};
use crate::expr::Math;

pub struct IneqSwap;

impl Rule for IneqSwap {
    fn name(&self) -> &'static str {
        "ineq_swap"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sides"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Ineq(l, rel, r) = m else { return vec![] };
        if l.has_var(cx.var) || !r.has_var(cx.var) {
            return vec![];
        }
        let result = Math::Ineq(r.clone(), rel.flip(), l.clone());
        let says = Line::new().t("Read it from the other side: ").m(m).t(" means ").m(&result).t(".");

        vec![Move { rule: "ineq_swap", variant: "sides", result, says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn puts_the_letter_left() {
        assert_eq!(test_moves(&IneqSwap, "solve 3 > x"), vec!["x < 3"]);
        assert!(test_moves(&IneqSwap, "solve x < 3").is_empty());
    }
}
