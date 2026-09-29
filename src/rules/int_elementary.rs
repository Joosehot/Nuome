//! The integrals to know by heart: sin, cos, e^x, a^x, tan, sec^2, csc^2,
//! sec tan, csc cot, 1/(1 + x^2), 1/sqrt(1 - x^2).

use super::int_linear::{scaled, split_const};
use super::int_power::log_abs;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Math};
use crate::q::Q;

pub struct IntElementary;

/// int h du for a function h of u in the table: (number factor, the rest),
/// so int sin(u) du = (-1, cos(u)).
pub fn table(h: &Expr, u: &Expr) -> Option<(Q, Expr)> {
    let f = |g: Func| expr::func(g, u.clone());
    let is = |a: &Expr| a == u;
    let square_of = |e: &Expr, g: Func| matches!(e, Expr::Pow(b, n) if n.is_num(2) && matches!(&**b, Expr::Func(k, a) if *k == g && is(a)));
    let one = Q::ONE;
    match h {
        Expr::Func(Func::Sin, a) if is(a) => Some((one.neg(), f(Func::Cos))),
        Expr::Func(Func::Cos, a) if is(a) => Some((one, f(Func::Sin))),
        Expr::Func(Func::Exp, a) if is(a) => Some((one, f(Func::Exp))),
        Expr::Func(Func::Tan, a) if is(a) => Some((one.neg(), log_abs(&f(Func::Cos)))),
        e if square_of(e, Func::Sec) => Some((one, f(Func::Tan))),
        e if square_of(e, Func::Csc) => Some((one.neg(), f(Func::Cot))),
        Expr::Div(n, d) if n.is_num(1) && square_of(d, Func::Cos) => Some((one, f(Func::Tan))),
        Expr::Div(n, d) if n.is_num(1) && square_of(d, Func::Sin) => Some((one.neg(), f(Func::Cot))),
        Expr::Mul(v) if v.len() == 2 => {
            let has = |g: Func| v.iter().any(|x| matches!(x, Expr::Func(k, a) if *k == g && is(a)));
            if has(Func::Sec) && has(Func::Tan) {
                Some((one, f(Func::Sec)))
            } else if has(Func::Csc) && has(Func::Cot) {
                Some((one.neg(), f(Func::Csc)))
            } else {
                None
            }
        }
        // a^u = a^u / ln(a)
        Expr::Pow(b, a) if is(a) => {
            let base = b.as_num().filter(|q| !q.is_neg() && !q.is_zero() && !q.is_one())?;
            Some((one, expr::div(h.clone(), expr::func(Func::Ln, Expr::Num(base)))))
        }
        // 1/(1 + u^2) and 1/sqrt(1 - u^2)
        Expr::Div(n, d) if n.is_num(1) => {
            let sq = expr::pow(u.clone(), expr::num(2));
            let one_plus = |e: &Expr| matches!(e, Expr::Add(t) if t.len() == 2 && t.contains(&expr::num(1)) && t.contains(&sq));
            let one_minus = |e: &Expr| matches!(e, Expr::Add(t) if t.len() == 2 && t[0].is_num(1) && t[1] == expr::neg(sq.clone()));
            match &**d {
                e if one_plus(e) => Some((one, f(Func::Atan))),
                Expr::Func(Func::Sqrt, a) if one_minus(a) => Some((one, f(Func::Asin))),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Rule for IntElementary {
    fn name(&self) -> &'static str {
        "int_elementary"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["table"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_elementary", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let (c, h) = split_const(inner);
            let Some((k, body)) = table(&h, &expr::var(v)) else { return vec![] };
            let Some(ck) = c.mul(&k) else { return vec![] };
            let base = Expr::Integral(Box::new(h), v.clone());
            let says = Line::new().t("Standard integral: ").e(&base).t(" = ").e(&scaled(k, body.clone())).t(".");
            vec![Rewrite { variant: "table", new: scaled(ck, body), says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn knows_the_table() {
        assert_eq!(test_moves(&IntElementary, "integrate sin x"), vec!["-cos(x)"]);
        assert_eq!(test_moves(&IntElementary, "integrate 3e^x"), vec!["3e^x"]);
        assert_eq!(test_moves(&IntElementary, "integrate sec^2 x"), vec!["tan(x)"]);
        assert_eq!(test_moves(&IntElementary, "integrate 1/(1 + x^2)"), vec!["arctan(x)"]);
        assert_eq!(test_moves(&IntElementary, "integrate 2^x"), vec!["2^x/ln(2)"]);
        assert!(test_moves(&IntElementary, "integrate sin(2x)").is_empty());
    }
}
