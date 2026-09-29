//! Negating a quantifier (predicate logic, agent L): "not everything is P"
//! is "something is not P", and "nothing is P" is "everything is not P":
//! ~(forall x P(x)) is exists x ~P(x); ~(exists x P(x)) is forall x ~P(x).

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor};

pub struct QuantNegation;

impl Rule for QuantNegation {
    fn name(&self) -> &'static str {
        "quant_negation"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forall", "exists"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Logic;
        let px = Expr::Pred("P".into(), "x".into());
        let q = |all: bool, e: Expr| Expr::Quant(all, "x".into(), Box::new(e));
        let says = |all: bool| logic::law_says(f, "Negating a quantifier", logic::not(f, q(all, px.clone())), q(!all, logic::not(f, px.clone())));
        logic::law_on("quant_negation", m, cx, |e| match e {
            Expr::Logic(Conn::Not, v) => match &v[0] {
                Expr::Quant(all, x, body) => {
                    let new = Expr::Quant(!all, x.clone(), Box::new(logic::not(f, (**body).clone())));
                    vec![Rewrite { variant: if *all { "forall" } else { "exists" }, new, says: says(*all), work: vec![] }]
                }
                _ => vec![],
            },
            _ => vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn not_all_is_some_not() {
        assert_eq!(test_moves(&QuantNegation, "prove not forall x P(x) <-> exists x not P(x)"), vec!["exists x ~P(x) <=> exists x ~P(x)"]);
        assert!(test_moves(&QuantNegation, "prove forall x P(x) -> exists x P(x)").is_empty());
    }
}
