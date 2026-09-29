//! Substitution, the chain rule backwards: int f(g(x)) g'(x) dx = F(g(x)).
//! Variants: the inside is linear (int cos(3x) dx = sin(3x)/3), or the
//! derivative of the inside is there up to a constant factor
//! (int x(x^2 + 1)^3 dx, with u = x^2 + 1 and du = 2x dx).

use super::int_elementary::table;
use super::int_linear::{scaled, split_const};
use super::int_power::{log_abs, power_form, reverse_power};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Func, Math};
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct IntSubstitution;

/// c * int h du, when h is a power of u or in the table.
pub fn integrate_in(c: Q, h: &Expr, u: &Expr) -> Option<Expr> {
    if let Some(n) = power_form(h, u) {
        return if n == Q::int(-1) { Some(scaled(c, log_abs(u))) } else { reverse_power(c, u, n) };
    }
    let (k, body) = table(h, u)?;
    Some(scaled(c.mul(&k)?, body))
}

/// int h dx straight from the table or the power rule, the inside allowed
/// to be linear: e^(2x) -> e^(2x)/2. For integration by parts, which needs v.
pub fn antiderivative(h: &Expr, v: &str) -> Option<Expr> {
    let x = expr::var(v);
    if h.is_num(1) {
        return Some(x);
    }
    integrate_in(Q::ONE, h, &x).or_else(|| insides(h, v).into_iter().find_map(|u| integrate_in(is_linear(&u, v)?.recip()?, h, &u)))
}

/// g', for the insides this rule knows: polynomials, sin, cos, e^x, ln x.
pub fn derivative(u: &Expr, v: &str) -> Option<Expr> {
    let x = expr::var(v);
    if let Some(p) = poly::from_expr(u, v) {
        let d: Option<Vec<Q>> = p.0.iter().enumerate().skip(1).map(|(i, c)| c.mul(&Q::int(i as i128))).collect();
        return Some(Poly(d?).to_expr(v));
    }
    match u {
        Expr::Func(Func::Sin, a) if **a == x => Some(expr::func(Func::Cos, x)),
        Expr::Func(Func::Cos, a) if **a == x => Some(expr::neg(expr::func(Func::Sin, x))),
        Expr::Func(Func::Exp, a) if **a == x => Some(u.clone()),
        Expr::Func(Func::Ln, a) if **a == x => Some(expr::div(expr::num(1), x)),
        _ => None,
    }
}

/// k with rest = k * du, if there is one.
fn ratio(rest: &Expr, du: &Expr, v: &str) -> Option<Q> {
    if let (Some(a), Some(b)) = (poly::from_expr(rest, v), poly::from_expr(du, v)) {
        let k = a.lead().div(&b.lead())?;
        return (b.scale(&k)? == a).then_some(k);
    }
    let ((ca, ra), (cb, rb)) = (split_const(rest), split_const(du));
    (ra == rb).then(|| ca.div(&cb)).flatten()
}

/// Replace every copy of `from` in `e` by `to`.
fn swap(e: &Expr, from: &Expr, to: &Expr) -> Expr {
    let mut paths: Vec<Vec<usize>> = Vec::new();
    for (p, n) in e.walk() {
        if n == from && !paths.iter().any(|q| p.starts_with(q)) {
            paths.push(p);
        }
    }
    paths.iter().fold(e.clone(), |acc, p| acc.replace_raw(p, to.clone()))
}

fn is_linear(u: &Expr, v: &str) -> Option<Q> {
    let p = poly::from_expr(u, v)?;
    (p.deg() == Some(1) && *u != expr::var(v)).then(|| p.coef(1))
}

fn factors(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Mul(v) => v.clone(),
        x if x.is_num(1) => vec![],
        x => vec![x.clone()],
    }
}

/// The insides worth trying as u for one factor: the argument of a
/// function or the base of a power, and a function of x itself.
fn insides(f: &Expr, v: &str) -> Vec<Expr> {
    let x = expr::var(v);
    let mut out = match f {
        Expr::Func(_, a) => vec![(**a).clone()],
        Expr::Pow(b, n) if n.as_num().is_some() => vec![(**b).clone()],
        Expr::Pow(b, n) if b.as_num().is_some() => vec![(**n).clone()],
        _ => vec![],
    };
    if matches!(f, Expr::Func(..)) {
        out.push(f.clone());
    }
    out.retain(|u| *u != x && u.has_var(v));
    out
}

impl Rule for IntSubstitution {
    fn name(&self) -> &'static str {
        "int_substitution"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["linear_inside", "derivative_present"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_substitution", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let (c, h) = split_const(inner);
            let big_u = expr::var(if v == "u" { "w" } else { "u" });
            let du_name = if v == "u" { "dw" } else { "du" };
            let mut out = Vec::new();
            // the inside is linear: u = ax + b, du = a dx
            let whole = match &h {
                Expr::Div(one, d) if one.is_num(1) => {
                    let mut o = vec![(**d).clone()];
                    o.extend(insides(d, v));
                    o
                }
                Expr::Mul(fs) if fs.len() == 2 => insides(&fs[0], v),
                f => insides(f, v),
            };
            for u in whole {
                let Some(a) = is_linear(&u, v) else { continue };
                let Some(new) = c.div(&a).and_then(|k| integrate_in(k, &h, &u)) else { continue };
                let in_u = swap(&h, &u, &big_u);
                let Some(res_u) = integrate_in(Q::ONE, &in_u, &big_u) else { continue };
                let factor = match a {
                    a if a.is_one() => String::new(),
                    a if a == Q::int(-1) => "-".to_string(),
                    a if a.is_int() => format!("{a} "),
                    a => format!("({a}) "),
                };
                let says = Line::new().t("Substitute u = ").e(&u).t(format!(", so {du_name} = {factor}d{v}."));
                let mut work = Line::new().e(&Expr::Integral(Box::new(in_u), big_u_name(&big_u))).t(" = ").e(&res_u);
                if !a.is_one() {
                    work = work.t(", then divide by ").e(&Expr::Num(a));
                }
                out.push(Rewrite { variant: "linear_inside", new, says, work: vec![work] });
                break;
            }
            // the derivative of the inside is there: rest = k * du
            let (num, den) = match &h {
                Expr::Div(n, d) => (factors(n), factors(d)),
                f => (factors(f), vec![]),
            };
            let mut tries: Vec<(Expr, Expr, Vec<Expr>, Vec<Expr>)> = Vec::new(); // (u, outer, rest top, rest bottom)
            for (i, f) in num.iter().enumerate() {
                let others: Vec<Expr> = num.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, g)| g.clone()).collect();
                for u in insides(f, v) {
                    tries.push((u, f.clone(), others.clone(), den.clone()));
                }
            }
            for (i, d) in den.iter().enumerate() {
                let others: Vec<Expr> = den.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, g)| g.clone()).collect();
                let mut us = vec![d.clone()];
                us.extend(insides(d, v).into_iter().filter(|u| u != d));
                us.retain(|u| *u != expr::var(v));
                for u in us {
                    tries.push((u, expr::div(expr::num(1), d.clone()), num.clone(), others.clone()));
                }
            }
            for (u, outer, top, bottom) in tries {
                if is_linear(&u, v).is_some() {
                    continue;
                }
                let rest = if bottom.is_empty() { expr::mul(top) } else { expr::div(expr::mul(top), expr::mul(bottom)) };
                let Some(du) = derivative(&u, v) else { continue };
                let Some(k) = ratio(&rest, &du, v) else { continue };
                let Some(ck) = c.mul(&k) else { continue };
                let Some(new) = integrate_in(ck, &outer, &u) else { continue };
                let in_u = swap(&outer, &u, &big_u);
                let Some(res_u) = integrate_in(ck, &in_u, &big_u) else { continue };
                let there = if k.is_one() { ", and it is already in the integral." } else { ", which is in the integral up to a constant factor." };
                let says = Line::new().t("Substitute u = ").e(&u).t(format!(": {du_name} = ")).e(&du).t(format!(" d{v}{there}"));
                let work = Line::new().e(&with_coeff(ck, Expr::Integral(Box::new(in_u), big_u_name(&big_u)))).t(" = ").e(&res_u);
                out.push(Rewrite { variant: "derivative_present", new, says, work: vec![work] });
            }
            out
        })
    }
}

fn big_u_name(u: &Expr) -> String {
    match u {
        Expr::Var(n) => n.clone(),
        _ => "u".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn substitutes() {
        assert_eq!(test_moves(&IntSubstitution, "integrate cos(3x)"), vec!["sin(3x)/3"]);
        assert_eq!(test_moves(&IntSubstitution, "integrate x(x^2 + 1)^3"), vec!["(x^2 + 1)^4/8"]);
        assert_eq!(test_moves(&IntSubstitution, "integrate 2x/(x^2 + 1)"), vec!["ln|x^2 + 1|"]);
        assert!(test_moves(&IntSubstitution, "integrate x^2").is_empty());
    }
}
