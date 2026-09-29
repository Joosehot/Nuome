//! The mean: add the values, divide by how many there are.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, Named};
use crate::expr::{self, Expr, Math};

pub struct Mean;

impl Rule for Mean {
    fn name(&self) -> &'static str {
        "mean"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum_over_count"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("mean", m, |e, _| {
            let Expr::Call(Named::Mean, args) = e else { return vec![] };
            if nums(args).is_none() || args.is_empty() {
                return vec![];
            }
            let n = args.len() as i128;
            let new = expr::div(Expr::Add(args.clone()), expr::num(n));
            vec![Rewrite { variant: "sum_over_count", new, says: Line::new().t(format!("Add the {n} values and divide by {n}.")), work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn mean_is_sum_over_count() {
        assert_eq!(test_moves(&Mean, "mean of 3, 5, 7 and 9"), vec!["(3 + 5 + 7 + 9)/4"]);
        assert!(test_moves(&Mean, "median of 3, 5, 7").is_empty());
    }
}
