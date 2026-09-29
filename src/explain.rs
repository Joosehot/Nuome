//! `--explain`: the tokens, the request with the words behind every value,
//! the profile, the search, the finalists with their scores, the chosen
//! path step by step with the judges' notes, and every check.

use crate::config::Config;
use crate::lexicon;
use crate::model::Request;
use crate::print::{self, Style};
use crate::search::Outcome;
use std::fmt::Write as _;

pub fn explain(req: &Request, out: &Outcome, cfg: &Config) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "tokens");
    if let Ok(toks) = lexicon::lex(&req.sentence) {
        let t: Vec<String> = toks
            .iter()
            .map(|t| match &t.tok {
                lexicon::Tok::Num(q) => format!("Num({q}) <- \"{}\"", t.words),
                tok => format!("{tok:?} <- \"{}\"", t.words),
            })
            .collect();
        for line in t.chunks(4) {
            let _ = writeln!(s, "  {}", line.join("   "));
        }
    }
    let _ = writeln!(s, "request");
    let _ = writeln!(s, "  task     {:<14} <- \"{}\"", req.task.value.key(), req.task.words);
    let _ = writeln!(s, "  problem  {:<14} <- \"{}\"", print::math(&req.problem.value, Style::Ascii), req.problem.words);
    let _ = writeln!(s, "  letter   {:<14} <- \"{}\"", req.var.value, req.var.words);
    for g in &req.given {
        let _ = writeln!(s, "  given    {:<14} <- \"{}\"", format!("{} = {}", g.value.0, print::expr(&g.value.1, Style::Ascii)), g.words);
    }
    if let Some(m) = &req.method {
        let _ = writeln!(s, "  method   {:<14} <- \"{}\"", m.value, m.words);
    }
    if let Some(d) = &req.decimals {
        let _ = writeln!(s, "  decimals {:<14} <- \"{}\"", d.value, d.words);
    }
    let p = out.profile;
    let _ = writeln!(s, "profile  brevity {:.2}  clarity {:.2}  elegance {:.2}", p.brevity, p.clarity, p.elegance);
    let _ = writeln!(s, "  base [profile] {:.2} {:.2} {:.2}", cfg.profile.brevity, cfg.profile.clarity, cfg.profile.elegance);
    for m in &req.modifiers {
        let a = cfg.modifiers[m.value.key()];
        let _ = writeln!(s, "  {:<14} {:+.2} {:+.2} {:+.2}  <- \"{}\"", m.value.key(), a.brevity, a.clarity, a.elegance, m.words);
    }
    let _ = writeln!(s, "rules    {}", cfg.tasks[req.task.value.key()].join(", "));
    let _ = writeln!(s, "search   beam {}{}, {} states expanded, {} finished paths", out.width, if out.widened { " (widened)" } else { "" }, out.expanded, out.finalists.len());
    let _ = writeln!(s, "finalists (best score first)");
    for (i, f) in out.finalists.iter().enumerate().take(8) {
        let mark = if i == out.best {
            "=> "
        } else if f.checks.is_empty() {
            "   "
        } else if f.passed() {
            " ok"
        } else {
            " X "
        };
        let _ = writeln!(s, "  {mark} {:+7.3}  {:>2} steps  {}", f.path.score, f.path.steps.len(), f.path.describe());
        if !f.checks.is_empty() && !f.passed() {
            for c in f.checks.iter().filter(|c| !c.ok) {
                let _ = writeln!(s, "            failed {}: {}", c.name, c.detail);
            }
        }
    }
    if out.finalists.len() > 8 {
        let _ = writeln!(s, "  ... {} more", out.finalists.len() - 8);
    }
    let _ = writeln!(s, "chosen path");
    for (i, st) in out.path().steps.iter().enumerate() {
        let _ = writeln!(s, "  {:>2}. {:+.3}  {}/{}  -> {}", i + 1, st.total(), st.mv.rule, st.mv.variant, print::math(&st.mv.result, Style::Ascii));
        for n in &st.notes {
            let _ = writeln!(s, "            {:+.3} {}: {}", n.score, n.judge, n.why);
        }
    }
    let _ = writeln!(s, "checks");
    for c in out.checks() {
        let _ = writeln!(s, "  {} {:<10} {}", if c.ok { "ok" } else { "X " }, c.name, c.detail);
    }
    s
}
