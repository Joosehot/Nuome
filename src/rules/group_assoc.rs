//! Group axiom: associativity. In a product of three the brackets may
//! move, (xy)z = x(yz); the order of the factors may not.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(xy)z = x(yz)";

pub struct GroupAssoc;

impl Rule for GroupAssoc {
    fn name(&self) -> &'static str {
        "group_assoc"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["right", "left"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_assoc", m, cx, "right", Some("left"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupAssoc, "in a group, prove (ab)c = a(bc)");
        assert!(got.contains(&"a(bc) = a(bc)".to_string()), "{got:?}");
        assert!(test_moves(&GroupAssoc, "in a group, prove e^-1 = e").is_empty());
    }
}
