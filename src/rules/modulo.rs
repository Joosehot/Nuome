//! a mod b: divide, the remainder is the answer. 17 = 3 * 5 + 2, so 17 mod 5 = 2.
//! The remainder is never negative: -7 = -2 * 5 + 3.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{ints, Named};
use crate::expr::{self, Expr, Math};

pub struct Modulo;

impl Rule for Modulo {
    fn name(&self) -> &'static str {
        "modulo"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["divide"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("modulo", m, |e, _| {
            let Expr::Call(Named::Mod, args) = e else { return vec![] };
            let Some(ns) = ints(args) else { return vec![] };
            let [a, b] = ns[..] else { return vec![] };
            if b == 0 {
                return vec![];
            }
            let (q, r) = (a.div_euclid(b), a.rem_euclid(b));
            let says = Line::new().t(format!("Divide {a} by {b}: the remainder is {r}."));
            vec![Rewrite { variant: "divide", new: expr::num(r), says, work: vec![Line::new().t(format!("{a} = {q} * {b} + {r}"))] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn remainder() {
        assert_eq!(test_moves(&Modulo, "what is 17 mod 5"), vec!["2"]);
        assert_eq!(test_moves(&Modulo, "the remainder when 17 is divided by 5"), vec!["2"]);
        assert!(test_moves(&Modulo, "what is 17 / 5").is_empty());
    }
}
