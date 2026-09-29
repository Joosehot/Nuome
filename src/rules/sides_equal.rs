//! The end of a proof by rewriting: both sides have become the same
//! expression, up to the order of terms and factors (addition and
//! multiplication of numbers commute). Obligations of a system (a base case,
//! an inductive step) are crossed off one at a time.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::model::Task;
use crate::print::{self, Style};

pub struct SidesEqual;

/// A canonical spelling up to the order of terms and factors.
pub fn canon(e: &Expr) -> String {
    match e {
        Expr::Add(v) | Expr::Mul(v) => {
            let mut parts: Vec<String> = v.iter().map(canon).collect();
            parts.sort();
            format!("{}[{}]", if matches!(e, Expr::Add(_)) { "+" } else { "*" }, parts.join(","))
        }
        Expr::Neg(a) => format!("-({})", canon(a)),
        Expr::Div(a, b) => format!("({})/({})", canon(a), canon(b)),
        Expr::Pow(a, b) => format!("({})^({})", canon(a), canon(b)),
        Expr::Func(f, a) => format!("{}({})", f.name(), canon(a)),
        e => print::expr(e, Style::Ascii),
    }
}

impl Rule for SidesEqual {
    fn name(&self) -> &'static str {
        "sides_equal"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same", "obligation"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if cx.task() != Task::Prove {
            return vec![];
        }
        match m {
            Math::Eq(l, r) if canon(l) == canon(r) => {
                let says = Line::new().t("Both sides are the same: ").e(l).t(".");
                vec![Move { rule: "sides_equal", variant: "same", result: Math::Proved, says, work: vec![] }]
            }
            Math::System(eqs) => {
                let Some(k) = eqs.iter().position(|(l, r)| canon(l) == canon(r)) else { return vec![] };
                let mut rest = eqs.clone();
                let (l, _) = rest.remove(k);
                let result = if rest.is_empty() { Math::Proved } else { Math::System(rest) };
                let says = Line::new().t("This part holds: both sides are ").e(&l).t(".");
                vec![Move { rule: "sides_equal", variant: "obligation", result, says, work: vec![] }]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn same_up_to_order() {
        assert_eq!(test_moves(&SidesEqual, "prove x + 1 = 1 + x"), vec!["proved"]);
        assert!(test_moves(&SidesEqual, "prove (x + 1)^2 = x^2 + 2x + 1").is_empty());
    }
}
