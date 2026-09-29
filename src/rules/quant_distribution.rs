//! Quantifiers and connectives (predicate logic, agent L): "everything is P
//! and Q" is "everything is P, and everything is Q"; "something is P or Q"
//! is "something is P, or something is Q". Read backwards, two such parts
//! are gathered under one quantifier. (Not for all over or, nor exists over
//! and: those are not equivalences.)

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Conn, Expr, Math};
use crate::logic::{self, Flavor};

pub struct QuantDistribution;

impl Rule for QuantDistribution {
    fn name(&self) -> &'static str {
        "quant_distribution"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["forall_and", "exists_or", "gather_forall", "gather_exists"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let f = Flavor::Logic;
        let (px, qx) = (Expr::Pred("P".into(), "x".into()), Expr::Pred("Q".into(), "x".into()));
        let q = |all: bool, e: Expr| Expr::Quant(all, "x".into(), Box::new(e));
        // forall over and, exists over or
        let join = |all: bool, v: Vec<Expr>| if all { logic::and(f, v) } else { logic::or(f, v) };
        let law = |all: bool| (q(all, join(all, vec![px.clone(), qx.clone()])), join(all, vec![q(all, px.clone()), q(all, qx.clone())]));
        logic::law_on("quant_distribution", m, cx, |e| {
            let mut out = Vec::new();
            // spread: forall x (A and B) is forall x A and forall x B
            if let Expr::Quant(all, x, body) = e {
                if let Expr::Logic(c, v) = &**body {
                    if (*all && *c == Conn::And) || (!*all && *c == Conn::Or) {
                        let (lhs, rhs) = law(*all);
                        let new = join(*all, v.iter().map(|b| Expr::Quant(*all, x.clone(), Box::new(b.clone()))).collect());
                        out.push(Rewrite { variant: if *all { "forall_and" } else { "exists_or" }, new, says: logic::law_says(f, "Quantifier over a connective", lhs, rhs), work: vec![] });
                    }
                }
            }
            // gather: forall x A and forall x B is forall x (A and B)
            if let Expr::Logic(c @ (Conn::And | Conn::Or), v) = e {
                let all = *c == Conn::And;
                let under: Vec<usize> = (0..v.len()).filter(|&k| matches!(&v[k], Expr::Quant(a, _, _) if *a == all)).collect();
                if let Some(&first) = under.first() {
                    let Expr::Quant(_, x, _) = &v[first] else { return out };
                    let group: Vec<usize> = under.into_iter().filter(|&k| matches!(&v[k], Expr::Quant(_, y, _) if y == x)).collect();
                    if group.len() >= 2 {
                        let bodies: Vec<Expr> = group.iter().map(|&k| if let Expr::Quant(_, _, b) = &v[k] { (**b).clone() } else { unreachable!("a quantifier") }).collect();
                        let gathered = Expr::Quant(all, x.clone(), Box::new(join(all, bodies)));
                        let mut parts = Vec::new();
                        for (k, p) in v.iter().enumerate() {
                            if k == group[0] {
                                parts.push(gathered.clone());
                            } else if !group.contains(&k) {
                                parts.push(p.clone());
                            }
                        }
                        let (lhs, rhs) = law(all);
                        out.push(Rewrite { variant: if all { "gather_forall" } else { "gather_exists" }, new: join(all, parts), says: logic::law_says(f, "Quantifier over a connective", rhs, lhs), work: vec![] });
                    }
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
    fn forall_spreads_over_and() {
        assert_eq!(test_moves(&QuantDistribution, "prove forall x (P(x) and Q(x)) -> forall x P(x)"), vec!["(forall x P(x) and forall x Q(x)) -> forall x P(x)"]);
        // forall does not spread over or
        assert!(test_moves(&QuantDistribution, "prove forall x P(x) -> forall x (P(x) or Q(x))").is_empty());
    }
}
