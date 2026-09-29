//! Take a logarithm of both sides: 2^x = 5 -> x = log_2(5), e^(2x) = 7 ->
//! 2x = ln(7). A power of a positive base is always positive, so
//! 2^x = -4 has no solution. Variants: a log to the equation's base, ln
//! for base e, or no solution.

use super::log_laws::log;
use super::power_base::{as_exp, power};
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{exact_log, Expr, Konst, Math};

pub struct TakeLog;

impl Rule for TakeLog {
    fn name(&self) -> &'static str {
        "take_log"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["log_base", "natural", "never"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("take_log", m, |l, r| {
            let (flip, (b, a), other) = match (as_exp(l, v), as_exp(r, v)) {
                (Some(x), None) => (false, x, r),
                (None, Some(x)) => (true, x, l),
                _ => return vec![],
            };
            if other.has_var(v) {
                return vec![];
            }
            // the other side's sign decides; a constant like sqrt(3) counts too
            let x = other.eval_f(&|_| f64::NAN);
            if !x.is_finite() {
                return vec![];
            }
            if x <= 0.0 {
                let says = Line::new().e(&power(b.clone(), a)).t(format!(" is positive for every {v}, so it can never equal ")).e(other).t(".");
                return vec![EqRewrite { variant: "never", to: Branch::Nothing, says, work: vec![] }];
            }
            // an exact power is the same-base method's job
            if let (Some(bq), Some(c)) = (b.as_num(), other.eval_q(&|_| None)) {
                if exact_log(&bq, &c).is_some() {
                    return vec![];
                }
            }
            let new = log(b.clone(), other.clone());
            let (variant, says) = if b == Expr::Const(Konst::E) {
                ("natural", Line::new().t("Take ln of both sides: ln undoes e^."))
            } else {
                ("log_base", Line::new().t("Take the log to base ").e(&b).t(" of both sides: it undoes the power."))
            };
            vec![EqRewrite { variant, to: if flip { Branch::One(new, a) } else { Branch::One(a, new) }, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn takes_logs_of_both_sides() {
        assert_eq!(test_moves(&TakeLog, "solve 2^x = 5"), vec!["x = log_2(5)"]);
        assert_eq!(test_moves(&TakeLog, "solve e^(2x) = 7"), vec!["2x = ln(7)"]);
        assert_eq!(test_moves(&TakeLog, "solve 2^x = -4"), vec!["no real solution"]);
        assert!(test_moves(&TakeLog, "solve 2^x = 8").is_empty());
    }
}
