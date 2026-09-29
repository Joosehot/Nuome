//! Quadratic trinomials: x^2 - 5x + 6 = (x - 2)(x - 3). Variants: two
//! numbers that multiply to c and add to b (when a = 1), the ac method
//! (a != 1), and perfect squares.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct Trinomial;

fn linear(v: &str, q: i128, p: i128) -> Expr {
    // qx - p
    let qx = with_coeff(Q::int(q), expr::var(v));
    if p == 0 {
        qx
    } else {
        expr::add(vec![qx, expr::num(-p)])
    }
}

/// Factor a quadratic with rational roots into linear factors over the
/// integers: (the factored form, variant, side working).
pub fn by_roots(p: &Poly, v: &str) -> Option<(Expr, &'static str, Vec<Line>)> {
    if p.deg()? != 2 {
        return None;
    }
    let roots = p.rational_roots();
    let (r1, r2) = match roots.as_slice() {
        [r] => (*r, *r),
        [a, b] => (*a, *b),
        _ => return None,
    };
    // (q1 x - p1)(q2 x - p2) * k
    let (p1, q1, p2, q2) = (r1.num(), r1.den(), r2.num(), r2.den());
    let k = p.lead().div(&Q::int(q1.checked_mul(q2)?))?;
    let (a, b, c) = (p.coef(2), p.coef(1), p.coef(0));
    let mut work = Vec::new();
    let (core, variant) = if c.is_zero() && r1 != r2 {
        work.push(Line::new().t(format!("Every term contains {v}.")));
        (Expr::Mul(vec![linear(v, q1, p1), linear(v, q2, p2)]), "common_factor")
    } else if r1 == r2 {
        (expr::pow(linear(v, q1, p1), expr::num(2)), "perfect_square")
    } else if q1 == 1 && q2 == 1 && k.is_one() {
        let (m, n) = (-p1, -p2);
        work.push(Line::new().t(format!("{m} and {n} multiply to {c} and add to {b}.")));
        (Expr::Mul(vec![linear(v, 1, p1), linear(v, 1, p2)]), "sum_product")
    } else {
        let (m, n) = (-q1 * p2, -q2 * p1);
        let ac = a.mul(&c)?;
        work.push(Line::new().t(format!("a * c = {ac}; {m} and {n} multiply to {ac} and add to {b}.")));
        let x = expr::var(v);
        let split = Expr::Add(vec![with_coeff(a, expr::pow(x.clone(), expr::num(2))), with_coeff(Q::int(m), x.clone()), with_coeff(Q::int(n), x), Expr::Num(c)]);
        work.push(Line::new().t("Split the middle term: ").e(&split).t("."));
        (Expr::Mul(vec![linear(v, q1, p1), linear(v, q2, p2)]), "ac_method")
    };
    let out = if k.is_one() {
        core
    } else if k == Q::int(-1) {
        Expr::Neg(Box::new(core))
    } else {
        let mut f = vec![Expr::Num(k)];
        match core {
            Expr::Mul(v) => f.extend(v),
            c => f.push(c),
        }
        Expr::Mul(f)
    };
    Some((out, variant, work))
}

impl Rule for Trinomial {
    fn name(&self) -> &'static str {
        "trinomial"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sum_product", "ac_method", "perfect_square", "common_factor"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("trinomial", m, |e, _| {
            let Expr::Add(ts) = e else { return vec![] };
            let vars = e.vars();
            if ts.len() != 3 || vars.len() != 1 {
                return vec![];
            }
            let v = vars.iter().next().expect("one letter");
            let Some(p) = poly::from_expr(e, v) else { return vec![] };
            // common factors come out first (factor_common)
            if p.primitive().is_none_or(|ints| ints.iter().map(|&c| Q::int(c)).collect::<Vec<_>>() != p.0) {
                return vec![];
            }
            let Some((new, variant, work)) = by_roots(&p, v) else { return vec![] };
            let says = Line::new().t(match variant {
                "perfect_square" => "It is a perfect square.",
                "sum_product" => "Find two numbers that multiply to the constant and add to the middle coefficient.",
                _ => "Factor by the ac method.",
            });
            vec![Rewrite { variant, new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factors_trinomials() {
        assert_eq!(test_moves(&Trinomial, "factor x^2 - 5x + 6"), vec!["(x - 2)(x - 3)"]);
        assert_eq!(test_moves(&Trinomial, "factor 2x^2 + 7x + 3"), vec!["(x + 3)(2x + 1)"]);
        assert_eq!(test_moves(&Trinomial, "factor x^2 + 6x + 9"), vec!["(x + 3)^2"]);
        assert!(test_moves(&Trinomial, "factor x^2 + x + 1").is_empty());
    }
}
