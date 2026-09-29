//! 5 = x -> x = 5: put the letter on the left.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::Math;

pub struct Swap;

impl Rule for Swap {
    fn name(&self) -> &'static str {
        "swap"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sides"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        per_eq("swap", m, |l, r| {
            if l.has_var(cx.var) || !r.has_var(cx.var) {
                return vec![];
            }
            vec![EqRewrite { variant: "sides", to: Branch::One(r.clone(), l.clone()), says: Line::new().t("Swap the sides."), work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn puts_the_letter_left() {
        assert_eq!(test_moves(&Swap, "solve 5 = x"), vec!["x = 5"]);
        assert!(test_moves(&Swap, "solve x = 5").is_empty());
    }
}
