//! Difference of two squares: a^2 - b^2 = (a - b)(a + b).

use super::powers::{base_exp, power_of};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};

pub struct DiffSquares;

/// The square root of a positive term that is a perfect square: 9x^2 -> 3x.
pub fn term_sqrt(t: &Expr) -> Option<Expr> {
    let (c, rest) = coeff(t);
    let r = c.sqrt()?;
    let factors = match rest {
        Expr::Mul(v) => v,
        x if x.is_num(1) => vec![],
        x => vec![x],
    };
    let mut out = Vec::new();
    for f in factors {
        let (b, e) = base_exp(&f)?;
        let half = e.div(&crate::q::Q::int(2))?;
        if !half.is_int() {
            return None;
        }
        out.push(power_of(b, half));
    }
    Some(with_coeff(r, expr::mul(out)))
}

impl Rule for DiffSquares {
    fn name(&self) -> &'static str {
        "diff_squares"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["identity"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("diff_squares", m, |e, _| {
            let Expr::Add(ts) = e else { return vec![] };
            if ts.len() != 2 {
                return vec![];
            }
            for (p, n) in [(&ts[0], &ts[1]), (&ts[1], &ts[0])] {
                let (cn, rn) = coeff(n);
                if !cn.is_neg() || coeff(p).0.is_neg() {
                    continue;
                }
                let (Some(a), Some(b)) = (term_sqrt(p), term_sqrt(&with_coeff(cn.neg(), rn))) else { continue };
                let new = Expr::Mul(vec![expr::add(vec![a.clone(), expr::neg(b.clone())]), expr::add(vec![a.clone(), b.clone()])]);
                let says = Line::new().t("Difference of two squares: ").e(&expr::pow(a.clone(), expr::num(2))).t(" - ").e(&expr::pow(b.clone(), expr::num(2))).t(" = (a - b)(a + b).");
                return vec![Rewrite { variant: "identity", new, says, work: vec![] }];
            }
            vec![]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factors_difference_of_squares() {
        assert_eq!(test_moves(&DiffSquares, "factor x^2 - 9"), vec!["(x - 3)(x + 3)"]);
        assert_eq!(test_moves(&DiffSquares, "factor 4x^2 - 25y^2"), vec!["(2x - 5y)(2x + 5y)"]);
        assert!(test_moves(&DiffSquares, "factor x^2 + 9").is_empty());
    }
}
