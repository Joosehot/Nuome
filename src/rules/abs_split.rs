//! Split an absolute value equation into cases: |A| = 5 means A = 5 or
//! A = -5; |A| = |B| means A = B or A = -B; |A| = B (B with the letter)
//! means the same two cases, but only where B >= 0, so each answer is
//! checked afterwards (abs_check). No absolute value is negative.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Func, Math};

pub struct AbsSplit;

/// -B with the sign taken inside: -(2x + 3) -> -2x - 3.
fn opposite(b: &Expr) -> Expr {
    expr::add(expr::terms(b).iter().map(|t| with_coeff(coeff(t).0.neg(), coeff(t).1)).collect())
}

fn inside(e: &Expr) -> Option<&Expr> {
    match e {
        Expr::Func(Func::Abs, a) => Some(a),
        _ => None,
    }
}

impl Rule for AbsSplit {
    fn name(&self) -> &'static str {
        "abs_split"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["cases", "zero", "never", "both_abs", "cases_check"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("abs_split", m, |l, r| {
            let (abs, other) = match (inside(l), inside(r)) {
                (Some(a), Some(b)) => {
                    let says = Line::new().e(l).t(" = ").e(r).t(" means the insides are equal or opposite.");
                    let to = Branch::Many(vec![(a.clone(), b.clone()), (a.clone(), opposite(b))]);
                    return vec![EqRewrite { variant: "both_abs", to, says, work: vec![] }];
                }
                (Some(a), None) => (a, r),
                (None, Some(b)) => (b, l),
                _ => return vec![],
            };
            if !abs.has_var(v) {
                return vec![];
            }
            let whole = expr::func(Func::Abs, abs.clone());
            if other.has_var(v) {
                let says = Line::new().e(&whole).t(" = ").e(other).t(" means ").e(abs).t(" = ").e(other).t(" or ").e(abs).t(" = ").e(&opposite(other)).t(", where ").e(other).t(" is not negative.");
                let work = vec![Line::new().t("Each answer is checked in the original equation.")];
                let to = Branch::Many(vec![(abs.clone(), other.clone()), (abs.clone(), opposite(other))]);
                return vec![EqRewrite { variant: "cases_check", to, says, work }];
            }
            let Some(c) = other.eval_q(&|_| None) else { return vec![] };
            if c.is_neg() {
                let says = Line::new().t(format!("An absolute value is never negative, so it can't equal {c}."));
                return vec![EqRewrite { variant: "never", to: Branch::Nothing, says, work: vec![] }];
            }
            if c.is_zero() {
                let says = Line::new().t("Only 0 has absolute value 0.");
                return vec![EqRewrite { variant: "zero", to: Branch::One(abs.clone(), expr::num(0)), says, work: vec![] }];
            }
            let says = Line::new().e(&whole).t(format!(" = {c} means ")).e(abs).t(format!(" = {c} or ")).e(abs).t(format!(" = {}.", c.neg()));
            let to = Branch::Many(vec![(abs.clone(), Expr::Num(c)), (abs.clone(), Expr::Num(c.neg()))]);
            vec![EqRewrite { variant: "cases", to, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn splits_into_cases() {
        assert_eq!(test_moves(&AbsSplit, "solve |2x - 3| = 5"), vec!["2x - 3 = 5 or 2x - 3 = -5"]);
        assert_eq!(test_moves(&AbsSplit, "solve |x + 1| = -2"), vec!["no real solution"]);
        assert_eq!(test_moves(&AbsSplit, "solve |x - 1| = |2x + 3|"), vec!["x - 1 = 2x + 3 or x - 1 = -2x - 3"]);
        assert!(test_moves(&AbsSplit, "solve 2|x| = 6").is_empty());
    }
}
