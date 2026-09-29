//! A negative power is one over the positive power: x^(-2) = 1/x^2,
//! 3x^(-2) = 3/x^2, 2^(-3) = 1/2^3.

use super::powers::{base_exp, power_of};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct NegExponent;

fn negative_power(f: &Expr) -> Option<(Expr, crate::q::Q)> {
    match f {
        Expr::Pow(b, n) => n.as_num().filter(|q| q.is_neg()).map(|q| ((**b).clone(), q.neg())),
        _ => None,
    }
}

impl Rule for NegExponent {
    fn name(&self) -> &'static str {
        "neg_exponent"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["reciprocal"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("neg_exponent", m, |e, at| {
            let says = |from: &Expr, to: &Expr| Line::new().t("A negative power is one over the positive power: ").e(from).t(" = ").e(to).t(".");
            match e {
                // in a product: every negative power moves under the line, once the same bases are combined
                Expr::Mul(fs) if fs.iter().any(|f| negative_power(f).is_some()) => {
                    let bases: Vec<Expr> = fs.iter().filter_map(|f| base_exp(f).map(|(b, _)| b)).collect();
                    if bases.iter().enumerate().any(|(i, b)| bases[..i].contains(b)) {
                        return vec![];
                    }
                    let (mut top, mut bottom) = (Vec::new(), Vec::new());
                    for f in fs {
                        match negative_power(f) {
                            Some((b, k)) => bottom.push(power_of(b, k)),
                            None => top.push(f.clone()),
                        }
                    }
                    let new = expr::div(expr::mul(top), expr::mul(bottom));
                    vec![Rewrite { variant: "reciprocal", new: new.clone(), says: says(e, &new), work: vec![] }]
                }
                Expr::Pow(..) => {
                    let Some((b, k)) = negative_power(e) else { return vec![] };
                    // a factor of a product is handled with the product
                    if let Some((_, parent)) = at.path.split_last() {
                        if matches!(at.math.slots()[at.slot].get(parent), Expr::Mul(_)) {
                            return vec![];
                        }
                    }
                    if b.is_num(0) {
                        return vec![];
                    }
                    let new = expr::div(expr::num(1), power_of(b, k));
                    vec![Rewrite { variant: "reciprocal", new: new.clone(), says: says(e, &new), work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn turns_negative_powers_into_fractions() {
        assert_eq!(test_moves(&NegExponent, "simplify x^-2"), vec!["1/x^2"]);
        assert_eq!(test_moves(&NegExponent, "simplify 3x^-2 y"), vec!["3y/x^2"]);
        assert!(test_moves(&NegExponent, "simplify x^-2 * x^5").is_empty());
        assert!(test_moves(&NegExponent, "simplify x^2").is_empty());
    }
}
