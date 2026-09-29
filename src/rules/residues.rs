//! Divisibility by cases: an integer polynomial's value mod d depends only
//! on n mod d, so checking n = 0, 1, ..., d - 1 covers every integer.

use super::{Cx, Line, Move, Rule};
use crate::calls::Named;
use crate::expr::{Expr, Math};
use crate::poly;
use crate::q::Q;

pub struct Residues;

impl Rule for Residues {
    fn name(&self) -> &'static str {
        "residues"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["every_remainder"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Expr(Expr::Call(Named::Divides, a)) = m else { return vec![] };
        let [d, e] = &a[..] else { return vec![] };
        let Some(d) = d.as_num().filter(|q| q.is_int()).map(|q| q.num()) else { return vec![] };
        if d as u64 > cx.cfg.proof.max_residues || e.vars().len() != 1 {
            return vec![];
        }
        let v = e.vars().into_iter().next().expect("one letter");
        let Some(p) = poly::from_expr(e, &v) else { return vec![] };
        if !p.0.iter().all(|c| c.is_int()) {
            return vec![]; // n(n + 1)/2 isn't an integer polynomial: the argument doesn't apply
        }
        let mut work = Vec::new();
        for r in 0..d {
            let Some(val) = p.eval(&Q::int(r)) else { return vec![] };
            let rem = val.num().rem_euclid(d);
            if rem != 0 {
                return vec![];
            }
            work.push(Line::new().t(format!("{v} = {r}: {val} = {} * {d}", val.num() / d)));
        }
        let says = Line::new().t(format!("Check every remainder of {v} mod {d}: the value mod {d} depends only on {v} mod {d}."));
        vec![Move { rule: "residues", variant: "every_remainder", result: Math::Proved, says, work }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn every_remainder() {
        assert_eq!(test_moves(&Residues, "prove n^2 + n is even"), vec!["proved"]);
        assert!(test_moves(&Residues, "prove n^2 + 1 is even").is_empty());
    }
}
