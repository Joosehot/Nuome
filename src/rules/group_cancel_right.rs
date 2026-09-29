//! Group lemma: right cancellation. If yx = zx then y = z.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

/// The law, as a sentence Nuome reads (letters are pattern variables).
pub const LAW: &str = "if yx = zx then y = z";

pub struct GroupCancelRight;

impl Rule for GroupCancelRight {
    fn name(&self) -> &'static str {
        "group_cancel_right"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["rewrite"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::law_moves("group_cancel_right", m, cx, "rewrite", None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&GroupCancelRight, "in a group, prove that if ba = ca then bd = cd");
        assert!(got.contains(&"cd = cd".to_string()), "{got:?}");
        assert!(test_moves(&GroupCancelRight, "in a group, prove that if ba = ca then b = c").is_empty());
    }
}
