//! A polynomial integrand in brackets is multiplied out first, so the
//! power rule can take it term by term: int (x^2 + 1)^2 dx = int (x^4 + 2x^2 + 1) dx.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::poly;

pub struct IntExpand;

impl Rule for IntExpand {
    fn name(&self) -> &'static str {
        "int_expand"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["polynomial"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_expand", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            // only brackets to multiply: a product or a power with a sum in it
            let brackets = inner.walk().iter().any(|(_, n)| match n {
                Expr::Mul(f) => f.iter().any(|x| matches!(x, Expr::Add(_))),
                Expr::Pow(b, _) => matches!(**b, Expr::Add(_)),
                _ => false,
            });
            if !brackets {
                return vec![];
            }
            let Some(p) = poly::from_expr(inner, v) else { return vec![] };
            let out = p.to_expr(v);
            let says = Line::new().t("Multiply out the integrand: ").e(inner).t(" = ").e(&out).t(".");
            vec![Rewrite { variant: "polynomial", new: Expr::Integral(Box::new(out), v.clone()), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn multiplies_out_polynomials() {
        assert_eq!(test_moves(&IntExpand, "integrate (x^2 + 1)^2"), vec!["int (x^4 + 2x^2 + 1) dx"]);
        assert_eq!(test_moves(&IntExpand, "integrate x(x + 2)"), vec!["int (x^2 + 2x) dx"]);
        assert!(test_moves(&IntExpand, "integrate x^2 + 1").is_empty());
    }
}
