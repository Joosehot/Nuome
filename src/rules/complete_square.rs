//! Complete the square: x^2 + 6x + 5 = 0 -> (x + 3)^2 = 4. With a != 1,
//! divide by a first (a second variant, one step either way).

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, Expr, Math};
use crate::poly;
use crate::q::Q;

pub struct CompleteSquare;

impl Rule for CompleteSquare {
    fn name(&self) -> &'static str {
        "complete_square"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["monic", "divide_by_a"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("complete_square", m, |l, r| {
            if !r.is_num(0) || l.vars().len() != 1 || !l.has_var(v) {
                return vec![];
            }
            let Some(p) = poly::from_expr(l, v) else { return vec![] };
            if p.deg() != Some(2) || p.coef(1).is_zero() {
                return vec![];
            }
            let (a, b, c) = (p.coef(2), p.coef(1), p.coef(0));
            let Some((h, k)) = (|| {
                let h = b.div(&a.mul(&Q::int(2))?)?;
                let k = h.mul(&h)?.sub(&c.div(&a)?)?;
                Some((h, k))
            })() else {
                return vec![];
            };
            let x = expr::var(v);
            let mut work = Vec::new();
            let variant = if a.is_one() {
                "monic"
            } else {
                work.push(Line::new().t(format!("Divide by {a} so that x^2 stands alone.")));
                "divide_by_a"
            };
            work.push(Line::new().t(format!("Half of the {v} coefficient is {h}; ")).e(&expr::pow(expr::add(vec![x.clone(), Expr::Num(h)]), expr::num(2))).t(" = ").e(&expr::add(vec![expr::pow(x.clone(), expr::num(2)), expr::mul(vec![Expr::Num(h.mul(&Q::int(2)).unwrap_or(h)), x.clone()]), Expr::Num(h.mul(&h).unwrap_or(h))])).t("."));
            let sq = expr::pow(expr::add(vec![x, Expr::Num(h)]), expr::num(2));
            let says = Line::new().t("Complete the square.");
            vec![EqRewrite { variant, to: Branch::One(sq, Expr::Num(k)), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn completes_the_square() {
        assert_eq!(test_moves(&CompleteSquare, "solve x^2 + 6x + 5 = 0"), vec!["(x + 3)^2 = 4"]);
        assert_eq!(test_moves(&CompleteSquare, "solve 2x^2 - 4x - 6 = 0"), vec!["(x - 1)^2 = 4"]);
        assert!(test_moves(&CompleteSquare, "solve x^2 - 9 = 0").is_empty());
    }
}
