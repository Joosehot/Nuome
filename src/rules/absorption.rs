//! Absorption (logic and sets, agent L): P or (P and Q) is P, and
//! P and (P or Q) is P; X union (X intersect Y) = X.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, B};

pub struct Absorption;

impl Rule for Absorption {
    fn name(&self) -> &'static str {
        "absorption"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["or", "and"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("absorption", m, cx, |_, f, b| {
            let (v, outer_or) = match b {
                B::Or(v) => (v, true),
                B::And(v) => (v, false),
                _ => return vec![],
            };
            // a part that has another part of the whole among its own parts
            let absorbed = |k: usize| -> bool {
                let inner = match logic::view(&v[k]) {
                    Some((g, B::And(w))) if g == f && outer_or => w,
                    Some((g, B::Or(w))) if g == f && !outer_or => w,
                    _ => return false,
                };
                v.iter().enumerate().any(|(j, y)| j != k && inner.iter().any(|z| logic::same(z, y)))
            };
            let Some(k) = (0..v.len()).find(|&k| absorbed(k)) else { return vec![] };
            let rest: Vec<Expr> = v.iter().enumerate().filter(|(n, _)| *n != k).map(|(_, x)| x.clone()).collect();
            let (p, q) = (logic::ph(f, 0), logic::ph(f, 1));
            let (new, lhs) = if outer_or {
                (logic::or(f, rest), logic::or(f, vec![p.clone(), logic::and(f, vec![p.clone(), q])]))
            } else {
                (logic::and(f, rest), logic::and(f, vec![p.clone(), logic::or(f, vec![p.clone(), q])]))
            };
            let says = logic::law_says(f, "Absorption", lhs, p);
            vec![Rewrite { variant: if outer_or { "or" } else { "and" }, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn swallows_the_longer_part() {
        assert_eq!(test_moves(&Absorption, "prove (p or (p and q)) -> p"), vec!["p -> p"]);
        assert!(test_moves(&Absorption, "prove (p or (q and r)) -> p or q").is_empty());
    }
}
