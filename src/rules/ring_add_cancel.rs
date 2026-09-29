//! Ring lemma: cancellation for addition. If x + y = x + z then y = z.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "if x + y = x + z then y = z";

pub struct RingAddCancel;

impl Rule for RingAddCancel {
    fn name(&self) -> &'static str {
        "ring_add_cancel"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rewrite", "reduce"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_add_cancel", m, cx, "rewrite", Some("reduce"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingAddCancel, "in a ring, prove that if a + b = a + c then b + d = c + d");
        assert!(got.contains(&"c + d = c + d".to_string()), "{got:?}");
        assert!(test_moves(&RingAddCancel, "in a ring, prove that if a + b = a + c then b = c").is_empty());
    }
}
