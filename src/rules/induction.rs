//! Proof by induction of a sum formula: 1 + 2 + ... + n = n(n + 1)/2.
//! The base case is worked out; the step uses the hypothesis to turn the
//! sum up to n + 1 into (formula at n) + (next term), and what remains is an
//! ordinary identity for the other rules to prove.

use super::{Cx, Line, Move, Rule};
use crate::calls::Named;
use crate::expr::{self, series_parts, Expr, Math};
use crate::model::Task;

pub struct Induction;

fn fold_sums(e: &Expr) -> Expr {
    let e = match e {
        Expr::Add(ts) => {
            let (nums, rest): (Vec<Expr>, Vec<Expr>) = ts.iter().map(fold_sums).partition(|t| t.as_num().is_some());
            let total = nums.iter().try_fold(crate::q::Q::ZERO, |a, t| a.add(&t.as_num()?));
            match total {
                Some(q) if nums.len() > 1 => {
                    let mut v = rest;
                    if !q.is_zero() {
                        v.push(Expr::Num(q));
                    }
                    expr::add(v)
                }
                _ => Expr::Add(ts.iter().map(fold_sums).collect()),
            }
        }
        Expr::Mul(fs) => Expr::Mul(fs.iter().map(fold_sums).collect()),
        Expr::Div(a, b) => expr::div(fold_sums(a), fold_sums(b)),
        Expr::Pow(a, b) => expr::pow(fold_sums(a), fold_sums(b)),
        Expr::Neg(a) => expr::neg(fold_sums(a)),
        e => e.clone(),
    };
    expr::tidy(e)
}

impl Rule for Induction {
    fn name(&self) -> &'static str {
        "induction"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["on_n"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if cx.task() != Task::Prove {
            return vec![];
        }
        let Math::Eq(l, r) = m else { return vec![] };
        let (sum, formula) = match (l, r) {
            (Expr::Call(Named::Series, a), f) if !f.has_call() => (a, f),
            (f, Expr::Call(Named::Series, a)) if !f.has_call() => (a, f),
            _ => return vec![],
        };
        let Some((term, lo, n)) = series_parts(sum) else { return vec![] };
        let Some(s) = lo.as_num().filter(|q| q.is_int()) else { return vec![] };
        // put n + 1 in, with the numbers inside each sum added up: n + 1 + 1 -> n + 2
        let at = |e: &Expr, k: Expr| fold_sums(&expr::tidy(e.subst(n, &k)));
        // the base case, in exact numbers
        let (Some(first), Some(claim)) = (at(term, Expr::Num(s)).eval_q(&|_| None), at(formula, Expr::Num(s)).eval_q(&|_| None)) else { return vec![] };
        if first != claim {
            return vec![];
        }
        let next = expr::add(vec![expr::var(n), expr::num(1)]);
        let step_left = expr::add(vec![formula.clone(), at(term, next.clone())]);
        let step_right = at(formula, next.clone());
        let series = Expr::Call(Named::Series, sum.clone());
        let work = vec![
            Line::new().t(format!("Base case {n} = {s}: the sum is its first term, ")).e(&Expr::Num(first)).t(", and the formula gives ").e(&at(formula, Expr::Num(s))).t(format!(" = {claim}.")),
            Line::new().t(format!("Hypothesis: for some {n} >= {s}, ")).e(&series).t(" = ").e(formula).t("."),
            Line::new().t(format!("Step: the sum up to {n} + 1 is the sum up to {n} plus the next term ")).e(&at(term, next.clone())).t(";"),
            Line::new().t("  by the hypothesis that is ").e(&step_left).t(format!(", which must equal the formula at {n} + 1:")),
        ];
        let says = Line::new().t(format!("Prove it by induction on {n}."));
        vec![Move { rule: "induction", variant: "on_n", result: Math::Eq(step_left, step_right), says, work }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn sets_up_the_inductive_step() {
        assert_eq!(test_moves(&Induction, "prove 1 + 2 + ... + n = n(n + 1)/2"), vec!["n(n + 1)/2 + n + 1 = (n + 1)(n + 2)/2"]);
        // a false base case: no induction
        assert!(test_moves(&Induction, "prove 1 + 2 + ... + n = n^2 + 1").is_empty());
    }
}
