//! Square roots of numbers: sqrt(49) = 7, sqrt(72) = sqrt(36 * 2) = 6sqrt(2).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Math};
use crate::q::split_square;

pub struct Roots;

impl Rule for Roots {
    fn name(&self) -> &'static str {
        "roots"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["exact", "pull_square"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("roots", m, |e, _| {
            let Expr::Func(Func::Sqrt, a) = e else { return vec![] };
            let Some(q) = a.as_num() else { return vec![] };
            if q.is_neg() {
                return vec![];
            }
            if let Some(r) = q.sqrt() {
                return vec![Rewrite { variant: "exact", new: Expr::Num(r), says: Line::new().e(e).t(" = ").e(&Expr::Num(r)).t(format!(", because {r} squared is {q}.")), work: vec![] }];
            }
            if q.is_int() {
                let (k, rest) = split_square(q.num());
                if k > 1 {
                    let new = expr::mul(vec![expr::num(k), expr::sqrt(expr::num(rest))]);
                    let split = expr::sqrt(Expr::Mul(vec![expr::num(k * k), expr::num(rest)]));
                    let says = Line::new().t(format!("Take the square factor {} out of the root.", k * k));
                    let work = vec![Line::new().e(e).t(" = ").e(&split).t(" = ").e(&new)];
                    return vec![Rewrite { variant: "pull_square", new, says, work }];
                }
            }
            vec![]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn simplifies_number_roots() {
        assert_eq!(test_moves(&Roots, "what is sqrt(49)"), vec!["7"]);
        assert_eq!(test_moves(&Roots, "what is sqrt 72"), vec!["6sqrt(2)"]);
        assert!(test_moves(&Roots, "what is sqrt 7").is_empty());
    }
}
