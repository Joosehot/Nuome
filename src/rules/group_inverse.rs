//! Group axiom: inverses. xx^-1 = e and x^-1 x = e: an element next to its
//! inverse cancels, or e is written as one (e = a^-1 a) when a proof needs it.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "xx^-1 = e and x^-1 x = e";

pub struct GroupInverse;

impl Rule for GroupInverse {
    fn name(&self) -> &'static str {
        "group_inverse"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["cancel", "insert"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_inverse", m, cx, "cancel", Some("insert"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupInverse, "in a group, prove aa^-1 = e");
        assert!(got.contains(&"e = e".to_string()), "{got:?}");
        assert!(test_moves(&GroupInverse, "in a ring, prove a + (-a) = 0").is_empty());
    }
}
