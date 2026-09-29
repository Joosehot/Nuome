//! Divide both sides of an inequality by the letter's coefficient. By a
//! positive number the sign stays; by a negative number it flips, because
//! multiplying by a negative reverses the order: -3x <= 9 -> x >= 9/(-3).
//! Variants: positive, negative (the flip, said out loud), and clearing
//! number fractions by their least common denominator (always positive).

use super::clear_denominators::ClearDenominators;
use super::{Cx, Line, Move, Rule};
use crate::expr::{self, coeff, Expr, Math};

pub struct IneqScale;

impl Rule for IneqScale {
    fn name(&self) -> &'static str {
        "ineq_scale"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["positive", "negative", "clear_fractions"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let Math::Ineq(l, rel, r) = m else { return vec![] };
        // x/2 + 1 > x - 3: multiply by the (positive) least common denominator
        let clear: Vec<Move> = ClearDenominators
            .moves(&Math::Eq(l.clone(), r.clone()), cx)
            .into_iter()
            .filter(|mv| mv.variant == "numbers")
            .filter_map(|mv| match mv.result {
                Math::Eq(a, b) => Some(Move { rule: "ineq_scale", variant: "clear_fractions", result: Math::Ineq(a, *rel, b), says: mv.says.t(" It is positive, so the sign stays."), work: mv.work }),
                _ => None,
            })
            .collect();
        if !clear.is_empty() {
            return clear;
        }
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
        let new_other = expr::tidy(if c.is_int() { expr::div(other.clone(), Expr::Num(c)) } else { expr::mul(vec![Expr::Num(c.recip().unwrap_or(c)), other.clone()]) });
        let by = if c.is_int() { Line::new().t("Divide both sides by ").e(&Expr::Num(c)) } else { Line::new().t("Multiply both sides by ").e(&Expr::Num(c.recip().unwrap_or(c))) };
        let (variant, new_rel, says) = if c.is_neg() {
            ("negative", rel.flip(), by.t(format!("; a negative number flips the sign, so {} becomes {}.", crate::print::rel(*rel, crate::print::Style::Ascii), crate::print::rel(rel.flip(), crate::print::Style::Ascii))))
        } else {
            ("positive", *rel, by.t("; a positive number keeps the sign."))
        };
        let result = if lv { Math::Ineq(rest, new_rel, new_other) } else { Math::Ineq(new_other, new_rel, rest) };
        vec![Move { rule: "ineq_scale", variant, result, says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn flips_the_sign_for_negatives() {
        assert_eq!(test_moves(&IneqScale, "solve 2x < 8"), vec!["x < 8/2"]);
        assert_eq!(test_moves(&IneqScale, "solve -3x <= 9"), vec!["x >= 9/(-3)"]);
        assert_eq!(test_moves(&IneqScale, "solve 6 > 2x"), vec!["6/2 > x"]);
        assert!(test_moves(&IneqScale, "solve 2x + 1 < 8").is_empty());
    }
}
