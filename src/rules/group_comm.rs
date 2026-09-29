//! Abelian group axiom: commutativity, xy = yx. Only in an abelian group;
//! a plain group never reorders a product.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "xy = yx";

pub struct GroupComm;

impl Rule for GroupComm {
    fn name(&self) -> &'static str {
        "group_comm"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["swap"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_comm", m, cx, "swap", Some("swap"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupComm, "in an abelian group, prove ab = ba");
        assert!(got.contains(&"ba = ba".to_string()), "{got:?}");
        assert!(test_moves(&GroupComm, "in a group, prove (ab)^-1 = b^-1 a^-1").is_empty());
    }
}
