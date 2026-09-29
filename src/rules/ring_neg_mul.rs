//! Ring lemma: a negative times, (-x)y = -(xy).

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(-x)y = -(xy)";

pub struct RingNegMul;

impl Rule for RingNegMul {
    fn name(&self) -> &'static str {
        "ring_neg_mul"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_neg_mul", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingNegMul, "in a ring, prove (-a)b + c = -(ab) + c");
        assert!(got.contains(&"-(ab) + c = -(ab) + c".to_string()), "{got:?}");
        assert!(test_moves(&RingNegMul, "in a ring, prove (-a)b = -(ab)").is_empty());
    }
}
