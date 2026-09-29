//! Ring axiom: addition commutes, x + y = y + x.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x + y = y + x";

pub struct RingAddComm;

impl Rule for RingAddComm {
    fn name(&self) -> &'static str {
        "ring_add_comm"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["swap"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_add_comm", m, cx, "swap", Some("swap"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingAddComm, "in a ring, prove a + b = b + a");
        assert!(got.contains(&"b + a = b + a".to_string()), "{got:?}");
        assert!(test_moves(&RingAddComm, "in a group, prove ab = ab").is_empty());
    }
}
