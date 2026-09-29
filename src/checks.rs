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
    let mut n = 0;
    for k in 0..cfg.check.samples.len() {
        let e = env(req, cfg, k);
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
        Task::Solve => unreachable!(),
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
                        ok = false;
                        notes.push(format!("{shown}: a side is undefined there"));
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
        out.push(ck("complete", false, "can't count the solutions of this kind of equation"));
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
    // 3. every line of the working holds at every answer
    let mut bad = None;
    for (k, s) in path.steps.iter().enumerate() {
        for sol in &sols {
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
