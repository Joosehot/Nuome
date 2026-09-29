//! Arithmetic on plain numbers. Structure gives the order of operations:
//! a node folds only when its own operands are numbers, so in 2 + 3 * 4 the
//! product goes first. Variants: one operation at a time, or a whole
//! number-only expression in one go.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::q::{lcm, Q};

pub struct Fold;

fn plain(e: &Expr) -> bool {
    e.vars().is_empty() && !e.walk().iter().any(|(_, n)| matches!(n, Expr::Const(_) | Expr::Deriv(..) | Expr::Func(..) | Expr::Log(..)))
}

/// Side working for adding fractions: 3/4 + 1/6 = 9/12 + 2/12.
fn common_denominator(nums: &[Q]) -> Option<Line> {
    if nums.iter().all(|q| q.is_int()) {
        return None;
    }
    let mut l = 1i128;
    for q in nums {
        l = lcm(l, q.den())?;
    }
    let mut scaled = Vec::new();
    for q in nums {
        let n = expr::num(q.num().abs().checked_mul(l / q.den())?);
        let f = expr::div(n, expr::num(l));
        scaled.push(if q.is_neg() { Expr::Neg(Box::new(f)) } else { f });
    }
    let orig = Expr::Add(nums.iter().map(|q| Expr::Num(*q)).collect());
    Some(Line::new().e(&orig).t(" = ").e(&Expr::Add(scaled)).t(format!("   (common denominator {l})")))
}

/// Is `a/b` just how the number a/b is written (3/4), not a division to do?
pub fn is_written_fraction(a: &Expr, b: &Expr) -> bool {
    match (a.as_num(), b.as_num()) {
        (Some(x), Some(y)) => x.is_int() && y.is_int() && y.num() > 1 && crate::q::gcd(x.num(), y.num()) == 1,
        _ => false,
    }
}

impl Rule for Fold {
    fn name(&self) -> &'static str {
        "fold"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["one", "all"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("fold", m, |e, at| {
            let mut out = Vec::new();
            let result = |r: Q| Expr::Num(r);
            match e {
                Expr::Add(v) => {
                    let nums: Vec<Q> = v.iter().filter_map(|t| t.as_num()).collect();
                    if nums.len() >= 2 {
                        if let Some(sum) = nums.iter().try_fold(Q::ZERO, |a, b| a.add(b)) {
                            let mut rest: Vec<Expr> = v.iter().filter(|t| t.as_num().is_none()).cloned().collect();
                            let before = Expr::Add(v.iter().filter(|t| t.as_num().is_some()).cloned().collect());
                            if !sum.is_zero() || rest.is_empty() {
                                rest.push(result(sum));
                            }
                            let work = common_denominator(&nums).into_iter().collect();
                            out.push(Rewrite { variant: "one", new: expr::add(rest), says: Line::new().t("Work out ").e(&before).t(" = ").e(&result(sum)).t("."), work });
                        }
                    }
                }
                Expr::Mul(v) => {
                    let nums: Vec<Q> = v.iter().filter_map(|t| t.as_num()).collect();
                    if nums.len() >= 2 {
                        if let Some(p) = nums.iter().try_fold(Q::ONE, |a, b| a.mul(b)) {
                            let before = Expr::Mul(v.iter().filter(|t| t.as_num().is_some()).cloned().collect());
                            let mut rest = vec![result(p)];
                            rest.extend(v.iter().filter(|t| t.as_num().is_none()).cloned());
                            out.push(Rewrite { variant: "one", new: expr::mul(rest), says: Line::new().t("Work out ").e(&before).t(" = ").e(&result(p)).t("."), work: vec![] });
                        }
                    }
                }
                Expr::Div(a, b) if !is_written_fraction(a, b) => {
                    if let (Some(x), Some(y)) = (a.as_num(), b.as_num()) {
                        if let Some(r) = x.div(&y) {
                            let says = if r.is_int() { Line::new().t("Work out ").e(e).t(" = ").e(&result(r)).t(".") } else { Line::new().t("Reduce ").e(e).t(" to ").e(&result(r)).t(".") };
                            out.push(Rewrite { variant: "one", new: result(r), says, work: vec![] });
                        }
                    }
                }
                Expr::Pow(a, b) => {
                    if let (Some(x), Some(y)) = (a.as_num(), b.as_num()) {
                        if y.is_int() && y.num().abs() <= 64 && !(x.is_zero() && y.is_neg()) {
                            if let Some(r) = x.pow(y.num() as i64) {
                                out.push(Rewrite { variant: "one", new: result(r), says: Line::new().t("Work out ").e(e).t(" = ").e(&result(r)).t("."), work: vec![] });
                            }
                        }
                    }
                }
                _ => {}
            }
            // all at once: a number-only subtree with two or more operations,
            // taken at its top (its parent isn't number-only too)
            let ops = e.walk().iter().filter(|(_, n)| !matches!(n, Expr::Num(_))).count()
                - e.walk().iter().filter(|(_, n)| matches!(n, Expr::Div(a, b) if is_written_fraction(a, b))).count();
            if ops >= 2 && plain(e) {
                let parent_plain = !at.path.is_empty() && plain(at.math.slots()[at.slot].get(&at.path[..at.path.len() - 1]));
                if !parent_plain {
                    if let Some(r) = e.eval_q(&|_| None) {
                        out.push(Rewrite { variant: "all", new: result(r), says: Line::new().t("Work out ").e(e).t(" = ").e(&result(r)).t("."), work: vec![] });
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
    fn folds_in_order_of_operations() {
        let m = test_moves(&Fold, "what is 2 + 3 * 4");
        assert_eq!(m, vec!["14", "2 + 12"]);
        assert_eq!(test_moves(&Fold, "what is 3/4 + 1/6"), vec!["11/12"]);
        assert!(test_moves(&Fold, "simplify 2x + y").is_empty());
    }
}
