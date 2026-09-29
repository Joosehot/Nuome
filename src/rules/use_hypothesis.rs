//! A hypothesis of the statement ("if ab = ac then ..."), used as an
//! equation: one side is replaced by the other. A hypothesis "for all a"
//! may be used with any element for a.

use super::{Cx, Move, Rule};
use crate::abstract_algebra::moves;
use crate::expr::Math;

pub struct UseHypothesis;

impl Rule for UseHypothesis {
    fn name(&self) -> &'static str {
        "use_hypothesis"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forward", "backward"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        moves::hypothesis_moves("use_hypothesis", m, cx, "forward", "backward")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_where_it_should() {
        let got = test_moves(&UseHypothesis, "in a group, prove that if ab = e then (ab)c = c");
        assert!(got.contains(&"ec = c".to_string()), "{got:?}");
        assert!(test_moves(&UseHypothesis, "in a group, prove (ab)^-1 = b^-1 a^-1").is_empty());
    }
}
