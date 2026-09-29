//! Beam search over solution paths.
//!
//! A path is a list of steps from the problem to an answer. Each round,
//! every path in the beam is extended by every move the task's rules offer;
//! a move scores its variant against the profile, minus the cost of a step,
//! plus the judges' notes. A move back to a state the path has already been
//! in is dropped. Paths are ranked by score minus the estimated distance to
//! done, and the beam keeps the best `search.beam`. A path where no rule
//! applies any more is finished if its state is an answer (and a dead end
//! if not). The finished paths are then checked, best score first; the
//! first that passes every check is the answer. Only if none passes is the
//! search rerun with the wider fallback beam. Ties keep candidate order, so
//! the result is deterministic.

use crate::checks::{self, Check};
use crate::config::{Axes, Config};
use crate::expr::{Expr, Math};
use crate::model::{Request, Task};
use crate::parser::Diag;
use crate::print::{self, Style};
use crate::rules::{Cx, Move};
use crate::scoring::{self, Note};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Step {
    pub mv: Move,
    pub before: Math,
    /// The step's own score (variant . profile - step cost).
    pub local: f64,
    pub notes: Vec<Note>,
}

impl Step {
    pub fn total(&self) -> f64 {
        self.local + self.notes.iter().map(|n| n.score).sum::<f64>()
    }
}

#[derive(Clone, Debug)]
pub struct Path {
    pub steps: Vec<Step>,
    pub state: Math,
    pub score: f64,
}

impl Path {
    pub fn describe(&self) -> String {
        if self.steps.is_empty() {
            return "(no steps)".into();
        }
        self.steps.iter().map(|s| format!("{}/{}", s.mv.rule, s.mv.variant)).collect::<Vec<_>>().join(" > ")
    }
    pub fn uses(&self, rule: &str) -> bool {
        self.steps.iter().any(|s| s.mv.rule == rule)
    }
}

#[derive(Clone, Debug)]
pub struct Finalist {
    pub path: Path,
    /// Empty when the finalist wasn't checked (a better one passed first).
    pub checks: Vec<Check>,
}

impl Finalist {
    pub fn passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|c| c.ok)
    }
}

pub struct Outcome {
    pub profile: Axes,
    pub width: usize,
    pub widened: bool,
    /// States expanded, over all rounds.
    pub expanded: usize,
    pub finalists: Vec<Finalist>,
    pub best: usize,
}

impl Outcome {
    pub fn path(&self) -> &Path {
        &self.finalists[self.best].path
    }
    pub fn checks(&self) -> &[Check] {
        &self.finalists[self.best].checks
    }
}

/// Is this state an answer to the task?
pub fn is_answer(m: &Math, req: &Request) -> bool {
    let v = &req.var.value;
    let solved = |l: &Expr, r: &Expr| *l == Expr::Var(v.clone()) && !r.has_var(v);
    match (req.task.value, m) {
        (Task::Solve, Math::NoSolution | Math::AllReals) => true,
        (Task::Solve, Math::Eq(l, r)) => solved(l, r),
        (Task::Solve, Math::Or(eqs)) => eqs.iter().all(|(l, r)| solved(l, r)),
        (Task::Solve, Math::Ineq(l, _, r)) => solved(l, r),
        (Task::Solve, Math::Intervals(..)) => true,
        (Task::Solve, Math::System(eqs)) => system_solved(eqs),
        (Task::Divide, Math::Expr(e)) => crate::rules::poly_divide::divided(e),
        (Task::Solve, _) => false,
        (Task::Evaluate, Math::Expr(e)) => e.vars().is_empty() && !e.has_deriv(),
        (_, Math::Expr(e)) => !e.has_deriv(),
        _ => false,
    }
}

/// Every equation reads "letter = ...", each letter once, and no right side
/// uses a solved letter (the others are free: infinitely many solutions).
pub fn system_solved(eqs: &[(Expr, Expr)]) -> bool {
    let lefts: Vec<&String> = eqs.iter().filter_map(|(l, _)| if let Expr::Var(v) = l { Some(v) } else { None }).collect();
    lefts.len() == eqs.len() && lefts.iter().enumerate().all(|(i, v)| !lefts[..i].contains(v)) && eqs.iter().all(|(_, r)| lefts.iter().all(|v| !r.has_var(v)))
}

struct Run {
    finished: Vec<Path>,
    dead: Vec<Path>,
    expanded: usize,
}

fn run(req: &Request, cfg: &Config, p: &Axes, width: usize) -> Run {
    let rules = cfg.task_rules(req.task.value);
    // "using the quadratic formula": the group's other methods are off, except
    // the forced method's own follow-ups once it has been used
    let forced = req.method.as_ref().and_then(|m| cfg.group_of(&m.value).map(|g| (g, m.value.as_str())));
    let allowed = |rule: &str, path: &Path| match forced {
        Some((g, m)) if cfg.group_of(rule) == Some(g) && rule != m => path.uses(m) && cfg.follows(m, rule),
        _ => true,
    };
    let cx = Cx { req, cfg, var: &req.var.value };
    let start = req.start();
    let mut beam = vec![Path { steps: vec![], state: start.clone(), score: 0.0 }];
    let mut seen: HashMap<Math, f64> = HashMap::from([(start, 0.0)]);
    let mut out = Run { finished: vec![], dead: vec![], expanded: 0 };
    for _ in 0..=cfg.search.max_steps {
        let mut next: Vec<Path> = Vec::new();
        let mut index: HashMap<Math, usize> = HashMap::new();
        for path in &beam {
            out.expanded += 1;
            let moves: Vec<Move> = rules.iter().filter(|r| allowed(r.name(), path)).flat_map(|r| r.moves(&path.state, &cx)).collect();
            if moves.is_empty() {
                if is_answer(&path.state, req) {
                    out.finished.push(path.clone());
                } else {
                    out.dead.push(path.clone());
                }
                continue;
            }
            let used: Vec<&'static str> = path.steps.iter().map(|s| s.mv.rule).collect();
            for mv in moves {
                if mv.result == path.state || path.steps.iter().any(|s| s.before == mv.result) {
                    continue; // no step may undo an earlier one
                }
                let local = scoring::step_score(&mv, p, cfg);
                let notes = scoring::judge(&mv, &path.state, &used, req, p, cfg);
                let step = Step { before: path.state.clone(), local, notes, mv };
                let score = path.score + step.total();
                let state = step.mv.result.clone();
                match index.get(&state) {
                    Some(&k) if next[k].score >= score => {}
                    Some(&k) => {
                        let mut steps = path.steps.clone();
                        steps.push(step);
                        next[k] = Path { steps, state, score };
                    }
                    None => {
                        let mut steps = path.steps.clone();
                        steps.push(step);
                        index.insert(state.clone(), next.len());
                        next.push(Path { steps, state, score });
                    }
                }
            }
        }
        // a state already reached stays out, unless this path reaches it with a better score
        next.retain(|p| seen.get(&p.state).is_none_or(|&best| p.score > best + 1e-9));
        let rank = |p: &Path| p.score - cfg.search.progress * scoring::distance(&p.state, req);
        next.sort_by(|a, b| rank(b).partial_cmp(&rank(a)).unwrap_or(std::cmp::Ordering::Equal));
        next.truncate(width);
        for p in &next {
            seen.insert(p.state.clone(), p.score);
        }
        if next.is_empty() {
            break;
        }
        beam = next;
    }
    out
}

pub fn search(req: &Request, cfg: &Config) -> Result<Outcome, Vec<Diag>> {
    let p = scoring::profile(req, cfg);
    let mut widened = false;
    let mut expanded = 0;
    let mut last: Option<(Vec<Finalist>, Vec<Path>)> = None;
    for width in [cfg.search.beam, cfg.search.fallback] {
        let r = run(req, cfg, &p, width);
        expanded += r.expanded;
        let mut paths = r.finished;
        paths.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        let mut finalists: Vec<Finalist> = paths.into_iter().map(|path| Finalist { path, checks: vec![] }).collect();
        for k in 0..finalists.len() {
            finalists[k].checks = checks::check(req, cfg, &finalists[k].path);
            if finalists[k].passed() {
                return Ok(Outcome { profile: p, width, widened, expanded, finalists, best: k });
            }
        }
        last = Some((finalists, r.dead));
        widened = true;
    }
    let (finalists, dead) = last.expect("ran at least once");
    if std::env::var_os("NUOME_TRACE").is_some() {
        for f in &finalists {
            eprintln!("finalist {:+.3} {} -> {}", f.path.score, f.path.describe(), print::math(&f.path.state, Style::Ascii));
            for c in f.checks.iter().filter(|c| !c.ok) {
                eprintln!("   failed {}: {}", c.name, c.detail);
            }
        }
    }
    let problem = print::math(&req.problem.value, Style::Ascii);
    if let Some(f) = finalists.first() {
        let failed: Vec<&Check> = f.checks.iter().filter(|c| !c.ok).collect();
        let mut d = Diag::new(format!("no way to {} {problem} passes the checks", req.task.value.key()));
        if let Some(c) = failed.first() {
            d.hint = Some(format!("{}: {}", c.name, c.detail));
        }
        return Err(vec![d]);
    }
    let furthest = dead.iter().min_by(|a, b| scoring::distance(&a.state, req).partial_cmp(&scoring::distance(&b.state, req)).unwrap_or(std::cmp::Ordering::Equal));
    let mut d = Diag::new(format!("none of my rules can {} {problem}", req.task.value.key()));
    d.hint = Some(match (furthest, &req.method) {
        (_, Some(m)) => format!("\"{}\" doesn't fit this problem; try without it", m.words),
        (Some(path), None) if !path.steps.is_empty() => format!("got as far as {} and no rule applies there", print::math(&path.state, Style::Ascii)),
        _ => "Nuome v0 solves linear, quadratic and simple rational equations; other kinds refuse rather than guess".into(),
    });
    Err(vec![d])
}
