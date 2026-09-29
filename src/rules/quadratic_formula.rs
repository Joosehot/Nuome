//! ax^2 + bx + c = 0 -> x = (-b +/- sqrt(b^2 - 4ac))/(2a). Works on every
//! quadratic; the discriminant says how many real solutions there are.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, Expr, Math};
use crate::poly;
use crate::q::Q;

pub struct QuadraticFormula;

impl Rule for QuadraticFormula {
    fn name(&self) -> &'static str {
        "quadratic_formula"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["two_roots", "double_root", "no_real_roots"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("quadratic_formula", m, |l, r| {
            if !r.is_num(0) || l.vars().len() != 1 || !l.has_var(v) {
                return vec![];
            }
            let Some(p) = poly::from_expr(l, v) else { return vec![] };
            if p.deg() != Some(2) {
                return vec![];
            }
            let (a, b, c) = (p.coef(2), p.coef(1), p.coef(0));
            let Some(d) = (|| b.mul(&b)?.sub(&Q::int(4).mul(&a)?.mul(&c)?))() else { return vec![] };
            let (Some(two_a), Some(minus_b)) = (a.mul(&Q::int(2)), Some(b.neg())) else { return vec![] };
            let mut work = vec![Line::new().t(format!("a = {a}, b = {b}, c = {c}."))];
            let disc_expr = Expr::Add(vec![expr::pow(Expr::Num(b), expr::num(2)), Expr::Mul(vec![expr::num(-4), Expr::Num(a), Expr::Num(c)])]);
            work.push(Line::new().t("b^2 - 4ac = ").e(&disc_expr).t(format!(" = {d}.")));
            let x = expr::var(v);
            let says = Line::new().t("Use the quadratic formula.");
            if d.is_neg() {
                work.push(Line::new().t(format!("{d} is negative, and no real number squares to a negative.")));
                return vec![EqRewrite { variant: "no_real_roots", to: Branch::Nothing, says, work }];
            }
            if d.is_zero() {
                work.push(Line::new().t("The discriminant is 0: one repeated solution, x = -b/(2a)."));
                let root = expr::tidy(expr::div(Expr::Num(minus_b), Expr::Num(two_a)));
                return vec![EqRewrite { variant: "double_root", to: Branch::One(x, root), says, work }];
            }
            let top = |plus: bool| {
                let s = expr::sqrt(Expr::Num(d));
                let s = if plus { s } else { Expr::Neg(Box::new(s)) };
                if minus_b.is_zero() {
                    s
                } else {
                    Expr::Add(vec![Expr::Num(minus_b), s])
                }
            };
            let den = Expr::Num(two_a);
            let pm = Line::new().e(&x).t(" = (").e(&Expr::Num(minus_b)).t(" ").pm().t(" ").e(&expr::sqrt(Expr::Num(d))).t(")/").e(&den);
            work.push(pm);
            // smaller solution first
            let (first, second) = if two_a.is_neg() { (true, false) } else { (false, true) };
            let roots = vec![(x.clone(), expr::div(top(first), den.clone())), (x, expr::div(top(second), den))];
            vec![EqRewrite { variant: "two_roots", to: Branch::Many(roots), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn applies_to_quadratics_in_standard_form() {
        assert_eq!(test_moves(&QuadraticFormula, "solve x^2 - 5x + 6 = 0"), vec!["x = (5 - sqrt(1))/2 or x = (5 + sqrt(1))/2"]);
        assert_eq!(test_moves(&QuadraticFormula, "solve x^2 + 1 = 0"), vec!["no real solution"]);
        assert!(test_moves(&QuadraticFormula, "solve x^2 = 4").is_empty());
    }
}
