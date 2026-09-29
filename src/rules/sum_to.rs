//! 1 + 2 + ... + 100, Gauss's way: pair the first with the last. Each pair
//! adds to the same total, and there are n/2 pairs: n(a + b)/2.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{ints, Named};
use crate::expr::{self, Expr, Math};

pub struct SumTo;

impl Rule for SumTo {
    fn name(&self) -> &'static str {
        "sum_to"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["pairs"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("sum_to", m, |e, _| {
            let Expr::Call(Named::SumTo, args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            let [a, b] = ns[..] else { return vec![] };
            if b < a {
                return vec![];
            }
            let n = b - a + 1;
            let new = expr::div(Expr::Mul(vec![expr::num(n), Expr::Add(vec![expr::num(a), expr::num(b)])]), expr::num(2));
            let says = Line::new().t(format!("There are {n} numbers; pair the first with the last: each pair adds to {}.", a + b));
            vec![Rewrite { variant: "pairs", new, says, work: vec![Line::new().t("sum = count * (first + last) / 2")] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn gauss_sum() {
        assert_eq!(test_moves(&SumTo, "sum of the numbers from 1 to 100"), vec!["100(1 + 100)/2"]);
        assert!(test_moves(&SumTo, "sum of 3, 5 and 7").is_empty());
    }
}
