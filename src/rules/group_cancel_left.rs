//! Group lemma: left cancellation. If xy = xz then y = z.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "if xy = xz then y = z";

pub struct GroupCancelLeft;

impl Rule for GroupCancelLeft {
    fn name(&self) -> &'static str {
        "group_cancel_left"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rewrite"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_cancel_left", m, cx, "rewrite", None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupCancelLeft, "in a group, prove that if ab = ac then bd = cd");
        assert!(got.contains(&"cd = cd".to_string()), "{got:?}");
        assert!(test_moves(&GroupCancelLeft, "in a group, prove that if ab = ac then b = c").is_empty());
    }
}
