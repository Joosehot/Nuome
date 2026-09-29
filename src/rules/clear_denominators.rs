//! Clear fractions by multiplying both sides. Variants: by the least
//! common denominator of number fractions (x/2 + x/3 = 5 -> 3x + 2x = 30),
//! or by a denominator that contains the letter (then the solutions are
//! checked against it: see `solutions`).

use super::move_term::var_in_denominator;
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Expr, Math};
use crate::q::{lcm, Q};

pub struct ClearDenominators;

/// d * e with d cancelled wherever e divides by exactly d.
fn times_cancel(d: &Expr, e: &Expr) -> Expr {
    match e {
        Expr::Add(ts) => expr::add(ts.iter().map(|t| times_cancel(d, t)).collect()),
        Expr::Div(n, dd) if **dd == *d => (**n).clone(),
        Expr::Neg(a) => expr::neg(times_cancel(d, a)),
        Expr::Mul(fs) => {
            if let Some(k) = fs.iter().position(|f| matches!(f, Expr::Div(_, dd) if **dd == *d)) {
                let mut fs = fs.clone();
                if let Expr::Div(n, _) = &fs[k] {
                    fs[k] = (**n).clone();
                }
                expr::mul(fs)
            } else {
                expr::mul(vec![d.clone(), e.clone()])
            }
        }
        e if e.as_num().is_some() => expr::mul(vec![e.clone(), d.clone()]),
        e => expr::mul(vec![d.clone(), e.clone()]),
    }
}

impl Rule for ClearDenominators {
    fn name(&self) -> &'static str {
        "clear_denominators"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["numbers", "letter"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("clear_denominators", m, |l, r| {
            if !(l.has_var(v) || r.has_var(v)) {
                return vec![];
            }
            // a solved equation keeps its fractions: x = pi/6 + 2k pi
            if *l == Expr::Var(v.to_string()) && !r.has_var(v) {
                return vec![];
            }
            if var_in_denominator(l, v) || var_in_denominator(r, v) {
                let d = [l, r].iter().flat_map(|s| s.walk().into_iter().map(|(_, n)| n.clone()).collect::<Vec<_>>()).find_map(|n| match n {
                    Expr::Div(_, d) if d.has_var(v) => Some(*d),
                    _ => None,
                });
                let Some(d) = d else { return vec![] };
                let says = Line::new().t("Multiply both sides by ").e(&d).t("; it must not be 0, which is checked at the end.");
                let to = Branch::One(times_cancel(&d, l), times_cancel(&d, r));
                return vec![EqRewrite { variant: "letter", to, says, work: vec![] }];
            }
            let all: Vec<Expr> = terms(l).into_iter().chain(terms(r)).collect();
            let dens: Vec<i128> = all.iter().map(|t| coeff(t).0.den()).collect();
            if all.len() < 3 || dens.iter().all(|&d| d == 1) {
                return vec![];
            }
            let Some(big) = dens.iter().try_fold(1i128, |a, &d| lcm(a, d)) else { return vec![] };
            let lq = Q::int(big);
            let scale = |s: &Expr| -> Option<Expr> { Some(expr::add(terms(s).iter().map(|t| Some(with_coeff(coeff(t).0.mul(&lq)?, coeff(t).1))).collect::<Option<Vec<_>>>()?)) };
            let (Some(nl), Some(nr)) = (scale(l), scale(r)) else { return vec![] };
            let says = Line::new().t(format!("Multiply both sides by {big}, the least common denominator."));
            vec![EqRewrite { variant: "numbers", to: Branch::One(nl, nr), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn clears_fractions() {
        assert_eq!(test_moves(&ClearDenominators, "solve x/2 + x/3 = 5"), vec!["3x + 2x = 30"]);
        assert_eq!(test_moves(&ClearDenominators, "solve 3/(x - 1) = 2"), vec!["3 = 2(x - 1)"]);
        assert!(test_moves(&ClearDenominators, "solve 2x + 1 = 5").is_empty());
    }
}
