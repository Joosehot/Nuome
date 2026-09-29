//! Exact values of e^x, ln and |x| where they are whole: e^0 = 1, e^1 = e,
//! ln(1) = 0, ln(e) = 1, ln(e^k) = k, |-3| = 3, |pi| = pi.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Konst, Math};

pub struct FuncExact;

impl Rule for FuncExact {
    fn name(&self) -> &'static str {
        "func_exact"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["exp_log", "absolute"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let zero = cx.cfg.calculus.zero;
        local("func_exact", m, |e, _| {
            let exp_log = |new: Expr, why: &str| vec![Rewrite { variant: "exp_log", says: Line::new().e(e).t(" = ").e(&new).t(format!(", {why}.")), new, work: vec![] }];
            match e {
                Expr::Func(Func::Exp, a) if a.is_num(0) => exp_log(expr::num(1), "since anything to the power 0 is 1"),
                Expr::Func(Func::Exp, a) if a.is_num(1) => exp_log(Expr::Const(Konst::E), "by definition"),
                Expr::Func(Func::Ln, a) if a.is_num(1) => exp_log(expr::num(0), "since e^0 = 1"),
                Expr::Func(Func::Ln, a) if **a == Expr::Const(Konst::E) => exp_log(expr::num(1), "since e^1 = e"),
                Expr::Func(Func::Ln, a) => match &**a {
                    Expr::Func(Func::Exp, k) => exp_log((**k).clone(), "since ln undoes e^"),
                    _ => vec![],
                },
                Expr::Func(Func::Abs, a) if a.vars().is_empty() && !a.is_pending() => {
                    let x = a.eval_f(&|_| f64::NAN);
                    if !x.is_finite() || x.abs() < zero {
                        return vec![];
                    }
                    let new = if x > 0.0 { (**a).clone() } else { expr::neg((**a).clone()) };
                    let why = if x > 0.0 { " is positive." } else { " is negative." };
                    vec![Rewrite { variant: "absolute", says: Line::new().e(e).t(" = ").e(&new).t(", since ").e(a).t(why), new, work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn exact_values() {
        assert_eq!(test_moves(&FuncExact, "what is e^0 + ln(1)"), vec!["1 + ln(1)", "e^0 + 0"]);
        assert_eq!(test_moves(&FuncExact, "what is abs(-3)"), vec!["3"]);
        assert!(test_moves(&FuncExact, "what is ln(2)").is_empty());
    }
}
