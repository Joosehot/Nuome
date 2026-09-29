//! A function's value at a point, [f]_(x=a) = f(a), once f is worked out:
//! the height and the slope of a tangent.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};

pub struct EvaluateAt;

impl Rule for EvaluateAt {
    fn name(&self) -> &'static str {
        "evaluate_at"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["substitute"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("evaluate_at", m, |e, _| {
            let Expr::At(f, v, p) = e else { return vec![] };
            if f.is_pending() {
                return vec![];
            }
            let put = f.subst(v, p);
            // a plain number comes out worked: 2x at x = 1 is 2 * 1 = 2
            let (new, says) = match put.eval_q(&|_| None) {
                Some(q) if put.as_num() != Some(q) => (Expr::Num(q), Line::new().t(format!("Put in {v} = ")).e(p).t(": ").e(f).t(" = ").e(&put).t(" = ").e(&Expr::Num(q)).t(".")),
                _ => (put.clone(), Line::new().t(format!("Put in {v} = ")).e(p).t(": ").e(f).t(" becomes ").e(&put).t(".")),
            };
            vec![Rewrite { variant: "substitute", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn evaluates_once_worked_out() {
        // the tangent's height [x^2]_(x=1) is ready; its slope waits for d/dx
        assert_eq!(test_moves(&EvaluateAt, "tangent to y = x^2 at x = 1"), vec!["y = 1 + [d/dx[x^2]]_(x=1) * (x - 1)"]);
        assert!(test_moves(&EvaluateAt, "differentiate x^2").is_empty());
    }
}
