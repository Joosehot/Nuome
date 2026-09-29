//! Trig identities that simplify: sin^2 u + cos^2 u = 1 (and 1 - sin^2 u =
//! cos^2 u, 1 + tan^2 u = sec^2 u), the double angle 2 sin u cos u =
//! sin(2u) and cos^2 u - sin^2 u = cos(2u), and sin u / cos u = tan u.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, coeff, with_coeff, Expr, Func, Math};
use crate::q::Q;

pub struct TrigIdentity;

/// f(u)^2 -> (f, u)
fn square(e: &Expr) -> Option<(Func, Expr)> {
    match e {
        Expr::Pow(b, n) if n.is_num(2) => match &**b {
            Expr::Func(f @ (Func::Sin | Func::Cos | Func::Tan | Func::Sec), u) => Some((*f, (**u).clone())),
            _ => None,
        },
        _ => None,
    }
}

fn sq(f: Func, u: &Expr) -> Expr {
    expr::pow(expr::func(f, u.clone()), expr::num(2))
}

fn double(u: &Expr) -> Expr {
    expr::mul(vec![expr::num(2), u.clone()])
}

/// Replace terms i and j of a sum by `new`.
fn merge(ts: &[Expr], i: usize, j: usize, new: Expr) -> Expr {
    let mut out: Vec<Expr> = Vec::new();
    for (k, t) in ts.iter().enumerate() {
        if k == i {
            out.push(new.clone());
        } else if k != j {
            out.push(t.clone());
        }
    }
    let out: Vec<Expr> = out.into_iter().filter(|t| !t.is_num(0)).collect();
    expr::add(out)
}

impl Rule for TrigIdentity {
    fn name(&self) -> &'static str {
        "trig_identity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["pythagorean", "double_angle", "quotient"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("trig_identity", m, |e, _| {
            let mut out = Vec::new();
            match e {
                Expr::Add(ts) => {
                    let parts: Vec<(Q, Expr)> = ts.iter().map(coeff).collect();
                    for i in 0..ts.len() {
                        for j in 0..ts.len() {
                            if i == j {
                                continue;
                            }
                            let ((ci, ri), (cj, rj)) = (&parts[i], &parts[j]);
                            let (si, sj) = (square(ri), square(rj));
                            // c sin^2 u + c cos^2 u = c
                            if let (Some((Func::Sin, u)), Some((Func::Cos, w))) = (&si, &sj) {
                                if u == w && ci == cj {
                                    let says = Line::new().t("Pythagorean identity: sin^2 + cos^2 = 1.");
                                    out.push(Rewrite { variant: "pythagorean", new: merge(ts, i.min(j), i.max(j), Expr::Num(*ci)), says, work: vec![] });
                                }
                                // cos^2 u - sin^2 u = cos(2u)
                                if u == w && cj.is_one() && *ci == Q::int(-1) {
                                    let says = Line::new().t("Double angle: cos^2 u - sin^2 u = cos(2u).");
                                    out.push(Rewrite { variant: "double_angle", new: merge(ts, i.min(j), i.max(j), expr::func(Func::Cos, double(u))), says, work: vec![] });
                                }
                            }
                            // n - n sin^2 u = n cos^2 u, n - n cos^2 u = n sin^2 u, n + n tan^2 u = n sec^2 u
                            if ri.is_num(1) {
                                let n = *ci;
                                let other = match &sj {
                                    Some((Func::Sin, u)) if *cj == n.neg() => Some((sq(Func::Cos, u), "1 - sin^2 = cos^2")),
                                    Some((Func::Cos, u)) if *cj == n.neg() => Some((sq(Func::Sin, u), "1 - cos^2 = sin^2")),
                                    Some((Func::Tan, u)) if *cj == n => Some((sq(Func::Sec, u), "1 + tan^2 = sec^2")),
                                    _ => None,
                                };
                                if let Some((new, rule)) = other {
                                    let says = Line::new().t(format!("Pythagorean identity: {rule}."));
                                    out.push(Rewrite { variant: "pythagorean", new: merge(ts, i.min(j), i.max(j), with_coeff(n, new)), says, work: vec![] });
                                }
                            }
                        }
                    }
                }
                // 2 sin u cos u = sin(2u)
                Expr::Mul(_) => {
                    let (c, rest) = coeff(e);
                    let Expr::Mul(fs) = &rest else { return out };
                    if fs.len() != 2 || !c.div(&Q::int(2)).is_some_and(|h| h.is_int()) {
                        return out;
                    }
                    let arg = |g: Func| fs.iter().find_map(|f| match f {
                        Expr::Func(k, u) if *k == g => Some((**u).clone()),
                        _ => None,
                    });
                    if let (Some(u), Some(w)) = (arg(Func::Sin), arg(Func::Cos)) {
                        if u == w {
                            let half = c.div(&Q::int(2)).unwrap_or(Q::ONE);
                            let says = Line::new().t("Double angle: 2 sin u cos u = sin(2u).");
                            out.push(Rewrite { variant: "double_angle", new: with_coeff(half, expr::func(Func::Sin, double(&u))), says, work: vec![] });
                        }
                    }
                }
                // sin u / cos u = tan u, cos u / sin u = cot u
                Expr::Div(a, b) => {
                    if let (Expr::Func(f, u), Expr::Func(g, w)) = (&**a, &**b) {
                        let new = match (f, g) {
                            (Func::Sin, Func::Cos) if u == w => Some((Func::Tan, "sin u / cos u = tan u")),
                            (Func::Cos, Func::Sin) if u == w => Some((Func::Cot, "cos u / sin u = cot u")),
                            _ => None,
                        };
                        if let Some((h, rule)) = new {
                            out.push(Rewrite { variant: "quotient", new: expr::func(h, (**u).clone()), says: Line::new().t(format!("Quotient identity: {rule}.")), work: vec![] });
                        }
                    }
                }
                _ => {}
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn identities() {
        assert_eq!(test_moves(&TrigIdentity, "simplify sin^2 x + cos^2 x"), vec!["1"]);
        assert_eq!(test_moves(&TrigIdentity, "simplify 1 - cos^2 x"), vec!["sin(x)^2"]);
        assert_eq!(test_moves(&TrigIdentity, "simplify 2 sin x cos x"), vec!["sin(2x)"]);
        assert_eq!(test_moves(&TrigIdentity, "simplify sin x / cos x"), vec!["tan(x)"]);
        assert!(test_moves(&TrigIdentity, "simplify sin x + cos x").is_empty());
    }
}
