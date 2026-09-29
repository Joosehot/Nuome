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
use crate::expr::{Expr, Math};
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
        Math::Eq(l, r) => {
            out.push(identity(l, r, cfg));
            out.push(steps_keep_sides(req, cfg, path));
        }
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

/// Is `l = r` for every value of the letters?
pub fn identity(l: &Expr, r: &Expr, cfg: &Config) -> Check {
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
    let names = logic::letters(m);
    let n = names.len();
    if n > cfg.logic.check_letters {
        out.push(ck("truth table", false, format!("{n} letters is more than the {} whose truth tables Nuome checks", cfg.logic.check_letters)));
        return;
    }
    let rows = 1usize << n;
    let sets = matches!(m, Math::Eq(..) | Math::Subset(..));
    let bad = (0..rows).map(|k| logic::row(n, k)).find(|v| logic::at(m, &names, v) != Some(true));
    out.push(match bad {
        Some(v) => ck("truth table", false, format!("fails when {}", logic::describe_row(&names, &v, sets))),
        None => {
            let what = match m {
                Math::Taut(_) => format!("true in all {rows} rows of its truth table ({})", listed(&names)),
                Math::Equiv(..) => format!("both sides agree in all {rows} rows of the truth table ({})", listed(&names)),
                Math::Eq(..) => format!("both sides contain the same regions, in all {rows} regions of the Venn diagram of {} (every element lies in exactly one)", listed(&names)),
                _ => format!("in all {rows} regions of the Venn diagram of {}, a region inside the left side is inside the right", listed(&names)),
            };
            ck("truth table", true, format!("{what}: every case, checked exactly"))
        }
    });
    // every step: a rewrite keeps each side's value in every row; a step that
    // changes the form of the statement (assume, chase an element) keeps the
    // statement's value in every row; the last step's claim holds in every row
    for (k, st) in path.steps.iter().enumerate() {
        let (a, b) = (&st.before, &st.mv.result);
        let fail = |why: String| ck("steps", false, format!("step {} ({}) {why}", k + 1, st.mv.rule));
        let every = |f: &dyn Fn(&[bool]) -> bool| (0..rows).map(|r| logic::row(n, r)).all(|v| f(&v));
        if *b == Math::Proved {
            if !every(&|v| logic::at(a, &names, v) == Some(true)) {
                out.push(fail("claims a statement that fails in some row".into()));
                return;
            }
            continue;
        }
        let same_shape = std::mem::discriminant(a) == std::mem::discriminant(b) && a.slots().len() == b.slots().len();
        let kept = same_shape && a.slots().iter().zip(b.slots()).all(|(x, y)| every(&|v| logic::value_at(x, &names, v).is_some() && logic::value_at(x, &names, v) == logic::value_at(y, &names, v)));
        let meaning = every(&|v| logic::at(a, &names, v).is_some() && logic::at(a, &names, v) == logic::at(b, &names, v));
        if !(kept || meaning) {
            out.push(fail("changes a truth value in some row".into()));
            return;
        }
    }
    let steps = path.steps.len();
    let cases = if sets { format!("{rows} regions") } else { format!("{rows} rows") };
    out.push(ck("steps", true, if steps == 1 { format!("the step's claim holds in all {cases}") } else { format!("each of the {steps} steps keeps every truth value in all {cases}") }));
}
