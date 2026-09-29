//! To prove an inequality, move everything to one side: a^2 + b^2 >= 2ab
//! becomes a^2 + b^2 - 2ab >= 0.

use super::{Cx, Line, Move, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Math};
use crate::model::Task;

pub struct IneqZero;

impl Rule for IneqZero {
    fn name(&self) -> &'static str {
        "ineq_zero"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["to_left"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if cx.task() != Task::Prove {
            return vec![];
        }
        let Math::Ineq(l, rel, r) = m else { return vec![] };
        if r.is_num(0) {
            return vec![];
        }
        let mut left = terms(l);
        left.extend(terms(r).iter().map(|t| {
            let (c, rest) = coeff(t);
            with_coeff(c.neg(), rest)
        }));
        let says = if terms(r).len() == 1 { Line::new().t("Subtract ").e(r).t(" from both sides.") } else { Line::new().t("Move everything to the left side.") };
        vec![Move { rule: "ineq_zero", variant: "to_left", result: Math::Ineq(expr::add(left), *rel, expr::num(0)), says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn moves_to_one_side() {
        assert_eq!(test_moves(&IneqZero, "prove a^2 + b^2 >= 2ab"), vec!["a^2 + b^2 - 2ab >= 0"]);
        assert!(test_moves(&IneqZero, "prove x^2 >= 0").is_empty());
    }
}
