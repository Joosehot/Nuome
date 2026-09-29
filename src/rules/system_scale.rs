//! An equation of a system with one letter left: divide by its
//! coefficient. -2y = -2 -> y = 1.

use super::eliminate::{form, isolated, letters, used};
use super::{Cx, Line, Move, Rule};
use crate::expr::{self, Expr, Math};

pub struct SystemScale;

impl Rule for SystemScale {
    fn name(&self) -> &'static str {
        "system_scale"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["divide"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::System(eqs) = m else { return vec![] };
        let vars = letters(cx);
        let mut out = Vec::new();
        for (i, e) in eqs.iter().enumerate() {
            if isolated(e).is_some() {
                continue;
            }
            let Some(f) = form(e, &vars) else { continue };
            if used(&f) != 1 {
                continue;
            }
            let Some(k) = f.0.iter().position(|c| !c.is_zero()) else { continue };
            let a = f.0[k];
            let Some(val) = f.1.div(&a) else { continue };
            let mut next = eqs.clone();
            next[i] = (expr::var(&vars[k]), Expr::Num(val));
            let says = if a == crate::q::Q::int(-1) { Line::new().t("Multiply by -1.") } else { Line::new().t(format!("Divide both sides by {a}.")) };
            out.push(Move { rule: "system_scale", variant: "divide", result: Math::System(next), says, work: vec![] });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn divides_by_the_coefficient() {
        assert_eq!(test_moves(&SystemScale, "solve x + y = 3 and -2y = -2"), vec!["x + y = 3, y = 1"]);
        assert!(test_moves(&SystemScale, "solve x + y = 3 and x - y = 1").is_empty());
    }
}
