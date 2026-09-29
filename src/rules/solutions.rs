//! Tidy the solution list: a solution found twice is one (a double root),
//! and a value that makes a denominator of the original equation zero is
//! not a solution at all (it crept in when both sides were multiplied).

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};

pub struct Solutions;

/// Denominators in the problem that contain the letter.
pub fn denominators(m: &Math, v: &str) -> Vec<Expr> {
    let mut out = Vec::new();
    for e in m.slots() {
        for (_, n) in e.walk() {
            match n {
                Expr::Div(_, d) if d.has_var(v) && !out.contains(&**d) => out.push((**d).clone()),
                Expr::Pow(b, p) if b.has_var(v) && p.as_num().is_some_and(|q| q.is_neg()) && !out.contains(&**b) => out.push((**b).clone()),
                _ => {}
            }
        }
    }
    out
}

/// Does substituting `val` make `d` zero?
pub fn vanishes(d: &Expr, v: &str, val: &Expr) -> bool {
    let at = d.subst(v, val);
    match at.eval_q(&|_| None) {
        Some(q) => q.is_zero(),
        None => at.eval_f(&|_| f64::NAN).abs() < 1e-12,
    }
}

impl Rule for Solutions {
    fn name(&self) -> &'static str {
        "solutions"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["repeated", "extraneous"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let x = Expr::Var(v.to_string());
        let eqs: Vec<(Expr, Expr)> = match m {
            Math::Eq(l, r) => vec![(l.clone(), r.clone())],
            Math::Or(e) => e.clone(),
            _ => return vec![],
        };
        let solved = |(l, r): &(Expr, Expr)| *l == x && !r.has_var(v);
        let rebuild = |mut rest: Vec<(Expr, Expr)>| match rest.len() {
            0 => Math::NoSolution,
            1 => {
                let (a, b) = rest.pop().unwrap();
                Math::Eq(a, b)
            }
            _ => Math::Or(rest),
        };
        // repeated
        for i in 0..eqs.len() {
            for j in i + 1..eqs.len() {
                if eqs[i] == eqs[j] && solved(&eqs[i]) {
                    let mut rest = eqs.clone();
                    rest.remove(j);
                    let says = Line::new().m(&Math::Eq(eqs[i].0.clone(), eqs[i].1.clone())).t(" appears twice: it is one solution (a double root).");
                    return vec![Move { rule: "solutions", variant: "repeated", result: rebuild(rest), says, work: vec![] }];
                }
            }
        }
        // extraneous
        let dens = denominators(&cx.req.problem.value, v);
        for (i, eq) in eqs.iter().enumerate() {
            if !solved(eq) {
                continue;
            }
            if let Some(d) = dens.iter().find(|d| vanishes(d, v, &eq.1)) {
                let mut rest = eqs.clone();
                rest.remove(i);
                let says = Line::new().m(&Math::Eq(eq.0.clone(), eq.1.clone())).t(" makes ").e(d).t(" zero in the original equation, so it is not a solution.");
                return vec![Move { rule: "solutions", variant: "extraneous", result: rebuild(rest), says, work: vec![] }];
            }
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
    fn drops_repeats_and_extraneous_roots() {
        let cfg = crate::config::Config::builtin();
        let req = test_cx_req("solve x/(x - 1) = 1/(x - 1)");
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        assert_eq!(Solutions.moves(&Math::Eq(var("x"), num(1)), &cx)[0].result, Math::NoSolution);
        let two = Math::Or(vec![(var("x"), num(3)), (var("x"), num(3))]);
        assert_eq!(Solutions.moves(&two, &cx)[0].result, Math::Eq(var("x"), num(3)));
        assert!(Solutions.moves(&Math::Eq(var("x"), num(2)), &cx).is_empty());
    }
}
