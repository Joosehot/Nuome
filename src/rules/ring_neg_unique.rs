//! Ring lemma: negatives are unique. If x + y = 0 then y = -x.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "if x + y = 0 then y = -x";

pub struct RingNegUnique;

impl Rule for RingNegUnique {
    fn name(&self) -> &'static str {
        "ring_neg_unique"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rewrite", "reduce"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_neg_unique", m, cx, "rewrite", Some("reduce"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingNegUnique, "in a ring, prove -(-a) = a");
        assert!(got.contains(&"a = a".to_string()), "{got:?}");
        assert!(test_moves(&RingNegUnique, "in a ring, prove that if a + b = 0 then b = -a").is_empty());
    }
}
