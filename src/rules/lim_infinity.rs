//! Limits of rational functions at infinity. Divide top and bottom by the
//! highest power in the bottom; then every c/x^k goes to 0. Variants:
//! divide by the highest power, let the c/x^k vanish, read the answer off
//! the leading terms in one go, or see that the top wins (degree of the top
//! higher: the fraction grows without bound).

use super::lim_substitute::infinite;
use super::powers::power_of;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Konst, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct LimInfinity;

/// c x^k, with k < 0 written as c/x^(-k).
fn term(c: Q, v: &str, k: i64) -> Expr {
    let x = expr::var(v);
    match k {
        0 => Expr::Num(c),
        k if k > 0 => with_coeff(c, power_of(x, Q::int(k as i128))),
        k => {
            let q = expr::div(expr::num(c.num().abs()), with_coeff(Q::int(c.den()), power_of(x, Q::int(-k as i128))));
            if c.is_neg() {
                expr::neg(q)
            } else {
                q
            }
        }
    }
}

/// A polynomial divided by x^m, term by term, highest power first.
fn divided(p: &Poly, v: &str, m: usize) -> Expr {
    let ts: Vec<Expr> = (0..p.0.len()).rev().filter(|&i| !p.coef(i).is_zero()).map(|i| term(p.coef(i), v, i as i64 - m as i64)).collect();
    expr::add(ts)
}

/// Is `d` a positive power of the letter, maybe times a number: x, x^2, 2x^3?
fn power_of_letter(d: &Expr, v: &str) -> bool {
    let x = expr::var(v);
    let (_, r) = expr::coeff(d);
    r == x || matches!(&r, Expr::Pow(b, k) if **b == x && k.as_num().is_some_and(|k| k.is_int() && !k.is_neg()))
}

/// Does h grow without bound as the letter heads for infinity (sign s)?
/// Read off its values out at the configured far points.
pub fn grows(h: &Expr, v: &str, s: f64, cfg: &crate::config::Config) -> bool {
    let far = &cfg.calculus.limit_far;
    let at = |x: f64| h.eval_f(&|n| if n == v { x * s } else { f64::NAN }).abs();
    let (a, b) = (at(far[0]), at(far[far.len() - 1]));
    b > cfg.calculus.unbounded && (b > a || b.is_infinite())
}

impl Rule for LimInfinity {
    fn name(&self) -> &'static str {
        "lim_infinity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["divide_highest", "vanish", "leading_terms", "unbounded"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        local("lim_infinity", m, |e, _| {
            let Expr::Limit(f, v, p) = e else { return vec![] };
            let Some(sign) = infinite(p) else { return vec![] };
            if f.is_pending() {
                return vec![];
            }
            let mut out = Vec::new();
            // every c/x^k -> 0
            let mut paths: Vec<Vec<usize>> = Vec::new();
            for (path, n) in f.walk() {
                if let Expr::Div(a, d) = n {
                    if !a.has_var(v) && (power_of_letter(d, v) || grows(d, v, sign, cx.cfg)) && !paths.iter().any(|q| path.starts_with(q)) {
                        paths.push(path);
                    }
                }
            }
            if !paths.is_empty() {
                let new = expr::tidy(paths.iter().fold((**f).clone(), |acc, q| acc.replace_raw(q, expr::num(0))));
                if !new.has_var(v) {
                    let says = Line::new().t(format!("As {v} -> ")).e(p).t(", a number over something that grows without bound goes to 0.");
                    out.push(Rewrite { variant: "vanish", new, says, work: vec![] });
                    return out;
                }
            }
            let Some((n, d)) = poly::rational_from_expr(f, v) else { return out };
            let (Some(dn), Some(dd)) = (n.deg(), d.deg()) else { return out };
            let (a, b) = (n.lead(), d.lead());
            let Some(ratio) = a.div(&b) else { return out };
            let lead = expr::div(term(a, v, dn as i64), term(b, v, dd as i64));
            if dn > dd {
                // the top wins: like (a/b) x^(n-m)
                let up = (if ratio.is_neg() { -1.0 } else { 1.0 }) * if (dn - dd) % 2 == 1 { sign } else { 1.0 };
                let new = if up > 0.0 { Expr::Const(Konst::Inf) } else { expr::neg(Expr::Const(Konst::Inf)) };
                let says = Line::new().t("The top has the higher degree, so the fraction grows without bound, like ").e(&lead).t(".");
                out.push(Rewrite { variant: "unbounded", new, says, work: vec![] });
                return out;
            }
            // divide top and bottom by x^m
            if dd >= 1 {
                let new_f = expr::div(divided(&n, v, dd), divided(&d, v, dd));
                if new_f != **f {
                    let says = Line::new().t("Divide the top and the bottom by ").e(&power_of(expr::var(v), Q::int(dd as i128))).t(", the highest power in the bottom.");
                    out.push(Rewrite { variant: "divide_highest", new: Expr::Limit(Box::new(new_f), v.clone(), p.clone()), says, work: vec![] });
                }
            }
            let value = if dn == dd { ratio } else { Q::ZERO };
            let says = Line::new().t(format!("Only the leading terms matter as {v} -> ")).e(p).t(": ").e(&lead).t(" -> ").e(&Expr::Num(value)).t(".");
            out.push(Rewrite { variant: "leading_terms", new: Expr::Num(value), says, work: vec![] });
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn limits_at_infinity() {
        assert_eq!(test_moves(&LimInfinity, "limit of (3x^2 + 1)/(x^2 - 5) as x approaches infinity"), vec!["lim(x->infinity) (3 + 1/x^2)/(1 - 5/x^2)", "3"]);
        assert_eq!(test_moves(&LimInfinity, "limit of 1/x as x approaches infinity"), vec!["0"]);
        assert_eq!(test_moves(&LimInfinity, "limit of x^3/(x + 1) as x approaches -infinity"), vec!["infinity"]);
        assert!(test_moves(&LimInfinity, "limit of 1/x as x approaches 0").is_empty());
    }
}
