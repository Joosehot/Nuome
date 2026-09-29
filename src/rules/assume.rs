//! Conditional proof (logic, agent L): to prove P -> Q, assume P and show Q.
//! Proving P -> Q from assumptions A is proving Q from A and P.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor};

pub struct Assume;

impl Rule for Assume {
    fn name(&self) -> &'static str {
        "assume"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["hypothesis", "again"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if !logic::applies("assume", m, cx) {
            return vec![];
        }
        let says = |a: &Expr, b: &Expr, also: bool| Line::new().t(if also { "Assume also " } else { "Assume " }).e(a).t("; it remains to show ").e(b).t(".");
        match m {
            Math::Taut(Expr::Logic(Conn::Implies, v)) => {
                vec![Move { rule: "assume", variant: "hypothesis", result: Math::Entails(v[0].clone(), v[1].clone()), says: says(&v[0], &v[1], false), work: vec![] }]
            }
            Math::Entails(l, Expr::Logic(Conn::Implies, v)) => {
                let result = Math::Entails(logic::and(Flavor::Logic, vec![l.clone(), v[0].clone()]), v[1].clone());
                vec![Move { rule: "assume", variant: "again", result, says: says(&v[0], &v[1], true), work: vec![] }]
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
    fn assume_the_premise() {
        assert_eq!(test_moves(&Assume, "prove ((p -> q) and p) -> q"), vec!["(p -> q) and p => q"]);
        assert!(test_moves(&Assume, "prove p or ~p is a tautology").is_empty());
    }
}
