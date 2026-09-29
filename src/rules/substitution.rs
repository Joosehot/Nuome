//! Substitution for simultaneous linear equations: solve one equation for
//! a letter (x + y = 3 -> x = 3 - y), then put that expression into
//! another equation, which loses the letter: x - y = 1 becomes
//! (3 - y) - y = 1, that is -2y = -2. Variants: isolate a letter whose
//! coefficient is 1 or -1, isolate one with another coefficient
//! (fractions), substitute.

use super::eliminate::{build, form, isolated, letters, name, used, Form};
use super::{Cx, Line, Move, Rule};
use crate::expr::{self, with_coeff, Expr, Math};

pub struct Substitution;

impl Rule for Substitution {
    fn name(&self) -> &'static str {
        "substitution"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["isolate", "isolate_fraction", "substitute"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::System(eqs) = m else { return vec![] };
        let vars = letters(cx);
        let Some(forms) = eqs.iter().map(|e| form(e, &vars)).collect::<Option<Vec<Form>>>() else { return vec![] };
        let mut out = Vec::new();
        // substitute an isolated letter (with letters on its right) into another equation that has it
        let mut pending = false;
        for (i, e) in eqs.iter().enumerate() {
            let Some(v) = isolated(e) else { continue };
            if e.1.vars().is_empty() {
                continue;
            }
            for (j, target) in eqs.iter().enumerate() {
                if i == j || !(target.0.has_var(v) || target.1.has_var(v)) || isolated(target) == Some(v) {
                    continue;
                }
                pending = true;
                let raw = (target.0.subst(v, &e.1), target.1.subst(v, &e.1));
                let Some(f) = form(&raw, &vars) else { continue };
                let mut next = eqs.clone();
                next[j] = build(&f, &vars);
                let says = Line::new().t("Substitute ").m(&Math::Eq(e.0.clone(), e.1.clone())).t(format!(" into {}.", name(j)));
                let work = vec![Line::new().m(&Math::Eq(target.0.clone(), target.1.clone())).t(" becomes ").m(&Math::Eq(raw.0, raw.1)).t(".")];
                out.push(Move { rule: "substitution", variant: "substitute", result: Math::System(next), says, work });
            }
        }
        if pending {
            return out;
        }
        // solve one equation for one letter
        for (i, f) in forms.iter().enumerate() {
            if isolated(&eqs[i]).is_some() || used(f) < 2 {
                continue;
            }
            for (k, v) in vars.iter().enumerate() {
                let a = f.0[k];
                if a.is_zero() {
                    continue;
                }
                // v = (d - the other terms)/a, the number first: x = 3 - y
                let Some(d) = f.1.div(&a) else { continue };
                let mut rhs = Vec::new();
                if !d.is_zero() {
                    rhs.push(Expr::Num(d));
                }
                let mut ok = true;
                for (c, w) in f.0.iter().zip(&vars) {
                    if w == v || c.is_zero() {
                        continue;
                    }
                    match c.div(&a) {
                        Some(q) => rhs.push(with_coeff(q.neg(), expr::var(w))),
                        None => ok = false,
                    }
                }
                if !ok {
                    continue;
                }
                let solved = (expr::var(v), expr::add(rhs));
                let mut next = eqs.clone();
                next[i] = solved.clone();
                let variant = if a.abs().is_one() { "isolate" } else { "isolate_fraction" };
                let says = Line::new().t(format!("Solve {} for {v}: ", name(i))).m(&Math::Eq(solved.0, solved.1)).t(".");
                out.push(Move { rule: "substitution", variant, result: Math::System(next), says, work: vec![] });
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
    fn isolates_and_substitutes() {
        assert_eq!(test_moves(&Substitution, "solve y = 2x + 1 and 3x + y = 11"), vec!["y = 2x + 1, 5x = 10"]);
        assert_eq!(test_moves(&Substitution, "solve x + y = 3 and x - y = 1")[0], "x = 3 - y, x - y = 1");
        assert!(test_moves(&Substitution, "solve x = 2 and y = 1").is_empty());
    }
}
