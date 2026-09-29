//! The end of an inequality proof: a sum of squares (each times a positive
//! number) and positive numbers is never negative, and it is positive once
//! a positive number is in it.

use super::{Cx, Line, Move, Rule};
use crate::expr::{coeff, terms, Expr, Math, Rel};
use crate::model::Task;

pub struct Nonneg;

/// Is this term a positive number times even powers? (Some(true) when it is
/// a positive number alone.)
pub fn square_term(t: &Expr) -> Option<bool> {
    let (c, rest) = coeff(t);
    if !c.is_neg() && !c.is_zero() {
        if rest.is_num(1) {
            return Some(true);
        }
        let factors = match rest {
            Expr::Mul(v) => v,
            r => vec![r],
        };
        let even = |f: &Expr| matches!(f, Expr::Pow(_, n) if n.as_num().is_some_and(|q| q.is_int() && q.num() % 2 == 0 && q.num() > 0));
        if factors.iter().all(even) {
            return Some(false);
        }
    }
    None
}

impl Rule for Nonneg {
    fn name(&self) -> &'static str {
        "nonneg"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum_of_squares"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if cx.task() != Task::Prove {
            return vec![];
        }
        let Math::Ineq(e, rel, z) = m else { return vec![] };
        if !z.is_num(0) || !matches!(rel, Rel::Ge | Rel::Gt) {
            return vec![];
        }
        let kinds: Option<Vec<bool>> = terms(e).iter().map(square_term).collect();
        let Some(kinds) = kinds else { return vec![] };
        let positive_number = kinds.iter().any(|k| *k);
        if *rel == Rel::Gt && !positive_number {
            return vec![];
        }
        let says = if kinds.len() == 1 && !positive_number {
            Line::new().t("A square is never negative.")
        } else if *rel == Rel::Gt {
            Line::new().t("Squares are never negative, and a positive number is added, so the sum is positive.")
        } else {
            Line::new().t("Each term is a square times a positive number, or a positive number, so the sum is never negative.")
        };
        vec![Move { rule: "nonneg", variant: "sum_of_squares", result: Math::Proved, says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn squares_are_nonnegative() {
        assert_eq!(test_moves(&Nonneg, "prove x^2 + 1 > 0"), vec!["proved"]);
        assert!(test_moves(&Nonneg, "prove x^2 - 1 >= 0").is_empty());
    }
}
