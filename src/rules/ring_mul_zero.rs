//! Ring lemma: anything times zero is zero, x0 = 0.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x0 = 0";

pub struct RingMulZero;

impl Rule for RingMulZero {
    fn name(&self) -> &'static str {
        "ring_mul_zero"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_mul_zero", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingMulZero, "in a ring, prove (a0)b = 0b");
        assert!(got.contains(&"0b = 0b".to_string()), "{got:?}");
        assert!(test_moves(&RingMulZero, "in a ring, prove a0 = 0").is_empty());
    }
}
