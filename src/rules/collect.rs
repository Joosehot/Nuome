//! Like terms: 3x + 2x -> 5x, x^2 - x^2 -> 0. Variants: one kind of term at
//! a time, or every kind in one step.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Math};
use crate::q::Q;

pub struct Collect;

/// Groups of like terms in a sum: (the shared part, indices), in order of
/// first appearance; only groups of two or more.
fn groups(v: &[Expr]) -> Vec<(Expr, Vec<usize>)> {
    let mut out: Vec<(Expr, Vec<usize>)> = Vec::new();
    for (i, t) in v.iter().enumerate() {
        let (_, rest) = coeff(t);
        if rest.is_num(1) {
            continue;
        }
        match out.iter_mut().find(|(r, _)| *r == rest) {
            Some((_, ix)) => ix.push(i),
            None => out.push((rest, vec![i])),
        }
    }
    out.retain(|(_, ix)| ix.len() >= 2);
    out
}

/// Combine the terms of the given groups; each combined term takes the
/// place of its group's first term.
fn combine(v: &[Expr], gs: &[(Expr, Vec<usize>)]) -> Option<(Expr, Vec<Line>)> {
    let mut slots: Vec<Option<Expr>> = v.iter().cloned().map(Some).collect();
    let mut work = Vec::new();
    for (rest, ix) in gs {
        let mut c = Q::ZERO;
        for &i in ix {
            c = c.add(&coeff(&v[i]).0)?;
        }
        let combined = with_coeff(c, rest.clone());
        let before = Expr::Add(ix.iter().map(|&i| v[i].clone()).collect());
        let coeffs = Expr::Add(ix.iter().map(|&i| Expr::Num(coeff(&v[i]).0)).collect());
        work.push(Line::new().e(&before).t(" = ").e(&expr::mul(vec![coeffs, rest.clone()])).t(" = ").e(&combined));
        for &i in ix {
            slots[i] = None;
        }
        slots[ix[0]] = Some(combined);
    }
    let terms: Vec<Expr> = slots.into_iter().flatten().filter(|t| !t.is_num(0)).collect();
    Some((expr::add(terms), work))
}

impl Rule for Collect {
    fn name(&self) -> &'static str {
        "collect"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["one", "all"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("collect", m, |e, _| {
            let Expr::Add(v) = e else { return vec![] };
            let gs = groups(v);
            let mut out = Vec::new();
            if let Some(first) = gs.first() {
                if let Some((new, work)) = combine(v, std::slice::from_ref(first)) {
                    let says = Line::new().t("Collect the ").e(&first.0).t(" terms.");
                    out.push(Rewrite { variant: "one", new, says, work });
                }
            }
            if gs.len() >= 2 {
                if let Some((new, work)) = combine(v, &gs) {
                    out.push(Rewrite { variant: "all", new, says: Line::new().t("Collect like terms."), work });
                }
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn combines_like_terms() {
        assert_eq!(test_moves(&Collect, "simplify 3x + 2x"), vec!["5x"]);
        assert_eq!(test_moves(&Collect, "simplify x^2 + 3x - x^2 - x"), vec!["3x - x", "2x"]);
        assert!(test_moves(&Collect, "simplify x^2 + x + 1").is_empty());
    }
}
