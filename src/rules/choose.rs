//! Counting selections. Ordered (permutations): n(n-1)...(n-k+1).
//! Unordered (combinations): the same product divided by k!, because each
//! group of k was counted k! times, once per order.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{ints, Named};
use crate::expr::{self, Expr, Math};

pub struct Choose;

impl Rule for Choose {
    fn name(&self) -> &'static str {
        "choose"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["falling_product", "symmetry"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("choose", m, |e, _| {
            let Expr::Call(f @ (Named::Choose | Named::Perm), args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            let [n, k] = ns[..] else { return vec![] };
            if n < 0 || k < 0 || k > n || n > 60 {
                return vec![];
            }
            let mut out = Vec::new();
            // C(10, 8) = C(10, 2): choosing what to leave out
            if *f == Named::Choose && 2 * k > n {
                let new = Expr::Call(Named::Choose, vec![expr::num(n), expr::num(n - k)]);
                out.push(Rewrite { variant: "symmetry", new, says: Line::new().t(format!("Choosing {k} of {n} is the same as choosing the {} to leave out.", n - k)), work: vec![] });
                return out;
            }
            let prod = if k == 0 { expr::num(1) } else { Expr::Mul((0..k).map(|i| expr::num(n - i)).collect()) };
            let (new, says) = if *f == Named::Perm {
                (prod, Line::new().t(format!("{n} choices for the first, {} for the second, ...: {k} factors.", n - 1)))
            } else {
                (expr::div(prod, Expr::Call(Named::Factorial, vec![expr::num(k)])), Line::new().t(format!("Count ordered picks, then divide by {k}!, the orders of each group.")))
            };
            out.push(Rewrite { variant: "falling_product", new, says, work: vec![] });
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn counts_selections() {
        assert_eq!(test_moves(&Choose, "what is 10 choose 3"), vec!["(10 * 9 * 8)/3!"]);
        assert_eq!(test_moves(&Choose, "permutations of 3 from 10"), vec!["10 * 9 * 8"]);
        assert_eq!(test_moves(&Choose, "what is 10 choose 8"), vec!["C(10, 2)"]);
        assert!(test_moves(&Choose, "what is 10 * 3").is_empty());
    }
}
