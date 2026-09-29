//! Modus ponens and modus tollens (logic, agent L): knowing P -> Q and P
//! gives Q, and knowing P -> Q and ~Q gives ~P. As equivalences, so they can
//! stand anywhere: (P -> Q) and P is Q and P; (P -> Q) and ~Q is ~P and ~Q.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor, B};

pub struct ModusPonens;

impl Rule for ModusPonens {
    fn name(&self) -> &'static str {
        "modus_ponens"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["ponens", "tollens"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Logic;
        let (p, q) = (logic::ph(f, 0), logic::ph(f, 1));
        let pq = logic::implies(p.clone(), q.clone());
        let says_ponens = logic::law_says(f, "Modus ponens", logic::and(f, vec![pq.clone(), p.clone()]), logic::and(f, vec![q.clone(), p.clone()]));
        let says_tollens = logic::law_says(f, "Modus tollens", logic::and(f, vec![pq, logic::not(f, q.clone())]), logic::and(f, vec![logic::not(f, p), logic::not(f, q)]));
        logic::law("modus_ponens", m, cx, |_, g, b| {
            let B::And(v) = b else { return vec![] };
            if g != f {
                return vec![];
            }
            let mut out = Vec::new();
            for (k, x) in v.iter().enumerate() {
                let Expr::Logic(Conn::Implies, ab) = x else { continue };
                let (a, b) = (&ab[0], &ab[1]);
                let with = |new: Expr| {
                    let mut w = v.to_vec();
                    w[k] = new;
                    logic::and(f, w)
                };
                if v.iter().enumerate().any(|(j, y)| j != k && logic::same(y, a)) {
                    out.push(Rewrite { variant: "ponens", new: with(b.clone()), says: says_ponens.clone(), work: vec![] });
                } else if v.iter().enumerate().any(|(j, y)| j != k && matches!(logic::view(y), Some((_, B::Not(z))) if logic::same(z, b))) {
                    out.push(Rewrite { variant: "tollens", new: with(logic::not(f, a.clone())), says: says_tollens.clone(), work: vec![] });
                }
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn from_an_implication_and_its_premise() {
        assert_eq!(test_moves(&ModusPonens, "prove ((p -> q) and p) -> q"), vec!["(q and p) -> q"]);
        assert_eq!(test_moves(&ModusPonens, "prove ((p -> q) and ~q) -> ~p"), vec!["(~p and ~q) -> ~p"]);
        assert!(test_moves(&ModusPonens, "prove ((p -> q) and q) -> p or q").is_empty());
    }
}
