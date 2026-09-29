//! Solve sin(u) = c, cos(u) = c, tan(u) = c for an angle u = ax + b. The
//! table (or arcsin, arccos, arctan) gives one angle; the unit circle gives
//! the rest: sin u = c at a and at pi - a, cos u = c at a and -a, both
//! repeating every 2pi; tan u = c at a, repeating every pi. Variants: the
//! general solution with k, every solution in an interval, or none at all
//! (sin and cos never leave [-1, 1]).

use super::trig_exact::{angle, exact, turns};
use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, Expr, Func, Konst, Math};
use crate::poly;
use crate::q::Q;

pub struct TrigSolve;

/// The letter of the general solution: k, any whole number.
pub const K: &str = "k";

/// r + s k turns: pi/6 + 2k pi, 30 deg + 360k deg.
fn family(r: Q, s: Q, degrees: bool) -> Expr {
    let k = expr::var(K);
    let step = if degrees {
        expr::mul(vec![Expr::Num(s.mul(&Q::int(180)).unwrap_or(s)), k, Expr::Const(Konst::Deg)])
    } else {
        let top = expr::mul(vec![Expr::Num(Q::int(s.num())), k, Expr::Const(Konst::Pi)]);
        let top = if s.num() == 1 { expr::mul(vec![expr::var(K), Expr::Const(Konst::Pi)]) } else { top };
        if s.den() == 1 {
            top
        } else {
            expr::div(top, expr::num(s.den()))
        }
    };
    if r.is_zero() {
        step
    } else {
        expr::add(vec![angle(r, degrees), step])
    }
}

/// x mod m, in [0, m)
fn modulo(x: Q, m: Q) -> Option<Q> {
    let k = x.div(&m)?;
    let fl = Q::int(k.num().div_euclid(k.den()));
    x.sub(&fl.mul(&m)?)
}

/// The angles in the table where f takes the value c: (principal angles
/// in turns, period in turns).
fn table_angles(f: Func, c: f64, zero: f64) -> Option<(Vec<Q>, Q)> {
    let candidates: Vec<Q> = (-12..=12).filter_map(|n| Q::new(n, 12)).chain((-8..=8).filter_map(|n| Q::new(n, 8))).collect();
    let hits = |lo: Q, hi: Q| -> Option<Q> {
        let mut best: Option<Q> = None;
        for t in candidates.iter().filter(|t| **t >= lo && **t <= hi) {
            let Some((v, _)) = exact(f, *t) else { continue };
            if (v.eval_f(&|_| f64::NAN) - c).abs() < zero && best.is_none_or(|b| t.abs() < b.abs()) {
                best = Some(*t);
            }
        }
        best
    };
    let half = Q::new(1, 2)?;
    match f {
        Func::Sin => {
            let a = hits(half.neg(), half)?;
            let b = Q::ONE.sub(&a)?;
            Some((vec![a, b], Q::int(2)))
        }
        Func::Cos => {
            let a = hits(Q::ZERO, Q::ONE)?;
            Some((vec![a, a.neg()], Q::int(2)))
        }
        Func::Tan => Some((vec![hits(half.neg(), half)?], Q::ONE)),
        _ => None,
    }
}

impl Rule for TrigSolve {
    fn name(&self) -> &'static str {
        "trig_solve"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["general", "interval", "impossible", "inverse"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let degrees = cx.req.calc.degrees.is_some();
        let zero = cx.cfg.calculus.zero;
        per_eq("trig_solve", m, |l, r| {
            let Expr::Func(f @ (Func::Sin | Func::Cos | Func::Tan), u) = l else { return vec![] };
            if r.has_var(v) || !u.has_var(v) {
                return vec![];
            }
            let c = r.eval_f(&|_| f64::NAN);
            if !c.is_finite() {
                return vec![];
            }
            let fx = expr::func(*f, (**u).clone());
            if *f != Func::Tan && c.abs() > 1.0 + zero {
                let says = Line::new().e(&fx).t(" is always between -1 and 1, so it never equals ").e(r).t(".");
                return vec![EqRewrite { variant: "impossible", to: Branch::Nothing, says, work: vec![] }];
            }
            // u = ax + b, with b a multiple of pi
            let with_letter = expr::add(expr::terms(u).into_iter().filter(|t| t.has_var(v)).collect());
            let Some(p) = poly::from_expr(&with_letter, v).filter(|p| p.deg() == Some(1) && p.coef(0).is_zero()) else { return vec![] };
            let a = p.coef(1);
            let Some(b) = turns(&turns_free(u, v)) else { return vec![] };
            let Some((angles, period)) = table_angles(*f, c, zero) else {
                return inverse(*f, &fx, r, u, v, degrees || cx.req.calc.bounds.is_some()).into_iter().collect();
            };
            let mut angles = angles;
            angles.dedup_by(|x, y| modulo(x.sub(y).unwrap_or(Q::ONE), period).is_some_and(|d| d.is_zero()));
            // x = (angle - b)/a + (period/a) k, each written in [0, |period/a|)
            let Some(step) = period.div(&a) else { return vec![] };
            let mut fams: Vec<Q> = Vec::new();
            for t in &angles {
                let Some(x0) = t.sub(&b).and_then(|d| d.div(&a)) else { return vec![] };
                let Some(x0) = modulo(x0, step.abs()) else { return vec![] };
                if !fams.contains(&x0) {
                    fams.push(x0);
                }
            }
            fams.sort();
            // a list of expressions joined by words, printed in the output's style
            let listed = |es: Vec<Expr>, joint: &str| {
                es.iter().enumerate().fold(Line::new(), |l, (i, e)| if i == 0 { l.e(e) } else { l.t(joint).e(e) })
            };
            let at = listed(angles.iter().map(|t| angle(*t, degrees)).collect(), " and ");
            let mut work = vec![Line::new().e(&fx).t(" = ").e(r).t(" at ").e(u).t(" = ").join(at)];
            let x = expr::var(v);
            match &cx.req.calc.bounds {
                None => {
                    if !a.is_one() || !b.is_zero() {
                        let per = listed(angles.iter().map(|t| family(*t, period, degrees)).collect(), " or ");
                        let mut line = Line::new().e(u).t(" = ").join(per).t("; ");
                        if !b.is_zero() {
                            line = line.t("subtract ").e(&angle(b, degrees));
                        }
                        if !a.is_one() {
                            line = line.t(if b.is_zero() { "divide by " } else { ", then divide by " }).e(&Expr::Num(a));
                        }
                        work.push(line);
                    }
                    let sols: Vec<(Expr, Expr)> = fams.iter().map(|x0| (x.clone(), family(*x0, step.abs(), degrees))).collect();
                    let says = Line::new().t(format!("Read the angles off the unit circle; {} repeats every ", f.name())).e(&angle(period, degrees)).t(", so add ").e(&family(Q::ZERO, period, degrees)).t(" for any whole number k.");
                    let to = if sols.len() == 1 { Branch::One(sols[0].0.clone(), sols[0].1.clone()) } else { Branch::Many(sols) };
                    vec![EqRewrite { variant: "general", to, says, work }]
                }
                Some(bounds) => {
                    let (Some(lo), Some(hi)) = (turns(&bounds.value.0), turns(&bounds.value.1)) else { return vec![] };
                    let mut xs: Vec<Q> = Vec::new();
                    for x0 in &fams {
                        // x0 + n * |step| in [lo, hi]
                        let s = step.abs();
                        let Some(first) = lo.sub(x0).and_then(|d| d.div(&s)) else { return vec![] };
                        let n0 = first.num().div_euclid(first.den()) - 1;
                        for n in n0..n0 + 1000 {
                            let Some(xn) = s.mul(&Q::int(n)).and_then(|d| d.add(x0)) else { return vec![] };
                            if xn > hi {
                                break;
                            }
                            if xn >= lo && !xs.contains(&xn) {
                                xs.push(xn);
                            }
                        }
                    }
                    xs.sort();
                    let says = Line::new().t("Read the angles off the unit circle and keep the ones from ").e(&bounds.value.0).t(" to ").e(&bounds.value.1).t(".");
                    let to = match xs.len() {
                        0 => Branch::Nothing,
                        1 => Branch::One(x.clone(), angle(xs[0], degrees)),
                        _ => Branch::Many(xs.iter().map(|t| (x.clone(), angle(*t, degrees))).collect()),
                    };
                    vec![EqRewrite { variant: "interval", to, says, work }]
                }
            }
        })
    }
}

/// A value off the table (sin x = 3/10): the inverse function gives the
/// first angle, the unit circle the rest. General solution of f(x) = c
/// only (not in degrees or an interval, which would need the angle's value).
fn inverse(f: Func, fx: &Expr, c: &Expr, u: &Expr, v: &str, skip: bool) -> Option<EqRewrite> {
    if skip || *u != expr::var(v) {
        return None;
    }
    let inv = |g: Func| expr::func(g, c.clone());
    let pi = Expr::Const(Konst::Pi);
    let (first, second, step) = match f {
        Func::Sin => (inv(Func::Asin), Some(expr::add(vec![pi.clone(), expr::neg(inv(Func::Asin))])), Q::int(2)),
        Func::Cos => (inv(Func::Acos), Some(expr::add(vec![expr::mul(vec![expr::num(2), pi.clone()]), expr::neg(inv(Func::Acos))])), Q::int(2)),
        Func::Tan => (inv(Func::Atan), None, Q::ONE),
        _ => return None,
    };
    let x = expr::var(v);
    let fam = |a: Expr| expr::add(vec![a, family(Q::ZERO, step, false)]);
    let mut sols = vec![(x.clone(), fam(first.clone()))];
    if let Some(s) = &second {
        sols.push((x.clone(), fam(s.clone())));
    }
    let name = match f {
        Func::Sin => "arcsin",
        Func::Cos => "arccos",
        _ => "arctan",
    };
    let says = Line::new().e(c).t(format!(" is not in the table: {name} gives one angle, the unit circle the rest; add ")).e(&family(Q::ZERO, step, false)).t(" for any whole number k.");
    let mut work = Line::new().e(fx).t(" = ").e(c).t(" at ").e(&x).t(" = ").e(&first);
    if let Some(s) = &second {
        work = work.t(" and ").e(s);
    }
    let to = if sols.len() == 1 { Branch::One(sols[0].0.clone(), sols[0].1.clone()) } else { Branch::Many(sols) };
    Some(EqRewrite { variant: "inverse", to, says, work: vec![work] })
}

/// The part of u without the letter: b in ax + b.
fn turns_free(u: &Expr, v: &str) -> Expr {
    expr::add(expr::terms(u).into_iter().filter(|t| !t.has_var(v)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn solves_trig_equations() {
        assert_eq!(test_moves(&TrigSolve, "solve sin x = 1/2"), vec!["x = pi/6 + 2k pi or x = 5pi/6 + 2k pi"]);
        assert_eq!(test_moves(&TrigSolve, "solve cos x = 1/2 for x between 0 and 2pi"), vec!["x = pi/3 or x = 5pi/3"]);
        assert_eq!(test_moves(&TrigSolve, "solve tan(2x) = 1"), vec!["x = pi/8 + k pi/2"]);
        assert_eq!(test_moves(&TrigSolve, "solve sin x = 2"), vec!["no real solution"]);
        assert_eq!(test_moves(&TrigSolve, "solve tan x = 2"), vec!["x = arctan(2) + k pi"]);
        assert!(test_moves(&TrigSolve, "solve 2 sin x = 1").is_empty());
    }
}
