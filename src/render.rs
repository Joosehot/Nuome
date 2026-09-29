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
    if let (Task::Divide, Math::Expr(Expr::Div(a, b))) = (req.task.value, &req.problem.value) {
        return format!("Divide {} by {}", print::expr(a, s), print::expr(b, s));
    }
    // a phrased problem ("gcd of 48 and 18") reads best as it was asked
    if req.problem.value.slots().iter().any(|e| e.has_call()) {
        let mut c = req.sentence.chars();
        return c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str());
    }
    let problem = print::math(&req.problem.value, s);
    let mut h = format!("{} {problem}", req.task.value.title());
    match req.task.value {
        Task::Solve => h += &format!(" for {}", letters(req)),
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
    if let Some(a) = algebra_answer(req, out, s) {
        return a;
    }
    // an amount of money reads as a decimal too
    let money = matches!(&req.problem.value, Math::Expr(Expr::Call(crate::calls::Named::Compound | crate::calls::Named::Raise, _)));
    let asked_decimal = req.decimals.is_some() || money;
    // "what percent of 80 is 12" answers in percent
    let pct = match &req.problem.value {
        Math::Expr(Expr::Call(f, _)) if f.is_percent() => "%",
        _ => "",
    };
    let one = |e: &Expr| {
        let exact = format!("{}{pct}", print::expr(e, s));
        match approx(e, places, s) {
            // an exact fraction too long to read leads with its decimal
            Some(a) if exact.len() > cfg.display.answer_digits => format!("{a}{pct} (exactly {exact})"),
            Some(a) if asked_decimal || e.eval_q(&|_| None).is_none() => format!("{exact} {a}{pct}"),
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

/// For compound growth: the factor and the gain in percent, in words.
fn growth_line(req: &Request, out: &Outcome) -> Option<String> {
    let Math::Expr(Expr::Call(crate::calls::Named::Compound, args)) = &req.problem.value else { return None };
    let Math::Expr(fin) = &out.path().state else { return None };
    let start = args.first()?.eval_f(&|_| f64::NAN);
    let end = fin.eval_f(&|_| f64::NAN);
    if !(start > 0.0 && end.is_finite()) {
        return None;
    }
    let factor = end / start;
    Some(format!("That is {} times the start: a {} of {}%.", q::decimal(factor, 2), if factor >= 1.0 { "gain" } else { "loss" }, q::decimal((factor - 1.0).abs() * 100.0, 2)))
}

// ---- algebra (agent A): answers to inequalities, systems, divisions; restrictions ----

/// The letters solved for: "x", or "x and y" for a system.
fn letters(req: &Request) -> String {
    let Math::System(eqs) = &req.problem.value else { return req.var.value.clone() };
    let vars: Vec<String> = eqs.iter().flat_map(|(l, r)| l.vars().into_iter().chain(r.vars())).collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    match vars.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => vars.join(""),
    }
}

/// Values of the letter where the original expression is undefined but the
/// answer isn't: (x^2 - 1)/(x - 1) = x + 1 only for x != 1.
pub fn restrictions(req: &Request, fin: &Expr) -> Vec<q::Q> {
    let v = &req.var.value;
    let mut out: Vec<q::Q> = Vec::new();
    for d in crate::rules::solutions::denominators(&req.problem.value, v) {
        let Some(p) = crate::poly::from_expr(&d, v) else { continue };
        for r in p.rational_roots() {
            let defined = fin.eval_f(&|n| if n == v { r.to_f64() } else { f64::NAN }).is_finite();
            if defined && !out.contains(&r) {
                out.push(r);
            }
        }
    }
    out.sort();
    out
}

fn algebra_answer(req: &Request, out: &Outcome, s: Style) -> Option<String> {
    let v = &req.var.value;
    let state = &out.path().state;
    let ne = match s {
        Style::Ascii => "!=",
        Style::Unicode => "≠",
        Style::Latex => "\\neq",
    };
    match (req.task.value, state) {
        (Task::Divide, Math::Expr(e)) => {
            let Math::Expr(Expr::Div(_, d)) = &req.problem.value else { return None };
            let (quot, rem) = crate::checks::quotient_remainder(e, d);
            Some(format!("quotient {}, remainder {}", print::expr(&quot, s), print::expr(&rem, s)))
        }
        (Task::Simplify, Math::Expr(e)) if req.given.is_empty() => {
            let rs = restrictions(req, e);
            if rs.is_empty() {
                return None;
            }
            let cond: Vec<String> = rs.iter().map(|r| format!("{v} {ne} {}", print::number(r, s))).collect();
            Some(format!("{}, for {}", print::expr(e, s), cond.join(" and ")))
        }
        (_, Math::Ineq(l, r, rr)) => {
            let iv = crate::expr::Interval::of(*r, rr);
            Some(format!("{} {} {}, in interval notation {}", print::expr(l, s), print::rel(*r, s), print::expr(rr, s), print::interval_notation(&[iv], s)))
        }
        (_, Math::Intervals(_, ivs)) if !ivs.is_empty() => Some(format!("{}, in interval notation {}", print::math(state, s), print::interval_notation(ivs, s))),
        (_, Math::System(eqs)) => {
            // in the letters' order: x = 3, y = 2, z = 1
            let mut eqs = eqs.clone();
            eqs.sort_by(|a, b| print::expr(&a.0, Style::Ascii).cmp(&print::expr(&b.0, Style::Ascii)));
            let solved: Vec<String> = eqs.iter().map(|(l, r)| format!("{} = {}", print::expr(l, s), print::expr(r, s))).collect();
            let free: std::collections::BTreeSet<String> = eqs.iter().flat_map(|(_, r)| r.vars()).collect();
            if free.is_empty() {
                Some(solved.join(", "))
            } else {
                let f: Vec<String> = free.into_iter().collect();
                Some(format!("{}, for any {} (infinitely many solutions)", solved.join(", "), f.join(" and ")))
            }
        }
        _ => None,
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
    for n in &req.notes {
        let _ = writeln!(o, "  Note: {}", n.value);
    }
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
    if let Some(line) = growth_line(req, out) {
        let _ = writeln!(o, "{line}");
    }
    let mark = if s == Style::Ascii { "ok" } else { "✓" };
    for c in out.checks() {
        let _ = writeln!(o, "  {mark} {}: {}", c.name, c.detail);
    }
    o
}
