//! An honest attempt at an open problem. Nuome doesn't stop at recognising
//! the name: it
//!
//! 1. writes the problem as a Nuome question (its `formal` sentence in
//!    rules.toml) and runs the whole engine on it (lexicon, parser, rules,
//!    search, checks), reporting how far that gets and where it stops;
//! 2. computes what can be computed (every case up to a limit, zeros of
//!    zeta on the critical line), reported as evidence for those cases only;
//! 3. gives the verdict: proved only if step 1 produced a checked proof.

use crate::config::{Config, OpenProblem};
use crate::parser::{self, ParseOptions};
use crate::search;

pub fn report(key: &str, p: &OpenProblem, cfg: &Config, recognised_from_statement: bool, wants_guess: bool) -> String {
    let mut out = Vec::new();
    let from = if recognised_from_statement { " (recognised from its statement)" } else { "" };
    out.push(format!("{}{from}: an attempt", p.name));
    out.push(format!("  statement: {}", p.statement));
    out.push(format!("  status: {}", p.status));
    for k in &p.known {
        out.push(format!("  known: {k}"));
    }
    // 1. state it in Nuome's language and run the engine on it
    let mut proved = false;
    match &p.formal {
        Some(sentence) => {
            out.push(format!("  attempt 1, run the engine on: \"{sentence}\""));
            match parser::parse_with(sentence, &ParseOptions { lenient: false, no_open: true }, cfg) {
                Err(ds) => {
                    out.push(format!("    stopped while reading it ({} problem{}):", ds.len(), if ds.len() == 1 { "" } else { "s" }));
                    for d in ds.iter().take(4) {
                        out.push(format!("      {}", d.message.lines().next().unwrap_or("")));
                    }
                    if ds.len() > 4 {
                        out.push(format!("      ... and {} more", ds.len() - 4));
                    }
                    out.push("    so the proof search has no statement to start from: Nuome has no words, and no rules, for these notions".into());
                }
                Ok(req) => match search::search(&req, cfg) {
                    Ok(o) => {
                        proved = true;
                        out.push(format!("    PROVED in {} steps, and every check passed", o.path().steps.len()));
                    }
                    Err(ds) => {
                        out.push("    it reads the statement, and the proof search runs:".into());
                        for d in ds.iter().take(2) {
                            out.push(format!("      {}", d.message));
                            if let Some(h) = &d.hint {
                                out.push(format!("      ({h})"));
                            }
                        }
                    }
                },
            }
        }
        None => out.push("  attempt 1: there is no way to write this as a Nuome question".into()),
    }
    // 2. compute what can be computed
    let computed = match (key, p.check_up_to) {
        ("goldbach", Some(n)) => crate::evidence::goldbach(n),
        ("collatz", Some(n)) => crate::evidence::collatz(n),
        ("riemann", Some(n)) => crate::evidence::riemann(n as f64),
        _ => vec!["nothing: no finite calculation bears on this statement".into()],
    };
    out.push("  attempt 2, compute:".into());
    for c in computed {
        out.push(format!("    {c}"));
    }
    // 3. the verdict
    out.push(if proved {
        "  result: proved".into()
    } else {
        "  result: not proved. No rule set Nuome has, or that anyone has found, derives it; a finite computation can find a counterexample but never proves it for all cases".into()
    });
    // 4. only if asked: a best guess, clearly not a result
    if wants_guess && !proved {
        match &p.guess {
            Some(g) => {
                out.push("  best guess (asked for; NOT checked, NOT a proof, may be wrong):".into());
                out.push(format!("    answer: {}", g.answer));
                out.push(format!("    confidence: {}", g.confidence));
                for b in &g.basis {
                    out.push(format!("    basis: {b}"));
                }
            }
            None => out.push("  best guess: none on file for this problem".into()),
        }
    } else if !proved && p.guess.is_some() {
        out.push("  (ask for a \"best guess\" to see what is expected, clearly marked as unchecked)".into());
    }
    out.join("\n")
}
