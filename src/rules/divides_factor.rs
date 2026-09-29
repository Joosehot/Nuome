//! To show d divides a polynomial, first factor it into linear factors:
//! n^3 - n = (n - 1)n(n + 1). Offered only when every root is rational.

use super::{Cx, Line, Move, Rule};
use crate::calls::Named;
use crate::expr::{self, with_coeff, Expr, Math};
use crate::poly;
use crate::q::Q;

pub struct DividesFactor;

/// e as (constant, linear factors qn - p), smallest root first, when every
/// root is rational.
pub fn linear_factors(e: &Expr, v: &str) -> Option<(Q, Vec<Expr>)> {
    let p = poly::from_expr(e, v)?;
    if p.deg()? < 2 {
        return None;
    }
    let mut rest = p.clone();
    let mut factors = Vec::new();
    let mut scale = Q::ONE;
    for r in p.rational_roots() {
        while let Some(d) = rest.deflate(&r) {
            rest = d;
            let (num, den) = (r.num(), r.den());
            let x = with_coeff(Q::int(den), expr::var(v));
            factors.push(if num == 0 { x } else { expr::add(vec![x, expr::num(-num)]) });
            scale = scale.mul(&Q::int(den))?;
        }
    }
    if rest.deg()? != 0 {
        return None;
    }
    // smallest factor first: (n - 1)n(n + 1)
    factors.reverse();
    Some((rest.coef(0).div(&scale)?, factors))
}

impl Rule for DividesFactor {
    fn name(&self) -> &'static str {
        "divides_factor"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["linear_factors"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        let Math::Expr(Expr::Call(Named::Divides, a)) = m else { return vec![] };
        let [d, e] = &a[..] else { return vec![] };
        if matches!(e, Expr::Mul(_)) || e.vars().len() != 1 {
            return vec![];
        }
        let v = e.vars().into_iter().next().expect("one letter");
        let Some((c, fs)) = linear_factors(e, &v) else { return vec![] };
        let mut all = if c.is_one() { vec![] } else { vec![Expr::Num(c)] };
        all.extend(fs);
        let product = Expr::Mul(all);
        let says = Line::new().t("Factor: ").e(e).t(" = ").e(&product).t(".");
        vec![Move { rule: "divides_factor", variant: "linear_factors", result: Math::Expr(Expr::Call(Named::Divides, vec![d.clone(), product])), says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factors_into_linear_factors() {
        assert_eq!(test_moves(&DividesFactor, "prove 6 divides n^3 - n"), vec!["6 divides (n - 1)n(n + 1)"]);
        assert!(test_moves(&DividesFactor, "prove 2 divides n^2 + n + 2").is_empty());
    }
}
