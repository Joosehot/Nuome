//! Least common multiple. Variants: through the gcd, lcm(a, b) = ab/gcd(a, b),
//! or from the prime factorisations (each prime to its higher power).

use super::gcd::factor_line;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{ints, Named};
use crate::expr::{self, Expr, Math};
use crate::q::gcd;

pub struct Lcm;

impl Rule for Lcm {
    fn name(&self) -> &'static str {
        "lcm"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["via_gcd", "prime_factors"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("lcm", m, |e, _| {
            let Expr::Call(Named::Lcm, args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            if ns.len() < 2 || ns[0] == 0 || ns[1] == 0 {
                return vec![];
            }
            let (a, b) = (ns[0].abs(), ns[1].abs());
            let g = gcd(a, b);
            let Some(l) = (a / g).checked_mul(b) else { return vec![] };
            let rest = |x: Expr| if ns.len() == 2 { x } else { Expr::Call(Named::Lcm, std::iter::once(x).chain(args[2..].iter().cloned()).collect()) };
            let pair = Expr::Call(Named::Lcm, vec![expr::num(a), expr::num(b)]);
            let product = expr::div(Expr::Mul(vec![expr::num(a), expr::num(b)]), expr::num(g));
            vec![
                Rewrite {
                    variant: "via_gcd",
                    new: rest(product),
                    says: Line::new().t("Use ").e(&pair).t(format!(" = {a} * {b} / gcd({a}, {b})")).t("."),
                    work: vec![Line::new().t(format!("gcd({a}, {b}) = {g}"))],
                },
                Rewrite {
                    variant: "prime_factors",
                    new: rest(expr::num(l)),
                    says: Line::new().t("Compare the prime factors: ").e(&pair).t(format!(" = {l}.")),
                    work: vec![factor_line(a), factor_line(b), Line::new().t(format!("Take each prime to its higher power: {l}."))],
                },
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn least_common_multiple() {
        assert_eq!(test_moves(&Lcm, "lcm of 4 and 6"), vec!["(4 * 6)/2", "12"]);
        assert!(test_moves(&Lcm, "gcd of 4 and 6").is_empty());
    }
}
