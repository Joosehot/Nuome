//! L'Hopital's rule: when a fraction's limit has the form 0/0 or
//! infinity/infinity, the limit of f/g equals the limit of f'/g'.

use super::lim_infinity::grows;
use super::lim_substitute::infinite;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};

pub struct LimLhopital;

impl Rule for LimLhopital {
    fn name(&self) -> &'static str {
        "lim_lhopital"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["zero_over_zero", "infinity_over_infinity"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let c = &cx.cfg.calculus;
        local("lim_lhopital", m, |e, _| {
            let Expr::Limit(q, v, p) = e else { return vec![] };
            let Expr::Div(f, g) = &**q else { return vec![] };
            if q.is_pending() || !f.has_var(v) || !g.has_var(v) {
                return vec![];
            }
            let at = |h: &Expr, x: f64| h.eval_f(&|n| if n == v { x } else { f64::NAN });
            let variant = match infinite(p) {
                None => {
                    let x = p.eval_f(&|_| f64::NAN);
                    let zero = |h: &Expr| match h.subst(v, p).eval_q(&|_| None) {
                        Some(q) => q.is_zero(),
                        None => at(h, x).abs() < c.zero,
                    };
                    if !(zero(f) && zero(g)) {
                        return vec![];
                    }
                    "zero_over_zero"
                }
                Some(s) => {
                    if !(grows(f, v, s, cx.cfg) && grows(g, v, s, cx.cfg)) {
                        return vec![];
                    }
                    "infinity_over_infinity"
                }
            };
            let d = |h: &Expr| Expr::Deriv(Box::new(h.clone()), v.clone());
            let new = Expr::Limit(Box::new(crate::expr::div(d(f), d(g))), v.clone(), p.clone());
            let form = if variant == "zero_over_zero" { "0/0" } else { "infinity/infinity" };
            let says = Line::new().t(format!("The limit has the form {form}: by L'Hopital's rule, differentiate the top and the bottom."));
            vec![Rewrite { variant, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn lhopital() {
        assert_eq!(test_moves(&LimLhopital, "limit of (e^x - 1)/x as x approaches 0"), vec!["lim(x->0) d/dx[e^x - 1]/d/dx[x]"]);
        assert_eq!(test_moves(&LimLhopital, "limit of x/e^x as x approaches infinity"), vec!["lim(x->infinity) d/dx[x]/d/dx[e^x]"]);
        assert!(test_moves(&LimLhopital, "limit of (x + 1)/x as x approaches 1").is_empty());
    }
}
