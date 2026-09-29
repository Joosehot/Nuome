//! Distributive laws (logic and sets, agent L): P and (Q or R) is
//! (P and Q) or (P and R), and P or (Q and R) is (P or Q) and (P or R); read
//! backwards they take out a common part: (P and Q) or (P and R) is
//! P and (Q or R). The same for intersection and union.

use super::{Cx, Move, Rewrite, Rule};
use crate::expr::{Expr, Math};
use crate::logic::{self, Flavor, B};

pub struct Distributive;

/// Spread `rest` (the other parts of an and, or an or) over the parts of `inner`.
fn spread(f: Flavor, outer_and: bool, rest: &[Expr], inner: &[Expr]) -> Expr {
    let piece = |t: &Expr| {
        let mut v = rest.to_vec();
        v.push(t.clone());
        if outer_and {
            logic::and(f, v)
        } else {
            logic::or(f, v)
        }
    };
    let parts: Vec<Expr> = inner.iter().map(piece).collect();
    if outer_and {
        logic::or(f, parts)
    } else {
        logic::and(f, parts)
    }
}

/// Take a part common to two or more of the parts out: (c and a) or (c and b)
/// is c and (a or b) (`outer_or`), or its dual.
fn take_out(f: Flavor, outer_or: bool, v: &[Expr]) -> Option<Expr> {
    let inner = |e: &Expr| -> Option<Vec<Expr>> {
        match logic::view(e) {
            Some((g, B::And(w))) if g == f && outer_or => Some(w.to_vec()),
            Some((g, B::Or(w))) if g == f && !outer_or => Some(w.to_vec()),
            _ => None,
        }
    };
    for (i, a) in v.iter().enumerate() {
        let Some(parts) = inner(a) else { continue };
        for c in &parts {
            let group: Vec<usize> = (0..v.len()).filter(|&j| inner(&v[j]).is_some_and(|w| w.iter().any(|x| logic::same(x, c)))).collect();
            if group.len() < 2 || group[0] != i {
                continue;
            }
            // what is left of each part once c is taken out
            let rests: Vec<Expr> = group
                .iter()
                .map(|&j| {
                    let w: Vec<Expr> = inner(&v[j]).unwrap_or_default();
                    let k = w.iter().position(|x| logic::same(x, c)).unwrap_or(0);
                    let left: Vec<Expr> = w.iter().enumerate().filter(|(n, _)| *n != k).map(|(_, x)| x.clone()).collect();
                    if outer_or {
                        logic::and(f, left)
                    } else {
                        logic::or(f, left)
                    }
                })
                .collect();
            let joined = if outer_or { logic::and(f, vec![c.clone(), logic::or(f, rests)]) } else { logic::or(f, vec![c.clone(), logic::and(f, rests)]) };
            let mut out: Vec<Expr> = Vec::new();
            for (j, x) in v.iter().enumerate() {
                if j == i {
                    out.push(joined.clone());
                } else if !group.contains(&j) {
                    out.push(x.clone());
                }
            }
            return Some(if outer_or { logic::or(f, out) } else { logic::and(f, out) });
        }
    }
    None
}

impl Rule for Distributive {
    fn name(&self) -> &'static str {
        "distributive"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["and_over_or", "or_over_and", "factor_and", "factor_or"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        logic::law("distributive", m, cx, |_, f, b| {
            let (p, q, r) = (logic::ph(f, 0), logic::ph(f, 1), logic::ph(f, 2));
            // P and (Q or R) <=> (P and Q) or (P and R), or its dual
            let over = |outer_and: bool| {
                let lhs = if outer_and { logic::and(f, vec![p.clone(), logic::or(f, vec![q.clone(), r.clone()])]) } else { logic::or(f, vec![p.clone(), logic::and(f, vec![q.clone(), r.clone()])]) };
                (lhs, spread(f, outer_and, &[p.clone()], &[q.clone(), r.clone()]))
            };
            let mut out = Vec::new();
            let (v, outer_and) = match b {
                B::And(v) => (v, true),
                B::Or(v) => (v, false),
                _ => return out,
            };
            // spread over the first part of the other kind
            let other = v.iter().position(|x| match logic::view(x) {
                Some((g, B::Or(_))) => g == f && outer_and,
                Some((g, B::And(_))) => g == f && !outer_and,
                _ => false,
            });
            if let Some(k) = other {
                let inner = match logic::view(&v[k]) {
                    Some((_, B::Or(w) | B::And(w))) => w.to_vec(),
                    _ => vec![],
                };
                let rest: Vec<Expr> = v.iter().enumerate().filter(|(n, _)| *n != k).map(|(_, x)| x.clone()).collect();
                let (lhs, rhs) = over(outer_and);
                let says = logic::law_says(f, "Distributive law", lhs, rhs);
                out.push(Rewrite { variant: if outer_and { "and_over_or" } else { "or_over_and" }, new: spread(f, outer_and, &rest, &inner), says, work: vec![] });
            }
            // take a common part out
            if let Some(new) = take_out(f, !outer_and, v) {
                let (lhs, rhs) = over(!outer_and);
                let says = logic::law_says(f, "Distributive law, taking out the common part", rhs, lhs);
                out.push(Rewrite { variant: if outer_and { "factor_or" } else { "factor_and" }, new, says, work: vec![] });
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
    fn spreads_and_takes_out() {
        assert_eq!(test_moves(&Distributive, "prove (p and (q or r)) -> p"), vec!["((p and q) or (p and r)) -> p"]);
        assert_eq!(test_moves(&Distributive, "prove ((p and q) or (p and ~q)) <-> p"), vec!["((p and ~q) or p) and ((p and ~q) or q) <=> p", "p and (q or ~q) <=> p"]);
        assert!(test_moves(&Distributive, "prove (p and q) -> p").is_empty());
    }
}
