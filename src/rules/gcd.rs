//! Greatest common divisor. Variants: Euclid's algorithm (divide, keep the
//! remainder, repeat), or compare prime factorisations. More than two
//! numbers go two at a time: gcd(a, b, c) = gcd(gcd(a, b), c).

use super::prime_factors::factorise;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{ints, Named};
use crate::expr::{self, Expr, Math};
use crate::q::gcd;

pub struct Gcd;

/// Euclid's working: 48 = 2 * 18 + 12, 18 = 1 * 12 + 6, 12 = 2 * 6 + 0.
pub fn euclid_lines(a: i128, b: i128) -> Vec<Line> {
    let (mut x, mut y) = (a.abs().max(b.abs()), a.abs().min(b.abs()));
    let mut out = Vec::new();
    while y != 0 {
        out.push(Line::new().t(format!("{x} = {} * {y} + {}", x / y, x % y)));
        (x, y) = (y, x % y);
    }
    out
}

/// "360 = 2^3 * 3^2 * 5" as a line.
pub fn factor_line(n: i128) -> Line {
    let fs = factorise(n);
    let text: Vec<String> = fs.iter().map(|(p, k)| if *k == 1 { p.to_string() } else { format!("{p}^{k}") }).collect();
    Line::new().t(format!("{n} = {}", if text.is_empty() { "1".into() } else { text.join(" * ") }))
}

impl Rule for Gcd {
    fn name(&self) -> &'static str {
        "gcd"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["euclid", "prime_factors"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("gcd", m, |e, _| {
            let Expr::Call(Named::Gcd, args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            if ns.len() < 2 || ns[..2].iter().all(|&n| n == 0) {
                return vec![];
            }
            let (a, b) = (ns[0], ns[1]);
            let g = gcd(a, b);
            let new = if ns.len() == 2 { expr::num(g) } else { Expr::Call(Named::Gcd, std::iter::once(expr::num(g)).chain(args[2..].iter().cloned()).collect()) };
            let pair = Expr::Call(Named::Gcd, vec![expr::num(a), expr::num(b)]);
            let mut out = Vec::new();
            let mut work = euclid_lines(a, b);
            work.push(Line::new().t(format!("The last nonzero remainder is {g}.")));
            out.push(Rewrite { variant: "euclid", new: new.clone(), says: Line::new().t("Euclid's algorithm: ").e(&pair).t(format!(" = {g}.")), work });
            if a != 0 && b != 0 {
                let work = vec![factor_line(a.abs()), factor_line(b.abs()), Line::new().t(format!("Take each common prime to its lower power: {g}."))];
                out.push(Rewrite { variant: "prime_factors", new, says: Line::new().t("Compare the prime factors: ").e(&pair).t(format!(" = {g}.")), work });
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn greatest_common_divisor() {
        assert_eq!(test_moves(&Gcd, "gcd of 48 and 18"), vec!["6", "6"]);
        assert_eq!(test_moves(&Gcd, "gcd of 12, 18 and 8"), vec!["gcd(6, 8)", "gcd(6, 8)"]);
        assert!(test_moves(&Gcd, "what is 48 + 18").is_empty());
    }
}
