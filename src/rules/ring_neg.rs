//! Ring axiom: negatives. x + (-x) = 0 and (-x) + x = 0.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x + (-x) = 0 and (-x) + x = 0";

pub struct RingNeg;

impl Rule for RingNeg {
    fn name(&self) -> &'static str {
        "ring_neg"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["cancel", "insert"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_neg", m, cx, "cancel", Some("insert"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingNeg, "in a ring, prove a + (-a) = 0");
        assert!(got.contains(&"0 = 0".to_string()), "{got:?}");
        assert!(test_moves(&RingNeg, "in a group, prove aa^-1 = e").is_empty());
    }
}
