//! Ring axiom: zero. x + 0 = x and 0 + x = x.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x + 0 = x and 0 + x = x";

pub struct RingZero;

impl Rule for RingZero {
    fn name(&self) -> &'static str {
        "ring_zero"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["remove", "insert"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_zero", m, cx, "remove", Some("insert"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingZero, "in a ring, prove a + 0 = a");
        assert!(got.contains(&"a = a".to_string()), "{got:?}");
        assert!(test_moves(&RingZero, "in a group, prove ea = a").is_empty());
    }
}
