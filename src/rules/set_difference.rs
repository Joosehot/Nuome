//! Set difference (sets, agent L): X \ Y = X intersect Y', the elements of X
//! that are not in Y.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math, SetOp};
use crate::logic::{self, Flavor};

pub struct SetDifference;

impl Rule for SetDifference {
    fn name(&self) -> &'static str {
        "set_difference"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["complement"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Sets;
        let (x, y) = (logic::ph(f, 0), logic::ph(f, 1));
        let says = logic::law_says(f, "Set difference", Expr::Set(SetOp::Diff, vec![x.clone(), y.clone()]), logic::and(f, vec![x, logic::not(f, y)]));
        logic::law_on("set_difference", m, cx, |e| match e {
            Expr::Set(SetOp::Diff, v) => vec![Rewrite { variant: "complement", new: logic::and(f, vec![v[0].clone(), logic::not(f, v[1].clone())]), says: says.clone(), work: vec![] }],
            _ => vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn difference_is_intersection_with_complement() {
        assert_eq!(test_moves(&SetDifference, "prove (A \\ B) ∪ B = A ∪ B"), vec!["(A intersect B') union B = A union B"]);
        // the statement is the definition itself
        assert!(test_moves(&SetDifference, "prove A \\ B = A ∩ B'").is_empty());
    }
}
