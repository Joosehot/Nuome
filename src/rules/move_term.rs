//! Do the same to both sides: add or subtract a term. Variants: move a
//! number off the letter's side, move a letter term to the other side, or
//! (degree 2 and up) move everything to one side to get "... = 0".

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Expr, Math};
use crate::poly;

pub struct MoveTerm;

fn negate(t: &Expr) -> Expr {
    let (c, r) = coeff(t);
    with_coeff(c.neg(), r)
}

/// "Subtract 3 from both sides." / "Add 2x to both sides."
fn says(t: &Expr) -> Line {
    let (c, r) = coeff(t);
    if c.is_neg() {
        Line::new().t("Add ").e(&with_coeff(c.neg(), r)).t(" to both sides.")
    } else {
        Line::new().t("Subtract ").e(t).t(" from both sides.")
    }
}

/// Move term `k` of `from` to `to`: (new from, new to).
fn shift(from: &Expr, k: usize, to: &Expr) -> (Expr, Expr) {
    let mut f = terms(from);
    let t = f.remove(k);
    let mut o = if to.is_num(0) { vec![] } else { terms(to) };
    o.push(negate(&t));
    (expr::add(f), expr::add(o))
}

pub fn var_in_denominator(e: &Expr, v: &str) -> bool {
    e.walk().iter().any(|(_, n)| match n {
        Expr::Div(_, d) => d.has_var(v),
        Expr::Pow(b, p) => b.has_var(v) && p.as_num().is_some_and(|q| q.is_neg()),
        _ => false,
    })
}

pub fn degree(l: &Expr, r: &Expr, v: &str) -> Option<usize> {
    poly::from_expr(&expr::add(vec![l.clone(), expr::neg(r.clone())]), v)?.deg()
}

impl Rule for MoveTerm {
    fn name(&self) -> &'static str {
        "move_term"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["constant", "variable", "standard_form"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("move_term", m, |l, r| {
            let mut out = Vec::new();
            let (lv, rv) = (l.has_var(v), r.has_var(v));
            if !(lv || rv) || var_in_denominator(l, v) || var_in_denominator(r, v) {
                return out;
            }
            let deg = degree(l, r, v);
            let high = deg.is_some_and(|d| d >= 2);
            if high && !r.is_num(0) && !(l.is_num(0) && rv) {
                let mut left = terms(l);
                if !r.is_num(0) {
                    left.extend(terms(r).iter().map(negate));
                }
                let says = if terms(r).len() == 1 { says(r) } else { Line::new().t("Move every term to the left side.") };
                out.push(EqRewrite { variant: "standard_form", to: Branch::One(expr::add(left), expr::num(0)), says, work: vec![] });
            }
            let one_var_term = |s: &Expr| terms(s).iter().filter(|t| t.has_var(v)).count() == 1;
            if lv && rv {
                if !high {
                    for (k, t) in terms(r).iter().enumerate() {
                        if t.has_var(v) {
                            let (nr, nl) = shift(r, k, l);
                            out.push(EqRewrite { variant: "variable", to: Branch::One(nl, nr), says: says(t), work: vec![] });
                        }
                    }
                    for (k, t) in terms(l).iter().enumerate() {
                        if t.has_var(v) {
                            let (nl, nr) = shift(l, k, r);
                            out.push(EqRewrite { variant: "variable", to: Branch::One(nl, nr), says: says(t), work: vec![] });
                        }
                    }
                }
            } else {
                // numbers off the letter's side: only while that leaves one way to finish
                let (side, other, left) = if lv { (l, r, true) } else { (r, l, false) };
                if matches!(side, Expr::Add(_)) && (!high || one_var_term(side)) {
                    for (k, t) in terms(side).iter().enumerate() {
                        if !t.has_var(v) {
                            let (ns, no) = shift(side, k, other);
                            let to = if left { Branch::One(ns, no) } else { Branch::One(no, ns) };
                            out.push(EqRewrite { variant: "constant", to, says: says(t), work: vec![] });
                        }
                    }
                }
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn moves_terms_across() {
        assert_eq!(test_moves(&MoveTerm, "solve 2x + 3 = 7"), vec!["2x = 7 - 3"]);
        assert_eq!(test_moves(&MoveTerm, "solve 5x = 3x + 4"), vec!["5x - 3x = 4", "0 = 3x + 4 - 5x"]);
        assert_eq!(test_moves(&MoveTerm, "solve x^2 + 3x = 4"), vec!["x^2 + 3x - 4 = 0"]);
        assert!(test_moves(&MoveTerm, "solve 2x = 8").is_empty());
    }
}
