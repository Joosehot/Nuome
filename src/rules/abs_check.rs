//! Check each case of an absolute value equation in the original: a
//! candidate from |A| = B with B negative there doesn't fit, and is
//! rejected with the numbers that show it.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Func, Math};
use crate::print::{self, Style};

pub struct AbsCheck;

impl Rule for AbsCheck {
    fn name(&self) -> &'static str {
        "abs_check"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rejected"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let x = Expr::Var(v.to_string());
        let Math::Eq(ol, or) = &cx.req.problem.value else { return vec![] };
        if !ol.walk().iter().chain(or.walk().iter()).any(|(_, n)| matches!(n, Expr::Func(Func::Abs, _))) {
            return vec![];
        }
        let eqs: Vec<(Expr, Expr)> = match m {
            Math::Eq(l, r) => vec![(l.clone(), r.clone())],
            Math::Or(e) => e.clone(),
            _ => return vec![],
        };
        for (i, (l, r)) in eqs.iter().enumerate() {
            if *l != x || r.has_var(v) {
                continue;
            }
            let Some(val) = r.eval_q(&|_| None) else { continue };
            let at = |e: &Expr| e.eval_q(&|n| if n == v { Some(val) } else { None });
            let (Some(a), Some(b)) = (at(ol), at(or)) else { continue };
            if a == b {
                continue;
            }
            let mut rest = eqs.clone();
            rest.remove(i);
            let result = match rest.len() {
                0 => Math::NoSolution,
                1 => Math::Eq(rest[0].0.clone(), rest[0].1.clone()),
                _ => Math::Or(rest),
            };
            let shown = print::math(&Math::Eq(ol.clone(), or.clone()), Style::Ascii);
            let says = Line::new().m(&Math::Eq(l.clone(), r.clone())).t(" doesn't fit the original equation: ");
            let says = says.t(format!("the sides of {shown} come out as {a} and {b}, so it is rejected."));
            return vec![Move { rule: "abs_check", variant: "rejected", result, says, work: vec![] }];
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{num, q, var};
    use crate::q::Q;
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn rejects_cases_that_do_not_fit() {
        let cfg = crate::config::Config::builtin();
        let req = test_cx_req("solve |x - 1| = 2x + 1");
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let both = Math::Or(vec![(var("x"), num(-2)), (var("x"), q(Q::ZERO))]);
        assert_eq!(AbsCheck.moves(&both, &cx)[0].result, Math::Eq(var("x"), num(0)));
        assert!(AbsCheck.moves(&Math::Eq(var("x"), num(0)), &cx).is_empty());
    }
}
