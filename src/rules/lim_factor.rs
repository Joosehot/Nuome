//! Factor and cancel: when top and bottom of a fraction are both 0 at
//! x = a, both have the factor x - a. Near the limit x != a, so it cancels:
//! lim(x->2) (x^2 - 4)/(x - 2) = lim(x->2) (x + 2).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct LimFactor;

/// x - a
fn factor(a: &Q, v: &str) -> Expr {
    Poly(vec![a.neg(), Q::ONE]).to_expr(v)
}

impl Rule for LimFactor {
    fn name(&self) -> &'static str {
        "lim_factor"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["cancel"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("lim_factor", m, |e, _| {
            let Expr::Limit(f, v, p) = e else { return vec![] };
            let (Some(a), Expr::Div(top, bottom)) = (p.eval_q(&|_| None), &**f) else { return vec![] };
            let (Some(n), Some(d)) = (poly::from_expr(top, v), poly::from_expr(bottom, v)) else { return vec![] };
            let zero_at = |q: &Poly| q.eval(&a).is_some_and(|x| x.is_zero()) && !q.is_zero();
            if !(zero_at(&n) && zero_at(&d)) {
                return vec![];
            }
            let (mut n1, mut d1, mut k) = (n.clone(), d.clone(), 0);
            while zero_at(&n1) && zero_at(&d1) {
                let (Some(x), Some(y)) = (n1.deflate(&a), d1.deflate(&a)) else { return vec![] };
                (n1, d1, k) = (x, y, k + 1);
            }
            let xa = factor(&a, v);
            let common = if k == 1 { xa.clone() } else { expr::pow(xa.clone(), expr::num(k)) };
            let rest = match d1.deg() {
                Some(0) => {
                    let Some(n2) = d1.coef(0).recip().and_then(|r| n1.scale(&r)) else { return vec![] };
                    n2.to_expr(v)
                }
                _ => expr::div(n1.to_expr(v), d1.to_expr(v)),
            };
            let mut work = vec![Line::new().e(top).t(" = ").e(&expr::mul(vec![common.clone(), n1.to_expr(v)]))];
            if d1.deg() != Some(0) || !d1.coef(0).is_one() {
                work.push(Line::new().e(bottom).t(" = ").e(&expr::mul(vec![common.clone(), d1.to_expr(v)])));
            }
            let says = Line::new().t(format!("Top and bottom are both 0 at {v} = {a}: factor out ")).e(&xa).t(format!(" and cancel it ({v} is never {a} on the way to the limit)."));
            vec![Rewrite { variant: "cancel", new: Expr::Limit(Box::new(rest), v.clone(), p.clone()), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factors_and_cancels() {
        assert_eq!(test_moves(&LimFactor, "limit of (x^2 - 4)/(x - 2) as x approaches 2"), vec!["lim(x->2) (x + 2)"]);
        assert_eq!(test_moves(&LimFactor, "limit of (x^2 - 1)/(x^2 + x - 2) as x approaches 1"), vec!["lim(x->1) (x + 1)/(x + 2)"]);
        assert!(test_moves(&LimFactor, "limit of (x^2 - 4)/(x - 2) as x approaches 3").is_empty());
    }
}
