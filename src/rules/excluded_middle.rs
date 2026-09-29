//! Excluded middle and contradiction (logic and sets, agent L): P or ~P is
//! T and P and ~P is F; X union X' = U and X intersect X' is empty.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, Flavor, B};

pub struct ExcludedMiddle;

impl Rule for ExcludedMiddle {
    fn name(&self) -> &'static str {
        "excluded_middle"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["middle", "contradiction"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("excluded_middle", m, cx, |_, f, b| {
            let (v, is_or) = match b {
                B::Or(v) => (v, true),
                B::And(v) => (v, false),
                _ => return vec![],
            };
            // a part and its negation
            let negation_of = |x: &Expr, y: &Expr| matches!(logic::view(y), Some((g, B::Not(z))) if g == f && logic::same(z, x));
            let pair = (0..v.len()).flat_map(|i| (0..v.len()).map(move |j| (i, j))).find(|&(i, j)| i != j && negation_of(&v[i], &v[j]));
            let Some((i, j)) = pair else { return vec![] };
            let mut out: Vec<Expr> = Vec::new();
            for (k, x) in v.iter().enumerate() {
                if k == i.min(j) {
                    out.push(logic::konst(f, is_or));
                } else if k != i.max(j) {
                    out.push(x.clone());
                }
            }
            let p = logic::ph(f, 0);
            let name = |logic_name: &'static str| if f == Flavor::Logic { logic_name } else { "Complement law" };
            let (new, says, variant) = if is_or {
                (logic::or(f, out), logic::law_says(f, name("Excluded middle"), logic::or(f, vec![p.clone(), logic::not(f, p)]), logic::konst(f, true)), "middle")
            } else {
                (logic::and(f, out), logic::law_says(f, name("Contradiction"), logic::and(f, vec![p.clone(), logic::not(f, p)]), logic::konst(f, false)), "contradiction")
            };
            vec![Rewrite { variant, new, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn a_statement_and_its_negation() {
        assert_eq!(test_moves(&ExcludedMiddle, "prove (q or p or ~q) -> r or ~r"), vec!["(T or p) -> T"]);
        // the statement is the law itself: the truth table proves it
        assert!(test_moves(&ExcludedMiddle, "prove p or ~p is a tautology").is_empty());
        assert!(test_moves(&ExcludedMiddle, "prove (p or q) -> (q or p)").is_empty());
    }
}
