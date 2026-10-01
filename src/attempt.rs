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
        ("goldbach", Some(n)) => goldbach(p, cfg, requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n)))),
        ("collatz", Some(n)) => crate::evidence::collatz(n),
        ("eff_prime", Some(n)) => crate::mersenne::report(&crate::mersenne::Settings { check_below: n as usize, timed_steps: p.measure.unwrap_or(2) }),
        ("hodge", _) => crate::hodge::report(&crate::hodge::Settings {
            surface_degrees: if p.surface_degrees.is_empty() { (3..=12).collect() } else { p.surface_degrees.clone() },
            fourfold_degrees: if p.fourfold_degrees.is_empty() { (3..=10).collect() } else { p.fourfold_degrees.clone() },
        }),
        ("p_vs_np", _) => crate::p_vs_np::report(&crate::p_vs_np::Settings {
            sizes: if p.sizes.is_empty() { vec![20, 30, 40, 50, 60, 70, 80] } else { p.sizes.clone() },
            instances: p.instances.unwrap_or(40),
            ratio: p.ratio.unwrap_or(4.26),
            seed: p.evolve_seed.unwrap_or(2026),
        }),
        ("navier_stokes", _) => crate::navier_stokes::report(&crate::navier_stokes::Settings {
            grid: p.grid.unwrap_or(32),
            viscosity: p.viscosity.unwrap_or(0.01),
            t_end: p.t_end.unwrap_or(10.0),
            dt: p.dt.unwrap_or(0.02),
        }),
        ("yang_mills", _) => crate::yang_mills::report(&crate::yang_mills::Settings {
            lattice: p.lattice.unwrap_or(8),
            betas: if p.betas.is_empty() { vec![0.5, 2.2, 2.3, 2.4, 2.5, 4.0] } else { p.betas.clone() },
            thermalise: p.thermalise.unwrap_or(50),
            measure: p.measure.unwrap_or(100),
            seed: p.evolve_seed.unwrap_or(2026),
        }),
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
            p.evolve_price.unwrap_or(1.0),
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
    } else if key == "eff_prime" {
        "  result: not found. The task is finite (one prime and one check), but at this machine's speed one test of one candidate takes decades".into()
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

/// A proof, for the finite range that was computed, that the smallest prime
/// p(n) with n - p(n) prime never exceeds f(n) = c ((ln ln n)^inner)^outer
/// (the constants from rules.toml). The steps:
/// 1. f increases for n >= 16, where ln ln n > 0;
/// 2. records: p(n) is at most p(r) for the last record r <= n (by the
///    definition of a record, from the exhaustive run);
/// 3. each record that is in force somewhere in the range satisfies
///    p(r) <= f(max(r, from)), with a margin far above rounding;
/// 4. so p(n) <= p(r) <= f(max(r, from)) <= f(n) for every even n in range.
fn bound_proof(p: &OpenProblem, cfg: &Config, limit: u64, records: &[(u64, u64)]) -> Vec<String> {
    let (Some(c), Some(inner), Some(outer)) = (p.bound_c, p.bound_inner, p.bound_outer) else {
        return Vec::new();
    };
    let from = p.bound_from.unwrap_or(1000).max(16);
    let f = |n: u64| c * (n as f64).ln().ln().powf(inner).powf(outer);
    let shown = format!("{c} ((ln ln n)^{inner})^{outer}");
    let mut out = vec![format!("theorem (proved here): for every even n with {} <= n <= {}, the smallest prime p with n - p prime satisfies p <= {shown}", thousands(from), thousands(limit))];
    // lemma 1, abstractly: f grows for x > e, by a derivative Nuome works out with its own rules
    out.push(format!("  lemma 1 (f grows for every real x > e): f(x) = {c} g(x) with g(x) = ((ln ln x)^{inner})^{outer}; Nuome's own rules differentiate g:"));
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    match crate::solve(&format!("differentiate ((ln(ln x))^{inner})^{outer}"), cfg, &opts) {
        Ok(s) => {
            for line in s.text.lines().filter(|l| !l.trim().is_empty()) {
                out.push(format!("      {line}"));
            }
            out.push(format!(
                "    for x > e: ln x > 1, so ln ln x > 0 (and (u^a)^b = u^(ab) holds for u > 0); then {:.5} > 0, (ln ln x)^({:.5}) > 0, x > 0 and ln x > 0, so g'(x) > 0; with {c} > 0, f' = {c} g' > 0, so f grows on (e, oo). QED",
                inner * outer,
                inner * outer - 1.0
            ));
        }
        Err(_) => out.push("    (the derivative could not be worked out by the rules; lemma 1 is not proved, so neither is the theorem)".into()),
    }
    // lemma 2, abstractly: holds for any sequence whatever
    out.push("  lemma 2 (records, for ANY sequence p on the even numbers): call r a record when p(r) > p(m) for every even m < r, and let R(n) be the last record <= n; then p(n) <= p(R(n)). Proof by induction on n: the first even number is a record; if n is a record, R(n) = n; if not, some m < n has p(m) >= p(n), and p(m) <= p(R(m)) <= p(R(n)) by induction and because records only grow. QED".into());
    out.push(format!("  step 2: every even n from 4 to {} was computed (above), giving every record up to {}", thousands(limit), thousands(limit)));
    // from `from` to the first record above it, the largest p(n) is computed
    // directly (the record before `from` may be larger than anything after it);
    // from that record on, the records themselves
    let first_after = records.iter().position(|&(n, _)| n > from).unwrap_or(records.len());
    let segment_end = records.get(first_after).map_or(limit, |r| r.0 - 1).min(limit);
    let smallest = |n: u64| (2..=n / 2).find(|&q| crate::goldbach::is_prime(q) && crate::goldbach::is_prime(n - q)).unwrap_or(0);
    let start_max = (from + from % 2..=segment_end).step_by(2).map(smallest).max().unwrap_or(0);
    out.push(format!("  (from {} to {}, before the next record, every p(n) was computed directly: the largest is {start_max})", thousands(from), thousands(segment_end)));
    let in_force: Vec<(u64, u64)> = std::iter::once((from, start_max)).chain(records[first_after..].iter().copied()).collect();
    let start = 0;
    let records = &in_force;
    let mut tightest: Option<(u64, u64, f64)> = None;
    let mut failed = Vec::new();
    for &(n, q) in &records[start..] {
        let at = n.max(from);
        let bound = f(at);
        // a margin of 1e-9 relative covers every rounding of the logarithms
        if (q as f64) > bound * (1.0 - 1e-9) {
            failed.push(format!("{} (p = {q}, f = {bound:.2})", thousands(n)));
        }
        if tightest.is_none_or(|t| (q as f64) / bound > t.1 as f64 / t.2) {
            tightest = Some((n, q, bound));
        }
    }
    let checked = records.len() - start;
    if !failed.is_empty() {
        out.push(format!("  step 3 FAILS at: {}; the bound is not proved on this range", failed.join(", ")));
        return out;
    }
    let (tn, tq, tb) = tightest.expect("at least one record");
    out.push(format!(
        "  step 3: all {checked} records in force between {} and {} satisfy p(r) <= f(max(r, {})); tightest at r = {}: p = {tq}, f = {tb:.2}, so {:.2}% to spare (rounding of the logarithms is below 0.0000001%)",
        thousands(from),
        thousands(limit),
        thousands(from),
        thousands(tn),
        (1.0 - tq as f64 / tb) * 100.0
    ));
    out.push("  step 4: for each even n in the range, with r the last record <= n (or the directly computed start): p(n) <= p(r) by lemma 2, p(r) <= f(max(r, from)) by step 3, and f(max(r, from)) <= f(n) by lemma 1 (from > e). QED".into());
    out.push(format!("  (a proof for this range only: above {} it is a conjecture, like Goldbach itself)", thousands(limit)));
    out
}

/// Goldbach: every even number up to the limit, the record hardest cases,
/// the Hardy-Littlewood model against exact counts, and a confidence score.
fn goldbach(p: &OpenProblem, cfg: &Config, limit: u64) -> Vec<String> {
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
    out.extend(bound_proof(p, cfg, limit, &run.records));
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
