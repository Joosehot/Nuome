//! The end of a proof of an equivalence or a tautology (logic and sets,
//! agent L): both sides have become the same statement (or the same set),
//! up to the order of the parts of and, or, union and intersection (they
//! commute and associate); or the statement has become T.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::logic;

pub struct EquivEnd;

impl Rule for EquivEnd {
    fn name(&self) -> &'static str {
        "equiv_end"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same", "true"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if !logic::applies("equiv_end", m, cx) || logic::table_first(m, cx) {
            return vec![];
        }
        let done = |says: Line, variant: &'static str| vec![Move { rule: "equiv_end", variant, result: Math::Proved, says, work: vec![] }];
        let both = |l: &Expr, r: &Expr| Line::new().t(if l == r { "Both sides are the same: " } else { "Both sides are the same, up to the order of the parts: " }).e(l).t(".");
        match m {
            Math::Equiv(l, r) if logic::same(l, r) => {
                let mut says = both(l, r);
                // a chased set equality: say what it means for the sets
                if let Math::Eq(a, b) = &cx.req.problem.value {
                    says = says.t(format!(" So any {} is in ", cx.var)).e(a).t(" exactly when it is in ").e(b).t(": the sets are equal.");
                }
                done(says, "same")
            }
            Math::Eq(l, r) if logic::is_set(l) && logic::same(l, r) => done(both(l, r), "same"),
            Math::Taut(Expr::Truth(true)) => done(Line::new().t("The statement has become T: it is true whatever the letters stand for."), "true"),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{test_expr, test_moves, test_moves_from};

    #[test]
    fn same_up_to_order() {
        // the statement as asked is the commutative law itself: that is the truth table's
        assert!(test_moves(&EquivEnd, "prove p or q <-> q or p").is_empty());
        assert_eq!(test_moves_from(&EquivEnd, "prove (p or q) and T <-> q or p", Math::Equiv(test_expr("p or q"), test_expr("q or p"))), vec!["proved"]);
        assert_eq!(test_moves_from(&EquivEnd, "prove p or ~p is a tautology", Math::Taut(Expr::Truth(true))), vec!["proved"]);
        assert!(test_moves(&EquivEnd, "prove p or q <-> ~(~p and ~q)").is_empty());
    }
}
