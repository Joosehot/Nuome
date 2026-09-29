//! Group axiom: the identity. ex = x and xe = x: e can be dropped from a
//! product, or written in (a = ae) when a proof needs it.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "ex = x and xe = x";

pub struct GroupIdentity;

impl Rule for GroupIdentity {
    fn name(&self) -> &'static str {
        "group_identity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["remove", "insert"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_identity", m, cx, "remove", Some("insert"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupIdentity, "in a group, prove ea = a");
        assert!(got.contains(&"a = a".to_string()), "{got:?}");
        assert!(test_moves(&GroupIdentity, "in a ring, prove a + 0 = a").is_empty());
    }
}
