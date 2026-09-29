//! Prime factorisation of a whole number: divide by the smallest prime
//! that goes in, again and again. 360 = 2^3 * 3^2 * 5.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct PrimeFactors;

/// (prime, power) pairs, smallest prime first, by trial division.
pub fn factorise(n: i128) -> Vec<(i128, u32)> {
    let mut n = n.abs();
    let mut out = Vec::new();
    let mut p = 2i128;
    while p * p <= n {
        let mut k = 0;
        while n % p == 0 {
            n /= p;
            k += 1;
        }
        if k > 0 {
            out.push((p, k));
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if n > 1 {
        out.push((n, 1));
    }
    out
}

impl Rule for PrimeFactors {
    fn name(&self) -> &'static str {
        "prime_factors"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["repeated_division"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("prime_factors", m, |e, at| {
            // only the whole number being factored, not a coefficient inside it
            if !at.path.is_empty() {
                return vec![];
            }
            let Some(q) = e.as_num().filter(|q| q.is_int() && q.num().abs() >= 4) else { return vec![] };
            let n = q.num();
            let fs = factorise(n);
            if fs.len() == 1 && fs[0].1 == 1 {
                return vec![]; // prime: nothing to split
            }
            let mut work = Vec::new();
            let mut rest = n.abs();
            for (p, k) in &fs {
                for _ in 0..*k {
                    if rest == *p {
                        break;
                    }
                    work.push(Line::new().t(format!("{rest} / {p} = {}", rest / p)));
                    rest /= p;
                }
            }
            let mut factors: Vec<Expr> = fs.iter().map(|&(p, k)| if k == 1 { expr::num(p) } else { expr::pow(expr::num(p), expr::num(k as i128)) }).collect();
            if n < 0 {
                factors.insert(0, expr::num(-1));
            }
            let says = Line::new().t("Divide by the smallest prime that goes in, again and again.");
            vec![Rewrite { variant: "repeated_division", new: Expr::Mul(factors), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factorises_whole_numbers() {
        assert_eq!(test_moves(&PrimeFactors, "prime factors of 360"), vec!["2^3 * 3^2 * 5"]);
        assert!(test_moves(&PrimeFactors, "factor 97").is_empty());
    }
}
