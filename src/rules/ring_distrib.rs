//! Ring axiom: distributivity, x(y + z) = xy + xz and (x + y)z = xz + yz.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "x(y + z) = xy + xz and (x + y)z = xz + yz";

pub struct RingDistrib;

impl Rule for RingDistrib {
    fn name(&self) -> &'static str {
        "ring_distrib"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["expand", "factor"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("ring_distrib", m, cx, "expand", Some("factor"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&RingDistrib, "in a ring, prove a(b + c) = ab + ac");
        assert!(got.contains(&"ab + ac = ab + ac".to_string()), "{got:?}");
        assert!(test_moves(&RingDistrib, "in a group, prove (ab)c = a(bc)").is_empty());
    }
}
