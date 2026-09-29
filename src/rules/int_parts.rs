//! Integration by parts: int u dv = uv - int v du. The choice of u follows
//! LIATE (logarithm, inverse trig, algebraic, trig, exponential): u is the
//! factor that gets simpler when differentiated. Variants: two factors
//! (int x e^x dx), or one factor times 1 (int ln(x) dx, with dv = dx).

use super::int_linear::{scaled, split_const};
use super::int_substitution::{antiderivative, derivative};
use super::powers::{base_exp, power_of};
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, Expr, Func, Math};
use crate::poly;
use crate::q::Q;

pub struct IntParts;

/// LIATE rank of a factor, lower is a better u; None if the rule doesn't
/// know how to treat it.
fn liate(f: &Expr, v: &str) -> Option<u8> {
    let x = expr::var(v);
    let linear = |a: &Expr| poly::from_expr(a, v).is_some_and(|p| p.deg() == Some(1));
    match f {
        Expr::Func(Func::Ln, a) if **a == x => Some(0),
        Expr::Func(Func::Asin | Func::Acos | Func::Atan, a) if **a == x => Some(1),
        _ if poly::from_expr(f, v).is_some_and(|p| p.deg().unwrap_or(0) >= 1) => Some(2),
        Expr::Func(Func::Sin | Func::Cos, a) if linear(a) => Some(3),
        Expr::Func(Func::Exp, a) if linear(a) => Some(4),
        Expr::Pow(b, a) if b.as_num().is_some() && linear(a) => Some(4),
        _ => None,
    }
}

/// du for the u's LIATE picks.
fn du(u: &Expr, v: &str) -> Option<Expr> {
    let x = expr::var(v);
    let one_minus_sq = || expr::sqrt(expr::add(vec![expr::num(1), expr::neg(expr::pow(x.clone(), expr::num(2)))]));
    match u {
        Expr::Func(Func::Atan, _) => Some(expr::div(expr::num(1), expr::add(vec![expr::num(1), expr::pow(x.clone(), expr::num(2))]))),
        Expr::Func(Func::Asin, _) => Some(expr::div(expr::num(1), one_minus_sq())),
        Expr::Func(Func::Acos, _) => Some(expr::neg(expr::div(expr::num(1), one_minus_sq()))),
        _ => derivative(u, v),
    }
}

/// v du, multiplied the way a person would: e^x * 2x -> 2x e^x,
/// x^2/2 * 1/x -> x/2.
fn times(v_: &Expr, du: &Expr, x: &str) -> Expr {
    let (kv, body) = split_const(v_);
    if let Expr::Div(one, d) = du {
        if one.is_num(1) {
            // a power of x over x: lower the power
            if let (Some((b, n)), Expr::Var(_)) = (base_exp(&body), &**d) {
                if b == **d && b == expr::var(x) {
                    if let Some(m) = n.sub(&Q::ONE) {
                        return if m.is_zero() { Expr::Num(kv) } else { scaled(kv, power_of(b, m)) };
                    }
                }
            }
            if body == **d {
                return Expr::Num(kv);
            }
            return scaled(kv, expr::div(body, (**d).clone()));
        }
    }
    let (kd, rd) = coeff(du);
    let k = kv.mul(&kd).unwrap_or(Q::ONE);
    if rd.is_num(1) {
        return scaled(k, body);
    }
    scaled(k, expr::mul(vec![rd, body]))
}

impl Rule for IntParts {
    fn name(&self) -> &'static str {
        "int_parts"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["liate", "times_one"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("int_parts", m, |e, _| {
            let Expr::Integral(inner, v) = e else { return vec![] };
            let (c, h) = split_const(inner);
            if !c.is_one() {
                return vec![];
            }
            // (u, dv, variant)
            let (u, dv, variant) = match &h {
                Expr::Mul(fs) if fs.len() == 2 => {
                    let (Some(a), Some(b)) = (liate(&fs[0], v), liate(&fs[1], v)) else { return vec![] };
                    if a == b {
                        return vec![];
                    }
                    let (u, dv) = if a < b { (&fs[0], &fs[1]) } else { (&fs[1], &fs[0]) };
                    (u.clone(), dv.clone(), "liate")
                }
                f if liate(f, v).is_some_and(|r| r <= 1) => (f.clone(), expr::num(1), "times_one"),
                _ => return vec![],
            };
            let (Some(d), Some(vv)) = (du(&u, v), antiderivative(&dv, v)) else { return vec![] };
            // the new integral must be one the rules can go on with
            let prod = times(&vv, &d, v);
            let uv = if poly::from_expr(&u, v).is_some() { times_plain(&u, &vv) } else { times_plain(&vv, &u) };
            let (cp, bp) = split_const(&prod);
            let second = if cp.is_neg() { Expr::Integral(Box::new(scaled(cp.neg(), bp)), v.clone()) } else { expr::neg(Expr::Integral(Box::new(prod), v.clone())) };
            let new = expr::add(vec![uv, second]);
            let says = Line::new().t("Integrate by parts: ").e(&Expr::Integral(Box::new(expr::var("u")), "v".into())).t(" = uv - ").e(&Expr::Integral(Box::new(expr::var("v")), "u".into())).t(".");
            let d_line = if d.is_num(1) { Line::new().t(format!("du = d{v}")) } else { Line::new().t("du = ").e(&d).t(format!(" d{v}")) };
            let dv_line = if dv.is_num(1) { format!("dv = d{v}") } else { String::new() };
            let mut choose = Line::new().t("u = ").e(&u).t(", ");
            choose = if dv_line.is_empty() { choose.t("dv = ").e(&dv).t(format!(" d{v}")) } else { choose.t(dv_line) };
            let why = if variant == "liate" { " (LIATE)" } else { " (dv = 1 dx)" };
            let work = vec![choose.t(why), d_line.t(", v = ").e(&vv)];
            vec![Rewrite { variant, new, says, work }]
        })
    }
}

/// A product of two factors, numbers pulled to the front: x * (-cos(x)) -> -x cos(x).
fn times_plain(a: &Expr, b: &Expr) -> Expr {
    let (ka, ra) = split_const(a);
    let (kb, rb) = split_const(b);
    let k = ka.mul(&kb).unwrap_or(Q::ONE);
    let fs: Vec<Expr> = [ra, rb].into_iter().filter(|f| !f.is_num(1)).collect();
    scaled(k, expr::mul(fs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn by_parts() {
        assert_eq!(test_moves(&IntParts, "integrate x e^x"), vec!["x e^x - int e^x dx"]);
        assert_eq!(test_moves(&IntParts, "integrate x sin x"), vec!["-x cos(x) + int cos(x) dx"]);
        assert_eq!(test_moves(&IntParts, "integrate ln x"), vec!["x ln(x) - int 1 dx"]);
        assert!(test_moves(&IntParts, "integrate x^2").is_empty());
    }
}
