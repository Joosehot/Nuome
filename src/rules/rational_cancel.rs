//! Cancel a common factor of a fraction of polynomials: factor the top and
//! the bottom, then divide both by what they share:
//! (x^2 - 1)/(x - 1) = (x - 1)(x + 1)/(x - 1) = x + 1. The value that made
//! the cancelled factor 0 stays excluded (the answer says "for x != 1").

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct RationalCancel;

/// Same roots, integer coefficients, positive leading coefficient.
fn primitive(p: &Poly) -> Option<Poly> {
    let ints = p.primitive()?;
    let s = if ints.last().is_some_and(|c| *c < 0) { -1 } else { 1 };
    Some(Poly(ints.iter().map(|&a| Q::int(s * a)).collect()))
}

/// (numerator, denominator) with the common factor `g` divided out, factored.
pub fn cancel(n: &Poly, d: &Poly, g: &Poly, v: &str) -> Option<Expr> {
    let (nq, r1) = n.divmod(g)?;
    let (dq, r2) = d.divmod(g)?;
    if !r1.is_zero() || !r2.is_zero() || nq.is_zero() {
        return None;
    }
    let (fn_, fd) = (nq.factor()?, dq.factor()?);
    let k = fn_.c.div(&fd.c)?;
    let strip = |f: &poly::Factored| {
        let mut f = f.clone();
        f.c = Q::ONE;
        let e = f.to_expr(v);
        if e.is_num(1) {
            vec![]
        } else {
            vec![e]
        }
    };
    let top = with_coeff(Q::int(k.num()), expr::mul(strip(&fn_)));
    let bottom = with_coeff(Q::int(k.den()), expr::mul(strip(&fd)));
    Some(if bottom.is_num(1) { top } else { expr::div(top, bottom) })
}

impl Rule for RationalCancel {
    fn name(&self) -> &'static str {
        "rational_cancel"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["common_factor"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("rational_cancel", m, |e, _| {
            let Expr::Div(a, b) = e else { return vec![] };
            let vars = e.vars();
            if vars.len() != 1 || !b.has_var(vars.iter().next().expect("one letter")) {
                return vec![];
            }
            let v = vars.iter().next().expect("one letter");
            let (Some(n), Some(d)) = (poly::from_expr(a, v), poly::from_expr(b, v)) else { return vec![] };
            // x^5/x^2 is the power law's job
            let single = |p: &Poly| p.0.iter().filter(|c| !c.is_zero()).count() == 1;
            if single(&n) && single(&d) {
                return vec![];
            }
            let Some(g) = n.gcd(&d).and_then(|g| primitive(&g)) else { return vec![] };
            if g.deg().is_none_or(|k| k == 0) {
                return vec![];
            }
            let Some(new) = cancel(&n, &d, &g, v) else { return vec![] };
            let gx = g.factor().map_or_else(|| g.to_expr(v), |f| f.to_expr(v));
            let mut work = Vec::new();
            for (side, p) in [(a, &n), (b, &d)] {
                if let Some(f) = p.factor() {
                    let fx = f.to_expr(v);
                    if fx != **side {
                        work.push(Line::new().e(side).t(" = ").e(&fx));
                    }
                }
            }
            let zeros: Vec<String> = g.rational_roots().iter().map(|r| r.to_string()).collect();
            let keep = if zeros.is_empty() {
                Line::new()
            } else {
                let stay = if zeros.len() == 1 { "stays" } else { "stay" };
                Line::new().t(format!("; {v} = {} {stay} excluded", zeros.join(format!(" and {v} = ").as_str())))
            };
            let says = Line::new().t("Cancel the common factor ").e(&gx).join(keep).t(".");
            vec![Rewrite { variant: "common_factor", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn cancels_common_factors() {
        assert_eq!(test_moves(&RationalCancel, "simplify (x^2 - 1)/(x - 1)"), vec!["x + 1"]);
        assert_eq!(test_moves(&RationalCancel, "simplify (x^2 + 5x + 6)/(x^2 - 4)"), vec!["(x + 3)/(x - 2)"]);
        assert_eq!(test_moves(&RationalCancel, "simplify (2x^2 - 8)/(4x + 8)"), vec!["(x - 2)/2"]);
        assert!(test_moves(&RationalCancel, "simplify (x^2 + 1)/(x - 1)").is_empty());
    }
}
