//! Group lemma: the inverse of an inverse is the element, (x^-1)^-1 = x.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(x^-1)^-1 = x";

pub struct GroupInverseInverse;

impl Rule for GroupInverseInverse {
    fn name(&self) -> &'static str {
        "group_inverse_inverse"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_inverse_inverse", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupInverseInverse, "in a group, prove ((a^-1)^-1)b = ab");
        assert!(got.contains(&"ab = ab".to_string()), "{got:?}");
        assert!(test_moves(&GroupInverseInverse, "in a group, prove (a^-1)^-1 = a").is_empty());
    }
}
