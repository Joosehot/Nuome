//! Equal powers of the same base have equal exponents: 2^x = 2^3 -> x = 3
//! (a power of a positive base other than 1 takes each value once).

use super::power_base::as_exp;
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{Expr, Math};

pub struct EquateExponents;

impl Rule for EquateExponents {
    fn name(&self) -> &'static str {
        "equate_exponents"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same_base"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("equate_exponents", m, |l, r| {
            // 2^(x + 1) = 2^3: the right side's exponent may be a plain number
            let side = |e: &Expr| as_exp(e, v).or_else(|| match e {
                Expr::Pow(b, a) if !e.has_var(v) && b.as_num().is_some() => Some(((**b).clone(), (**a).clone())),
                _ => None,
            });
            let (Some((b1, a1)), Some((b2, a2))) = (side(l), side(r)) else { return vec![] };
            if b1 != b2 || !(l.has_var(v) || r.has_var(v)) {
                return vec![];
            }
            let says = Line::new().t("Same base on both sides, so the exponents are equal.");
            vec![EqRewrite { variant: "same_base", to: Branch::One(a1, a2), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn equates_exponents_of_one_base() {
        assert_eq!(test_moves(&EquateExponents, "solve 2^x = 2^3"), vec!["x = 3"]);
        assert_eq!(test_moves(&EquateExponents, "solve 3^(2x) = 3^(x + 1)"), vec!["2x = x + 1"]);
        assert!(test_moves(&EquateExponents, "solve 2^x = 3^2").is_empty());
    }
}
