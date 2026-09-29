//! The fundamental theorem of calculus: once the antiderivative F is
//! found, the definite integral is [F]_a^b = F(b) - F(a).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct IntBounds;

impl Rule for IntBounds {
    fn name(&self) -> &'static str {
        "int_bounds"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["subtract"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_bounds", m, |e, _| {
            let Expr::Bounds(f, v, a, b) = e else { return vec![] };
            if f.is_pending() {
                return vec![];
            }
            let new = expr::add(vec![f.subst(v, b), expr::neg(f.subst(v, a))]);
            let says = Line::new().t("Put in the bounds: F(").e(b).t(") - F(").e(a).t(").");
            vec![Rewrite { variant: "subtract", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_cx_req;

    #[test]
    fn evaluates_between_the_bounds() {
        let req = test_cx_req("integrate x^2 from 0 to 3");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let f = expr::div(expr::pow(expr::var("x"), expr::num(3)), expr::num(3));
        let state = Math::Expr(Expr::Bounds(Box::new(f), "x".into(), Box::new(expr::num(0)), Box::new(expr::num(3))));
        assert_eq!(crate::print::math(&IntBounds.moves(&state, &cx)[0].result, crate::print::Style::Ascii), "3^3/3 - 0^3/3");
        assert!(IntBounds.moves(&req.start(), &cx).is_empty());
    }
}
