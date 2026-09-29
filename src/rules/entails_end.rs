//! The end of a proof from assumptions, or of a subset (logic and sets,
//! agent L): what is to be shown is what was assumed, or one of its
//! and-parts ("in particular"), or it has what was assumed as one of its
//! or-parts; a set is inside a union it is part of, and an intersection is
//! inside each of its sets.

use super::{Cx, Line, Move, Rule};
use crate::expr::Math;
use crate::logic;

pub struct EntailsEnd;

impl Rule for EntailsEnd {
    fn name(&self) -> &'static str {
        "entails_end"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same", "conjunct", "disjunct", "part", "instance", "trivial"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if !logic::applies("entails_end", m, cx) || logic::table_first(m, cx) {
            return vec![];
        }
        let (Math::Entails(l, r) | Math::Subset(l, r)) = m else { return vec![] };
        let Some(why) = logic::entails_by_parts(l, r) else { return vec![] };
        let sets = matches!(m, Math::Subset(..));
        let mut says = match (why, sets) {
            ("same", false) => Line::new().t("What is to be shown is what was assumed."),
            ("same", true) => Line::new().t("Both sides are the same set: ").e(l).t("."),
            ("conjunct", false) => Line::new().t("In particular ").e(r).t(": it is part of what was assumed."),
            ("conjunct", true) => Line::new().t("An intersection is inside each of its sets: ").m(m).t("."),
            ("disjunct", false) => Line::new().e(l).t(" is one of the alternatives in ").e(r).t(", so ").e(r).t(" holds."),
            ("disjunct", true) => Line::new().t("A set is inside any union it is part of: ").m(m).t("."),
            ("part", false) => {
                let c = logic::shared_part(l, r).unwrap_or_else(|| l.clone());
                Line::new().t("In particular ").e(&c).t(", one of the alternatives in ").e(r).t(", so ").e(r).t(" holds.")
            }
            ("instance", _) => {
                let c = logic::instance(l, r).unwrap_or_else(|| l.clone());
                Line::new().t("A domain is never empty, so from ").e(&c).t(" there is something it holds for: ").e(r).t(".")
            }
            ("part", true) => Line::new().t("An intersection is inside each of its sets, and each set is inside any union it is part of: ").m(m).t("."),
            (_, false) => Line::new().t("It holds whatever was assumed: ").m(m).t("."),
            (_, true) => Line::new().t("Every set is inside U, and the empty set is inside every set: ").m(m).t("."),
        };
        // a chased subset: say what it means for the sets
        if let Math::Subset(a, b) = &cx.req.problem.value {
            if !sets {
                says = says.t(format!(" So every {} in ", cx.var)).e(a).t(" is in ").e(b).t(".");
            }
        }
        vec![Move { rule: "entails_end", variant: why, result: Math::Proved, says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{test_expr, test_moves_from};

    #[test]
    fn the_goal_is_among_the_assumptions() {
        assert_eq!(test_moves_from(&EntailsEnd, "prove ((p -> q) and p) -> q", Math::Entails(test_expr("q and p"), test_expr("q"))), vec!["proved"]);
        assert_eq!(test_moves_from(&EntailsEnd, "prove ((p -> q) and p) -> q", Math::Entails(test_expr("p"), test_expr("p or q"))), vec!["proved"]);
        assert!(test_moves_from(&EntailsEnd, "prove ((p -> q) and p) -> q", Math::Entails(test_expr("(p -> q) and p"), test_expr("q"))).is_empty());
    }
}
