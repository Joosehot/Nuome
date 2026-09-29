//! Solve a quadratic by factoring: x^2 - 5x + 6 = 0 -> (x - 2)(x - 3) = 0.
//! Offered only when the roots are rational, i.e. when it factors nicely.

use super::trinomial::by_roots;
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::poly;

pub struct FactorSolve;

impl Rule for FactorSolve {
    fn name(&self) -> &'static str {
        "factor_solve"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum_product", "ac_method", "perfect_square", "common_factor"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("factor_solve", m, |l, r| {
            if !r.is_num(0) || !matches!(l, Expr::Add(_)) || l.vars().len() != 1 {
                return vec![];
            }
            let Some(p) = poly::from_expr(l, v) else { return vec![] };
            let Some((f, variant, work)) = by_roots(&p, v) else { return vec![] };
            let says = Line::new().t("Factor the left side.");
            vec![EqRewrite { variant, to: Branch::One(f, r.clone()), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factors_when_roots_are_rational() {
        assert_eq!(test_moves(&FactorSolve, "solve x^2 - 5x + 6 = 0"), vec!["(x - 2)(x - 3) = 0"]);
        assert_eq!(test_moves(&FactorSolve, "solve x^2 - 3x = 0"), vec!["x(x - 3) = 0"]);
        assert!(test_moves(&FactorSolve, "solve x^2 - 2 = 0").is_empty());
    }
}
