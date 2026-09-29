//! Direct substitution: where a function is continuous, its limit is its
//! value, lim(x->2) (x^2 + 1) = 2^2 + 1. Continuous here means: no
//! division by zero, no log of a number <= 0, no root of a negative, no tan
//! or sec where cos is 0 (cot, csc where sin is 0).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{Expr, Func, Math};

pub struct LimSubstitute;

/// The point as a sign of infinity: x -> infinity is 1, x -> -infinity is -1.
pub fn infinite(p: &Expr) -> Option<f64> {
    let x = p.eval_f(&|_| f64::NAN);
    x.is_infinite().then(|| x.signum())
}

/// Is `f` defined and continuous at v = p (p finite)?
pub fn continuous(f: &Expr, v: &str, p: &Expr, zero: f64) -> bool {
    let at = p.eval_f(&|_| f64::NAN);
    if !at.is_finite() || f.is_pending() {
        return false;
    }
    let val = |e: &Expr| e.eval_f(&|n| if n == v { at } else { f64::NAN });
    let exact_zero = |e: &Expr| match e.subst(v, p).eval_q(&|_| None) {
        Some(q) => q.is_zero(),
        None => val(e).abs() < zero,
    };
    if !val(f).is_finite() {
        return false;
    }
    f.walk().iter().all(|(_, n)| match n {
        Expr::Div(_, d) => !exact_zero(d),
        Expr::Pow(b, k) => !(k.as_num().is_some_and(|k| k.is_neg()) && exact_zero(b)),
        Expr::Func(Func::Ln, a) => val(a) > zero,
        Expr::Func(Func::Sqrt, a) => val(a) >= 0.0,
        Expr::Func(Func::Tan | Func::Sec, a) => val(a).cos().abs() > zero,
        Expr::Func(Func::Cot | Func::Csc, a) => val(a).sin().abs() > zero,
        _ => true,
    })
}

/// Is the function tidy (no + 0, / 1 or number work left), so the value
/// put in is the one a person would write? Checked with the task's own
/// bookkeeping rules.
fn tidy(f: &Expr, cx: &Cx) -> bool {
    let m = Math::Expr(f.clone());
    cx.cfg.task_rules(cx.task()).into_iter().filter(|r| matches!(r.name(), "fold" | "identity")).all(|r| r.moves(&m, cx).is_empty())
}

impl Rule for LimSubstitute {
    fn name(&self) -> &'static str {
        "lim_substitute"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["direct", "constant"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let zero = cx.cfg.calculus.zero;
        local("lim_substitute", m, |e, _| {
            let Expr::Limit(f, v, p) = e else { return vec![] };
            if f.is_pending() {
                return vec![];
            }
            if !f.has_var(v) {
                return vec![Rewrite { variant: "constant", new: (**f).clone(), says: Line::new().t("The limit of a constant is the constant."), work: vec![] }];
            }
            if !continuous(f, v, p, zero) || !tidy(f, cx) {
                return vec![];
            }
            let says = Line::new().t(format!("The function is continuous at {v} = ")).e(p).t(format!(", so put in {v} = ")).e(p).t(".");
            vec![Rewrite { variant: "direct", new: f.subst(v, p), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn substitutes_where_continuous() {
        assert_eq!(test_moves(&LimSubstitute, "limit of x^2 + 1 as x approaches 2"), vec!["2^2 + 1"]);
        assert!(test_moves(&LimSubstitute, "limit of (x^2 - 4)/(x - 2) as x approaches 2").is_empty());
        assert!(test_moves(&LimSubstitute, "limit of 1/x as x approaches infinity").is_empty());
    }
}
