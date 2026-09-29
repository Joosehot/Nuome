//! Group lemma: the inverse of a product, (xy)^-1 = y^-1 x^-1 (socks and
//! shoes: the last one on comes off first).

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "(xy)^-1 = y^-1 x^-1";

pub struct GroupInverseProduct;

impl Rule for GroupInverseProduct {
    fn name(&self) -> &'static str {
        "group_inverse_product"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_inverse_product", m, cx, "forward", Some("backward"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupInverseProduct, "in a group, prove (ab)^-1 (ab) = e");
        assert!(got.contains(&"(b^-1 a^-1)(ab) = e".to_string()), "{got:?}");
        assert!(test_moves(&GroupInverseProduct, "in a group, prove (ab)^-1 = b^-1 a^-1").is_empty());
    }
}
