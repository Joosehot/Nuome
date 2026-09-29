//! The derivatives to know by heart: sin, cos, tan, e^x, ln, sqrt, a^x.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Math};

pub struct DiffElementary;

/// f'(u) for an outer function f, written in terms of u.
pub fn outer(f: Func, u: &Expr) -> Expr {
    let u = u.clone();
    match f {
        Func::Sin => expr::func(Func::Cos, u),
        Func::Cos => expr::neg(expr::func(Func::Sin, u)),
        Func::Tan => expr::div(expr::num(1), expr::pow(expr::func(Func::Cos, u), expr::num(2))),
        Func::Exp => expr::func(Func::Exp, u),
        Func::Ln => expr::div(expr::num(1), u),
        Func::Sqrt => expr::div(expr::num(1), expr::mul(vec![expr::num(2), expr::sqrt(u)])),
        Func::Abs => expr::div(u.clone(), expr::func(Func::Abs, u)),
    }
}

impl Rule for DiffElementary {
    fn name(&self) -> &'static str {
        "diff_elementary"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["table"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_elementary", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            let x = expr::var(v);
            let new = match &**inner {
                Expr::Func(f, a) if **a == x => outer(*f, &x),
                Expr::Pow(b, a) if **a == x && b.as_num().is_some_and(|q| !q.is_neg()) => expr::mul(vec![(**inner).clone(), expr::func(Func::Ln, (**b).clone())]),
                _ => return vec![],
            };
            let says = Line::new().t("Standard derivative: ").e(e).t(" = ").e(&new).t(".");
            vec![Rewrite { variant: "table", new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn knows_the_table() {
        assert_eq!(test_moves(&DiffElementary, "differentiate sin x"), vec!["cos(x)"]);
        assert_eq!(test_moves(&DiffElementary, "differentiate ln x"), vec!["1/x"]);
        assert!(test_moves(&DiffElementary, "differentiate sin(2x)").is_empty());
    }
}
