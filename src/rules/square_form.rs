//! Recognise a perfect square in several letters: a^2 - 2ab + b^2 = (a - b)^2.

use super::diff_squares::term_sqrt;
use super::distribute::times;
use super::sides_equal::canon;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::model::Task;

pub struct SquareForm;

impl Rule for SquareForm {
    fn name(&self) -> &'static str {
        "square_form"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["binomial"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        // only where a square helps: proving an inequality
        if cx.task() != Task::Prove || !matches!(m, Math::Ineq(..)) {
            return vec![];
        }
        local("square_form", m, |e, _| {
            let Expr::Add(ts) = e else { return vec![] };
            if ts.len() != 3 {
                return vec![];
            }
            for (i, j, k) in [(0, 1, 2), (0, 2, 1), (1, 2, 0)] {
                let (Some(s1), Some(s3)) = (term_sqrt(&ts[i]), term_sqrt(&ts[j])) else { continue };
                let twice = times(&expr::num(2), &times(&s1, &s3));
                let minus = times(&expr::num(-2), &times(&s1, &s3));
                let sign = if canon(&ts[k]) == canon(&twice) {
                    1
                } else if canon(&ts[k]) == canon(&minus) {
                    -1
                } else {
                    continue;
                };
                let inner = expr::add(vec![s1.clone(), if sign > 0 { s3.clone() } else { expr::neg(s3.clone()) }]);
                let new = expr::pow(inner, expr::num(2));
                let says = Line::new().t("Recognise a perfect square: ").e(e).t(" = ").e(&new).t(".");
                return vec![Rewrite { variant: "binomial", new, says, work: vec![] }];
            }
            vec![]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn perfect_squares() {
        assert_eq!(test_moves(&SquareForm, "prove a^2 - 2ab + b^2 >= 0"), vec!["(a - b)^2 >= 0"]);
        assert!(test_moves(&SquareForm, "prove a^2 + ab + b^2 >= 0").is_empty());
    }
}
