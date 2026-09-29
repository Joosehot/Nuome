//! Take the square root of both sides: (x - 1)^2 = 9 -> x - 1 = 3 or
//! x - 1 = -3. Both signs, because both square to 9.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, Expr, Math};

pub struct SquareRoot;

impl Rule for SquareRoot {
    fn name(&self) -> &'static str {
        "square_root"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["both_signs", "zero", "negative"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("square_root", m, |l, r| {
            let Expr::Pow(base, n) = l else { return vec![] };
            if !n.is_num(2) || !base.has_var(v) || r.has_var(v) {
                return vec![];
            }
            let Some(k) = r.eval_q(&|_| None) else { return vec![] };
            let a = (**base).clone();
            if k.is_neg() {
                let says = Line::new().t(format!("A square is never negative, so it can't equal {k}."));
                return vec![EqRewrite { variant: "negative", to: Branch::Nothing, says, work: vec![] }];
            }
            if k.is_zero() {
                return vec![EqRewrite { variant: "zero", to: Branch::One(a, expr::num(0)), says: Line::new().t("Only 0 squares to 0."), work: vec![] }];
            }
            let root = expr::sqrt(r.clone());
            let says = Line::new().t("Take the square root of both sides, with ").pm().t(": both signs square to ").e(r).t(".");
            let to = Branch::Many(vec![(a.clone(), root.clone()), (a, Expr::Neg(Box::new(root)))]);
            vec![EqRewrite { variant: "both_signs", to, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn roots_both_sides() {
        assert_eq!(test_moves(&SquareRoot, "solve x^2 = 9"), vec!["x = sqrt(9) or x = -sqrt(9)"]);
        assert_eq!(test_moves(&SquareRoot, "solve (x - 1)^2 = -4"), vec!["no real solution"]);
        assert!(test_moves(&SquareRoot, "solve x^2 + 1 = 9").is_empty());
    }
}
