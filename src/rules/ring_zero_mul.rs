//! Ring lemma: zero times anything is zero, 0x = 0.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "0x = 0";

pub struct RingZeroMul;

impl Rule for RingZeroMul {
    fn name(&self) -> &'static str {
        "ring_zero_mul"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_zero_mul", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingZeroMul, "in a ring, prove b(0a) = b0");
        assert!(got.contains(&"b0 = b0".to_string()), "{got:?}");
        assert!(test_moves(&RingZeroMul, "in a ring, prove 0a = 0").is_empty());
    }
}
