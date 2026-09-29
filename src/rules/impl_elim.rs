//! Implication (logic, agent L): P -> Q is ~P or Q. An implication is false
//! only when its premise is true and its conclusion false.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor};

pub struct ImplElim;

impl Rule for ImplElim {
    fn name(&self) -> &'static str {
        "impl_elim"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["or"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Logic;
        let (p, q) = (logic::ph(f, 0), logic::ph(f, 1));
        let says = logic::law_says(f, "Implication law", logic::implies(p.clone(), q.clone()), logic::or(f, vec![logic::not(f, p), q]));
        logic::law_on("impl_elim", m, cx, |e| match e {
            Expr::Logic(Conn::Implies, v) => vec![Rewrite { variant: "or", new: logic::or(f, vec![logic::not(f, v[0].clone()), v[1].clone()]), says: says.clone(), work: vec![] }],
            _ => vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn implication_becomes_or() {
        assert_eq!(test_moves(&ImplElim, "prove (p -> q) <-> (~q -> ~p)"), vec!["~p or q <=> ~~q or ~p"]);
        assert!(test_moves(&ImplElim, "prove not (p and q) is equivalent to not p or not q").is_empty());
    }
}
