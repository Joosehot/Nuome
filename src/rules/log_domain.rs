//! A logarithm needs a positive argument. A candidate that makes the
//! argument of a log in the original equation 0 or negative is not a
//! solution: it crept in when the logs were combined or undone.

use super::log_laws::as_log;
use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};

pub struct LogDomain;

/// Every log argument in the problem that contains the letter.
pub fn log_arguments(m: &Math, v: &str) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    for e in m.slots() {
        for (_, n) in e.walk() {
            if let Some((_, a)) = as_log(n) {
                if a.has_var(v) && !out.contains(&a) {
                    out.push(a);
                }
            }
        }
    }
    out
}

/// Is `a` 0 or negative at the value?
fn not_positive(a: &Expr, v: &str, val: &Expr) -> bool {
    let at = a.subst(v, val);
    match at.eval_q(&|_| None) {
        Some(q) => q.is_neg() || q.is_zero(),
        None => at.eval_f(&|_| f64::NAN) <= 1e-12,
    }
}

impl Rule for LogDomain {
    fn name(&self) -> &'static str {
        "log_domain"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["outside_domain"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let x = Expr::Var(v.to_string());
        let eqs: Vec<(Expr, Expr)> = match m {
            Math::Eq(l, r) => vec![(l.clone(), r.clone())],
            Math::Or(e) => e.clone(),
            _ => return vec![],
        };
        let args = log_arguments(&cx.req.problem.value, v);
        for (i, (l, r)) in eqs.iter().enumerate() {
            if *l != x || r.has_var(v) {
                continue;
            }
            let Some(a) = args.iter().find(|a| not_positive(a, v, r)) else { continue };
            let mut rest = eqs.clone();
            rest.remove(i);
            let result = match rest.len() {
                0 => Math::NoSolution,
                1 => Math::Eq(rest[0].0.clone(), rest[0].1.clone()),
                _ => Math::Or(rest),
            };
            let sol = Math::Eq(l.clone(), r.clone());
            let says = if *a == x {
                Line::new().m(&sol).t(" is not a solution: a log needs a positive number.")
            } else {
                let at = a.subst(v, r).eval_q(&|_| None).map_or_else(|| crate::expr::tidy(a.subst(v, r)), Expr::Num);
                Line::new().m(&sol).t(" is not a solution: it makes ").e(a).t(" = ").e(&at).t(", and a log needs a positive number.")
            };
            return vec![Move { rule: "log_domain", variant: "outside_domain", result, says, work: vec![] }];
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{num, var};
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn drops_values_outside_the_domain() {
        let cfg = crate::config::Config::builtin();
        let req = test_cx_req("solve log(x) + log(x - 3) = 1");
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let both = Math::Or(vec![(var("x"), num(5)), (var("x"), num(-2))]);
        assert_eq!(LogDomain.moves(&both, &cx)[0].result, Math::Eq(var("x"), num(5)));
        assert!(LogDomain.moves(&Math::Eq(var("x"), num(5)), &cx).is_empty());
    }
}
