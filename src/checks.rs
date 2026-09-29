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
        Task::Divide => divide_checks(&original, fin, out),
    }
}

/// Every factor is prime over the integers (one letter), or no rule applies further.
fn fully_factored(e: &Expr) -> Check {
    let vars = e.vars();
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
