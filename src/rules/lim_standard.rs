//! The standard limits at 0: sin(t)/t -> 1, tan(t)/t -> 1, (e^t - 1)/t -> 1,
//! (1 - cos(t))/t -> 0. With t = ax over bx the limit is scaled: sin(3x)/(2x) -> 3/2.

use super::int_linear::split_const;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Math};
use crate::poly;
use crate::q::Q;

pub struct LimStandard;

/// a when e = ax (no constant term).
fn through_zero(e: &Expr, v: &str) -> Option<Q> {
    let p = poly::from_expr(e, v)?;
    (p.deg() == Some(1) && p.coef(0).is_zero()).then(|| p.coef(1))
}

/// (variant, a, the standard limit of the form, its value) for a top with
/// inside ax: sin(ax), tan(ax), e^(ax) - 1, 1 - cos(ax).
fn form(top: &Expr, v: &str) -> Option<(&'static str, Q, Expr, Q)> {
    let t = expr::var("t");
    let over_t = |e: Expr| expr::div(e, t.clone());
    match top {
        Expr::Func(f @ (Func::Sin | Func::Tan), a) => Some(("sine", through_zero(a, v)?, over_t(expr::func(*f, t.clone())), Q::ONE)),
        Expr::Add(ts) if ts.len() == 2 && ts[1].is_num(-1) => match &ts[0] {
            Expr::Func(Func::Exp, a) => Some(("exponential", through_zero(a, v)?, over_t(expr::add(vec![expr::func(Func::Exp, t.clone()), expr::num(-1)])), Q::ONE)),
            _ => None,
        },
        Expr::Add(ts) if ts.len() == 2 && ts[0].is_num(1) => match &ts[1] {
            Expr::Neg(c) => match &**c {
                Expr::Func(Func::Cos, a) => Some(("cosine", through_zero(a, v)?, over_t(expr::add(vec![expr::num(1), expr::neg(expr::func(Func::Cos, t.clone()))])), Q::ZERO)),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

impl Rule for LimStandard {
    fn name(&self) -> &'static str {
        "lim_standard"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["sine", "exponential", "cosine"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("lim_standard", m, |e, _| {
            let Expr::Limit(f, v, p) = e else { return vec![] };
            if !p.is_num(0) {
                return vec![];
            }
            let (c, h) = split_const(f);
            let Expr::Div(top, bottom) = &h else { return vec![] };
            let Some(b) = through_zero(bottom, v) else { return vec![] };
            let Some((variant, a, shape, value)) = form(top, v) else { return vec![] };
            // c * top/(bx) = c(a/b) * top/(ax) -> c(a/b) * value
            let Some(k) = c.mul(&a).and_then(|x| x.div(&b)) else { return vec![] };
            let Some(new) = k.mul(&value) else { return vec![] };
            let says = Line::new().t("Standard limit: ").e(&shape).t(" -> ").e(&Expr::Num(value)).t(" as t -> 0.");
            let mut work = Vec::new();
            if !k.is_one() {
                let t = expr::with_coeff(a, expr::var(v));
                work.push(Line::new().t("with t = ").e(&t).t(": ").e(f).t(" = ").e(&Expr::Num(k)).t(" * ").e(&expr::div((**top).clone(), t)).t(" -> ").e(&Expr::Num(k)).t(" * ").e(&Expr::Num(value)));
            }
            vec![Rewrite { variant, new: Expr::Num(new), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn standard_limits() {
        assert_eq!(test_moves(&LimStandard, "limit of sin x / x as x approaches 0"), vec!["1"]);
        assert_eq!(test_moves(&LimStandard, "limit of sin(3x)/(2x) as x approaches 0"), vec!["3/2"]);
        assert_eq!(test_moves(&LimStandard, "limit of (1 - cos x)/x as x approaches 0"), vec!["0"]);
        assert!(test_moves(&LimStandard, "limit of sin x / x as x approaches 1").is_empty());
    }
}
