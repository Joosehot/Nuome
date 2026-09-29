//! Cancel a common number factor between a numerator and a number
//! denominator: 6x/4 -> 3x/2, (2 + 2sqrt(2))/2 -> 1 + sqrt(2).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, terms, with_coeff, Expr, Math};
use crate::q::{gcd, Q};

pub struct Cancel;

impl Rule for Cancel {
    fn name(&self) -> &'static str {
        "cancel"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["common_factor"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("cancel", m, |e, _| {
            // (1/x) * x -> 1: a factor cancels the same denominator
            if let Expr::Mul(fs) = e {
                for (i, f) in fs.iter().enumerate() {
                    let Expr::Div(n, d) = f else { continue };
                    if let Some(j) = fs.iter().position(|g| g == &**d) {
                        let mut nf = fs.clone();
                        nf[i] = (**n).clone();
                        nf.remove(j);
                        return vec![Rewrite { variant: "common_factor", new: expr::mul(nf), says: Line::new().t("Cancel ").e(d).t(" against the denominator."), work: vec![] }];
                    }
                }
                return vec![];
            }
            let Expr::Div(a, b) = e else { return vec![] };
            // 2x/(2sqrt(u)) -> x/sqrt(u): a number factor on top and below
            if b.as_num().is_none() {
                let ((ca, ra), (cb, rb)) = (coeff(a), coeff(b));
                if ca.is_int() && cb.is_int() && !cb.is_zero() && !matches!(**a, Expr::Add(_)) {
                    let g = gcd(ca.num(), cb.num());
                    if g > 1 {
                        let gq = Q::int(g);
                        let (Some(na), Some(nb)) = (ca.div(&gq), cb.div(&gq)) else { return vec![] };
                        let new = expr::div(with_coeff(na, ra), with_coeff(nb, rb));
                        return vec![Rewrite { variant: "common_factor", new, says: Line::new().t(format!("Cancel the common factor {g}.")), work: vec![] }];
                    }
                }
                return vec![];
            }
            let Some(d) = b.as_num().filter(|d| d.is_int() && !d.is_zero() && !d.is_one()) else { return vec![] };
            let ts = terms(a);
            let parts: Vec<(Q, Expr)> = ts.iter().map(coeff).collect();
            if parts.iter().any(|(c, _)| !c.is_int()) || (ts.len() == 1 && parts[0].1.is_num(1)) {
                return vec![];
            }
            let g = parts.iter().fold(d.num(), |g, (c, _)| gcd(g, c.num()));
            let g = if d.is_neg() { -g } else { g };
            if g.abs() <= 1 && !(g == -1 && d.num() == -1) {
                return vec![];
            }
            let gq = Q::int(g);
            let Some(new_terms) = parts.iter().map(|(c, r)| Some(with_coeff(c.div(&gq)?, r.clone()))).collect::<Option<Vec<_>>>() else { return vec![] };
            let top = expr::add(new_terms);
            let dd = d.div(&gq).unwrap_or(Q::ONE);
            let new = if dd.is_one() { top } else { expr::div(top, Expr::Num(dd)) };
            vec![Rewrite { variant: "common_factor", new, says: Line::new().t(format!("Cancel the common factor {}.", g.abs())), work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn cancels_number_factors() {
        assert_eq!(test_moves(&Cancel, "simplify 6x/4"), vec!["3x/2"]);
        assert_eq!(test_moves(&Cancel, "simplify (4x + 6)/2"), vec!["2x + 3"]);
        assert!(test_moves(&Cancel, "simplify (x + 1)/2").is_empty());
    }
}
