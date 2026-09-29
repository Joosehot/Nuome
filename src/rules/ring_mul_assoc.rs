//! Ring axiom: multiplication is associative, (xy)z = x(yz). It need not
//! commute.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(xy)z = x(yz)";

pub struct RingMulAssoc;

impl Rule for RingMulAssoc {
    fn name(&self) -> &'static str {
        "ring_mul_assoc"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["right", "left"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_mul_assoc", m, cx, "right", Some("left"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingMulAssoc, "in a ring, prove (ab)c = a(bc)");
        assert!(got.contains(&"a(bc) = a(bc)".to_string()), "{got:?}");
        assert!(test_moves(&RingMulAssoc, "in a group, prove (ab)c = a(bc)").is_empty());
    }
}
