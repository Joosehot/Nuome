//! Scores: the profile the sentence asks for, what each step is worth, the
//! judges that look at a step in the context of the whole path, and the
//! estimate of how far a state is from done (to rank unfinished paths).

use crate::config::{Axes, Config};
use crate::expr::{Expr, Math};
use crate::model::{Request, Task};
use crate::rules::Move;

/// The profile: [profile] shifted by every modifier in the sentence.
pub fn profile(req: &Request, cfg: &Config) -> Axes {
    req.modifiers.iter().fold(cfg.profile, |p, m| p.add(&cfg.modifiers[m.value.key()]))
}

pub fn is_minor(rule: &str, cfg: &Config) -> bool {
    cfg.rules[rule].minor
}

/// What a step is worth on its own: its variant's axes against the
/// profile, minus the cost of writing a step down.
pub fn step_score(mv: &Move, p: &Axes, cfg: &Config) -> f64 {
    let rc = &cfg.rules[mv.rule];
    let value = rc.weight * rc.variants[mv.variant].dot(p);
    let cost = cfg.steps.cost + cfg.steps.brevity_cost * p.brevity.max(0.0);
    value - if rc.minor { cost * cfg.steps.minor_share } else { cost }
}

#[derive(Clone, Debug)]
pub struct Note {
    pub judge: &'static str,
    pub score: f64,
    pub why: String,
}

/// Numbers written as fractions, and number divisions still to do.
fn fractions(m: &Math) -> usize {
    m.slots()
        .iter()
        .flat_map(|e| e.walk())
        .filter(|(_, n)| match n {
            Expr::Num(q) => !q.is_int(),
            Expr::Div(_, d) => d.as_num().is_some(),
            _ => false,
        })
        .count()
}

/// Does the letter stand with a minus in front on its side: -2x = 6?
fn negative_lead(m: &Math, v: &str) -> bool {
    let Math::Eq(l, r) = m else { return false };
    let side = match (l.has_var(v), r.has_var(v)) {
        (true, false) => l,
        (false, true) => r,
        _ => return false,
    };
    let ts = crate::expr::terms(side);
    let var_terms: Vec<&Expr> = ts.iter().filter(|t| t.has_var(v)).collect();
    var_terms.len() == 1 && crate::expr::coeff(var_terms[0]).0.is_neg()
}

/// Plain number work waiting to be done: 7 - 3, 2 * 3, 4/2, 3^2.
fn pending_arithmetic(m: &Math) -> bool {
    m.slots().iter().any(|e| {
        e.walk().iter().any(|(_, n)| match n {
            Expr::Add(v) | Expr::Mul(v) => v.iter().filter(|t| t.as_num().is_some()).count() >= 2,
            Expr::Div(a, b) | Expr::Pow(a, b) => a.as_num().is_some() && b.as_num().is_some(),
            _ => false,
        })
    })
}

/// The judges' notes on taking `mv` after `path` (the rules used so far).
pub fn judge(mv: &Move, before: &Math, path: &[&'static str], req: &Request, p: &Axes, cfg: &Config) -> Vec<Note> {
    let mut notes = Vec::new();
    let mut add = |judge: &'static str, amount: f64, why: String| {
        let j = &cfg.judges[judge];
        let score = j.weight * j.axes.dot(p) * amount;
        if score != 0.0 {
            notes.push(Note { judge, score, why });
        }
    };
    let task = req.task.value;
    if matches!(task, Task::Solve | Task::Evaluate) {
        let (a, b) = (fractions(before), fractions(&mv.result));
        // a fraction the answer itself needs is fine; making one early isn't
        let solved = match &mv.result {
            Math::Eq(l, _) => matches!(l, Expr::Var(_)),
            Math::Or(eqs) => eqs.iter().all(|(l, _)| matches!(l, Expr::Var(_))),
            _ => false,
        };
        if b > a && !solved {
            add("fractions", (b - a) as f64, format!("{} new fraction(s) before the end", b - a));
        }
    }
    if task == Task::Solve && negative_lead(&mv.result, &req.var.value) && !negative_lead(before, &req.var.value) {
        add("negative_lead", 1.0, format!("{} left with a minus sign in front", req.var.value));
    }
    if let Some(g) = cfg.group_of(mv.rule) {
        if let Some(prev) = path.iter().find(|r| cfg.group_of(r) == Some(g) && **r != mv.rule && !cfg.follows(r, mv.rule)) {
            add("method_switch", 1.0, format!("{} after starting with {prev}", mv.rule));
        }
    }
    if task == Task::Solve && mv.rule == "move_term" {
        let brackets = before.slots().iter().any(|e| e.walk().iter().any(|(_, n)| matches!(n, Expr::Mul(f) if f.iter().any(|x| matches!(x, Expr::Add(_))))));
        if brackets && !matches!(before, Math::Eq(_, r) if r.is_num(0)) {
            add("brackets_first", 1.0, "moves terms before multiplying out brackets".to_string());
        }
    }
    if !cfg.rules[mv.rule].minor && pending_arithmetic(before) {
        add("arithmetic_first", 1.0, "a real step while number work is pending".to_string());
    }
    if let (Math::Or(a), Math::Or(b)) = (before, &mv.result) {
        if a.len() == b.len() {
            let v = &req.var.value;
            let done = |(l, r): &(Expr, Expr)| *l == Expr::Var(v.clone()) && !r.has_var(v);
            if let Some(k) = (0..a.len()).find(|&k| a[k] != b[k]) {
                if (0..k).any(|i| !done(&a[i])) {
                    add("branch_order", 1.0, format!("works on alternative {} before alternative 1 is done", k + 1));
                }
            }
        }
    }
    let grow = mv.result.size() as f64 - before.size() as f64;
    if grow > 0.0 && task != Task::Expand && task != Task::Differentiate {
        add("growth", grow, format!("grows by {grow}"));
    }
    notes
}

/// How far a state is from done, roughly in steps.
pub fn distance(m: &Math, req: &Request) -> f64 {
    let v = &req.var.value;
    let unfinished = |e: &Expr| -> f64 {
        e.walk()
            .iter()
            .map(|(_, n)| match n {
                Expr::Deriv(a, _) => 3.0 + a.size() as f64,
                Expr::Mul(f) if matches!(req.task.value, Task::Expand | Task::Simplify) && f.iter().filter(|x| matches!(x, Expr::Add(_))).count() > 0 => 3.0,
                Expr::Pow(b, _) if matches!(req.task.value, Task::Expand | Task::Simplify) && matches!(**b, Expr::Add(_)) => 3.0,
                _ => 0.0,
            })
            .sum()
    };
    match m {
        Math::Expr(e) => e.size() as f64 * 0.5 + unfinished(e),
        Math::Eq(l, r) => eq_distance(l, r, v),
        Math::Or(eqs) => eqs.iter().map(|(l, r)| eq_distance(l, r, v)).sum(),
        Math::NoSolution | Math::AllReals => 0.0,
    }
}

fn eq_distance(l: &Expr, r: &Expr, v: &str) -> f64 {
    if *l == Expr::Var(v.to_string()) && !r.has_var(v) {
        return (r.size() - 1) as f64 * 0.5;
    }
    let both = l.has_var(v) && r.has_var(v);
    (l.size() + r.size()) as f64 * 0.5 + if both { 3.0 } else { 1.0 }
}
