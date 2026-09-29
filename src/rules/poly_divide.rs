//! Polynomial division: (x^3 - 1)/(x - 1) = x^2 + x + 1, remainder 0.
//! Variants: long division, one round per step (divide the leading terms,
//! multiply back, subtract), or synthetic division by x - r in one step.
//! The state reads quotient + remainder/divisor throughout.

use super::factor_theorem::row;
use super::{Cx, Line, Move, Rule};
use crate::expr::{self, terms, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct PolyDivide;

/// The term of a state still to divide: (index in the sum, numerator, divisor, letter).
fn pending(e: &Expr) -> Option<(usize, Poly, Poly, String)> {
    for (i, t) in terms(e).iter().enumerate() {
        // - n/d counts as (-n)/d
        let (t, sign) = match t {
            Expr::Neg(a) => (&**a, Q::int(-1)),
            t => (t, Q::ONE),
        };
        let Expr::Div(n, d) = t else { continue };
        let vars = t.vars();
        let v = vars.iter().next()?;
        let (pn, pd) = (poly::from_expr(n, v)?.scale(&sign)?, poly::from_expr(d, v)?);
        if pd.deg()? >= 1 && pn.deg().is_some_and(|k| k >= pd.deg().unwrap_or(0)) {
            return Some((i, pn, pd, v.clone()));
        }
    }
    None
}

/// Is the division finished: no term left whose numerator's degree reaches the divisor's?
pub fn divided(e: &Expr) -> bool {
    pending(e).is_none()
}

/// c x^k
fn monomial(c: Q, k: usize) -> Poly {
    let mut v = vec![Q::ZERO; k];
    v.push(c);
    Poly(v)
}

/// The state after dividing: quotient terms so far, the new quotient part, remainder/divisor.
fn rebuild(e: &Expr, i: usize, add: &Poly, rem: &Poly, d: &Expr, v: &str) -> Expr {
    let mut ts = terms(e);
    ts.remove(i);
    ts.extend(terms(&add.to_expr(v)).into_iter().filter(|t| !t.is_num(0)));
    if !rem.is_zero() {
        // a negative remainder reads "- 8/(x + 2)"
        let (neg, r) = if rem.lead().is_neg() { (true, rem.scale(&Q::int(-1)).unwrap_or_else(|| rem.clone())) } else { (false, rem.clone()) };
        let f = expr::div(r.to_expr(v), d.clone());
        ts.push(if neg { Expr::Neg(Box::new(f)) } else { f });
    }
    expr::add(ts)
}

impl Rule for PolyDivide {
    fn name(&self) -> &'static str {
        "poly_divide"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["long", "synthetic"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        let Math::Expr(e) = m else { return vec![] };
        let Some((i, n, d, v)) = pending(e) else { return vec![] };
        let dx = match &terms(e)[i] {
            Expr::Div(_, dx) => (**dx).clone(),
            Expr::Neg(a) => match &**a {
                Expr::Div(_, dx) => (**dx).clone(),
                _ => return vec![],
            },
            _ => return vec![],
        };
        let mut out = Vec::new();
        // one round of long division
        let (Some(nd), Some(dd)) = (n.deg(), d.deg()) else { return out };
        let Some(c) = n.lead().div(&d.lead()) else { return out };
        let t = monomial(c, nd - dd);
        if let Some(rem) = t.mul(&d).and_then(|td| n.sub(&td)) {
            let (ln, ld) = (monomial(n.lead(), nd), monomial(d.lead(), dd));
            let tx = t.to_expr(&v);
            let says = Line::new().t("Divide the leading terms: ").e(&ln.to_expr(&v)).t(" / ").e(&ld.to_expr(&v)).t(" = ").e(&tx).t(".");
            let back = t.mul(&d).map(|p| p.to_expr(&v)).unwrap_or_else(|| expr::num(0));
            let work = vec![
                Line::new().t("Multiply back: ").e(&tx).t(" * (").e(&dx).t(") = ").e(&back).t("."),
                Line::new().t("Subtract: (").e(&n.to_expr(&v)).t(") - (").e(&back).t(") = ").e(&rem.to_expr(&v)).t("."),
            ];
            out.push(Move { rule: "poly_divide", variant: "long", result: Math::Expr(rebuild(e, i, &t, &rem, &dx, &v)), says, work });
        }
        // all at once, when the divisor is x - r
        if dd == 1 && d.lead().is_one() {
            let r = d.coef(0).neg();
            if let Some((q, rem)) = n.divmod(&d) {
                let says = Line::new().t("Divide by ").e(&dx).t(format!(" with synthetic division, using {r}."));
                let rem_c = rem.coef(0);
                let work = vec![Line::new().t(format!("Coefficients {}  ->  {}, remainder {rem_c}.", row(&n), row(&q)))];
                out.push(Move { rule: "poly_divide", variant: "synthetic", result: Math::Expr(rebuild(e, i, &q, &rem, &dx, &v)), says, work });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn divides_polynomials() {
        assert_eq!(test_moves(&PolyDivide, "divide x^3 - 1 by x - 1"), vec!["x^2 + (x^2 - 1)/(x - 1)", "x^2 + x + 1"]);
        assert_eq!(test_moves(&PolyDivide, "divide x^2 + 1 by x^2 - 1"), vec!["1 + 2/(x^2 - 1)"]);
        assert!(test_moves(&PolyDivide, "divide x by x^2 + 1").is_empty());
    }
}
