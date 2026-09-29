//! Put the given values in: "evaluate x^2 + 1 when x = 3". For a
//! derivative, only once it has been worked out ("at x = 2").

use super::{Cx, Line, Move, Rule};
use crate::expr::Math;

pub struct Substitute;

impl Rule for Substitute {
    fn name(&self) -> &'static str {
        "substitute"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["all"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::Expr(e) = m else { return vec![] };
        if e.has_deriv() || cx.req.given.is_empty() {
            return vec![];
        }
        let mut new = e.clone();
        let mut says = Line::new().t("Substitute ");
        let mut any = false;
        for g in &cx.req.given {
            let (v, val) = &g.value;
            if !new.has_var(v) {
                continue;
            }
            new = new.subst(v, val);
            if any {
                says = says.t(", ");
            }
            says = says.t(format!("{v} = ")).e(val);
            any = true;
        }
        if !any {
            return vec![];
        }
        vec![Move { rule: "substitute", variant: "all", result: Math::Expr(crate::expr::tidy(new)), says: says.t("."), work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_only_with_given_values() {
        assert_eq!(test_moves(&Substitute, "evaluate x^2 + 1 when x = 3"), vec!["3^2 + 1"]);
        assert!(test_moves(&Substitute, "simplify x + x").is_empty());
    }
}
