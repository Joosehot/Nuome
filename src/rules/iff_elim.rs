//! Biconditional (logic, agent L): P <-> Q is (P -> Q) and (Q -> P), or,
//! by cases, (P and Q) or (~P and ~Q).

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor};

pub struct IffElim;

impl Rule for IffElim {
    fn name(&self) -> &'static str {
        "iff_elim"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["both_ways", "cases"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Logic;
        let (p, q) = (logic::ph(f, 0), logic::ph(f, 1));
        let iff = |a: Expr, b: Expr| Expr::Logic(Conn::Iff, vec![a, b]);
        let both = |a: &Expr, b: &Expr| logic::and(f, vec![logic::implies(a.clone(), b.clone()), logic::implies(b.clone(), a.clone())]);
        let cases = |a: &Expr, b: &Expr| logic::or(f, vec![logic::and(f, vec![a.clone(), b.clone()]), logic::and(f, vec![logic::not(f, a.clone()), logic::not(f, b.clone())])]);
        let says_both = logic::law_says(f, "Biconditional law", iff(p.clone(), q.clone()), both(&p, &q));
        let says_cases = logic::law_says(f, "Biconditional law, by cases", iff(p.clone(), q.clone()), cases(&p, &q));
        logic::law_on("iff_elim", m, cx, |e| match e {
            Expr::Logic(Conn::Iff, v) => vec![
                Rewrite { variant: "both_ways", new: both(&v[0], &v[1]), says: says_both.clone(), work: vec![] },
                Rewrite { variant: "cases", new: cases(&v[0], &v[1]), says: says_cases.clone(), work: vec![] },
            ],
            _ => vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn biconditional_splits() {
        assert_eq!(test_moves(&IffElim, "prove (p <-> q) -> ((p -> q) or (r and ~r))"), vec!["((p -> q) and (q -> p)) -> ((p -> q) or (r and ~r))", "((p and q) or (~p and ~q)) -> ((p -> q) or (r and ~r))"]);
        // by cases would grow this small statement past [logic] growth
        assert_eq!(test_moves(&IffElim, "prove (p <-> q) -> (p -> q)"), vec!["((p -> q) and (q -> p)) -> (p -> q)"]);
        assert!(test_moves(&IffElim, "prove (p -> q) or (q -> p)").is_empty());
    }
}
