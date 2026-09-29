//! Chain rule: d/dx[f(u)] = f'(u) * d/dx[u], for a function or a power of
//! something other than the bare letter.

use super::diff_elementary::outer;
use super::powers::power_of;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::q::Q;

pub struct DiffChain;

impl Rule for DiffChain {
    fn name(&self) -> &'static str {
        "diff_chain"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["function", "power"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_chain", m, |e, _| {
            let Expr::Deriv(inner, v) = e else { return vec![] };
            let x = expr::var(v);
            let (variant, u, fprime) = match &**inner {
                Expr::Func(f, u) if **u != x && u.has_var(v) => ("function", (**u).clone(), outer(*f, u)),
                Expr::Pow(b, n) if **b != x && b.has_var(v) && !n.has_var(v) => {
                    let Some(n) = n.as_num() else { return vec![] };
                    let Some(n1) = n.sub(&Q::ONE) else { return vec![] };
                    ("power", (**b).clone(), expr::mul(vec![Expr::Num(n), power_of((**b).clone(), n1)]))
                }
                _ => return vec![],
            };
            let new = expr::mul(vec![fprime, Expr::Deriv(Box::new(u.clone()), v.clone())]);
            let says = Line::new().t("Chain rule: differentiate the outside, times the derivative of the inside.");
            let work = vec![Line::new().t("inside: u = ").e(&u).t(".")];
            vec![Rewrite { variant, new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn chain_rule() {
        assert_eq!(test_moves(&DiffChain, "differentiate sin(2x)"), vec!["cos(2x) * d/dx[2x]"]);
        assert_eq!(test_moves(&DiffChain, "differentiate (x^2 + 1)^3"), vec!["3(x^2 + 1)^2 * d/dx[x^2 + 1]"]);
        assert!(test_moves(&DiffChain, "differentiate sin x").is_empty());
    }
}
