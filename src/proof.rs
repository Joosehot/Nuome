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
