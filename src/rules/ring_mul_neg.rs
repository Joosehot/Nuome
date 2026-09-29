//! Ring lemma: times a negative, x(-y) = -(xy).

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x(-y) = -(xy)";

pub struct RingMulNeg;

impl Rule for RingMulNeg {
    fn name(&self) -> &'static str {
        "ring_mul_neg"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_mul_neg", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingMulNeg, "in a ring, prove a(-b) + c = -(ab) + c");
        assert!(got.contains(&"-(ab) + c = -(ab) + c".to_string()), "{got:?}");
        assert!(test_moves(&RingMulNeg, "in a ring, prove a(-b) = -(ab)").is_empty());
    }
}
