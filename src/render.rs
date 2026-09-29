//! The worked solution as text: the problem, each step with the rule that
//! took it, the answer, and what was checked.
//!
//! The profile decides the layout too: a brief profile folds bookkeeping
//! steps (arithmetic, tidying) into the step before and hides side working;
//! a clear one shows every line.

use crate::config::Config;
use crate::expr::{Expr, Math};
use crate::model::{Request, Task};
use crate::print::{self, Style};
use crate::q;
use crate::rules::Line;
use crate::scoring;
use crate::search::Outcome;
use std::fmt::Write as _;

pub fn header(req: &Request, s: Style) -> String {
    let problem = print::math(&req.problem.value, s);
    let mut h = format!("{} {problem}", req.task.value.title());
    match req.task.value {
        Task::Solve => h += &format!(" for {}", req.var.value),
        Task::Differentiate => h += &format!(" with respect to {}", req.var.value),
        _ => {}
    }
    if !req.given.is_empty() {
        let g: Vec<String> = req.given.iter().map(|g| format!("{} = {}", g.value.0, print::expr(&g.value.1, s))).collect();
        h += &format!(" when {}", g.join(", "));
    }
    h
}

struct Shown {
    says: Line,
    rule: String,
    work: Vec<Line>,
    state: Math,
}

fn approx(e: &Expr, places: u32, s: Style) -> Option<String> {
    let rational = e.eval_q(&|_| None);
    let x = e.eval_f(&|_| f64::NAN);
    if !x.is_finite() {
        return None;
    }
    let approx = if s == Style::Ascii { "~" } else if s == Style::Latex { "\\approx" } else { "≈" };
    match rational {
        Some(q) if q.is_int() => None,
        _ => Some(format!("{approx} {}", q::decimal(x, places))),
    }
}

pub fn answer(req: &Request, out: &Outcome, cfg: &Config, s: Style) -> String {
    let places = req.decimals.as_ref().map_or(cfg.display.decimals, |d| d.value);
    let asked_decimal = req.decimals.is_some();
    let one = |e: &Expr| {
        let exact = print::expr(e, s);
        match approx(e, places, s) {
            Some(a) if asked_decimal || e.eval_q(&|_| None).is_none() => format!("{exact} {a}"),
            _ => exact,
        }
    };
    match &out.path().state {
        Math::Expr(e) if req.task.value == Task::Evaluate || !req.given.is_empty() => one(e),
        Math::Expr(e) => print::expr(e, s),
        Math::Eq(l, r) => format!("{} = {}", print::expr(l, s), one(r)),
        Math::Or(eqs) => eqs.iter().map(|(l, r)| format!("{} = {}", print::expr(l, s), one(r))).collect::<Vec<_>>().join(" or "),
        m => print::math(m, s),
    }
}

pub fn render(req: &Request, out: &Outcome, cfg: &Config, s: Style) -> String {
    let p = out.profile;
    let brief = p.brevity > p.clarity;
    let show_work = !brief;
    let path = out.path();
    let mut shown: Vec<Shown> = Vec::new();
    for st in &path.steps {
        let minor = scoring::is_minor(st.mv.rule, cfg);
        if brief && minor {
            if let Some(last) = shown.last_mut() {
                last.state = st.mv.result.clone();
                continue;
            }
        }
        shown.push(Shown { says: st.mv.says.clone(), rule: st.mv.rule.to_string(), work: st.mv.work.clone(), state: st.mv.result.clone() });
    }
    let mut o = String::new();
    let _ = writeln!(o, "{}", header(req, s));
    let _ = writeln!(o);
    let _ = writeln!(o, "      {}", print::math(&req.start(), s));
    for (i, sh) in shown.iter().enumerate() {
        let _ = writeln!(o, "{:>3}.  {}   [{}]", i + 1, sh.says.render(s), sh.rule);
        if show_work {
            for w in &sh.work {
                let _ = writeln!(o, "        {}", w.render(s));
            }
        }
        let _ = writeln!(o, "      {}", print::math(&sh.state, s));
    }
    if shown.is_empty() {
        let _ = writeln!(o, "  (already in its simplest form: no rule applies)");
    }
    let _ = writeln!(o);
    let _ = writeln!(o, "Answer: {}", answer(req, out, cfg, s));
    let mark = if s == Style::Ascii { "ok" } else { "✓" };
    for c in out.checks() {
        let _ = writeln!(o, "  {mark} {}: {}", c.name, c.detail);
    }
    o
}
