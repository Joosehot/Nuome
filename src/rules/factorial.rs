//! n! = n * (n - 1) * ... * 1, and 0! = 1. Variants: write the product out
//! (small n), or state the value (large n, where the product is a wall of
//! numbers).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{self, ints, Named};
use crate::expr::{self, Expr, Math};

pub struct Factorial;

impl Rule for Factorial {
    fn name(&self) -> &'static str {
        "factorial"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["write_out", "direct"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let longest = cx.cfg.numbers.write_out_factorial;
        local("factorial", m, |e, _| {
            let Expr::Call(Named::Factorial, args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            let [n] = ns[..] else { return vec![] };
            if n < 0 {
                return vec![];
            }
            if n <= 1 {
                return vec![Rewrite { variant: "write_out", new: expr::num(1), says: Line::new().t(format!("{n}! = 1 (the empty product).")), work: vec![] }];
            }
            if n as u64 <= longest {
                let new = Expr::Mul((1..=n).rev().map(expr::num).collect());
                return vec![Rewrite { variant: "write_out", new, says: Line::new().t(format!("{n}! is the product of 1 to {n}.")), work: vec![] }];
            }
            let Some(v) = calls::eval_q(Named::Factorial, &[crate::q::Q::int(n)]) else { return vec![] };
            vec![Rewrite { variant: "direct", new: Expr::Num(v), says: Line::new().t(format!("{n}! is the product of 1 to {n}: {v}.")), work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn factorials() {
        assert_eq!(test_moves(&Factorial, "what is 5!"), vec!["5 * 4 * 3 * 2 * 1"]);
        assert_eq!(test_moves(&Factorial, "what is 0!"), vec!["1"]);
        assert!(test_moves(&Factorial, "what is 5 * 4").is_empty());
    }
}
