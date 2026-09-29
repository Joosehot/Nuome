//! The range: largest value minus smallest.

use super::median::sorted_line;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, Named};
use crate::expr::{Expr, Math};

pub struct Range;

impl Rule for Range {
    fn name(&self) -> &'static str {
        "range"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["max_minus_min"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("range", m, |e, _| {
            let Expr::Call(Named::Range, args) = e else { return vec![] };
            let Some(mut xs) = nums(args) else { return vec![] };
            if xs.len() < 2 {
                return vec![];
            }
            xs.sort();
            let (lo, hi) = (xs[0], xs[xs.len() - 1]);
            let new = Expr::Add(vec![Expr::Num(hi), Expr::Num(lo.neg())]);
            vec![Rewrite { variant: "max_minus_min", new, says: Line::new().t(format!("Largest minus smallest: {hi} and {lo}.")), work: vec![sorted_line(&xs)] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn largest_minus_smallest() {
        assert_eq!(test_moves(&Range, "range of 4, 9, 1"), vec!["9 - 1"]);
        assert!(test_moves(&Range, "mean of 4, 9, 1").is_empty());
    }
}
