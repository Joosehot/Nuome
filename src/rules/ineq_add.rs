//! Adding or subtracting the same thing on both sides keeps an inequality
//! true: 2x + 3 < 7 -> 2x < 7 - 3. Variants as for equations: move a
//! number, move a letter term, or (degree 2 and up, or letters in a
//! denominator) move everything to one side to compare with 0.

use super::move_term::{self, MoveTerm};
use super::{Cx, Line, Move, Rule};
use crate::expr::{self, terms, Math};

pub struct IneqAdd;

impl Rule for IneqAdd {
    fn name(&self) -> &'static str {
        "ineq_add"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["constant", "variable", "standard_form"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Ineq(l, rel, r) = m else { return vec![] };
        let v = cx.var;
        // a letter under the line: we can't multiply by it (its sign is unknown), so compare with 0
        if move_term::var_in_denominator(l, v) || move_term::var_in_denominator(r, v) {
            if r.is_num(0) {
                return vec![];
            }
            let mut left = terms(l);
            left.extend(terms(r).iter().map(|t| expr::neg(t.clone())));
            let says = if terms(r).len() == 1 { move_term::says(r) } else { Line::new().t("Move every term to the left side.") };
            let result = Math::Ineq(expr::add(left), *rel, expr::num(0));
            return vec![Move { rule: "ineq_add", variant: "standard_form", result, says, work: vec![] }];
        }
        // the same moves as for the equation l = r, with the sign kept
        MoveTerm
            .moves(&Math::Eq(l.clone(), r.clone()), cx)
            .into_iter()
            .filter_map(|mv| match mv.result {
                Math::Eq(a, b) => {
                    let variant = self.variants().iter().copied().find(|x| *x == mv.variant)?;
                    Some(Move { rule: "ineq_add", variant, result: Math::Ineq(a, *rel, b), says: mv.says, work: mv.work })
                }
                _ => None,
            })
            .collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn moves_terms_keeping_the_sign() {
        assert_eq!(test_moves(&IneqAdd, "solve 2x + 3 < 7"), vec!["2x < 7 - 3"]);
        assert_eq!(test_moves(&IneqAdd, "solve x^2 > 4"), vec!["x^2 - 4 > 0"]);
        assert_eq!(test_moves(&IneqAdd, "solve (x - 1)/(x + 2) >= 1"), vec!["(x - 1)/(x + 2) - 1 >= 0"]);
        assert!(test_moves(&IneqAdd, "solve 2x < 8").is_empty());
    }
}
