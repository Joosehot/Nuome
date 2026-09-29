//! Linearity: the derivative of a sum is the sum of the derivatives, and a
//! constant factor stays outside: d/dx[3x^2 + x] = 3 d/dx[x^2] + d/dx[x].

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct DiffLinear;

fn d(e: Expr, v: &str) -> Expr {
    Expr::Deriv(Box::new(e), v.to_string())
}

impl Rule for DiffLinear {
    fn name(&self) -> &'static str {
        "diff_linear"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum", "constant"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_linear", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            match &**inner {
                Expr::Add(ts) => {
                    let new = Expr::Add(ts.iter().map(|t| d(t.clone(), v)).collect());
                    vec![Rewrite { variant: "sum", new, says: Line::new().t("Differentiate term by term."), work: vec![] }]
                }
                Expr::Mul(fs) if fs.iter().any(|f| !f.has_var(v)) && fs.iter().any(|f| f.has_var(v)) => {
                    let mut c: Vec<Expr> = fs.iter().filter(|f| !f.has_var(v)).cloned().collect();
                    let rest = expr::mul(fs.iter().filter(|f| f.has_var(v)).cloned().collect());
                    let k = expr::mul(c.clone());
                    c.push(d(rest, v));
                    vec![Rewrite { variant: "constant", new: Expr::Mul(c), says: Line::new().t("The constant factor ").e(&k).t(" stays outside."), work: vec![] }]
                }
                Expr::Neg(a) if a.has_var(v) => vec![Rewrite { variant: "constant", new: expr::neg(d((**a).clone(), v)), says: Line::new().t("The minus sign stays outside."), work: vec![] }],
                Expr::Div(a, b) if !b.has_var(v) && a.has_var(v) => {
                    vec![Rewrite { variant: "constant", new: expr::div(d((**a).clone(), v), (**b).clone()), says: Line::new().t("Dividing by ").e(b).t(" stays outside."), work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn splits_sums_and_constants() {
        assert_eq!(test_moves(&DiffLinear, "differentiate x^2 + x"), vec!["d/dx[x^2] + d/dx[x]"]);
        assert_eq!(test_moves(&DiffLinear, "differentiate 3x^2"), vec!["3 * d/dx[x^2]"]);
        assert!(test_moves(&DiffLinear, "differentiate sin(x)").is_empty());
    }
}
