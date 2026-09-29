//! Group lemma: the identity is its own inverse, e^-1 = e.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "e^-1 = e";

pub struct GroupInverseIdentity;

impl Rule for GroupInverseIdentity {
    fn name(&self) -> &'static str {
        "group_inverse_identity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_inverse_identity", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupInverseIdentity, "in a group, prove e^-1 a = a");
        assert!(got.contains(&"ea = a".to_string()), "{got:?}");
        assert!(test_moves(&GroupInverseIdentity, "in a group, prove e^-1 = e").is_empty());
    }
}
