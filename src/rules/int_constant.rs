//! The constant of integration: an indefinite integral is a family of
//! functions, F(x) + C. C is written once, when the working is done.

use super::{Cx, Line, Move, Rule};
use crate::expr::{self, Expr, Math};
use crate::model::Task;

pub struct IntConstant;

/// The letter for the constant of integration. The lexicon lowercases every
/// word, so a question can never contain it.
pub const C: &str = "C";

impl Rule for IntConstant {
    fn name(&self) -> &'static str {
        "int_constant"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["add"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Expr(e) = m else { return vec![] };
        if cx.task() != Task::Integrate || cx.req.calc.bounds.is_some() || e.is_pending() || e.has_var(C) {
            return vec![];
        }
        // only once every other step is done
        let others = cx.cfg.task_rules(cx.task()).into_iter().filter(|r| r.name() != self.name());
        if others.into_iter().any(|r| !r.moves(m, cx).is_empty()) {
            return vec![];
        }
        let result = Math::Expr(expr::add(vec![e.clone(), Expr::Var(C.to_string())]));
        vec![Move { rule: "int_constant", variant: "add", result, says: Line::new().t("Add the constant of integration, C."), work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn adds_c_at_the_end() {
        let req = test_cx_req("integrate x^2");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let done = Math::Expr(expr::div(expr::pow(expr::var("x"), expr::num(3)), expr::num(3)));
        assert_eq!(crate::print::math(&IntConstant.moves(&done, &cx)[0].result, crate::print::Style::Ascii), "x^3/3 + C");
        assert!(IntConstant.moves(&req.start(), &cx).is_empty());
    }
}
