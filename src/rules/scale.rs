//! Divide (or multiply) both sides by the letter's coefficient: 2x = 8 ->
//! x = 8/2. Variants: divide by a whole number, or multiply by the
//! reciprocal of a fraction (2/3 x = 4 -> x = 3/2 * 4).

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, coeff, Expr, Math};

pub struct Scale;

impl Rule for Scale {
    fn name(&self) -> &'static str {
        "scale"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["divide", "reciprocal"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("scale", m, |l, r| {
            let (lv, rv) = (l.has_var(v), r.has_var(v));
            if lv == rv {
                return vec![];
            }
            let (side, other) = if lv { (l, r) } else { (r, l) };
            if matches!(side, Expr::Add(_)) {
                return vec![];
            }
            let (c, rest) = coeff(side);
            if c.is_one() || c.is_zero() || !rest.has_var(v) {
                return vec![];
            }
            let (variant, new_other, says) = if c.is_int() {
                if c.num() == -1 {
                    ("divide", expr::neg(other.clone()), Line::new().t("Multiply both sides by -1."))
                } else {
                    ("divide", expr::div(other.clone(), Expr::Num(c)), Line::new().t(format!("Divide both sides by {c}.")))
                }
            } else {
                let Some(inv) = c.recip() else { return vec![] };
                ("reciprocal", expr::mul(vec![Expr::Num(inv), other.clone()]), Line::new().t("Multiply both sides by ").e(&Expr::Num(inv)).t("."))
            };
            let new_other = expr::tidy(new_other);
            let to = if lv { Branch::One(rest, new_other) } else { Branch::One(new_other, rest) };
            vec![EqRewrite { variant, to, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn divides_by_the_coefficient() {
        assert_eq!(test_moves(&Scale, "solve 2x = 8"), vec!["x = 8/2"]);
        assert_eq!(test_moves(&Scale, "solve 2x = 7"), vec!["x = 7/2"]);
        assert_eq!(test_moves(&Scale, "solve x/3 = 4"), vec!["x = 3 * 4"]);
        assert!(test_moves(&Scale, "solve 2x + 1 = 7").is_empty());
    }
}
