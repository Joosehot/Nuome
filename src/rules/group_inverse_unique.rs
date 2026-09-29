//! Group lemma: inverses are unique. If xy = e then y = x^-1. Used to
//! rewrite when xy = e is a hypothesis or an axiom, or to reduce a goal
//! y = x^-1 to showing xy = e.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "if xy = e then y = x^-1";

pub struct GroupInverseUnique;

impl Rule for GroupInverseUnique {
    fn name(&self) -> &'static str {
        "group_inverse_unique"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rewrite", "reduce"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_inverse_unique", m, cx, "rewrite", Some("reduce"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupInverseUnique, "in a group, prove (a^-1)^-1 = a");
        assert!(got.contains(&"a = a".to_string()), "{got:?}");
        assert!(test_moves(&GroupInverseUnique, "in a group, prove that if ab = e then b = a^-1").is_empty());
    }
}
