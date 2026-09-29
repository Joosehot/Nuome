//! Ring axiom: addition is associative, (x + y) + z = x + (y + z).

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(x + y) + z = x + (y + z)";

pub struct RingAddAssoc;

impl Rule for RingAddAssoc {
    fn name(&self) -> &'static str {
        "ring_add_assoc"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["right", "left"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_add_assoc", m, cx, "right", Some("left"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingAddAssoc, "in a ring, prove (a + b) + c = a + (b + c)");
        assert!(got.contains(&"a + (b + c) = a + (b + c)".to_string()), "{got:?}");
        assert!(test_moves(&RingAddAssoc, "in a group, prove (ab)c = a(bc)").is_empty());
    }
}
