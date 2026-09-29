//! Element chasing (sets, agent L): two sets are equal when an element is in
//! one exactly when it is in the other, and X is a subset of Y when every
//! element of X is in Y. Belonging is written out by the definitions: x in
//! X intersect Y is x in X and x in Y, x in X union Y is x in X or x in Y,
//! x in X' is x not in X, x in X \ Y is x in X and x not in Y. What is left
//! is a statement of logic, proved with the laws of logic.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::logic;

pub struct ElementChase;

impl Rule for ElementChase {
    fn name(&self) -> &'static str {
        "element_chase"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["equal", "subset"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if !logic::applies("element_chase", m, cx) || *m != cx.req.start() {
            return vec![];
        }
        let x = cx.var;
        let means = |s: &Expr| Line::new().e(&Expr::Member(x.to_string(), Box::new(s.clone()))).t(" means ").e(&logic::member(x, s));
        let work = |l: &Expr, r: &Expr| [l, r].iter().filter(|s| !matches!(s, Expr::Var(_))).map(|s| means(s)).collect::<Vec<_>>();
        match m {
            Math::Eq(l, r) if logic::is_set(l) && logic::is_set(r) => {
                let says = Line::new().t(format!("Chase an element: the sets are equal when any {x} is in one exactly when it is in the other. Write out what that means."));
                vec![Move { rule: "element_chase", variant: "equal", result: Math::Equiv(logic::member(x, l), logic::member(x, r)), says, work: work(l, r) }]
            }
            Math::Subset(l, r) => {
                let says = Line::new().t(format!("Chase an element: take any {x} in ")).e(l).t(" and show it is in ").e(r).t(".");
                vec![Move { rule: "element_chase", variant: "subset", result: Math::Entails(logic::member(x, l), logic::member(x, r)), says, work: work(l, r) }]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn belonging_becomes_logic() {
        assert_eq!(test_moves(&ElementChase, "prove (A ∪ B)' = A' ∩ B'"), vec!["~(x in A or x in B) <=> x not in A and x not in B"]);
        assert_eq!(test_moves(&ElementChase, "prove A ∩ B ⊆ A"), vec!["x in A and x in B => x in A"]);
        assert!(test_moves(&ElementChase, "prove p or ~p is a tautology").is_empty());
    }
}
