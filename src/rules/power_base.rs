//! Write both sides of an exponential equation as powers of one base:
//! 2^x = 8 -> 2^x = 2^3, 4^x = 8 -> 2^(2x) = 2^3, 9^x = 3^(x + 1) ->
//! 3^(2x) = 3^(x + 1). Variants: the number is a power of the base
//! already there, or both are powers of a smaller common base.

use super::distribute::times;
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, exact_log, Expr, Func, Konst, Math};
use crate::q::Q;

pub struct PowerBase;

/// (base, exponent) of an exponential in the letter: 2^(x + 1), e^(2x).
pub fn as_exp(e: &Expr, v: &str) -> Option<(Expr, Expr)> {
    match e {
        Expr::Pow(b, a) if a.has_var(v) && !b.has_var(v) => {
            let q = b.as_num()?;
            (!q.is_neg() && !q.is_zero() && !q.is_one()).then(|| ((**b).clone(), (**a).clone()))
        }
        Expr::Func(Func::Exp, a) if a.has_var(v) => Some((Expr::Const(Konst::E), (**a).clone())),
        _ => None,
    }
}

/// b^a, written e^a for base e.
pub fn power(b: Expr, a: Expr) -> Expr {
    if b == Expr::Const(Konst::E) {
        expr::func(Func::Exp, a)
    } else {
        expr::pow(b, a)
    }
}

/// The smallest whole base g with every number a whole power of it: (g, powers).
fn common_base(nums: &[Q]) -> Option<(Q, Vec<Q>)> {
    let top = nums.iter().filter(|q| q.is_int()).map(|q| q.num()).max()?.min(1000);
    for g in 2..=top.max(2) {
        let g = Q::int(g);
        let ks: Option<Vec<Q>> = nums.iter().map(|n| exact_log(&g, n).filter(|k| k.is_int())).collect();
        if let Some(ks) = ks {
            return Some((g, ks));
        }
    }
    None
}

impl Rule for PowerBase {
    fn name(&self) -> &'static str {
        "power_base"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same_base", "common_base"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("power_base", m, |l, r| {
            // the exponential side first; the other side is a number or another exponential
            let (flip, (b, a), other) = match (as_exp(l, v), as_exp(r, v)) {
                (Some(x), None) => (false, x, r),
                (None, Some(x)) => (true, x, l),
                (Some(x), Some(_)) => (false, x, r),
                _ => return vec![],
            };
            let Some(bq) = b.as_num() else { return vec![] };
            let orient = |p: Expr, q: Expr| if flip { Branch::One(q, p) } else { Branch::One(p, q) };
            if let Some((b2, a2)) = as_exp(other, v) {
                let Some(b2q) = b2.as_num() else { return vec![] };
                if b2q == bq {
                    return vec![];
                }
                let Some((g, ks)) = common_base(&[bq, b2q]) else { return vec![] };
                let ge = Expr::Num(g);
                let new_l = power(ge.clone(), times(&Expr::Num(ks[0]), &a));
                let new_r = power(ge.clone(), times(&Expr::Num(ks[1]), &a2));
                let mut work = Vec::new();
                for (q, k) in [(bq, ks[0]), (b2q, ks[1])] {
                    if q != g {
                        work.push(Line::new().t(format!("{q} = ")).e(&expr::pow(ge.clone(), Expr::Num(k))).t("."));
                    }
                }
                let says = Line::new().t(format!("Write both sides as powers of {g}."));
                return vec![EqRewrite { variant: "common_base", to: orient(new_l, new_r), says, work }];
            }
            let Some(c) = other.as_num() else { return vec![] };
            if c.is_neg() || c.is_zero() {
                return vec![];
            }
            // the number as a power of the base itself
            if let Some(k) = exact_log(&bq, &c).filter(|k| k.is_int()) {
                let says = Line::new().t(format!("Write {c} as a power of {bq}: {c} = ")).e(&expr::pow(b.clone(), Expr::Num(k))).t(".");
                return vec![EqRewrite { variant: "same_base", to: orient(power(b.clone(), a), expr::pow(b, Expr::Num(k))), says, work: vec![] }];
            }
            let Some((g, ks)) = common_base(&[bq, c]) else { return vec![] };
            let ge = Expr::Num(g);
            let new_l = power(ge.clone(), times(&Expr::Num(ks[0]), &a));
            let work = vec![
                Line::new().t(format!("{bq} = ")).e(&expr::pow(ge.clone(), Expr::Num(ks[0]))).t(", so ").e(&power(b, a.clone())).t(" = ").e(&new_l).t("."),
                Line::new().t(format!("{c} = ")).e(&expr::pow(ge.clone(), Expr::Num(ks[1]))).t("."),
            ];
            let says = Line::new().t(format!("Write both sides as powers of {g}."));
            vec![EqRewrite { variant: "common_base", to: orient(new_l, expr::pow(ge, Expr::Num(ks[1]))), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn writes_sides_as_powers_of_one_base() {
        assert_eq!(test_moves(&PowerBase, "solve 2^x = 8"), vec!["2^x = 2^3"]);
        assert_eq!(test_moves(&PowerBase, "solve 4^x = 8"), vec!["2^(2x) = 2^3"]);
        assert_eq!(test_moves(&PowerBase, "solve 9^x = 3^(x + 1)"), vec!["3^(2x) = 3^(x + 1)"]);
        assert!(test_moves(&PowerBase, "solve 2^x = 5").is_empty());
    }
}
