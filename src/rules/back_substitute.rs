//! Back-substitution: once a letter's value is known (y = 1), put it into
//! the equations that still have it: x + y = 3 becomes x + 1 = 3, that is
//! x = 2.

use super::eliminate::{build, form, isolated, letters, name};
use super::{Cx, Line, Move, Rule};
use crate::expr::Math;

pub struct BackSubstitute;

impl Rule for BackSubstitute {
    fn name(&self) -> &'static str {
        "back_substitute"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["value"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::System(eqs) = m else { return vec![] };
        let vars = letters(cx);
        let mut out = Vec::new();
        for (i, e) in eqs.iter().enumerate() {
            let Some(v) = isolated(e) else { continue };
            if !e.1.vars().is_empty() {
                continue;
            }
            for (j, target) in eqs.iter().enumerate() {
                if i == j || !(target.0.has_var(v) || target.1.has_var(v)) {
                    continue;
                }
                let raw = (target.0.subst(v, &e.1), target.1.subst(v, &e.1));
                let Some(f) = form(&raw, &vars) else { continue };
                // an isolated letter stays on the left: x = 3 - y becomes x = 2
                let mut next = eqs.clone();
                next[j] = build(&f, &vars);
                let says = Line::new().t("Put ").m(&Math::Eq(e.0.clone(), e.1.clone())).t(format!(" into {}.", name(j)));
                let work = vec![Line::new().m(&Math::Eq(target.0.clone(), target.1.clone())).t(" becomes ").m(&Math::Eq(raw.0, raw.1)).t(".")];
                out.push(Move { rule: "back_substitute", variant: "value", result: Math::System(next), says, work });
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
    fn puts_known_values_back() {
        assert_eq!(test_moves(&BackSubstitute, "solve x + y = 3 and y = 1"), vec!["x = 2, y = 1"]);
        assert!(test_moves(&BackSubstitute, "solve x + y = 3 and x - y = 1").is_empty());
    }
}
