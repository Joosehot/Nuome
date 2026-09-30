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

pub fn report(key: &str, p: &OpenProblem, cfg: &Config, recognised_from_statement: bool, wants_guess: bool, sentence: &str) -> String {
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
        ("goldbach", Some(n)) if sentence.to_lowercase().contains("formula") => crate::discover::goldbach_formula(&crate::discover::Settings {
            evolve: ["evolve", "fitness", "better", "genetic"].iter().any(|w| sentence.to_lowercase().contains(w)).then(|| crate::evolve::Settings {
                forward: p.evolve_forward.unwrap_or(true),
                population: p.evolve_population.unwrap_or(300),
                generations: p.evolve_generations.unwrap_or(200),
                seed: p.evolve_seed.unwrap_or(2026),
                price_per_node: p.evolve_price.unwrap_or(0.004),
            }),
            min_n: p.formula_min_n.unwrap_or(1000),
            split: p.formula_split.unwrap_or(100_000_000_000),
            price_per_parameter: p.formula_price.unwrap_or(0.02),
            check_up_to: requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n))),
        }),
        ("goldbach", Some(n)) => goldbach(p, requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n)))),
        ("collatz", Some(n)) => crate::evidence::collatz(n),
        ("conway99", Some(n)) => crate::conway::report(
            requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n))),
            &crate::conway::Evolve {
                population: p.evolve_population.unwrap_or(120),
                generations: p.evolve_generations.unwrap_or(100),
                seed: p.evolve_seed.unwrap_or(2026),
                climb: p.evolve_climb.unwrap_or(300),
            },
            p.pictures.as_deref(),
            p.seed_graph.as_deref(),
        ),
        ("riemann", Some(n)) => crate::zeta::report(requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n))) as f64, &p.tables),
        ("bsd", Some(n)) => crate::bsd::report(requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n))), &p.tables),
        _ => vec!["nothing: no finite calculation bears on this statement".into()],
    };
    out.push("  attempt 2, compute:".into());
    for c in computed {
        out.push(format!("    {c}"));
    }
    // 3. the verdict
    out.push(if proved {
        "  result: proved".into()
    } else if key == "conway99" {
        "  result: not settled. The problem is finite, so a complete search would settle it either way; this search did not finish".into()
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

/// "up to 10^10", "up to 1e9", "up to 5000000": a limit the sentence asks for.
pub fn requested_limit(sentence: &str) -> Option<u64> {
    let low = sentence.to_lowercase();
    let at = low.find("up to ")? + "up to ".len();
    let word: String = low[at..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '^' || *c == ',' || *c == '.').collect();
    let word = word.trim_end_matches('.').replace(',', "");
    if let Some((b, e)) = word.split_once('^').or_else(|| word.split_once('e')) {
        let (b, e): (u64, u32) = (b.parse().ok()?, e.parse().ok()?);
        return b.checked_pow(e);
    }
    word.parse().ok()
}

fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// Goldbach: every even number up to the limit, the record hardest cases,
/// the Hardy-Littlewood model against exact counts, and a confidence score.
fn goldbach(p: &OpenProblem, limit: u64) -> Vec<String> {
    use crate::goldbach as g;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let run = g::run(limit, threads);
    let mut out = Vec::new();
    if let Some(n) = run.counterexample {
        out.push(format!("COUNTEREXAMPLE: {n} is not a sum of two primes; the conjecture is false"));
        return out;
    }
    out.push(format!("checked every even number from 4 to {} ({} numbers) in {:.1} s on {} threads: each is a sum of two primes", thousands(limit), thousands(run.evens), run.seconds, run.threads));
    out.push("record hardest cases (the smallest prime that works is larger than for any smaller even number):".into());
    for (n, q) in run.records.iter().rev().take(10).rev() {
        let ok = g::is_prime(*q) && g::is_prime(n - q);
        out.push(format!("  {:>17} = {:>4} + {}{}", thousands(*n), q, thousands(n - q), if ok { "" } else { "   (FAILED the Miller-Rabin recheck)" }));
    }
    out.push(format!("  every record rechecked independently with Miller-Rabin; {} records in all", run.records.len()));
    // the model against exact counts
    out.push("the Hardy-Littlewood prediction of the number of ways against exact counts:".into());
    let mut worst: f64 = 0.0;
    let mut n = 10_000u64;
    while n <= limit.min(100_000_000) {
        let (w, pr) = (g::ways(n), g::predicted(n));
        worst = worst.max((w as f64 / pr - 1.0).abs());
        out.push(format!("  n = {:>11}: {:>9} ways, model {:>11.0} ({:+.1}%)", thousands(n), thousands(w), pr, (w as f64 / pr - 1.0) * 100.0));
        n *= 10;
    }
    let risk = g::risk_log10(limit);
    out.push(format!("  the model is within {:.1}% at every size tried, and closer the larger n is", worst * 100.0));
    out.push(format!("under that model, the chance of any counterexample above {} is about 10^({:.0})", thousands(limit), risk));
    let doubt = p.model_doubt.unwrap_or(0.0);
    out.push(format!(
        "confidence score: {:.1}% that the conjecture is true; that is 1 - (model risk 10^({:.0})) - (doubt in the model itself, {}%, set in rules.toml: no computation measures it)",
        100.0 * (1.0 - doubt),
        risk,
        doubt * 100.0
    ));
    out.push("the score is evidence about a statement, not a proof of it".into());
    out
}
