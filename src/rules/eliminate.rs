//! Elimination for simultaneous linear equations: add or subtract two
//! equations (scaled first if needed) so that one letter cancels:
//! x + y = 3, x - y = 1 -> adding gives 2x = 4. Variants: add, subtract,
//! scale then combine. The new equation replaces the second one, and it
//! only ever loses letters, so elimination always ends.
//!
//! Also the helpers every system rule shares: reading an equation as
//! coefficients, writing coefficients back as an equation.

use super::{Cx, Line, Move, Rule};
use crate::expr::{self, with_coeff, Expr, Math};
use crate::q::{lcm, Q};

pub struct Eliminate;

/// The letters of the system, in alphabetical order.
pub fn letters(cx: &Cx) -> Vec<String> {
    match &cx.req.problem.value {
        Math::System(eqs) => eqs.iter().flat_map(|(l, r)| l.vars().into_iter().chain(r.vars())).collect::<std::collections::BTreeSet<_>>().into_iter().collect(),
        _ => vec![],
    }
}

/// An equation as coefficients and right side: 2x - y = 4 -> ([2, -1], 4).
pub type Form = (Vec<Q>, Q);

pub fn form(eq: &(Expr, Expr), vars: &[String]) -> Option<Form> {
    crate::poly::linear_form(&eq.0, &eq.1, vars)
}

/// Coefficients back to an equation: ([2, -1], 4) -> 2x - y = 4.
pub fn build(f: &Form, vars: &[String]) -> (Expr, Expr) {
    let ts: Vec<Expr> = f.0.iter().zip(vars).filter(|(c, _)| !c.is_zero()).map(|(c, v)| with_coeff(*c, expr::var(v))).collect();
    (expr::add(ts), Expr::Num(f.1))
}

pub fn eq(f: &Form, vars: &[String]) -> Math {
    let (l, r) = build(f, vars);
    Math::Eq(l, r)
}

/// How many letters an equation still has.
pub fn used(f: &Form) -> usize {
    f.0.iter().filter(|c| !c.is_zero()).count()
}

/// "x = 3 - y": the letter it is solved for, if it is.
pub fn isolated(eq: &(Expr, Expr)) -> Option<&str> {
    match &eq.0 {
        Expr::Var(v) if !eq.1.has_var(v) => Some(v),
        _ => None,
    }
}

/// "the first equation"
pub fn name(i: usize) -> String {
    let n = ["first", "second", "third", "fourth"].get(i).copied().unwrap_or("next");
    format!("the {n} equation")
}

/// a * f
pub fn times(a: &Q, f: &Form) -> Option<Form> {
    Some((f.0.iter().map(|c| c.mul(a)).collect::<Option<Vec<_>>>()?, f.1.mul(a)?))
}

/// f - g
pub fn minus(f: &Form, g: &Form) -> Option<Form> {
    Some((f.0.iter().zip(&g.0).map(|(a, b)| a.sub(b)).collect::<Option<Vec<_>>>()?, f.1.sub(&g.1)?))
}

impl Rule for Eliminate {
    fn name(&self) -> &'static str {
        "eliminate"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["add", "subtract", "scale"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::System(eqs) = m else { return vec![] };
        let vars = letters(cx);
        let Some(forms) = eqs.iter().map(|e| form(e, &vars)).collect::<Option<Vec<Form>>>() else { return vec![] };
        let two = eqs.len() == 2;
        let mut out = Vec::new();
        for (k, v) in vars.iter().enumerate() {
            for i in 0..eqs.len() {
                for j in 0..eqs.len() {
                    let (fi, fj) = (&forms[i], &forms[j]);
                    if i == j || isolated(&eqs[i]).is_some() || isolated(&eqs[j]).is_some() || used(fi) < 2 || used(fj) < 2 {
                        continue;
                    }
                    let (ai, aj) = (fi.0[k], fj.0[k]);
                    // the new equation may only lose letters
                    if ai.is_zero() || aj.is_zero() || fi.0.iter().zip(&fj.0).any(|(a, b)| !a.is_zero() && b.is_zero()) {
                        continue;
                    }
                    // scale so the letter's coefficients are equal (subtract) or opposite (add)
                    let (si, sj) = if ai.abs() == aj.abs() {
                        (Q::ONE, Q::ONE)
                    } else if ai.is_int() && aj.is_int() {
                        let Some(l) = lcm(ai.num(), aj.num()) else { continue };
                        (Q::int(l / ai.num().abs()), Q::int(l / aj.num().abs()))
                    } else {
                        continue;
                    };
                    let (Some(gi), Some(gj)) = (times(&si, fi), times(&sj, fj)) else { continue };
                    let add = ai.is_neg() != aj.is_neg();
                    let Some(neg_gi) = times(&Q::int(-1), &gi) else { continue };
                    let Some(new) = minus(&gj, if add { &neg_gi } else { &gi }) else { continue };
                    let (variant, says) = match (si.is_one() && sj.is_one(), add) {
                        (true, true) if two => ("add", Line::new().t(format!("Add the two equations to eliminate {v}."))),
                        (true, true) => ("add", Line::new().t(format!("Add {} to {} to eliminate {v}.", name(i), name(j)))),
                        (true, false) => ("subtract", Line::new().t(format!("Subtract {} from {} to eliminate {v}.", name(i), name(j)))),
                        (false, add) => {
                            let op = if add { "add" } else { "subtract" };
                            let scaled = [(si, i), (sj, j)].iter().filter(|(s, _)| !s.is_one()).map(|(s, n)| format!("{} by {s}", name(*n))).collect::<Vec<_>>().join(" and ");
                            ("scale", Line::new().t(format!("Multiply {scaled}, then {op} to eliminate {v}.")))
                        }
                    };
                    let mut work = Vec::new();
                    for (s, f, g) in [(si, fi, &gi), (sj, fj, &gj)] {
                        if !s.is_one() {
                            work.push(Line::new().t(format!("{s} * (")).m(&eq(f, &vars)).t(") gives ").m(&eq(g, &vars)).t("."));
                        }
                    }
                    let mut next = eqs.clone();
                    next[j] = build(&new, &vars);
                    out.push(Move { rule: "eliminate", variant, result: Math::System(next), says, work });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn eliminates_a_letter() {
        assert_eq!(test_moves(&Eliminate, "solve x + y = 3 and x - y = 1"), vec!["x + y = 3, -2y = -2", "2y = 2, x - y = 1", "x + y = 3, 2x = 4", "2x = 4, x - y = 1"]);
        assert!(test_moves(&Eliminate, "solve x = 3 - y and x - y = 1").is_empty());
    }
}
