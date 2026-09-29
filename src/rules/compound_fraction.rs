//! A fraction inside a fraction: (a/b)/c = a/(bc), and a/(b/c) = ac/b
//! (dividing by a fraction multiplies by its reciprocal).

use super::distribute::times;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct CompoundFraction;

impl Rule for CompoundFraction {
    fn name(&self) -> &'static str {
        "compound_fraction"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["over", "under"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("compound_fraction", m, |e, _| {
            let Expr::Div(a, b) = e else { return vec![] };
            if e.vars().is_empty() {
                return vec![];
            }
            match (&**a, &**b) {
                (Expr::Div(n, d), c) => {
                    let new = expr::div((**n).clone(), times(d, c));
                    let says = Line::new().t("Dividing ").e(a).t(" by ").e(c).t(" multiplies its denominator by ").e(c).t(".");
                    vec![Rewrite { variant: "over", new, says, work: vec![] }]
                }
                (n, Expr::Div(d, c)) => {
                    let new = expr::div(times(n, c), (**d).clone());
                    let says = Line::new().t("Dividing by ").e(b).t(" is multiplying by ").e(&expr::div((**c).clone(), (**d).clone())).t(".");
                    vec![Rewrite { variant: "under", new, says, work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn flattens_fractions_of_fractions() {
        assert_eq!(test_moves(&CompoundFraction, "simplify ((x + 1)/x)/x"), vec!["(x + 1)/x^2"]);
        assert_eq!(test_moves(&CompoundFraction, "simplify 1/(x/2)"), vec!["2/x"]);
        assert!(test_moves(&CompoundFraction, "simplify (x + 1)/x").is_empty());
    }
}
