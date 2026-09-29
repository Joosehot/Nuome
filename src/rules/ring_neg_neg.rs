//! Ring lemma: the negative of a negative, -(-x) = x.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "-(-x) = x";

pub struct RingNegNeg;

impl Rule for RingNegNeg {
    fn name(&self) -> &'static str {
        "ring_neg_neg"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_neg_neg", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingNegNeg, "in a ring, prove -(-a) + b = a + b");
        assert!(got.contains(&"a + b = a + b".to_string()), "{got:?}");
        assert!(test_moves(&RingNegNeg, "in a ring, prove -(-a) = a").is_empty());
    }
}
