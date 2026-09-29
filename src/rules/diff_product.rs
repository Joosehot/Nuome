//! Product rule: d/dx[fg] = f'g + fg'.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct DiffProduct;

impl Rule for DiffProduct {
    fn name(&self) -> &'static str {
        "diff_product"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["two_factors"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_product", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            if inner.has_deriv() {
                return vec![]; // a higher derivative: the inner one first
            }
            let Expr::Mul(fs) = &**inner else { return vec![] };
            if fs.iter().any(|f| !f.has_var(v)) || fs.len() < 2 {
                return vec![];
            }
            let f = fs[0].clone();
            let g = expr::mul(fs[1..].to_vec());
            let d = |x: &Expr| Expr::Deriv(Box::new(x.clone()), v.clone());
            let new = Expr::Add(vec![expr::mul(vec![d(&f), g.clone()]), expr::mul(vec![f.clone(), d(&g)])]);
            let says = Line::new().t("Product rule: (fg)' = f'g + fg'.");
            let work = vec![Line::new().t("f = ").e(&f).t(", g = ").e(&g).t(".")];
            vec![Rewrite { variant: "two_factors", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn product_rule() {
        assert_eq!(test_moves(&DiffProduct, "differentiate x^3 sin x"), vec!["d/dx[x^3] * sin(x) + x^3 * d/dx[sin(x)]"]);
        assert!(test_moves(&DiffProduct, "differentiate 3x^2").is_empty());
    }
}
