//! A straight line is written y = mx + c: the x term first, the number last.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::poly;

pub struct LineForm;

impl Rule for LineForm {
    fn name(&self) -> &'static str {
        "line_form"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["slope_intercept"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Eq(y @ Expr::Var(_), r) = m else { return vec![] };
        if r.is_pending() || !r.has_var(cx.var) {
            return vec![];
        }
        let Some(p) = poly::from_expr(r, cx.var).filter(|p| p.deg() == Some(1)) else { return vec![] };
        // only the order changes: same terms, written as mx + c
        let tidy = p.to_expr(cx.var);
        let same_terms = crate::expr::terms(&tidy).len() == crate::expr::terms(r).len();
        if tidy == *r || !same_terms {
            return vec![];
        }
        let says = Line::new().t("Write it as y = mx + c.");
        vec![Move { rule: "line_form", variant: "slope_intercept", result: Math::Eq(y.clone(), tidy), says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_cx_req;

    #[test]
    fn writes_mx_plus_c() {
        let req = test_cx_req("tangent to y = e^x at x = 0");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let m = Math::Eq(crate::expr::var("y"), crate::expr::add(vec![crate::expr::num(1), crate::expr::var("x")]));
        assert_eq!(crate::print::math(&LineForm.moves(&m, &cx)[0].result, crate::print::Style::Ascii), "y = x + 1");
        assert!(LineForm.moves(&req.start(), &cx).is_empty());
    }
}
