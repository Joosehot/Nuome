//! Quotient rule: d/dx[f/g] = (f'g - fg')/g^2.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct DiffQuotient;

impl Rule for DiffQuotient {
    fn name(&self) -> &'static str {
        "diff_quotient"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rule"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_quotient", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            let Expr::Div(f, g) = &**inner else { return vec![] };
            if !g.has_var(v) {
                return vec![];
            }
            let d = |x: &Expr| Expr::Deriv(Box::new(x.clone()), v.clone());
            let top = Expr::Add(vec![expr::mul(vec![d(f), (**g).clone()]), Expr::Neg(Box::new(expr::mul(vec![(**f).clone(), d(g)])))]);
            let new = expr::div(top, expr::pow((**g).clone(), expr::num(2)));
            let says = Line::new().t("Quotient rule: (f/g)' = (f'g - fg')/g^2.");
            let work = vec![Line::new().t("f = ").e(f).t(", g = ").e(g).t(".")];
            vec![Rewrite { variant: "rule", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn quotient_rule() {
        assert_eq!(test_moves(&DiffQuotient, "differentiate x/(x + 1)"), vec!["(d/dx[x] * (x + 1) - x * d/dx[x + 1])/(x + 1)^2"]);
        assert!(test_moves(&DiffQuotient, "differentiate x/2").is_empty());
    }
}
