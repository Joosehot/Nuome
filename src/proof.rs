//! Checking proofs. A proof is a path that ends in `Math::Proved`; like every
//! other answer it is shown only if checks independent of the rules confirm
//! it. Each kind of statement has its own checker:
//!
//! - identities (`A = B` for all values of the letters): polynomial sides are
//!   compared exactly on a grid of (degree + 1) points per letter, which is
//!   a proof for polynomials; anything else is compared numerically at many
//!   points, which is evidence, and the check says so
//! - every step of the proof keeps each side's value
//!
//! Other kinds of statement (inequalities, induction, logic, sets, groups)
//! add their own checker to `checks` below.

use crate::checks::Check;
use crate::config::Config;
use crate::calls::Named;
use crate::expr::{self, series_parts, terms, Expr, Math, Rel};
use crate::model::Request;
use crate::print::{self, Style};
use crate::q::Q;
use crate::search::Path;
use std::collections::BTreeSet;

fn ck(name: &'static str, ok: bool, detail: impl Into<String>) -> Check {
    Check { name, ok, detail: detail.into() }
}

pub fn checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    if path.state != Math::Proved {
        out.push(ck("proved", false, "the proof doesn't end with the statement proved"));
        return;
    }
    match &req.problem.value {
        // logic and sets (agent L)
        m if crate::logic::is_statement(m) => logic_checks(req, cfg, path, out),
        Math::Eq(l, r) if l.has_call() || r.has_call() => induction(l, r, cfg, path, out),
        Math::Eq(l, r) => {
            if let Some(why) = circular(path) {
                out.push(ck("circular", false, why));
                return;
            }
            out.push(identity(l, r, cfg));
            out.push(steps_keep_sides(req, cfg, path));
        }
        Math::Expr(Expr::Call(Named::Divides, a)) => divides(a, cfg, path, out),
        Math::Ineq(l, rel, r) => inequality(l, *rel, r, cfg, path, out),
        m => out.push(ck("proof", false, format!("no checker for statements like {}", print::math(m, Style::Ascii)))),
    }
}

/// An upper bound on the power of `v` in `e`, read off the structure, when
/// `e` is a polynomial in `v` (other letters count as constants).
fn degree_in(e: &Expr, v: &str) -> Option<usize> {
    match e {
        Expr::Num(_) | Expr::Const(_) => Some(0),
        Expr::Var(x) => Some(usize::from(x == v)),
        Expr::Add(ts) => ts.iter().map(|t| degree_in(t, v)).try_fold(0, |a, d| Some(a.max(d?))),
        Expr::Mul(fs) => fs.iter().map(|t| degree_in(t, v)).try_fold(0, |a, d| Some(a + d?)),
        Expr::Neg(a) => degree_in(a, v),
        Expr::Div(a, b) if !b.has_var(v) && b.vars().is_empty() => degree_in(a, v),
        Expr::Pow(b, n) => {
            let n = n.as_num().filter(|q| q.is_int() && !q.is_neg())?;
            Some(degree_in(b, v)? * usize::try_from(n.num()).ok()?)
        }
        e if !e.has_var(v) && e.vars().is_empty() => Some(0),
        _ => None,
    }
}

/// e as (numerator, denominator), both free of letter denominators.
fn as_fraction(e: &Expr) -> Option<(Expr, Expr)> {
    let one = || expr::num(1);
    Some(match e {
        Expr::Add(ts) => {
            let mut acc = (expr::num(0), one());
            for t in ts {
                let (n, d) = as_fraction(t)?;
                acc = (expr::add(vec![expr::mul(vec![acc.0, d.clone()]), expr::mul(vec![n, acc.1.clone()])]), expr::mul(vec![acc.1, d]));
            }
            acc
        }
        Expr::Mul(fs) => {
            let mut acc = (one(), one());
            for f in fs {
                let (n, d) = as_fraction(f)?;
                acc = (expr::mul(vec![acc.0, n]), expr::mul(vec![acc.1, d]));
            }
            acc
        }
        Expr::Neg(a) => {
            let (n, d) = as_fraction(a)?;
            (expr::neg(n), d)
        }
        Expr::Div(a, b) => {
            let (n1, d1) = as_fraction(a)?;
            let (n2, d2) = as_fraction(b)?;
            (expr::mul(vec![n1, d2]), expr::mul(vec![d1, n2]))
        }
        Expr::Pow(b, k) => {
            let k = k.as_num().filter(|q| q.is_int())?;
            let (n, d) = as_fraction(b)?;
            let p = |x: Expr| expr::pow(x, expr::num(k.num().abs()));
            if k.is_neg() {
                (p(d), p(n))
            } else {
                (p(n), p(d))
            }
        }
        Expr::Num(_) | Expr::Var(_) | Expr::Const(_) => (e.clone(), one()),
        _ => return None,
    })
}

/// Is `l = r` for every value of the letters?
pub fn identity(l: &Expr, r: &Expr, cfg: &Config) -> Check {
    // fractions with letters below: cross-multiply, then it is a polynomial identity
    let has_letter_denominator = [l, r].iter().any(|e| e.walk().iter().any(|(_, n)| matches!(n, Expr::Div(_, d) if !d.vars().is_empty())));
    if has_letter_denominator {
        if let (Some((nl, dl)), Some((nr, dr))) = (as_fraction(l), as_fraction(r)) {
            let (a, b) = (expr::mul(vec![nl, dr]), expr::mul(vec![nr, dl]));
            let c = identity(&a, &b, cfg);
            if c.ok && c.detail.starts_with("both sides are polynomials") {
                return ck("identity", true, format!("cross-multiplied, {}; so the sides are equal wherever both are defined", c.detail));
            }
        }
    }
    let diff = crate::expr::add(vec![l.clone(), crate::expr::neg(r.clone())]);
    let vars: BTreeSet<String> = diff.vars();
    let shown = print::math(&Math::Eq(l.clone(), r.clone()), Style::Ascii);
    // polynomial: exact on a grid big enough to pin a polynomial down
    let degrees: Option<Vec<usize>> = vars.iter().map(|v| degree_in(&diff, v)).collect();
    if let Some(ds) = degrees {
        let points: usize = ds.iter().map(|d| d + 1).product();
        if points <= cfg.proof.max_grid {
            let names: Vec<&String> = vars.iter().collect();
            let mut idx = vec![0usize; names.len()];
            loop {
                let at = |n: &str| names.iter().position(|v| v.as_str() == n).map(|k| Q::int(idx[k] as i128 - 2));
                match diff.eval_q(&at) {
                    Some(x) if x.is_zero() => {}
                    Some(_) => return ck("identity", false, format!("{shown} fails at a whole-number point")),
                    None => break, // a division by zero on the grid: fall back to sampling
                }
                // next grid point
                let mut k = 0;
                while k < idx.len() {
                    idx[k] += 1;
                    if idx[k] <= ds[k] {
                        break;
                    }
                    idx[k] = 0;
                    k += 1;
                }
                if k == idx.len() {
                    let per: Vec<String> = vars.iter().zip(&ds).map(|(v, d)| format!("{} values of {v}", d + 1)).collect();
                    return ck("identity", true, format!("both sides are polynomials and agree exactly at all {points} points of a grid ({}), one more value per letter than its degree: that forces them to be equal", per.join(", ")));
                }
            }
        }
    }
    // anything else: many points, as evidence
    let mut tried = 0;
    for k in 0..cfg.proof.samples {
        let env = |n: &str| {
            let i = vars.iter().position(|v| v == n).unwrap_or(0) as f64;
            0.37 + 0.731 * k as f64 - 0.29 * i * (k % 3) as f64
        };
        let (a, b) = (l.eval_f(&env), r.eval_f(&env));
        if !(a.is_finite() && b.is_finite()) {
            continue;
        }
        tried += 1;
        if (a - b).abs() > cfg.check.tolerance * 1f64.max(a.abs()).max(b.abs()) {
            return ck("identity", false, format!("{shown} fails numerically ({a} vs {b})"));
        }
    }
    ck("identity", tried >= 10, format!("both sides agree at {tried} points (numerical evidence; the proof itself is the chain of rules)"))
}

/// Every step rewrites inside the sides, so each side keeps its value.
fn steps_keep_sides(req: &Request, cfg: &Config, path: &Path) -> Check {
    for (k, s) in path.steps.iter().enumerate() {
        let (a, b) = (s.before.slots(), s.mv.result.slots());
        if a.len() != b.len() {
            continue; // the last step: the statement is proved
        }
        for (x, y) in a.iter().zip(&b) {
            let c = identity(x, y, cfg);
            if !c.ok {
                return ck("steps", false, format!("step {} ({}) changes a side: {}", k + 1, s.mv.rule, c.detail));
            }
        }
    }
    let _ = req;
    ck("steps", true, if path.steps.len() == 1 { "the step keeps both sides' values".to_string() } else { format!("each of the {} steps keeps both sides' values", path.steps.len()) })
}

/// A proof that only cites the law it is asked to prove proves nothing.
fn circular(path: &Path) -> Option<String> {
    const LAWS: &[(&str, &str)] = &[("trig_identity", "the unit-circle definition of sin and cos"), ("log_laws", "the definition of the logarithm")];
    let real: Vec<&str> = path.steps.iter().map(|s| s.mv.rule).filter(|r| *r != "sides_equal").collect();
    match real.as_slice() {
        [only] => LAWS.iter().find(|(r, _)| r == only).map(|(r, from)| format!("the statement is Nuome's own {r} law, which it takes as given; proving it needs {from}, which Nuome doesn't have")),
        _ => None,
    }
}

/// Induction: the base case exactly, the step as an identity, and the formula
/// confirmed exactly for the first values.
fn induction(l: &Expr, r: &Expr, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let (sum, formula) = match (l, r) {
        (Expr::Call(Named::Series, a), f) | (f, Expr::Call(Named::Series, a)) => (a, f),
        _ => {
            out.push(ck("proof", false, "no checker for this statement"));
            return;
        }
    };
    let Some((term, lo, n)) = series_parts(sum) else { return };
    let Some(s) = lo.as_num() else { return };
    let whole = Expr::Call(Named::Series, sum.clone());
    let at = |e: &Expr, k: Q| e.subst(n, &Expr::Num(k)).eval_q(&|_| None);
    // exact values: sum and formula for n = s .. s + N
    let mut bad = None;
    for k in 0..cfg.proof.induction_checks {
        let v = s.add(&Q::int(k as i128)).expect("small");
        let lhs = whole.eval_q(&|x| (x == n).then_some(v));
        if lhs.is_none() || lhs != at(formula, v) {
            bad = Some(v);
            break;
        }
    }
    out.push(match bad {
        Some(v) => ck("values", false, format!("the formula is wrong at {n} = {v}")),
        None => ck("values", true, format!("sum and formula agree exactly for {n} = {s} to {}", s.num() + cfg.proof.induction_checks as i128 - 1)),
    });
    out.push(match (at(term, s), at(formula, s)) {
        (Some(a), Some(b)) if a == b => ck("base", true, format!("{n} = {s}: both sides are {a}")),
        _ => ck("base", false, format!("the base case {n} = {s} fails")),
    });
    let next = expr::add(vec![expr::var(n), expr::num(1)]);
    let step = identity(&expr::add(vec![formula.clone(), term.subst(n, &next)]), &formula.subst(n, &next), cfg);
    out.push(ck("step", step.ok, format!("formula({n}) + next term = formula({n} + 1): {}", step.detail)));
    if !path.uses("induction") {
        out.push(ck("method", false, "the proof doesn't use induction"));
    }
}

/// d divides e(n) for every integer n: e's value mod d repeats with period
/// d * (common denominator of its coefficients), so one period of exact
/// values decides it.
fn divides(a: &[Expr], cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let _ = cfg;
    let [d, e] = a else { return };
    let Some(d) = d.as_num().filter(|q| q.is_int()).map(|q| q.num()) else { return };
    let vars = e.vars();
    let Some(v) = vars.iter().next().filter(|_| vars.len() == 1) else {
        out.push(ck("divides", false, "only one letter is supported"));
        return;
    };
    let Some(p) = crate::poly::from_expr(e, v) else {
        out.push(ck("divides", false, "not a polynomial"));
        return;
    };
    let den = p.0.iter().fold(1i128, |acc, c| crate::q::lcm(acc, c.den()).unwrap_or(acc));
    let period = d * den;
    let mut bad = None;
    for k in 0..period {
        match p.eval(&Q::int(k)) {
            Some(x) if x.is_int() && x.num().rem_euclid(d) == 0 => {}
            x => {
                bad = Some((k, x));
                break;
            }
        }
    }
    out.push(match bad {
        Some((k, x)) => ck("divides", false, format!("{v} = {k} gives {}, not a multiple of {d}", x.map_or("no whole number".into(), |q| q.to_string()))),
        None => ck("divides", true, format!("its value mod {d} repeats every {period}, and {v} = 0 to {} all give multiples of {d}: that covers every integer", period - 1)),
    });
    // and, separately, straight computation for the integers near 0
    let direct = (-50i128..=50).all(|k| e.eval_q(&|n| (n == v).then_some(Q::int(k))).is_some_and(|x| x.is_int() && x.num().rem_euclid(d) == 0));
    out.push(ck("values", direct, if direct { format!("computed directly, {d} divides it for every {v} from -50 to 50") } else { format!("computed directly, it fails for some {v} between -50 and 50") }));
    // steps that rewrite the expression keep it equal
    let mut rewrites = 0;
    for s in &path.steps {
        if let (Math::Expr(Expr::Call(Named::Divides, x)), Math::Expr(Expr::Call(Named::Divides, y))) = (&s.before, &s.mv.result) {
            rewrites += 1;
            let c = identity(&x[1], &y[1], cfg);
            if !c.ok {
                out.push(ck("steps", false, format!("{} changes the expression: {}", s.mv.rule, c.detail)));
                return;
            }
        }
    }
    if rewrites > 0 {
        out.push(ck("steps", true, "the factoring keeps the expression equal"));
    }
}

/// Independently of the rules: is e a sum of positive multiples of even
/// powers and positive numbers (with one positive number if strict)?
fn sum_of_squares(e: &Expr, strict: bool) -> bool {
    let mut positive_number = false;
    for t in terms(e) {
        let (c, rest) = expr::coeff(&t);
        if c.is_neg() || c.is_zero() {
            return false;
        }
        if rest.is_num(1) {
            positive_number = true;
            continue;
        }
        let factors = match rest {
            Expr::Mul(v) => v,
            r => vec![r],
        };
        if !factors.iter().all(|f| matches!(f, Expr::Pow(_, k) if k.as_num().is_some_and(|q| q.is_int() && q.num() > 0 && q.num() % 2 == 0))) {
            return false;
        }
    }
    positive_number || !strict
}

fn inequality(l: &Expr, rel: Rel, r: &Expr, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    // every step keeps (left - right) and the sign
    let gap = |a: &Expr, b: &Expr| expr::add(vec![a.clone(), expr::neg(b.clone())]);
    for (k, s) in path.steps.iter().enumerate() {
        match (&s.before, &s.mv.result) {
            (Math::Ineq(a, ra, b), Math::Ineq(c, rc, d)) => {
                let same = identity(&gap(a, b), &gap(c, d), cfg);
                if ra != rc || !same.ok {
                    out.push(ck("steps", false, format!("step {} ({}) changes the inequality", k + 1, s.mv.rule)));
                    return;
                }
            }
            (Math::Ineq(a, ra, b), Math::Proved) => {
                let ok = b.is_num(0) && matches!(ra, Rel::Ge | Rel::Gt) && sum_of_squares(a, *ra == Rel::Gt);
                out.push(ck("final", ok, if ok { format!("{} is a sum of squares times positive numbers{}", print::expr(a, Style::Ascii), if *ra == Rel::Gt { " plus a positive number" } else { "" }) } else { "the last step's claim doesn't hold".into() }));
                if !ok {
                    return;
                }
            }
            _ => {}
        }
    }
    out.push(ck("steps", true, "each step keeps left side minus right side exactly"));
    // and, as evidence, the original at many points
    let vars = gap(l, r).vars();
    let mut tried = 0;
    for k in 0..cfg.proof.samples {
        let env = |n: &str| {
            let i = vars.iter().position(|v| v == n).unwrap_or(0) as f64;
            -3.1 + 0.113 * k as f64 + 0.71 * i * ((k % 5) as f64 - 2.0)
        };
        let (a, b) = (l.eval_f(&env), r.eval_f(&env));
        if !(a.is_finite() && b.is_finite()) {
            continue;
        }
        tried += 1;
        let holds = match rel {
            Rel::Ge => a >= b - 1e-9,
            Rel::Gt => a > b,
            Rel::Le => a <= b + 1e-9,
            Rel::Lt => a < b,
        };
        if !holds {
            out.push(ck("samples", false, format!("it fails at a sample point ({a} vs {b})")));
            return;
        }
    }
    out.push(ck("samples", true, format!("it holds at all {tried} sample points tried")));
}

/// For a statement that isn't true: a concrete case where it fails.
pub fn counterexample(req: &Request) -> Option<String> {
    let show = |v: &std::collections::BTreeMap<String, Q>| v.iter().map(|(k, x)| format!("{k} = {x}")).collect::<Vec<_>>().join(", ");
    let points = |vars: &BTreeSet<String>| -> Vec<std::collections::BTreeMap<String, Q>> {
        let vals: Vec<Q> = [1, 2, 3, -1, 0, 5, -2, 7].iter().map(|&k| Q::int(k)).collect();
        let mut out = Vec::new();
        for (i, _) in vals.iter().enumerate() {
            out.push(vars.iter().enumerate().map(|(j, v)| (v.clone(), vals[(i + j * 3) % vals.len()])).collect());
        }
        out
    };
    match &req.problem.value {
        Math::Expr(Expr::Call(Named::Divides, a)) => {
            let [d, e] = &a[..] else { return None };
            let v = e.vars().into_iter().next()?;
            let d = d.as_num()?;
            (0..200).find_map(|k| {
                let x = e.eval_q(&|n| (n == v).then_some(Q::int(k)))?;
                (!x.div(&d)?.is_int()).then(|| format!("{v} = {k} gives {x}, which {} doesn't divide", d))
            })
        }
        Math::Eq(l, r) | Math::Ineq(l, _, r) => {
            let vars: BTreeSet<String> = l.vars().union(&r.vars()).cloned().collect();
            for p in points(&vars) {
                let (Some(a), Some(b)) = (l.eval_q(&|n| p.get(n).copied()), r.eval_q(&|n| p.get(n).copied())) else { continue };
                let fails = match &req.problem.value {
                    Math::Ineq(_, rel, _) => !match rel {
                        Rel::Ge => a >= b,
                        Rel::Gt => a > b,
                        Rel::Le => a <= b,
                        Rel::Lt => a < b,
                    },
                    _ => a != b,
                };
                if fails {
                    return Some(format!("it is false: {} gives {a} on the left and {b} on the right", show(&p)));
                }
            }
            None
        }
        _ => None,
    }
}

// ---- logic and sets (agent L) ----

/// "p, q and r".
fn listed(names: &[String]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.join(""),
    }
}

/// A statement of logic or about sets: it holds in every row of its truth
/// table (every region of its Venn diagram), all 2^n of them, which for n
/// letters is an exact, exhaustive proof; and every step keeps the truth
/// value of what it rewrites in every row.
fn logic_checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    use crate::logic;
    let m = &req.problem.value;
    let names = logic::case_names(m);
    let n = names.len();
    let quantified = logic::quantified(m);
    if (!quantified && n > cfg.logic.check_letters) || (quantified && n > cfg.logic.predicates) {
        out.push(ck("truth table", false, format!("{n} letters or predicates is more than Nuome checks exhaustively")));
        return;
    }
    let cases = logic::cases(m);
    let count = cases.len();
    let sets = matches!(m, Math::Eq(..) | Math::Subset(..));
    let bad = cases.iter().find(|c| logic::holds(m, &names, c) != Some(true));
    out.push(match bad {
        Some(c) => ck(if quantified { "domains" } else { "truth table" }, false, format!("fails for {}", logic::describe(&names, c, sets))),
        None => {
            let what = match m {
                _ if quantified => format!("true in all {count} kinds of domain for {} (one for each choice of which combinations of the predicates occur; for one-place predicates these are all the cases there are)", listed(&names)),
                Math::Taut(_) => format!("true in all {count} rows of its truth table ({})", listed(&names)),
                Math::Equiv(..) => format!("both sides agree in all {count} rows of the truth table ({})", listed(&names)),
                Math::Eq(..) => format!("both sides contain the same regions, in all {count} regions of the Venn diagram of {} (every element lies in exactly one)", listed(&names)),
                _ => format!("in all {count} regions of the Venn diagram of {}, a region inside the left side is inside the right", listed(&names)),
            };
            ck(if quantified { "domains" } else { "truth table" }, true, format!("{what}: every case, checked exactly"))
        }
    });
    // every step: a rewrite keeps each side's value in every case; a step that
    // changes the form of the statement (assume, chase an element) keeps the
    // statement's value in every case; the last step's claim holds in every case
    for (k, st) in path.steps.iter().enumerate() {
        let (a, b) = (&st.before, &st.mv.result);
        let fail = |why: String| ck("steps", false, format!("step {} ({}) {why}", k + 1, st.mv.rule));
        let every = |f: &dyn Fn(&logic::Case) -> bool| cases.iter().all(f);
        if *b == Math::Proved {
            if !every(&|c| logic::holds(a, &names, c) == Some(true)) {
                out.push(fail("claims a statement that fails in some case".into()));
                return;
            }
            continue;
        }
        let same_shape = std::mem::discriminant(a) == std::mem::discriminant(b) && a.slots().len() == b.slots().len();
        let kept = same_shape && a.slots().iter().zip(b.slots()).all(|(x, y)| every(&|c| logic::value(x, &names, c).is_some() && logic::value(x, &names, c) == logic::value(y, &names, c)));
        let meaning = every(&|c| logic::holds(a, &names, c).is_some() && logic::holds(a, &names, c) == logic::holds(b, &names, c));
        if !(kept || meaning) {
            out.push(fail("changes a truth value in some case".into()));
            return;
        }
    }
    let steps = path.steps.len();
    let cases = if quantified { format!("{count} kinds of domain") } else if sets { format!("{count} regions") } else { format!("{count} rows") };
    out.push(ck("steps", true, if steps == 1 { format!("the step's claim holds in all {cases}") } else { format!("each of the {steps} steps keeps every truth value in all {cases}") }));
}
