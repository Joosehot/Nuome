//! Undo a logarithm in an equation. Variants: exponential form
//! (log_2(x) = 3 means x = 2^3, ln(x) = 2 means x = e^2), or equal logs of
//! the same base have equal arguments (log(2x) = log(x + 4) -> 2x = x + 4).

use super::log_laws::as_log;
use super::power_base::power;
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::Math;

pub struct LogToExp;

impl Rule for LogToExp {
    fn name(&self) -> &'static str {
        "log_to_exp"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["exponential_form", "one_to_one"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("log_to_exp", m, |l, r| {
            match (as_log(l), as_log(r)) {
                (Some((b1, a1)), Some((b2, a2))) if b1 == b2 && (a1.has_var(v) || a2.has_var(v)) => {
                    let says = Line::new().t("Equal logs to the same base have equal arguments.");
                    vec![EqRewrite { variant: "one_to_one", to: Branch::One(a1, a2), says, work: vec![] }]
                }
                (Some((b, a)), None) if a.has_var(v) && !r.has_var(v) && !b.has_var(v) => {
                    let new = power(b, r.clone());
                    let says = Line::new().t("Write it in exponential form: ").e(l).t(" = ").e(r).t(" means ").e(&a).t(" = ").e(&new).t(".");
                    vec![EqRewrite { variant: "exponential_form", to: Branch::One(a, new), says, work: vec![] }]
                }
                (None, Some((b, a))) if a.has_var(v) && !l.has_var(v) && !b.has_var(v) => {
                    let new = power(b, l.clone());
                    let says = Line::new().t("Write it in exponential form: ").e(r).t(" = ").e(l).t(" means ").e(&a).t(" = ").e(&new).t(".");
                    vec![EqRewrite { variant: "exponential_form", to: Branch::One(a, new), says, work: vec![] }]
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
    fn undoes_logs() {
        assert_eq!(test_moves(&LogToExp, "solve log_2(x) = 3"), vec!["x = 2^3"]);
        assert_eq!(test_moves(&LogToExp, "solve ln(x) = 2"), vec!["x = e^2"]);
        assert_eq!(test_moves(&LogToExp, "solve log(2x) = log(x + 4)"), vec!["2x = x + 4"]);
        assert!(test_moves(&LogToExp, "solve log(x) + 1 = 3").is_empty());
    }
}
