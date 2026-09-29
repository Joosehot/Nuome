//! Prove it before printing it. Every finished path is checked against the
//! original problem, independently of the rules that produced it:
//!
//! - every step keeps the value (expressions compared at sample points)
//! - evaluate: the number equals the problem computed directly, exactly
//! - solve: every answer satisfies the original equation, and no solution
//!   is missing: the real roots are counted from the equation's own
//!   polynomial (rational root theorem + discriminant), not from the steps
//! - factor: the product multiplies back out, and no factor factors further
//! - differentiate: the result matches a numerical derivative
//!
//! A path that fails a check is never shown; the next finalist is tried.

use crate::config::Config;
use crate::expr::{self, Expr, Math};
use crate::model::{Request, Task};
use crate::poly;
use crate::print::{self, Style};
use crate::q::{self, Q};
use crate::search::Path;

#[derive(Clone, Debug)]
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

fn ck(name: &'static str, ok: bool, detail: impl Into<String>) -> Check {
    Check { name, ok, detail: detail.into() }
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * 1f64.max(a.abs()).max(b.abs())
}

/// Letter values for sample `k`: given values stay fixed, the rest vary.
fn env<'a>(req: &'a Request, cfg: &'a Config, k: usize) -> impl Fn(&str) -> f64 + 'a {
    move |v: &str| {
        if let Some(g) = req.given.iter().find(|g| g.value.0 == v) {
            return g.value.1.eval_f(&|_| f64::NAN);
        }
        let i = v.bytes().next().map_or(0, |b| (b % 7) as usize);
        cfg.check.samples[k] + 0.31 * i as f64
    }
}

/// Do two expressions agree at the sample points? Ok(points compared).
fn agree(a: &Expr, b: &Expr, req: &Request, cfg: &Config) -> Result<usize, String> {
    // calculus and trig (agent B): when too few samples lie in the domain
    // (arcsin, sqrt(1 - x^2)), try them again scaled towards 0
    match agree_scaled(a, b, req, cfg, 1.0) {
        Err(e) if e.starts_with("undefined") => agree_scaled(a, b, req, cfg, cfg.calculus.narrow),
        r => r,
    }
}

fn agree_scaled(a: &Expr, b: &Expr, req: &Request, cfg: &Config, scale: f64) -> Result<usize, String> {
    let mut n = 0;
    for k in 0..cfg.check.samples.len() {
        let base = env(req, cfg, k);
        let e = |v: &str| if req.given.iter().any(|g| g.value.0 == v) { base(v) } else { base(v) * scale };
        let (x, y) = (a.eval_f(&e), b.eval_f(&e));
        if !x.is_finite() || !y.is_finite() {
            continue;
        }
        if !close(x, y, cfg.check.tolerance) {
            let at: Vec<String> = a.vars().union(&b.vars()).map(|v| format!("{v} = {}", q::decimal(e(v), 3))).collect();
            return Err(format!("{} vs {} at {}", q::decimal(x, 6), q::decimal(y, 6), if at.is_empty() { "-".into() } else { at.join(", ") }));
        }
        n += 1;
    }
    if n < 3 {
        return Err("undefined at the sample points".into());
    }
    Ok(n)
}

pub fn check(req: &Request, cfg: &Config, path: &Path) -> Vec<Check> {
    let mut out = Vec::new();
    if let Some(m) = &req.method {
        let used = path.uses(&m.value);
        out.push(ck("method", used, if used { format!("uses {} as asked (\"{}\")", m.value, m.words) } else { format!("\"{}\" doesn't apply to this problem", m.words) }));
    }
    match req.task.value {
        Task::Solve if matches!(req.problem.value, Math::Ineq(..)) => ineq_checks(req, cfg, path, &mut out),
        Task::Solve if matches!(req.problem.value, Math::System(_)) => system_checks(req, cfg, path, &mut out),
        // calculus and trig (agent B)
        Task::Solve if trig_equation(req) => trig_checks(req, cfg, path, &mut out),
        Task::Integrate | Task::Limit | Task::Tangent => calc_checks(req, cfg, path, &mut out),
        Task::Differentiate if req.calc.order() > 1 => calc_checks(req, cfg, path, &mut out),
        Task::Solve => solve_checks(req, cfg, path, &mut out),
        _ => expr_checks(req, cfg, path, &mut out),
    }
    out
}

fn expr_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let Math::Expr(original) = req.start() else { return };
    let Math::Expr(fin) = &path.state else {
        out.push(ck("answer", false, "the answer isn't an expression"));
        return;
    };
    // every step keeps the value
    let mut bad = None;
    for (k, s) in path.steps.iter().enumerate() {
        if let (Math::Expr(a), Math::Expr(b)) = (&s.before, &s.mv.result) {
            if let Err(e) = agree(a, b, req, cfg) {
                bad = Some(format!("step {} ({}) changes the value: {e}", k + 1, s.mv.rule));
                break;
            }
        }
    }
    out.push(match bad {
        Some(b) => ck("steps", false, b),
        None if path.steps.is_empty() => ck("steps", true, "no steps to check"),
        None if original.vars().is_empty() => ck("steps", true, if path.steps.len() == 1 { "the step keeps the value".to_string() } else { format!("each of the {} steps keeps the value", path.steps.len()) }),
        None => ck("steps", true, format!("all {} steps agree at {} sample points", path.steps.len(), cfg.check.samples.len())),
    });
    match req.task.value {
        Task::Evaluate => {
            let exact = |e: &Expr| {
                let e2 = req.given.iter().fold(e.clone(), |acc, g| acc.subst(&g.value.0, &g.value.1));
                e2.eval_q(&|_| None)
            };
            match (exact(&original), fin.eval_q(&|_| None)) {
                (Some(a), Some(b)) => out.push(ck("value", a == b, format!("computed directly, {} is exactly {a}", print::expr(&original, Style::Ascii)))),
                (None, _) if original.walk().iter().any(|(_, n)| matches!(n, Expr::Div(_, d) if d.eval_q(&|_| None).is_some_and(|q| q.is_zero()))) => {
                    out.push(ck("value", false, "division by zero"));
                }
                _ => match agree(&original, fin, req, cfg) {
                    Ok(_) => out.push(ck("value", true, format!("equals {} to {} decimal places", print::expr(&original, Style::Ascii), 9))),
                    Err(e) => out.push(ck("value", false, e)),
                },
            }
        }
        Task::Simplify | Task::Expand | Task::Factor => {
            match agree(&original, fin, req, cfg) {
                Ok(n) => out.push(ck("same", true, format!("equal to the original at {n} sample points"))),
                Err(e) => out.push(ck("same", false, e)),
            }
            if req.task.value == Task::Expand {
                let left = fin.walk().iter().any(|(_, n)| match n {
                    Expr::Mul(f) => f.iter().any(|x| matches!(x, Expr::Add(_))),
                    Expr::Pow(b, _) => matches!(**b, Expr::Add(_)),
                    _ => false,
                });
                out.push(ck("expanded", !left, if left { "brackets remain" } else { "no brackets remain" }));
            }
            if req.task.value == Task::Factor {
                out.push(fully_factored(fin));
            }
        }
        Task::Differentiate => {
            // the start state is already d/dx[problem]
            match agree(&original, fin, req, cfg) {
                Ok(n) => out.push(ck("derivative", true, format!("matches the slope measured numerically at {n} points"))),
                Err(e) => out.push(ck("derivative", false, e)),
            }
        }
        Task::Solve | Task::Integrate | Task::Limit | Task::Tangent => unreachable!(),
        Task::Divide => divide_checks(&original, fin, out),
    }
}

/// A whole number is factored when every factor is prime.
fn prime_factored(e: &Expr) -> Check {
    let mut bases = Vec::new();
    for f in match e {
        Expr::Mul(v) => v.clone(),
        x => vec![x.clone()],
    } {
        let b = match &f {
            Expr::Pow(b, _) => (**b).clone(),
            x => x.clone(),
        };
        match b.as_num().filter(|q| q.is_int()) {
            Some(q) if q.num() == -1 => {}
            Some(q) => bases.push(q.num()),
            None => return ck("factored", false, format!("{} isn't a whole number", print::expr(&b, Style::Ascii))),
        }
    }
    let composite = bases.iter().find(|&&p| p < 2 || (2..).take_while(|d| d * d <= p).any(|d| p % d == 0));
    match composite {
        Some(p) => ck("factored", false, format!("{p} is not prime")),
        None if bases.len() == 1 => ck("factored", true, format!("{} is prime: no number from 2 to its square root divides it", bases[0])),
        None => ck("factored", true, format!("every factor is prime ({})", bases.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "))),
    }
}

/// Every factor is prime over the integers (one letter), or no rule applies further.
fn fully_factored(e: &Expr) -> Check {
    let vars = e.vars();
    if vars.is_empty() {
        return prime_factored(e);
    }
    if vars.len() != 1 {
        return ck("factored", true, "several letters: equality checked; no factoring rule applies further");
    }
    let v = vars.iter().next().expect("one letter");
    let mut factors = Vec::new();
    fn collect(e: &Expr, out: &mut Vec<Expr>) {
        match e {
            Expr::Mul(f) => f.iter().for_each(|x| collect(x, out)),
            Expr::Neg(a) => collect(a, out),
            Expr::Pow(b, n) if n.as_num().is_some_and(|q| q.is_int()) => collect(b, out),
            x => out.push(x.clone()),
        }
    }
    collect(e, &mut factors);
    for f in factors.iter().filter(|f| f.has_var(v)) {
        let Some(p) = poly::from_expr(f, v) else { return ck("factored", false, format!("{} isn't a polynomial", print::expr(f, Style::Ascii))) };
        let d = p.deg().unwrap_or(0);
        let text = print::expr(f, Style::Ascii);
        if d >= 2 && !p.rational_roots().is_empty() {
            return ck("factored", false, format!("{text} has a rational root, so it factors further"));
        }
        if d >= 4 {
            return ck("factored", false, format!("can't prove {text} doesn't split into quadratics"));
        }
        // integer coefficients with a common factor left inside: 2x + 4
        if d >= 1 && p.0.iter().all(|c| c.is_int()) && p.0.iter().fold(0, |g, c| q::gcd(g, c.num())) > 1 {
            return ck("factored", false, format!("{text} still has a common factor"));
        }
    }
    ck("factored", true, "every factor is linear or has no rational root")
}

/// The solutions a final state claims: (as written, value).
fn claimed(m: &Math, v: &str) -> Vec<Expr> {
    let x = Expr::Var(v.to_string());
    match m {
        Math::Eq(l, r) if *l == x => vec![r.clone()],
        Math::Or(eqs) => eqs.iter().filter(|(l, _)| *l == x).map(|(_, r)| r.clone()).collect(),
        _ => vec![],
    }
}

fn solve_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = &req.var.value;
    let Math::Eq(l, r) = &req.problem.value else { return };
    let sols = claimed(&path.state, v);
    let tol = cfg.check.tolerance;
    // 1. every answer satisfies the original equation
    let mut notes = Vec::new();
    let mut ok = true;
    for s in &sols {
        let shown = format!("{v} = {}", print::expr(s, Style::Ascii));
        match s.eval_q(&|_| None) {
            Some(q) => {
                let at = |e: &Expr| e.eval_q(&|n| if n == v { Some(q) } else { None });
                match (at(l), at(r)) {
                    (Some(a), Some(b)) if a == b => notes.push(format!("{shown}: both sides are {a}")),
                    (Some(a), Some(b)) => {
                        ok = false;
                        notes.push(format!("{shown}: {a} vs {b}"));
                    }
                    _ => {
                        // not exact (a log, a root): compare decimals
                        let f = |e: &Expr| e.eval_f(&|n| if n == v { q.to_f64() } else { f64::NAN });
                        let (a, b) = (f(l), f(r));
                        if a.is_finite() && close(a, b, tol) {
                            notes.push(format!("{shown}: both sides ≈ {}", q::decimal(a, 5)));
                        } else {
                            ok = false;
                            notes.push(format!("{shown}: a side is undefined there"));
                        }
                    }
                }
            }
            None => {
                let x = s.eval_f(&|_| f64::NAN);
                let at = |e: &Expr| e.eval_f(&|n| if n == v { x } else { f64::NAN });
                let (a, b) = (at(l), at(r));
                if a.is_finite() && close(a, b, tol) {
                    notes.push(format!("{shown} ≈ {}: both sides ≈ {}", q::decimal(x, 5), q::decimal(a, 5)));
                } else {
                    ok = false;
                    notes.push(format!("{shown}: {} vs {}", q::decimal(a, 6), q::decimal(b, 6)));
                }
            }
        }
    }
    if !sols.is_empty() {
        out.push(ck("satisfies", ok, notes.join("; ")));
    }
    // 1b. "no solution" / "every number": test the original at the sample points
    if sols.is_empty() {
        let (mut held, mut tried) = (0, 0);
        for k in 0..cfg.check.samples.len() {
            let e = env(req, cfg, k);
            let (a, b) = (l.eval_f(&e), r.eval_f(&e));
            if a.is_finite() && b.is_finite() {
                tried += 1;
                if close(a, b, tol) {
                    held += 1;
                }
            }
        }
        out.push(match &path.state {
            Math::AllReals => ck("samples", tried >= 3 && held == tried, format!("the equation holds at all {tried} sample points tried")),
            _ => ck("samples", tried >= 3 && held == 0, format!("the equation fails at all {tried} sample points tried")),
        });
    }
    // 2. no solution missed: count real roots from the equation itself
    let diff = expr::add(vec![l.clone(), expr::neg(r.clone())]);
    let Some((n, d)) = poly::rational_from_expr(&diff, v) else {
        out.push(complete_other(l, r, v, &sols, &path.state));
        working(path, &sols, v, tol, out);
        return;
    };
    let complete = if n.is_zero() {
        let all = matches!(path.state, Math::AllReals);
        ck("complete", all, if all { "the two sides are the same expression".to_string() } else { "every number is a solution, but the answer says otherwise".to_string() })
    } else {
        match n.real_roots() {
            None => ck("complete", false, format!("degree {} is too high to count the real solutions", n.deg().unwrap_or(0))),
            Some((rats, irr)) => {
                let rats: Vec<Q> = rats.into_iter().filter(|q| !d.eval(q).is_some_and(|x| x.is_zero())).collect();
                let got_rats: Vec<Q> = {
                    let mut g: Vec<Q> = sols.iter().filter_map(|s| s.eval_q(&|_| None)).collect();
                    g.sort();
                    g.dedup();
                    g
                };
                let got_irr = sols.iter().filter(|s| s.eval_q(&|_| None).is_none()).count();
                let expected = rats.len() + irr;
                let ok = got_rats == rats && got_irr == irr && (expected > 0 || matches!(path.state, Math::NoSolution));
                let kind = match n.deg() {
                    Some(1) => "linear".to_string(),
                    Some(2) => "quadratic".to_string(),
                    Some(k) => format!("degree {k}"),
                    None => "constant".to_string(),
                };
                let detail = match expected {
                    0 => format!("counted from the {kind} equation itself: no real solution"),
                    1 => format!("counted from the {kind} equation itself: exactly one real solution"),
                    k => format!("counted from the {kind} equation itself: exactly {k} real solutions"),
                };
                ck("complete", ok, if ok { detail } else { format!("{detail}, but the answer has {}", got_rats.len() + got_irr) })
            }
        }
    };
    out.push(complete);
    working(path, &sols, v, tol, out);
}

/// 3. every line of the working holds at every answer
fn working(path: &Path, sols: &[Expr], v: &str, tol: f64, out: &mut Vec<Check>) {
    let mut bad = None;
    for (k, s) in path.steps.iter().enumerate() {
        for sol in sols {
            let x = sol.eval_f(&|_| f64::NAN);
            let holds = |m: &Math| -> Option<bool> {
                let eqs: Vec<(&Expr, &Expr)> = match m {
                    Math::Eq(a, b) => vec![(a, b)],
                    Math::Or(e) => e.iter().map(|(a, b)| (a, b)).collect(),
                    _ => return None,
                };
                let mut any_defined = false;
                for (a, b) in eqs {
                    let (p, q) = (a.eval_f(&|n| if n == v { x } else { f64::NAN }), b.eval_f(&|n| if n == v { x } else { f64::NAN }));
                    if p.is_finite() && q.is_finite() {
                        any_defined = true;
                        if close(p, q, tol) {
                            return Some(true);
                        }
                    }
                }
                if any_defined {
                    Some(false)
                } else {
                    None
                }
            };
            if holds(&s.mv.result) == Some(false) {
                bad = Some(format!("after step {} ({}), {v} = {} no longer fits", k + 1, s.mv.rule, print::expr(sol, Style::Ascii)));
            }
        }
    }
    if !sols.is_empty() {
        out.push(match bad {
            Some(b) => ck("working", false, b),
            None => ck("working", true, format!("every line of the working holds at every answer ({} steps)", path.steps.len())),
        });
    }
}

// ---- algebra (agent A): division, inequalities, exponential, log and absolute value equations, systems ----

/// A finished division q + r/d read back as (quotient, remainder).
pub fn quotient_remainder(fin: &Expr, d: &Expr) -> (Expr, Expr) {
    let mut q = Vec::new();
    let mut r = expr::num(0);
    for t in expr::terms(fin) {
        match &t {
            Expr::Div(n, dd) if **dd == *d => r = (**n).clone(),
            Expr::Neg(inner) if matches!(&**inner, Expr::Div(_, dd) if **dd == *d) => {
                if let Expr::Div(n, _) = &**inner {
                    r = expr::neg((**n).clone());
                }
            }
            _ => q.push(t.clone()),
        }
    }
    (expr::add(q), r)
}

/// Divide: quotient * divisor + remainder is the dividend, exactly, and the
/// remainder's degree is below the divisor's.
fn divide_checks(original: &Expr, fin: &Expr, out: &mut Vec<Check>) {
    let Expr::Div(a, d) = original else {
        out.push(ck("division", false, "the problem isn't a division"));
        return;
    };
    let v = original.vars().into_iter().next().unwrap_or_else(|| "x".into());
    let (q, r) = quotient_remainder(fin, d);
    let polys = (poly::from_expr(a, &v), poly::from_expr(d, &v), poly::from_expr(&q, &v), poly::from_expr(&r, &v));
    let (Some(pa), Some(pd), Some(pq), Some(pr)) = polys else {
        out.push(ck("division", false, "the quotient or remainder isn't a polynomial"));
        return;
    };
    let back = pq.mul(&pd).and_then(|p| p.add(&pr));
    let shown = |e: &Expr| print::expr(e, Style::Ascii);
    let ok = back.as_ref() == Some(&pa);
    let product = Expr::Add(vec![Expr::Mul(vec![q.clone(), (**d).clone()]), r.clone()]);
    out.push(ck("division", ok, format!("{} = {} exactly", shown(&product), shown(a))));
    let low = pr.deg().is_none_or(|k| Some(k) < pd.deg());
    out.push(ck("remainder", low, if low { format!("the remainder {} has lower degree than {}", shown(&r), shown(d)) } else { format!("the remainder {} could still be divided", shown(&r)) }));
}

/// Which way an expression moves as the letter grows, where it is defined:
/// 1 up, -1 down, 0 constant; None when it may turn.
fn monotone(e: &Expr, v: &str) -> Option<i8> {
    use crate::expr::Func;
    if !e.has_var(v) {
        return Some(0);
    }
    let sign = |x: &Expr| -> Option<i8> {
        let f = x.eval_f(&|_| f64::NAN);
        if f > 0.0 {
            Some(1)
        } else if f < 0.0 {
            Some(-1)
        } else {
            None
        }
    };
    match e {
        Expr::Var(_) => Some(1),
        Expr::Add(ts) => {
            let mut d = 0;
            for t in ts {
                match monotone(t, v)? {
                    0 => {}
                    k if d == 0 || d == k => d = k,
                    _ => return None,
                }
            }
            Some(d)
        }
        Expr::Neg(a) => Some(-monotone(a, v)?),
        Expr::Mul(fs) => {
            let with: Vec<&Expr> = fs.iter().filter(|f| f.has_var(v)).collect();
            if with.len() != 1 {
                return None;
            }
            let k = fs.iter().filter(|f| !f.has_var(v)).try_fold(1i8, |acc, f| Some(acc * sign(f)?))?;
            Some(k * monotone(with[0], v)?)
        }
        Expr::Div(a, b) if !b.has_var(v) => Some(sign(b)? * monotone(a, v)?),
        Expr::Pow(b, a) if !b.has_var(v) => {
            let base = b.eval_f(&|_| f64::NAN);
            let k = if base > 1.0 {
                1
            } else if base > 0.0 && base < 1.0 {
                -1
            } else {
                return None;
            };
            Some(k * monotone(a, v)?)
        }
        Expr::Pow(b, n) if n.as_num().is_some_and(|q| q.is_int() && q.num() > 0 && q.num() % 2 == 1) => monotone(b, v),
        Expr::Func(Func::Exp | Func::Ln | Func::Sqrt, a) => monotone(a, v),
        Expr::Log(b, a) if !b.has_var(v) => {
            let base = b.eval_f(&|_| f64::NAN);
            if base > 1.0 {
                monotone(a, v)
            } else if base > 0.0 && base < 1.0 {
                Some(-monotone(a, v)?)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Is the expression positive for every value of the letter?
fn always_positive(e: &Expr) -> bool {
    use crate::expr::Func;
    match e {
        Expr::Num(q) => !q.is_neg() && !q.is_zero(),
        Expr::Const(_) => true,
        Expr::Func(Func::Exp, _) => true,
        Expr::Pow(b, _) => b.eval_f(&|_| f64::NAN) > 0.0,
        Expr::Mul(fs) | Expr::Add(fs) => fs.iter().all(always_positive),
        _ => false,
    }
}

/// b^A = c^B (a number counts as c^1) where c is a rational power of b:
/// the real roots of A = (log_b c) B, which is the same equation.
fn exp_complete(l: &Expr, r: &Expr, v: &str) -> Option<Vec<f64>> {
    use crate::expr::Func;
    let side = |e: &Expr| -> Option<(Option<Q>, Expr)> {
        match e {
            Expr::Func(Func::Exp, a) => Some((None, (**a).clone())),
            Expr::Pow(b, a) if !b.has_var(v) => Some((Some(b.eval_q(&|_| None)?), (**a).clone())),
            e if !e.has_var(v) => Some((Some(e.eval_q(&|_| None)?), expr::num(1))),
            _ => None,
        }
    };
    let ((b1, a), (b2, b)) = (side(l)?, side(r)?);
    if !(l.has_var(v) || r.has_var(v)) {
        return None;
    }
    let valid = |q: &Q| !q.is_neg() && !q.is_zero() && !q.is_one();
    let eq = match (b1, b2) {
        (None, None) => expr::add(vec![a, expr::neg(b)]),
        (Some(p), Some(q)) => {
            if let Some(k) = Some(p).filter(valid).and_then(|p| crate::expr::exact_log(&p, &q)) {
                expr::add(vec![a, expr::neg(expr::mul(vec![Expr::Num(k), b]))])
            } else if let Some(k) = Some(q).filter(valid).and_then(|q| crate::expr::exact_log(&q, &p)) {
                expr::add(vec![expr::mul(vec![Expr::Num(k), a]), expr::neg(b)])
            } else {
                return None;
            }
        }
        _ => return None,
    };
    let p = poly::from_expr(&eq, v)?;
    if p.is_zero() {
        return None;
    }
    real_root_values(&p)
}

/// b^A with A linear in the letter: (ln b, slope of A).
fn exp_slope(e: &Expr, v: &str) -> Option<(f64, f64)> {
    use crate::expr::{Func, Konst};
    let (lnb, a) = match e {
        Expr::Func(Func::Exp, a) => (1.0, &**a),
        Expr::Pow(b, a) if !b.has_var(v) => {
            let base = b.eval_f(&|_| f64::NAN);
            if base <= 0.0 {
                return None;
            }
            (if **b == Expr::Const(Konst::E) { 1.0 } else { base.ln() }, &**a)
        }
        e if !e.has_var(v) => return Some((0.0, 0.0)),
        _ => return None,
    };
    let p = poly::from_expr(a, v)?;
    (p.deg().unwrap_or(0) <= 1).then(|| (lnb, p.coef(1).to_f64()))
}

/// Count the solutions of an equation that isn't polynomial, from its own
/// shape: a side that only rises against one that only falls meets it at
/// most once; powers on both sides become a linear equation under logs;
/// something always positive never equals a number that isn't.
fn complete_other(l: &Expr, r: &Expr, v: &str, sols: &[Expr], state: &Math) -> Check {
    if l.walk().iter().chain(r.walk().iter()).any(|(_, n)| matches!(n, Expr::Func(crate::expr::Func::Abs, _))) {
        return abs_complete(l, r, v, sols, state);
    }
    let shown = |e: &Expr| print::expr(e, Style::Ascii);
    let none = matches!(state, Math::NoSolution);
    if let Some((expected, how)) = log_complete(l, r, v) {
        let mut got: Vec<f64> = sols.iter().map(|s| s.eval_f(&|_| f64::NAN)).collect();
        got.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let ok = got.len() == expected.len() && got.iter().zip(&expected).all(|(a, b)| close(*a, *b, 1e-9)) && (!expected.is_empty() || none);
        let count = match expected.len() {
            0 => "no real solution".to_string(),
            1 => "exactly one real solution".to_string(),
            k => format!("exactly {k} real solutions"),
        };
        return ck("complete", ok, format!("{how}: {count}{}", if ok { "" } else { ", but the answer differs" }));
    }
    // b^A = c^B with c a power of b: the exponents' polynomial equation
    if let Some(expected) = exp_complete(l, r, v) {
        let mut got: Vec<f64> = sols.iter().map(|s| s.eval_f(&|_| f64::NAN)).collect();
        got.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let ok = got.len() == expected.len() && got.iter().zip(&expected).all(|(a, b)| close(*a, *b, 1e-9)) && (!expected.is_empty() || none);
        let count = match expected.len() {
            0 => "no real solution".to_string(),
            1 => "exactly one real solution".to_string(),
            k => format!("exactly {k} real solutions"),
        };
        return ck("complete", ok, format!("both sides are powers of one base, so the exponents are equal; that polynomial equation has {count}{}", if ok { "" } else { ", but the answer differs" }));
    }
    // b^A = c^B with A, B linear: under logs, A ln b = B ln c, a line
    if let (Some((lb, la)), Some((rb, ra))) = (exp_slope(l, v), exp_slope(r, v)) {
        let slope = lb * la - rb * ra;
        let positive = always_positive(l) && always_positive(r);
        if positive && slope.abs() > 1e-12 {
            let ok = sols.len() == 1;
            return ck("complete", ok, format!("both sides are positive powers; taking logs leaves a linear equation in {v}, so exactly one solution{}", if ok { "" } else { ", but the answer differs" }));
        }
    }
    // a positive side and a side that is 0 or negative
    for (p, c) in [(l, r), (r, l)] {
        if always_positive(p) && !c.has_var(v) && c.eval_f(&|_| f64::NAN) <= 0.0 {
            return ck("complete", none, format!("{} is positive for every {v} and {} is not, so there is no solution", shown(p), shown(c)));
        }
    }
    match (monotone(l, v), monotone(r, v)) {
        (Some(a), Some(b)) if a != 0 && (b == 0 || b == -a) || a == 0 && b != 0 => {
            let (side, dir) = if a != 0 { (l, a) } else { (r, b) };
            let words = if dir > 0 { "only increases" } else { "only decreases" };
            let ok = sols.len() == 1;
            let what = if a != 0 && b != 0 { format!("{} - ({})", shown(l), shown(r)) } else { shown(side) };
            ck("complete", ok, format!("{what} {words} where it is defined, so there is at most one solution"))
        }
        _ => ck("complete", false, "can't count the solutions of this kind of equation"),
    }
}

/// Every real root of a polynomial as a decimal: rational ones by the
/// rational root theorem, what is left (degree 2 at most) by the formula.
/// None when a factor of degree 3 or more without rational roots is left.
fn real_root_values(p: &poly::Poly) -> Option<Vec<f64>> {
    let rational = p.rational_roots();
    let mut rest = p.clone();
    for r in &rational {
        while let Some(d) = rest.deflate(r) {
            rest = d;
        }
    }
    let mut out: Vec<f64> = rational.iter().map(|r| r.to_f64()).collect();
    match rest.deg()? {
        0 => {}
        2 => {
            let (a, b, c) = (rest.coef(2).to_f64(), rest.coef(1).to_f64(), rest.coef(0).to_f64());
            let d = b * b - 4.0 * a * c;
            if d > 0.0 {
                out.push((-b - d.sqrt()) / (2.0 * a));
                out.push((-b + d.sqrt()) / (2.0 * a));
            }
        }
        _ => return None,
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some(out)
}

/// Replace every |A| by A or -A, as A's sign is at x = m.
fn unabs(e: &Expr, v: &str, m: f64) -> Expr {
    let mut out = e.clone();
    for i in 0..e.children().len() {
        out = out.replace_raw(&[i], unabs(e.children()[i], v, m));
    }
    match &out {
        Expr::Func(crate::expr::Func::Abs, a) => {
            if a.eval_f(&|n| if n == v { m } else { f64::NAN }) < 0.0 {
                expr::neg((**a).clone())
            } else {
                (**a).clone()
            }
        }
        _ => out,
    }
}

/// Absolute value equations: split the line where each |A| changes sign;
/// on each piece the equation is a polynomial one, whose roots in the piece
/// are counted. The answer must list exactly those.
fn abs_complete(l: &Expr, r: &Expr, v: &str, sols: &[Expr], state: &Math) -> Check {
    let fail = |why: &str| ck("complete", false, why.to_string());
    let mut cuts: Vec<f64> = Vec::new();
    for e in [l, r] {
        for (_, n) in e.walk() {
            if let Expr::Func(crate::expr::Func::Abs, a) = n {
                let Some(p) = poly::from_expr(a, v) else { return fail("can't split an absolute value of this kind") };
                let Some(rs) = real_root_values(&p) else { return fail("can't find where the absolute value changes sign") };
                cuts.extend(rs);
            }
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let at = |e: &Expr, x: f64| e.eval_f(&|n| if n == v { x } else { f64::NAN });
    let mut expected: Vec<f64> = Vec::new();
    // the cut points themselves
    for &c in &cuts {
        if close(at(l, c), at(r, c), 1e-9) {
            expected.push(c);
        }
    }
    // the open pieces between them
    let mut edges = vec![f64::NEG_INFINITY];
    edges.extend(cuts.iter().copied());
    edges.push(f64::INFINITY);
    for w in edges.windows(2) {
        let (a, b) = (w[0], w[1]);
        let m = match (a.is_finite(), b.is_finite()) {
            (true, true) => (a + b) / 2.0,
            (true, false) => a + 1.0,
            (false, true) => b - 1.0,
            (false, false) => 0.0,
        };
        let piece = expr::add(vec![unabs(l, v, m), expr::neg(unabs(r, v, m))]);
        let Some((n, _)) = poly::rational_from_expr(&piece, v) else { return fail("can't count the solutions on each piece") };
        if n.is_zero() {
            return fail("the equation holds on a whole interval; Nuome can't list that as an answer");
        }
        let Some(rs) = real_root_values(&n) else { return fail("a piece is a polynomial of too high a degree") };
        expected.extend(rs.into_iter().filter(|x| *x > a + 1e-9 && *x < b - 1e-9));
    }
    expected.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut got: Vec<f64> = sols.iter().map(|s| s.eval_f(&|_| f64::NAN)).collect();
    got.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    got.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    let ok = got.len() == expected.len() && got.iter().zip(&expected).all(|(a, b)| close(*a, *b, 1e-9)) && (!expected.is_empty() || matches!(state, Math::NoSolution));
    let pieces = cuts.len() + 1;
    let count = match expected.len() {
        0 => "no real solution".to_string(),
        1 => "exactly one real solution".to_string(),
        k => format!("exactly {k} real solutions"),
    };
    ck("complete", ok, format!("split into {pieces} pieces where the absolute values change sign, and counted on each: {count}{}", if ok { "" } else { ", but the answer differs" }))
}

/// A sum of logs to one base with coefficients +/-1: (base, [(sign, argument)]).
fn log_sum(e: &Expr) -> Option<(Expr, Vec<(bool, Expr)>)> {
    use crate::expr::{Func, Konst};
    let mut base: Option<Expr> = None;
    let mut out = Vec::new();
    for t in expr::terms(e) {
        let (c, rest) = expr::coeff(&t);
        let (b, a) = match rest {
            Expr::Log(b, a) => (*b, *a),
            Expr::Func(Func::Ln, a) => (Expr::Const(Konst::E), *a),
            _ => return None,
        };
        if base.as_ref().is_some_and(|x| *x != b) || !(c.is_one() || c == Q::int(-1)) {
            return None;
        }
        base = Some(b);
        out.push((c.is_one(), a));
    }
    Some((base?, out))
}

/// Log equations, counted on their domain (every log argument positive):
/// log(A) = log(B) is A = B there; a sum of logs = c is the product of the
/// arguments = base^c, a polynomial equation. (expected roots, how).
fn log_complete(l: &Expr, r: &Expr, v: &str) -> Option<(Vec<f64>, String)> {
    let at = |e: &Expr, x: f64| e.eval_f(&|n| if n == v { x } else { f64::NAN });
    let (poly_eq, args) = match (log_sum(l), log_sum(r)) {
        (Some((b1, a1)), Some((b2, a2))) if b1 == b2 && a1.len() == 1 && a2.len() == 1 && a1[0].0 && a2[0].0 => {
            (expr::add(vec![a1[0].1.clone(), expr::neg(a2[0].1.clone())]), vec![a1[0].1.clone(), a2[0].1.clone()])
        }
        (Some((b, ts)), None) | (None, Some((b, ts))) => {
            let c = if log_sum(l).is_some() { r } else { l };
            // base^c exactly
            let k = c.eval_q(&|_| None).filter(|k| k.is_int())?;
            let value = b.as_num()?.pow(k.num() as i64)?;
            let top = expr::mul(ts.iter().filter(|(p, _)| *p).map(|(_, a)| a.clone()).collect());
            let bottom = expr::mul(ts.iter().filter(|(p, _)| !*p).map(|(_, a)| a.clone()).collect());
            (expr::add(vec![top, expr::neg(expr::mul(vec![Expr::Num(value), bottom]))]), ts.iter().map(|(_, a)| a.clone()).collect())
        }
        _ => return None,
    };
    let (n, _) = poly::rational_from_expr(&poly_eq, v)?;
    if n.is_zero() {
        return None;
    }
    let roots = real_root_values(&n)?;
    let inside: Vec<f64> = roots.into_iter().filter(|x| args.iter().all(|a| at(a, *x) > 0.0)).collect();
    Some((inside, "where every log is defined, the logs undo into a polynomial equation, whose roots there were counted".into()))
}

/// Where an inequality's sign can change, found from the inequality itself:
/// the real roots of the numerator and denominator of l - r.
fn boundaries(l: &Expr, r: &Expr, v: &str) -> Option<Vec<f64>> {
    let (n, d) = poly::rational_from_expr(&expr::add(vec![l.clone(), expr::neg(r.clone())]), v)?;
    let mut out = real_root_values(&n)?;
    if d.deg().is_some_and(|k| k > 0) {
        out.extend(real_root_values(&d)?);
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    Some(out)
}

/// Does a state say x belongs to the answer? None where it can't tell (undefined).
fn member(m: &Math, v: &str, x: f64, tol: f64) -> Option<bool> {
    match m {
        Math::Ineq(l, rel, r) => {
            let at = |e: &Expr| e.eval_f(&|n| if n == v { x } else { f64::NAN });
            let (a, b) = (at(l), at(r));
            (a.is_finite() && b.is_finite()).then(|| rel.holds(a, b, tol))
        }
        Math::Intervals(_, ivs) => Some(ivs.iter().any(|iv| iv.contains(x, tol))),
        Math::AllReals => Some(true),
        Math::NoSolution => Some(false),
        _ => None,
    }
}

/// Inequalities: the answer set is tested against the original inequality
/// at points found independently of the steps: every boundary (root of the
/// numerator or denominator), just either side of it, between boundaries,
/// far out, and the usual samples.
fn ineq_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = &req.var.value;
    let Math::Ineq(l, rel, r) = &req.problem.value else { return };
    let tol = cfg.algebra.sign_tolerance;
    let Some(cuts) = boundaries(l, r, v) else {
        out.push(ck("intervals", false, "can't find where this kind of inequality changes sign"));
        return;
    };
    let mut points: Vec<f64> = cfg.check.samples.clone();
    for (i, &c) in cuts.iter().enumerate() {
        let gap = [i.checked_sub(1).map(|j| c - cuts[j]), cuts.get(i + 1).map(|n| n - c)].into_iter().flatten().fold(f64::INFINITY, f64::min);
        let d = (cfg.algebra.boundary_offset * 1f64.max(c.abs())).min(gap / 4.0);
        points.extend([c, c - d, c + d]);
        if let Some(n) = cuts.get(i + 1) {
            points.push((c + n) / 2.0);
        }
    }
    if let (Some(a), Some(b)) = (cuts.first(), cuts.last()) {
        points.extend([a - 1.0 - a.abs(), b + 1.0 + b.abs()]);
    }
    let truth = |x: f64| member(&Math::Ineq(l.clone(), *rel, r.clone()), v, x, tol);
    // 1. the answer holds exactly where the inequality does
    let (mut inside, mut outside, mut bad) = (0, 0, None);
    for &x in &points {
        let want = truth(x).unwrap_or(false);
        match member(&path.state, v, x, tol) {
            Some(got) if got == want => {
                if got {
                    inside += 1;
                } else {
                    outside += 1;
                }
            }
            got => {
                bad.get_or_insert(format!("at {v} = {}: the inequality {} but the answer {}", q::decimal(x, 4), if want { "holds" } else { "fails" }, if got == Some(true) { "includes it" } else { "leaves it out" }));
            }
        }
    }
    out.push(match bad {
        Some(b) => ck("intervals", false, b),
        None => ck("intervals", true, format!("the inequality holds at all {inside} test points inside the answer and fails at all {outside} outside")),
    });
    // 2. each boundary is in the answer exactly when the inequality holds there
    let mut notes = Vec::new();
    let mut ok = true;
    for &c in &cuts {
        let want = truth(c);
        let got = member(&path.state, v, c, tol).unwrap_or(false);
        let here = format!("{v} = {}", q::decimal(c, 3).trim_end_matches('0').trim_end_matches('.'));
        let why = match want {
            None => "undefined there",
            Some(true) => "holds there",
            Some(false) => "fails there",
        };
        if got != want.unwrap_or(false) {
            ok = false;
        }
        notes.push(format!("{here} {} ({why})", if got { "included" } else { "excluded" }));
    }
    if !cuts.is_empty() {
        out.push(ck("boundaries", ok, notes.join("; ")));
    }
    // 3. every line of the working says the same as the original at every test point
    let mut bad = None;
    for (k, s) in path.steps.iter().enumerate() {
        for &x in &points {
            if let (Some(a), Some(b)) = (truth(x), member(&s.mv.result, v, x, tol)) {
                if a != b && bad.is_none() {
                    bad = Some(format!("after step {} ({}), {v} = {} changes side", k + 1, s.mv.rule, q::decimal(x, 4)));
                }
            }
        }
    }
    out.push(match bad {
        Some(b) => ck("working", false, b),
        None => ck("working", true, format!("every line of the working agrees with the inequality at every test point ({} steps)", path.steps.len())),
    });
}

/// Exact Gaussian elimination: the rank of a matrix of rationals.
fn rank(mut rows: Vec<Vec<Q>>) -> Option<usize> {
    let cols = rows.first().map_or(0, |r| r.len());
    let mut rank = 0;
    for c in 0..cols {
        let Some(p) = (rank..rows.len()).find(|&i| !rows[i][c].is_zero()) else { continue };
        rows.swap(rank, p);
        for i in 0..rows.len() {
            if i != rank && !rows[i][c].is_zero() {
                let f = rows[i][c].div(&rows[rank][c])?;
                for j in 0..cols {
                    rows[i][j] = rows[i][j].sub(&f.mul(&rows[rank][j])?)?;
                }
            }
        }
        rank += 1;
    }
    Some(rank)
}

/// The determinant of a square matrix (expansion along the first row; n <= 3).
fn det(m: &[Vec<Q>]) -> Option<Q> {
    match m.len() {
        1 => Some(m[0][0]),
        n => {
            let mut total = Q::ZERO;
            for c in 0..n {
                let minor: Vec<Vec<Q>> = m[1..].iter().map(|row| row.iter().enumerate().filter(|(j, _)| *j != c).map(|(_, x)| *x).collect()).collect();
                let term = m[0][c].mul(&det(&minor)?)?;
                total = if c % 2 == 0 { total.add(&term)? } else { total.sub(&term)? };
            }
            Some(total)
        }
    }
}

/// Systems: the answer is put back into every original equation exactly;
/// how many solutions there are comes from the coefficients alone (the
/// determinant, and the ranks when it is 0).
fn system_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let Math::System(eqs) = &req.problem.value else { return };
    let vars: Vec<String> = eqs.iter().flat_map(|(l, r)| l.vars().into_iter().chain(r.vars())).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    let Some(forms) = eqs.iter().map(|(l, r)| poly::linear_form(l, r, &vars)).collect::<Option<Vec<_>>>() else {
        out.push(ck("complete", false, "an equation isn't linear"));
        return;
    };
    let a: Vec<Vec<Q>> = forms.iter().map(|(c, _)| c.clone()).collect();
    let ab: Vec<Vec<Q>> = forms.iter().map(|(c, d)| c.iter().copied().chain([*d]).collect()).collect();
    let (Some(ra), Some(rab)) = (rank(a.clone()), rank(ab)) else {
        out.push(ck("complete", false, "the numbers are too large to eliminate exactly"));
        return;
    };
    let n = vars.len();
    let solved: Vec<(Expr, Expr)> = match &path.state {
        Math::System(s) => s.clone(),
        _ => vec![],
    };
    let free: Vec<String> = vars.iter().filter(|x| !solved.iter().any(|(l, _)| *l == Expr::Var((*x).clone()))).cloned().collect();
    // 1. every equation holds: at the solution, or for several values of the free letters
    let tries: Vec<Q> = if free.is_empty() { vec![Q::ZERO] } else { vec![Q::ZERO, Q::ONE, Q::int(2)] };
    let mut ok = !solved.is_empty();
    for t in &tries {
        let value = |x: &str| -> Option<Q> {
            if free.iter().any(|f| f == x) {
                return Some(*t);
            }
            let (_, rhs) = solved.iter().find(|(l, _)| *l == Expr::Var(x.to_string()))?;
            rhs.eval_q(&|y| free.iter().any(|f| f == y).then_some(*t))
        };
        for (l, r) in eqs {
            match (l.eval_q(&|x| value(x)), r.eval_q(&|x| value(x))) {
                (Some(p), Some(q)) if p == q => {}
                _ => ok = false,
            }
        }
    }
    // 1b. "no solution": no sample point satisfies every equation
    if matches!(path.state, Math::NoSolution) {
        let tried = cfg.check.samples.len();
        let held = (0..tried)
            .filter(|&k| {
                let e = env(req, cfg, k);
                eqs.iter().all(|(l, r)| close(l.eval_f(&e), r.eval_f(&e), cfg.check.tolerance))
            })
            .count();
        out.push(ck("samples", held == 0, format!("no sample point satisfies every equation ({tried} tried)")));
    }
    if !solved.is_empty() {
        let mut at: Vec<String> = solved.iter().map(|(l, r)| format!("{} = {}", print::expr(l, Style::Ascii), print::expr(r, Style::Ascii))).collect();
        at.sort();
        let detail = if free.is_empty() { format!("{}: every equation holds exactly", at.join(", ")) } else { format!("every equation holds exactly for {} = 0, 1 and 2, so for every value (the equations are linear)", free.join(", ")) };
        out.push(ck("satisfies", ok, detail));
    }
    // 2. how many solutions, from the coefficients
    let square = eqs.len() == n;
    let d = if square { det(&a) } else { None };
    let (expected, why) = if ra < rab {
        ("none", "the equations contradict each other (eliminating leaves 0 = a number that isn't 0)".to_string())
    } else if ra == n {
        ("one", match d {
            Some(d) => format!("the determinant of the coefficients is {d}, not 0"),
            None => format!("the coefficients have rank {ra}, the number of letters"),
        })
    } else {
        ("many", format!("{}the equations leave {} free", if square { "the determinant of the coefficients is 0 and " } else { "" }, if n - ra == 1 { "1 letter".to_string() } else { format!("{} letters", n - ra) }))
    };
    let got = match &path.state {
        Math::NoSolution => "none",
        Math::System(_) if free.is_empty() => "one",
        Math::System(_) if free.len() == n - ra => "many",
        _ => "wrong",
    };
    let count = match expected {
        "none" => "no solution",
        "one" => "exactly one solution",
        _ => "infinitely many solutions",
    };
    out.push(ck("complete", got == expected, format!("{why}: {count}{}", if got == expected { "" } else { ", but the answer differs" })));
    // 3. every line of the working holds at the solution
    if got == "one" && expected == "one" {
        let value = |x: &str| solved.iter().find(|(l, _)| *l == Expr::Var(x.to_string())).and_then(|(_, r)| r.eval_q(&|_| None));
        let mut bad = None;
        for (k, s) in path.steps.iter().enumerate() {
            if let Math::System(lines) = &s.mv.result {
                if lines.iter().any(|(l, r)| l.eval_q(&|x| value(x)) != r.eval_q(&|x| value(x))) {
                    bad.get_or_insert(format!("after step {} ({}), the solution no longer fits", k + 1, s.mv.rule));
                }
            }
        }
        out.push(match bad {
            Some(b) => ck("working", false, b),
            None => ck("working", true, format!("every line of the working holds at the solution ({} steps)", path.steps.len())),
        });
    }
}

// ---- Calculus and trigonometry (agent B) ----------------------------------------
//
// - integrate: every step keeps the derivative (each integral measured from
//   the sample point); differentiating the answer numerically gives back
//   the integrand; a definite integral equals Simpson's rule on the integrand
// - limit: the function, evaluated ever closer to the point from both sides
//   (or ever farther out), settles on the answer; every step keeps the limit
// - tangent: the line touches the curve at the point, with the curve's slope
// - higher derivatives: the n-th central difference matches the answer
// - trig equations: every answer (for every k tried) satisfies the equation,
//   and a fine scan of the interval (or of one turn) finds no other solution

/// Value of `e` with `v` = x, other letters as in sample `k`.
fn at_x(e: &Expr, v: &str, x: f64, req: &Request, cfg: &Config, k: usize) -> f64 {
    let base = env(req, cfg, k);
    e.eval_f(&|n: &str| if n == v { x } else { base(n) })
}

/// A number to 6 decimals, without trailing zeros: 1.99999, 1000000, 0.25.
fn short(x: f64) -> String {
    let s = q::decimal(x, 6);
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// A rational number near x, to anchor integrals at.
fn q_near(x: f64) -> Expr {
    let q = Q::parse(&format!("{:.6}", x.abs())).unwrap_or(Q::ZERO);
    Expr::Num(if x < 0.0 { q.neg() } else { q })
}

/// Does every step keep `measure` (a number per state and sample)?
fn steps_keep(path: &Path, cfg: &Config, tol: f64, what: &str, measure: &dyn Fn(&Math, usize) -> f64) -> Check {
    // a line too steep to measure in floating point (e^1000) is skipped and
    // counted, never passed off as checked; the answer has its own check
    let mut unmeasured = 0;
    for (i, s) in path.steps.iter().enumerate() {
        let mut n = 0;
        for k in 0..cfg.check.samples.len() {
            let (a, b) = (measure(&s.before, k), measure(&s.mv.result, k));
            if a.is_infinite() && a == b {
                n += 1;
                continue;
            }
            if !a.is_finite() || !b.is_finite() {
                continue;
            }
            if !close(a, b, tol) {
                return ck("steps", false, format!("step {} ({}) changes the {what}: {} vs {}", i + 1, s.mv.rule, q::decimal(a, 6), q::decimal(b, 6)));
            }
            n += 1;
        }
        if n == 0 {
            unmeasured += 1;
        }
    }
    if unmeasured == path.steps.len() && unmeasured > 0 {
        return ck("steps", false, format!("the {what} can't be measured at any step"));
    }
    match unmeasured {
        0 => ck("steps", true, format!("all {} steps keep the {what}", path.steps.len())),
        k => ck("steps", true, format!("{} of {} steps keep the {what}; {k} can't be measured numerically", path.steps.len() - k, path.steps.len())),
    }
}

fn calc_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    match req.task.value {
        Task::Integrate if req.calc.bounds.is_some() => definite_checks(req, cfg, path, out),
        Task::Integrate => indefinite_checks(req, cfg, path, out),
        Task::Limit => limit_checks(req, cfg, path, out),
        Task::Tangent => tangent_checks(req, cfg, path, out),
        _ => higher_checks(req, cfg, path, out),
    }
}

/// d/dx of an expression that may hold integrals: each is measured from
/// the sample point itself, so no singularity lies between.
fn slope(e: &Expr, v: &str, req: &Request, cfg: &Config, k: usize) -> f64 {
    let x0 = env(req, cfg, k)(v);
    let anchored = expr::anchor_integrals(e, v, &q_near(x0));
    Expr::Deriv(Box::new(anchored), v.to_string()).eval_f(&env(req, cfg, k))
}

fn indefinite_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = req.var.value.as_str();
    let Math::Expr(f) = &req.problem.value else { return };
    out.push(steps_keep(path, cfg, cfg.check.tolerance, "derivative", &|m, k| match m {
        Math::Expr(e) => slope(e, v, req, cfg, k),
        _ => f64::NAN,
    }));
    let Math::Expr(fin) = &path.state else { return };
    let mut n = 0;
    for k in 0..cfg.check.samples.len() {
        let e = env(req, cfg, k);
        let (d, want) = (Expr::Deriv(Box::new(fin.clone()), v.to_string()).eval_f(&e), f.eval_f(&e));
        if !d.is_finite() || !want.is_finite() {
            continue;
        }
        if !close(d, want, cfg.check.tolerance) {
            out.push(ck("antiderivative", false, format!("the answer's slope is {} but the integrand is {} at {v} = {}", q::decimal(d, 6), q::decimal(want, 6), q::decimal(e(v), 3))));
            return;
        }
        n += 1;
    }
    out.push(if n >= 3 { ck("antiderivative", true, format!("differentiating the answer numerically gives back the integrand at {n} points")) } else { ck("antiderivative", false, "the integrand is undefined at the sample points") });
}

/// Simpson's rule on f over [a, b] with the configured panels, or the
/// first point where f is undefined.
fn quadrature(f: &Expr, v: &str, a: f64, b: f64, req: &Request, cfg: &Config) -> Result<f64, f64> {
    let bad = std::cell::Cell::new(None);
    let val = expr::simpson(
        &|t| {
            let y = at_x(f, v, t, req, cfg, 0);
            if !y.is_finite() && bad.get().is_none() {
                bad.set(Some(t));
            }
            y
        },
        a,
        b,
        cfg.calculus.panels,
    );
    match bad.get() {
        Some(t) => Err(t),
        None => Ok(val),
    }
}

fn definite_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = req.var.value.as_str();
    let (Math::Expr(f), Some(b)) = (&req.problem.value, &req.calc.bounds) else { return };
    let tol = cfg.calculus.integral_tolerance;
    let (lo, hi) = (b.value.0.eval_f(&|_| f64::NAN), b.value.1.eval_f(&|_| f64::NAN));
    let Math::Expr(fin) = &path.state else { return };
    let got = fin.eval_f(&|_| f64::NAN);
    // the value first: an improper integral is the reason to name
    out.push(match quadrature(f, v, lo, hi, req, cfg) {
        Err(t) => ck("value", false, format!("the integrand is undefined at {v} = {}, inside the interval: improper integrals are not in Nuome v0", short(t))),
        Ok(s) if close(s, got, tol) => ck("value", true, format!("Simpson's rule with {} panels gives {}", cfg.calculus.panels, q::decimal(s, 6))),
        Ok(s) => ck("value", false, format!("Simpson's rule gives {} but the answer is {}", q::decimal(s, 6), q::decimal(got, 6))),
    });
    out.push(steps_keep(path, cfg, tol, "value", &|m, k| match m {
        Math::Expr(e) => e.eval_f(&env(req, cfg, k)),
        _ => f64::NAN,
    }));
    // the antiderivative the working used, if it shows one: [F]_a^b
    let shown = path.steps.iter().flat_map(|s| [&s.before, &s.mv.result]).find_map(|m| match m {
        Math::Expr(Expr::Bounds(g, w, ..)) if w == v && !g.is_pending() => Some((**g).clone()),
        _ => None,
    });
    if let Some(g) = shown {
        let mut n = 0;
        let mut bad = None;
        for k in 0..cfg.check.samples.len() {
            let e = env(req, cfg, k);
            let (d, want) = (Expr::Deriv(Box::new(g.clone()), v.to_string()).eval_f(&e), f.eval_f(&e));
            if d.is_finite() && want.is_finite() {
                if !close(d, want, cfg.check.tolerance) {
                    bad = Some(format!("F' is {} but the integrand is {} at {v} = {}", q::decimal(d, 6), q::decimal(want, 6), q::decimal(e(v), 3)));
                }
                n += 1;
            }
        }
        out.push(match bad {
            Some(b) => ck("antiderivative", false, b),
            None if n >= 3 => ck("antiderivative", true, format!("F = {} differentiates back to the integrand at {n} points", print::expr(&g, Style::Ascii))),
            None => ck("antiderivative", false, "F can't be compared at the sample points"),
        });
    }
}

/// The function near the point: (distance, left value, right value) from
/// far to near; at infinity the values ever farther out sit on the right.
fn approach(f: &Expr, v: &str, p: f64, req: &Request, cfg: &Config) -> Vec<(f64, f64, f64)> {
    if p.is_infinite() {
        cfg.calculus.limit_far.iter().map(|&x| (x, f64::NAN, at_x(f, v, x * p.signum(), req, cfg, 0))).collect()
    } else {
        cfg.calculus.limit_steps.iter().map(|&h| (h, at_x(f, v, p - h, req, cfg, 0), at_x(f, v, p + h, req, cfg, 0))).collect()
    }
}

/// Where one side is heading, from its two nearest values: ±infinity when
/// it is past `unbounded` and still growing, else extrapolated (the error
/// shrinks like the distance, or like 1/x at infinity).
fn heading(d1: f64, a1: f64, d2: f64, a2: f64, at_infinity: bool, cfg: &Config) -> f64 {
    if !a1.is_finite() || !a2.is_finite() {
        return f64::NAN;
    }
    if a2.abs() > cfg.calculus.unbounded && a2.abs() > a1.abs() && a1.signum() == a2.signum() {
        return f64::INFINITY * a2.signum();
    }
    let r = if at_infinity { (1.0 / d2) / (1.0 / d1 - 1.0 / d2) } else { d2 / (d1 - d2) };
    a2 + (a2 - a1) * r
}

/// Where each side of f is heading at p: (left, right); NaN where undefined.
fn sides(f: &Expr, v: &str, p: f64, req: &Request, cfg: &Config) -> (f64, f64) {
    let pts = approach(f, v, p, req, cfg);
    let n = pts.len();
    let ((d1, l1, r1), (d2, l2, r2)) = (pts[n - 2], pts[n - 1]);
    (heading(d1, l1, d2, l2, p.is_infinite(), cfg), heading(d1, r1, d2, r2, p.is_infinite(), cfg))
}

/// The limit estimated numerically, NaN when the sides disagree.
fn estimate(f: &Expr, v: &str, p: f64, req: &Request, cfg: &Config) -> f64 {
    let (l, r) = sides(f, v, p, req, cfg);
    match (l.is_nan(), r.is_nan()) {
        (true, _) => r,
        (_, true) => l,
        _ if l.is_infinite() || r.is_infinite() => {
            if l == r {
                l
            } else {
                f64::NAN
            }
        }
        _ if close(l, r, cfg.calculus.limit_tolerance) => (l + r) / 2.0,
        _ => f64::NAN,
    }
}

fn limit_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    out.push(steps_keep(path, cfg, cfg.calculus.limit_tolerance, "limit", &|m, k| match m {
        Math::Expr(Expr::Limit(f, w, p)) => estimate(f, w, p.eval_f(&|_| f64::NAN), req, cfg),
        Math::Expr(e) => e.eval_f(&env(req, cfg, k)),
        _ => f64::NAN,
    }));
    let (Math::Expr(Expr::Limit(f, v, p)), Math::Expr(fin)) = (req.start(), &path.state) else { return };
    let p = p.eval_f(&|_| f64::NAN);
    let want = fin.eval_f(&|_| f64::NAN);
    let pts = approach(&f, &v, p, req, cfg);
    let (far, near) = (pts[0], pts[pts.len() - 1]);
    let fx = |x: f64| format!("f({})", short(x));
    let mut notes = Vec::new();
    let mut ok = true;
    let mut defined = 0;
    for (side, a0, a1, x) in [("left", far.1, near.1, p - near.0), ("right", far.2, near.2, if p.is_infinite() { near.0 * p.signum() } else { p + near.0 })] {
        if !a1.is_finite() {
            if a1.is_nan() && a0.is_nan() && !p.is_infinite() {
                notes.push(format!("undefined on the {side}"));
            }
            continue;
        }
        defined += 1;
        let fine = if want.is_infinite() {
            a1.abs() >= cfg.calculus.unbounded && a1.signum() == want.signum() && a1.abs() > a0.abs()
        } else {
            let (e0, e1) = ((a0 - want).abs(), (a1 - want).abs());
            e1 <= cfg.calculus.limit_tolerance * 1f64.max(want.abs()) && (e1 <= e0 || e1 < cfg.check.tolerance)
        };
        ok &= fine;
        notes.push(format!("{} = {}", fx(x), short(a1)));
    }
    ok &= defined > 0 && (p.is_infinite() || defined == 2 || notes.iter().any(|n| n.starts_with("undefined")));
    let target = if want.is_infinite() { print::expr(fin, Style::Ascii) } else { short(want) };
    let lead = if p.is_infinite() { "far out" } else { "on both sides" };
    out.push(ck("limit", ok, format!("{lead}, {}: heading for {target}", notes.join(", "))));
}

fn tangent_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = req.var.value.as_str();
    out.push(steps_keep(path, cfg, cfg.check.tolerance, "line", &|m, k| match m {
        Math::Eq(_, r) => r.eval_f(&env(req, cfg, k)),
        _ => f64::NAN,
    }));
    let (Math::Eq(_, line), Some(pt)) = (&path.state, &req.calc.point) else { return };
    let (_, f) = req.curve();
    let Some(p) = poly::from_expr(line, v) else {
        out.push(ck("tangent", false, "the answer isn't a straight line"));
        return;
    };
    let (m, c) = (p.coef(1), p.coef(0));
    let a = pt.value.eval_f(&|_| f64::NAN);
    let on = |x: f64| at_x(&f, v, x, req, cfg, 0);
    let touch = on(a);
    let slope = Expr::Deriv(Box::new(f.clone()), v.to_string()).eval_f(&|n: &str| if n == v { a } else { f64::NAN });
    let line_at = m.to_f64() * a + c.to_f64();
    let ok = close(touch, line_at, cfg.check.tolerance) && close(slope, m.to_f64(), cfg.check.tolerance);
    out.push(ck("tangent", ok, format!("at {v} = {} the curve is at {} with slope {} (measured numerically); the line is at {} with slope {m}", print::expr(&pt.value, Style::Ascii), q::decimal(touch, 6), q::decimal(slope, 6), q::decimal(line_at, 6))));
}

/// The n-th derivative by central differences: sum (-1)^j C(n, j) f(x + (n/2 - j)h) / h^n.
fn nth_difference(f: &Expr, v: &str, n: u32, x: f64, h: f64, req: &Request, cfg: &Config, k: usize) -> f64 {
    let mut sum = 0.0;
    let mut binom = 1.0;
    for j in 0..=n {
        let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
        sum += sign * binom * at_x(f, v, x + (n as f64 / 2.0 - j as f64) * h, req, cfg, k);
        binom = binom * (n - j) as f64 / (j + 1) as f64;
    }
    sum / h.powi(n as i32)
}

fn higher_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let v = req.var.value.as_str();
    let tol = cfg.calculus.higher_tolerance;
    // d/dx nested three deep is too noisy to evaluate numerically: not measured
    fn depth(e: &Expr) -> usize {
        let inner = e.children().into_iter().map(depth).max().unwrap_or(0);
        inner + usize::from(matches!(e, Expr::Deriv(..)))
    }
    out.push(steps_keep(path, cfg, tol, "value", &|m, k| match m {
        Math::Expr(e) if depth(e) <= 2 => e.eval_f(&env(req, cfg, k)),
        _ => f64::NAN,
    }));
    let (Math::Expr(f), Math::Expr(fin)) = (&req.problem.value, &path.state) else { return };
    let n = req.calc.order();
    let mut count = 0;
    for k in 0..cfg.check.samples.len() {
        let x = env(req, cfg, k)(v);
        let (want, got) = (nth_difference(f, v, n, x, cfg.calculus.higher_step, req, cfg, k), fin.eval_f(&env(req, cfg, k)));
        if !want.is_finite() || !got.is_finite() {
            continue;
        }
        if !close(want, got, tol) {
            out.push(ck("derivative", false, format!("{} vs {} measured at {v} = {}", q::decimal(got, 6), q::decimal(want, 6), q::decimal(x, 3))));
            return;
        }
        count += 1;
    }
    out.push(if count >= 3 { ck("derivative", true, format!("matches the {} difference measured numerically at {count} points", if n == 2 { "second" } else { "third" })) } else { ck("derivative", false, "undefined at the sample points") });
}

/// Is this an equation with a trig function of the letter?
pub fn trig_equation(req: &Request) -> bool {
    let v = &req.var.value;
    let Math::Eq(l, r) = &req.problem.value else { return false };
    [l, r].iter().any(|s| s.walk().iter().any(|(_, n)| matches!(n, Expr::Func(expr::Func::Sin | expr::Func::Cos | expr::Func::Tan | expr::Func::Sec | expr::Func::Csc | expr::Func::Cot, a) if a.has_var(v))))
}

/// The interval a trig equation is solved in: the bounds asked for, or one turn.
fn trig_interval(req: &Request) -> (f64, f64, bool) {
    match &req.calc.bounds {
        Some(b) => (b.value.0.eval_f(&|_| f64::NAN), b.value.1.eval_f(&|_| f64::NAN), true),
        None => (0.0, 2.0 * std::f64::consts::PI, false),
    }
}

/// Every root of g in [lo, hi]: sign changes, bisected, and touching zeros.
fn scan_roots(g: &dyn Fn(f64) -> f64, lo: f64, hi: f64, cfg: &Config) -> Vec<f64> {
    let n = ((hi - lo) * cfg.calculus.trig_scan as f64).ceil().max(10.0) as usize;
    let xs: Vec<f64> = (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|&x| g(x)).collect();
    let tol = cfg.check.tolerance;
    let mut roots: Vec<f64> = Vec::new();
    let push = |r: f64, roots: &mut Vec<f64>| {
        if g(r).abs() < tol && !roots.iter().any(|x| (x - r).abs() < 1e3 * tol) {
            roots.push(r);
        }
    };
    for i in 0..=n {
        if ys[i] == 0.0 {
            push(xs[i], &mut roots);
        }
        if i < n && ys[i].is_finite() && ys[i + 1].is_finite() && ys[i] * ys[i + 1] < 0.0 {
            let (mut a, mut b) = (xs[i], xs[i + 1]);
            for _ in 0..100 {
                let m = (a + b) / 2.0;
                if g(a) * g(m) <= 0.0 {
                    b = m;
                } else {
                    a = m;
                }
            }
            push((a + b) / 2.0, &mut roots);
        }
        // touching zero without crossing: a local minimum of |g|
        if i > 0 && i < n && ys[i].abs() <= ys[i - 1].abs() && ys[i].abs() <= ys[i + 1].abs() && ys[i].abs() < (xs[1] - xs[0]) {
            let (mut a, mut b) = (xs[i - 1], xs[i + 1]);
            for _ in 0..200 {
                let (m1, m2) = (a + (b - a) / 3.0, b - (b - a) / 3.0);
                if g(m1).abs() < g(m2).abs() {
                    b = m2;
                } else {
                    a = m1;
                }
            }
            push((a + b) / 2.0, &mut roots);
        }
    }
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    roots
}

fn trig_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    use crate::rules::trig_solve::K;
    let v = req.var.value.as_str();
    let Math::Eq(l, r) = &req.problem.value else { return };
    let tol = cfg.check.tolerance;
    let g = |x: f64| at_x(l, v, x, req, cfg, 0) - at_x(r, v, x, req, cfg, 0);
    let sols = claimed(&path.state, v);
    let kk = cfg.calculus.k_range;
    // what each answer stands for: one number, or a family over k
    let value = |s: &Expr, k: i64| s.eval_f(&|n: &str| if n == K { k as f64 } else { f64::NAN });
    let mut ok = true;
    let mut notes = Vec::new();
    for s in &sols {
        let ks: Vec<i64> = if s.has_var(K) { (-kk..=kk).collect() } else { vec![0] };
        let bad = ks.iter().find(|&&k| {
            let x = value(s, k);
            let (a, b) = (at_x(l, v, x, req, cfg, 0), at_x(r, v, x, req, cfg, 0));
            !(a.is_finite() && close(a, b, tol))
        });
        let shown = format!("{v} = {}", print::expr(s, Style::Ascii));
        match bad {
            Some(k) => {
                ok = false;
                notes.push(format!("{shown} fails at k = {k}"));
            }
            None if s.has_var(K) => notes.push(format!("{shown} holds for k = {}..{kk}", -kk)),
            None => notes.push(format!("{shown} holds")),
        }
    }
    if !sols.is_empty() {
        out.push(ck("satisfies", ok, notes.join("; ")));
    } else {
        let tried: Vec<f64> = (0..cfg.check.samples.len()).map(|k| g(env(req, cfg, k)(v))).filter(|y| y.is_finite()).collect();
        let held = tried.iter().filter(|y| y.abs() <= tol).count();
        out.push(ck("samples", tried.len() >= 3 && held == 0, format!("the equation fails at all {} sample points tried", tried.len())));
    }
    // no solution missed: scan the interval (or one turn) for every root
    let (lo, hi, closed) = trig_interval(req);
    let found = scan_roots(&g, lo, hi, cfg);
    let found: Vec<f64> = found.into_iter().filter(|&x| closed || x < hi - 1e3 * tol).collect();
    let mut claimed_here: Vec<f64> = Vec::new();
    for s in &sols {
        if s.has_var(K) {
            let (s0, s1) = (value(s, 0), value(s, 1));
            let period = (s1 - s0).abs();
            if !(period > 0.0) {
                continue;
            }
            let k0 = ((lo - s0) / period).floor() as i64 - 1;
            let k1 = ((hi - s0) / period).ceil() as i64 + 1;
            for k in k0.min(k1)..=k0.max(k1) {
                claimed_here.push(value(s, k));
            }
        } else {
            claimed_here.push(value(s, 0));
        }
    }
    let inside = |x: f64| x >= lo - 1e3 * tol && (if closed { x <= hi + 1e3 * tol } else { x < hi - 1e3 * tol });
    let mut want: Vec<f64> = claimed_here.into_iter().filter(|&x| inside(x)).collect();
    want.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    want.dedup_by(|a, b| (*a - *b).abs() < 1e3 * tol);
    let same = found.len() == want.len() && found.iter().zip(&want).all(|(a, b)| (a - b).abs() < 1e3 * tol);
    let range = format!("[{}, {}{}", q::decimal(lo, 3), q::decimal(hi, 3), if closed { "]" } else { ")" });
    let detail = match found.len() {
        1 => format!("a scan of {range} finds 1 solution"),
        n => format!("a scan of {range} finds {n} solutions"),
    };
    out.push(ck("complete", same, if same { format!("{detail}, the same as the answer") } else { format!("{detail}, but the answer gives {}", want.len()) }));
}

/// Why a calculus question can't be answered, when the numbers show it:
/// the sides of a limit disagree, or a definite integral is improper.
pub fn why_not(req: &Request, cfg: &Config) -> Option<String> {
    match (req.task.value, req.start()) {
        (Task::Limit, Math::Expr(Expr::Limit(f, v, p))) => {
            let at = p.eval_f(&|_| f64::NAN);
            if at.is_infinite() {
                // values far out that neither settle nor grow: nothing to read off
                let pts = approach(&f, &v, at, req, cfg);
                let (first, last) = (pts[0], pts[pts.len() - 1]);
                let (a, b) = (first.2, last.2);
                let settles = (a - b).abs() <= cfg.calculus.limit_tolerance * 1f64.max(a.abs()).max(b.abs());
                let grows = b.abs() > cfg.calculus.unbounded && b.abs() > a.abs();
                return (a.is_finite() && b.is_finite() && !settles && !grows).then(|| {
                    format!("the values far out don't settle: f({}) = {}, f({}) = {}, so there is no limit to read off", short(first.0 * at.signum()), short(a), short(last.0 * at.signum()), short(b))
                });
            }
            let (l, r) = sides(&f, &v, at, req, cfg);
            let say = |x: f64| if x.is_infinite() { if x > 0.0 { "infinity".to_string() } else { "-infinity".to_string() } } else { short(x) };
            let point = print::expr(&p, Style::Ascii);
            match (l.is_nan(), r.is_nan()) {
                (true, true) => Some(format!("the function is undefined on both sides of {v} = {point}")),
                (false, false) if !(l == r || (l.is_finite() && r.is_finite() && close(l, r, cfg.calculus.limit_tolerance))) => {
                    Some(format!("the limit doesn't exist: from the left it heads for {}, from the right for {}", say(l), say(r)))
                }
                _ => None,
            }
        }
        (Task::Integrate, Math::Expr(Expr::Bounds(g, v, a, b))) => {
            let Expr::Integral(f, _) = *g else { return None };
            match quadrature(&f, &v, a.eval_f(&|_| f64::NAN), b.eval_f(&|_| f64::NAN), req, cfg) {
                Err(t) => Some(format!("the integrand is undefined at {v} = {}, inside the interval: improper integrals are not in Nuome v0", short(t))),
                Ok(_) => None,
            }
        }
        (Task::Solve, _) if trig_equation(req) => Some("Nuome v0 solves sin, cos or tan of ax + b = c at the table's angles, and sin x, cos x, tan x = c in general with arcsin, arccos, arctan (not in an interval or in degrees)".into()),
        _ => None,
    }
}
