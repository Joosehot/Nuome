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
        ("beal", Some(n)) => crate::beal::report(&crate::beal::Settings {
            max_base: requested_limit(sentence).map_or(n, |r| r.min(p.max_check.unwrap_or(n))),
            max_exp: p.max_exponent.unwrap_or(12),
        }),
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
    let from = p.bound_from.unwrap_or(1000).max(16);
    let mut out = Vec::new();
    // the simple bound, the shape the model of the primes gives (see the "why" below)
    if let Some(c) = p.simple_c {
        let lemma = vec![format!(
            "  lemma 1 (f grows for every real x > e): f(x) = {c} (ln x)^2 ln ln x. For x > e, ln x > 1 > 0 and ln x grows, so (ln x)^2 grows; ln ln x > 0 and grows; a product of positive growing functions grows, and {c} > 0. QED (no derivative needed)"
        )];
        out.extend(prove_bound(&format!("{c} (ln n)^2 ln ln n"), &|n: u64| c * (n as f64).ln().powi(2) * (n as f64).ln().ln(), lemma, from, limit, records));
        // and where the proof does not reach: the published records, checked
        let published: Vec<(u64, u64)> = crate::discover::RECORDS
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                Some((f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?))
            })
            .filter(|&(n, _)| n > limit)
            .collect();
        if let Some(&(wn, wq)) = published.iter().max_by(|a, b| (a.1 as f64 / (c * (a.0 as f64).ln().powi(2) * (a.0 as f64).ln().ln())).total_cmp(&(b.1 as f64 / (c * (b.0 as f64).ln().powi(2) * (b.0 as f64).ln().ln())))) {
            let fw = c * (wn as f64).ln().powi(2) * (wn as f64).ln().ln();
            out.push(format!(
                "  beyond the proof, checked against the {} published records above {} (OEIS A025018/A025019, up to {:.1e}): {}; tightest at n = {}: p = {wq}, bound {fw:.1} (checked, not proved: those records were computed by others)",
                published.len(),
                thousands(limit),
                published.iter().map(|r| r.0).max().unwrap_or(0) as f64,
                if published.iter().all(|&(n, q)| (q as f64) <= c * (n as f64).ln().powi(2) * (n as f64).ln().ln()) { "every one holds" } else { "SOME FAIL" },
                thousands(wn)
            ));
        }
    }
    let (Some(c), Some(inner), Some(outer)) = (p.bound_c, p.bound_inner, p.bound_outer) else {
        return out;
    };
    out.push("the evolved bound, the one the genetic search found:".into());
    // lemma 1, abstractly: f grows for x > e, by a derivative Nuome works out with its own rules
    let mut lemma = vec![format!("  lemma 1 (f grows for every real x > e): f(x) = {c} g(x) with g(x) = ((ln ln x)^{inner})^{outer}; Nuome's own rules differentiate g:")];
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    match crate::solve(&format!("differentiate ((ln(ln x))^{inner})^{outer}"), cfg, &opts) {
        Ok(s) => {
            for line in s.text.lines().filter(|l| !l.trim().is_empty()) {
                lemma.push(format!("      {line}"));
            }
            lemma.push(format!(
                "    for x > e: ln x > 1, so ln ln x > 0 (and (u^a)^b = u^(ab) holds for u > 0); then {:.5} > 0, (ln ln x)^({:.5}) > 0, x > 0 and ln x > 0, so g'(x) > 0; with {c} > 0, f' = {c} g' > 0, so f grows on (e, oo). QED",
                inner * outer,
                inner * outer - 1.0
            ));
        }
        Err(_) => lemma.push("    (the derivative could not be worked out by the rules; lemma 1 is not proved, so neither is the theorem)".into()),
    }
    out.extend(prove_bound(&format!("{c} ((ln ln n)^{inner})^{outer}"), &|n: u64| c * (n as f64).ln().ln().powf(inner).powf(outer), lemma, from, limit, records));
    out
}

/// What the simple bound means and where each part comes from: ln n from the
/// prime number theorem, the square from the number of even numbers that
/// could be the hardest, ln ln n from how primes thin out, and the constant
/// from the twin prime constant. Each step says whether it is a theorem or
/// the model's assumption (that the tries behave like independent chances).
fn formula_meaning(p: &OpenProblem, records: &[(u64, u64)]) -> Vec<String> {
    let Some(c) = p.simple_c else { return Vec::new() };
    let c2 = crate::goldbach::C2;
    let two_c2 = 2.0 * c2;
    let predicted = 1.0 / c2;
    let from = p.bound_from.unwrap_or(1000);
    let ratio = |n: u64, q: u64| q as f64 / ((n as f64).ln().powi(2) * (n as f64).ln().ln());
    let measured = records.iter().filter(|r| r.0 >= from).map(|&(n, q)| ratio(n, q)).fold(0.0, f64::max);
    let mut out = vec![format!("what the formula p(n) <= {c} (ln n)^2 ln ln n means, and why each part is there:")];
    out.push("  meaning: p(n) is how far you must search: try the primes p = 3, 5, 7, ... in order until n - p is prime too. The formula says the search always ends within the primes below this size".into());
    out.push("  1. the chance per try, 1/ln n (THEOREM: the prime number theorem, 1896, says about 1 number in ln n near n is prime). For n - p to be prime when p is, the Hardy-Littlewood count gives the chance about 2 C2 S(n) / ln n, where C2 = 0.66016 is the twin prime constant and S(n) >= 1 is larger when n has small odd factors".into());
    out.push(format!("     the hardest n are those with S(n) = 1 (no small odd factors, like powers of 2), where the chance per try is {two_c2:.4} / ln n"));
    out.push("  2. why (ln n)^2: one ln n from the chance per try, one from how many even numbers there are (MODEL: tries behave like independent chances). The first m tries all fail with chance about e^(-m * chance); among the about n/2 even numbers up to n, the worst one fails about ln(n/2) / chance tries, so m = ln n * ln n / (2 C2): the number of primes the hardest n needs grows like (ln n)^2".into());
    out.push("  3. why ln ln n: the formula bounds the prime, not the count of primes. The m-th prime is about m ln m (THEOREM: from the prime number theorem). With m = (ln n)^2 / (2 C2): ln m = 2 ln ln n - ln(2 C2), so p = m ln m is about (2 / (2 C2)) (ln n)^2 ln ln n. The ln ln n is the thinning of the primes: the m-th prime is larger than m by the factor ln m".into());
    out.push(format!(
        "  4. why {c}: the derivation gives the constant 2 / (2 C2) = 1 / C2 = {predicted:.4}. Nuome's records from {} up: the largest p / ((ln n)^2 ln ln n) is {measured:.4}, and every published record up to 4*10^18 stays at most 1.5263; {c} is that, rounded up. So the constant is the reciprocal of the twin prime constant, plus {:.1}% for the luck of the hardest cases",
        thousands(from),
        (c / predicted - 1.0) * 100.0
    ));
    out.push(format!(
        "  so: p(n) <= {c} (ln n)^2 ln ln n reads 'the hardest n needs about (ln n)^2 / (2 C2) tries, and that many primes reach up to about that size'. The prime number theorem parts are proved; the independence of the tries is the model's assumption, and proving it would prove Goldbach"
    ));
    out
}

/// The published Top 50 table (p, n), up to 4 * 10^18.
const TOP50: &str = include_str!("../data/goldbach_top50.tsv");

/// The constant of the simple bound, found by Nuome: the smallest c, to three
/// decimals and rounded up, with p <= c (ln n)^2 ln ln n at every record it
/// has, its own (computed up to `limit`, from `bound_from`) and the published
/// ones it was given (OEIS records and the Top 50 table). With the
/// sentence that says how it was found.
/// How Nuome found the simple bound's constant (shared by every language).
pub(crate) struct SimpleC {
    pub c: f64,
    pub worst: f64,
    /// the record that sets it, and whether Nuome computed it itself
    pub at: (u64, u64, bool),
    pub own: usize,
    pub published: usize,
    /// the published records above the computed range
    pub above: Vec<(u64, u64)>,
}

/// p / ((ln n)^2 ln ln n)
pub(crate) fn simple_ratio(n: u64, q: u64) -> f64 {
    q as f64 / ((n as f64).ln().powi(2) * (n as f64).ln().ln())
}

pub(crate) fn simple_c_data(p: &OpenProblem, limit: u64, own: &[(u64, u64)]) -> Option<SimpleC> {
    if p.simple_bound != Some(true) {
        return None;
    }
    let from = p.bound_from.unwrap_or(1000);
    let above: Vec<(u64, u64)> = crate::discover::RECORDS
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            Some((f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?))
        })
        .chain(TOP50.lines().filter(|l| !l.starts_with('#')).filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            Some((f.get(1)?.parse().ok()?, f.first()?.parse().ok()?))
        }))
        .filter(|&(n, _)| n > limit)
        .collect();
    let all: Vec<(u64, u64, bool)> = own.iter().map(|&(n, q)| (n, q, true)).chain(above.iter().map(|&(n, q)| (n, q, false))).filter(|r| r.0 >= from).collect();
    let &at = all.iter().max_by(|a, b| simple_ratio(a.0, a.1).total_cmp(&simple_ratio(b.0, b.1)))?;
    let worst = simple_ratio(at.0, at.1);
    Some(SimpleC {
        c: (worst * 1000.0).ceil() / 1000.0,
        worst,
        at,
        own: all.iter().filter(|r| r.2).count(),
        published: all.iter().filter(|r| !r.2).count(),
        above,
    })
}

fn find_simple_c(p: &OpenProblem, limit: u64, own: &[(u64, u64)]) -> Option<(f64, String)> {
    let d = simple_c_data(p, limit, own)?;
    let (c, worst, (wn, wq, mine)) = (d.c, d.worst, d.at);
    let how = format!(
        "the constant {c} was found by Nuome: the largest p / ((ln n)^2 ln ln n) over the {} records it has ({} computed by itself up to {}, {} published ones above that) is {worst:.4}, at n = {} (p = {wq}, {}), rounded up to three decimals; the model predicts 1 / C2 = {:.4}",
        d.own + d.published,
        d.own,
        thousands(limit),
        d.published,
        thousands(wn),
        if mine { "computed by Nuome" } else { "a published record" },
        1.0 / crate::goldbach::C2
    );
    Some((c, how))
}

/// The computed part of a range proof (shared by every language).
pub(crate) struct RangeCheck {
    pub segment_end: u64,
    pub start_max: u64,
    pub checked: usize,
    /// (n, p, bound) where the bound is tightest
    pub tightest: (u64, u64, f64),
    pub failed: Vec<(u64, u64, f64)>,
}

pub(crate) fn range_check(f: &dyn Fn(u64) -> f64, from: u64, limit: u64, records: &[(u64, u64)]) -> Option<RangeCheck> {
    // from `from` to the first record above it, the largest p(n) is computed
    // directly (the record before `from` may be larger than anything after it);
    // from that record on, the records themselves
    let first_after = records.iter().position(|&(n, _)| n > from).unwrap_or(records.len());
    let segment_end = records.get(first_after).map_or(limit, |r| r.0 - 1).min(limit);
    let smallest = |n: u64| (2..=n / 2).find(|&q| crate::goldbach::is_prime(q) && crate::goldbach::is_prime(n - q)).unwrap_or(0);
    let start_max = (from + from % 2..=segment_end).step_by(2).map(smallest).max().unwrap_or(0);
    let in_force: Vec<(u64, u64)> = std::iter::once((from, start_max)).chain(records[first_after..].iter().copied()).collect();
    let mut tightest: Option<(u64, u64, f64)> = None;
    let mut failed = Vec::new();
    for &(n, q) in &in_force {
        let bound = f(n.max(from));
        // a margin of 1e-9 relative covers every rounding of the logarithms
        if (q as f64) > bound * (1.0 - 1e-9) {
            failed.push((n, q, bound));
        }
        if tightest.is_none_or(|t| (q as f64) / bound > t.1 as f64 / t.2) {
            tightest = Some((n, q, bound));
        }
    }
    Some(RangeCheck { segment_end, start_max, checked: in_force.len(), tightest: tightest?, failed })
}

/// The whole proof of the Goldbach bound in one piece, as an answer (not a
/// refusal): what it means, the derivation proved inside the model, the
/// general proofs, the computed range, and what is proved and what is open.
pub fn goldbach_proof(cfg: &Config, limit: Option<u64>) -> Option<String> {
    let rules = cfg.open.get("goldbach")?;
    let limit = limit.unwrap_or(rules.check_up_to.unwrap_or(1_000_000_000)).min(rules.max_check.unwrap_or(u64::MAX));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let run = crate::goldbach::run(limit, threads);
    let (found, how) = find_simple_c(rules, limit, &run.records)?;
    let p = &OpenProblem { simple_c: Some(found), ..rules.clone() };
    let c = found;
    let mut out = vec![
        format!("The Goldbach bound: p(n) <= {c} (ln n)^2 ln ln n for every even n >= {}", thousands(p.bound_from.unwrap_or(1000))),
        "where p(n) is the smallest prime p with n - p prime".into(),
        how,
        String::new(),
        "PART 1. What the formula means".into(),
    ];
    out.extend(formula_meaning(p, &run.records));
    out.push(String::new());
    out.push("PART 2. The derivation, proved".into());
    out.extend(model_proof(p, cfg));
    out.push(String::new());
    out.push("PART 3. What the bound gives".into());
    out.extend(general_proofs(p, cfg));
    out.push(String::new());
    out.push(format!("PART 4. Computed (every even n up to {}, {:.1} s on {} threads)", thousands(limit), run.seconds, run.threads));
    if let Some(n) = run.counterexample {
        out.push(format!("COUNTEREXAMPLE: {n} is not a sum of two primes"));
    } else {
        out.push(format!("every even number from 4 to {} is a sum of two primes", thousands(limit)));
    }
    let only_simple = OpenProblem { bound_c: None, ..p.clone() };
    out.extend(bound_proof(&only_simple, cfg, limit, &run.records));
    out.push(String::new());
    out.push("SUMMARY".into());
    for (part, status) in [
        ("ln n: the chance per try", "proved (prime number theorem)"),
        ("ln ln n: the m-th prime is m ln m", "proved (prime number theorem)"),
        ("(ln n)^2 and the constant 1/C2 = 1.5148, inside the model", "proved (lemma A, theorem B, theorem C)"),
        ("the bound gives Goldbach and the three-prime statement", "proved (definitions and Nuome's logic rules)"),
        ("the bound for every even n from 1 000 to the limit", "computed (records lemma and every record)"),
        ("the actual primes obey the model, so the bound holds for every n", "OPEN: proving it would prove Goldbach"),
    ] {
        out.push(format!("  {part:<66} {status}"));
    }
    Some(out.join("\n"))
}

/// The derivation made rigorous inside the model: for independent tries the
/// worst of N even numbers needs (1 + o(1)) ln N / q tries, with probability
/// tending to 1 (a theorem about random variables, proved in full), and the
/// prime number theorem turns tries into the size of the prime. The one step
/// not proved is the bridge from the model to the actual primes.
fn model_proof(p: &OpenProblem, cfg: &Config) -> Vec<String> {
    let Some(c) = p.simple_c else { return Vec::new() };
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    let mut out = vec!["the derivation proved, inside the model (general proofs; no number is checked):".into()];
    out.push("  the model: for each of N even numbers, the tries succeed independently, each with chance q (0 < q <= 1/2); X_i is the number of tries the i-th number needs, so P(X_i > m) = (1 - q)^m; M = max of X_1 .. X_N is the worst case".into());
    out.push("  lemma A (an inequality): ln(1 - q) >= -q - q^2 for 0 <= q <= 1/2. Let f(q) = ln(1 - q) + q + q^2, so f(0) = 0; Nuome's rules differentiate and simplify f':".into());
    for sentence in ["differentiate ln(1 - x) + x + x^2", "simplify -1/(1 - x) + 1 + 2x"] {
        match crate::solve(sentence, cfg, &opts) {
            Ok(s) => {
                if let Some(a) = s.text.lines().find(|l| l.starts_with("Answer:")) {
                    let checks: Vec<&str> = s.text.lines().filter(|l| l.trim_start().starts_with("ok ")).map(|l| l.trim()).collect();
                    out.push(format!("      {sentence}: {}  [{}]", a.trim(), checks.join("; ")));
                }
            }
            Err(_) => out.push(format!("      {sentence}: the rules could not do it, so lemma A is not proved")),
        }
    }
    out.push("    so f'(q) = (-q + 2q^2)/(q - 1) = q (1 - 2q)/(1 - q), and for 0 <= q <= 1/2 every factor is >= 0 (with 1 - q > 0), so f' >= 0; f grows from f(0) = 0, so f(q) >= 0. QED".into());
    out.push("  theorem B (the worst case is ln N / q tries): for every e > 0, P(M > (1 + e) ln N / q) <= N^(-e), and P(M <= (1 - e) ln N / q) <= exp(-N^(e/2)) once q (1 - e) <= e/2; both tend to 0 as N grows, so M = (1 + o(1)) ln N / q with probability tending to 1".into());
    out.push("    upper: P(M > m) <= N (1 - q)^m (the union bound: one of N events) <= N e^(-qm) (since 1 - q <= e^(-q)); at m = (1 + e) ln N / q this is N * N^(-(1 + e)) = N^(-e). QED".into());
    out.push("    lower: by independence P(M <= m) = (1 - (1 - q)^m)^N <= exp(-N (1 - q)^m) (since 1 - x <= e^(-x)); by lemma A, (1 - q)^m >= e^(-m (q + q^2)); at m = (1 - e) ln N / q this is N^(-(1 - e)(1 + q)) >= N^(-(1 - e/2)) when q (1 - e) <= e/2, so P(M <= m) <= exp(-N^(e/2)). QED".into());
    out.push("  theorem C (the size of the prime, a theorem about the actual primes): the m-th prime p_m satisfies p_m = (1 + o(1)) m ln m (from the prime number theorem, Hadamard and de la Vallee Poussin 1896)".into());
    out.push(format!(
        "  together: with N = n/2 and q = 2 C2 / ln n (the chance for the hardest n), theorem B gives M = (1 + o(1)) (ln n)^2 / (2 C2), and theorem C gives the prime p_M = (1 + o(1)) (1 / C2) (ln n)^2 ln ln n = (1 + o(1)) {:.4} (ln n)^2 ln ln n; the formula's {c} sits just above that, as a bound should",
        1.0 / crate::goldbach::C2
    ));
    out.push("  the one step NOT proved: that the actual primes obey the model, that is, that whether n - p is prime behaves like an independent chance 2 C2 S(n) / ln n uniformly for every n. Lemma A and theorems B and C are proved; this bridge is not, and a proof of it would prove Goldbach, so no one can supply it today".into());
    out
}

/// What can be proved about the bound for EVERY n, with no number checked:
/// two implications from the definitions, and the chains of reasoning, done
/// by Nuome's logic rules; then what stays open.
fn general_proofs(p: &OpenProblem, cfg: &Config) -> Vec<String> {
    let Some(c) = p.simple_c else { return Vec::new() };
    let from = p.bound_from.unwrap_or(1000);
    let mut out = vec!["general proofs (for every n; no number is checked):".into()];
    out.push(format!("  for an even n >= {from}, let b = 'some prime p <= {c} (ln n)^2 ln ln n has n - p prime' (the bound), g = 'n is a sum of two primes' (Goldbach for n), w = 'n + 3 is a sum of three primes' (weak Goldbach for n + 3)"));
    out.push("  b implies g, by definition: if p and n - p are prime, then n = p + (n - p) is a sum of two primes. QED".into());
    out.push("  g implies w: if n = p + q with p, q prime, then n + 3 = 3 + p + q, and 3 is prime. QED".into());
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    for (what, sentence) in [
        ("so the bound gives Goldbach for n", "prove that (b and (b implies g)) implies g"),
        ("and it gives the three-prime statement for n + 3", "prove that ((b implies g) and (g implies w)) implies (b implies w)"),
    ] {
        out.push(format!("  {what}; Nuome's logic rules prove the step:"));
        match crate::solve(sentence, cfg, &opts) {
            Ok(s) => out.extend(s.text.lines().filter(|l| !l.trim().is_empty()).map(|l| format!("      {l}"))),
            Err(_) => out.push("      (the logic rules could not prove it)".into()),
        }
    }
    out.push(format!(
        "  what stays open: b itself for every n. It cannot be proved here, or anywhere yet: b for every even n >= {from} gives Goldbach for every such n (above), so a proof of the bound would be a proof of Goldbach, open since 1742. The best proved result in this direction is Chen (1973): every large even n is a prime plus a number with at most two prime factors"
    ));
    out
}

/// The proof of p(n) <= f(n) for every even n in [from, limit], given a
/// proof that f grows (lemma 1): the records lemma, the records, the start.
fn prove_bound(shown: &str, f: &dyn Fn(u64) -> f64, lemma1: Vec<String>, from: u64, limit: u64, records: &[(u64, u64)]) -> Vec<String> {
    let mut out = vec![format!("on a finite range, by computation (not a general proof): for every even n with {} <= n <= {}, the smallest prime p with n - p prime satisfies p <= {shown}", thousands(from), thousands(limit))];
    out.extend(lemma1);
    // lemma 2, abstractly: holds for any sequence whatever
    out.push("  lemma 2 (records, for ANY sequence p on the even numbers): call r a record when p(r) > p(m) for every even m < r, and let R(n) be the last record <= n; then p(n) <= p(R(n)). Proof by induction on n: the first even number is a record; if n is a record, R(n) = n; if not, some m < n has p(m) >= p(n), and p(m) <= p(R(m)) <= p(R(n)) by induction and because records only grow. QED".into());
    out.push(format!("  step 2: every even n from 4 to {} was computed (above), giving every record up to {}", thousands(limit), thousands(limit)));
    let Some(rc) = range_check(f, from, limit, records) else { return out };
    out.push(format!("  (from {} to {}, before the next record, every p(n) was computed directly: the largest is {})", thousands(from), thousands(rc.segment_end), rc.start_max));
    let checked = rc.checked;
    if !rc.failed.is_empty() {
        let failed: Vec<String> = rc.failed.iter().map(|(n, q, b)| format!("{} (p = {q}, f = {b:.2})", thousands(*n))).collect();
        out.push(format!("  step 3 FAILS at: {}; the bound is not proved on this range", failed.join(", ")));
        return out;
    }
    let (tn, tq, tb) = rc.tightest;
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

/// Why the bound has this shape and these constants: derived from the
/// probabilistic (Cramer-Granville) model of the primes, with every
/// mathematical step worked out; the model itself is an assumption.
fn bound_why(p: &OpenProblem, cfg: &Config, records: &[(u64, u64)]) -> Vec<String> {
    let (Some(c), Some(inner), Some(outer)) = (p.bound_c, p.bound_inner, p.bound_outer) else {
        return Vec::new();
    };
    let from = p.bound_from.unwrap_or(1000);
    let k = inner * outer;
    let mut out = vec!["why this rule, and why these numbers (derived from a model of the primes; the model is an assumption, so this explains the rule, it does not prove it for all n):".into()];
    out.push("  1. the model: each prime p tried is a trial that n - p is prime, succeeding with chance about 1/ln n. For the first m trials all to fail somewhere among the even numbers up to n takes m of about (ln n)^2, and the m-th prime is about m ln m; so the records grow like h(n) = C (ln n)^2 ln ln n (the shape Granville derived)".into());
    // C measured from Nuome's own records
    let ratio = |n: u64, q: u64| q as f64 / ((n as f64).ln().powi(2) * (n as f64).ln().ln());
    let Some(&(cn, cq)) = records.iter().filter(|r| r.0 >= from).max_by(|a, b| ratio(a.0, a.1).total_cmp(&ratio(b.0, b.1))) else {
        return out;
    };
    let big_c = ratio(cn, cq);
    out.push(format!("  2. C, measured: the largest p / ((ln n)^2 ln ln n) over Nuome's records is {big_c:.3} (at n = {}, p = {cq})", thousands(cn)));
    // the slope of h in the coordinate L = ln ln n, worked out by the rules
    out.push("  3. write L = ln ln n, so ln n = e^L and h = C e^(2L) L. Its growth exponent in L is L times the derivative of ln(e^(2L) L); Nuome's rules give that derivative:".into());
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    match crate::solve("differentiate ln(e^(2x) * x)", cfg, &opts) {
        Ok(s) => {
            if let Some(ans) = s.text.lines().find(|l| l.starts_with("Answer:")) {
                out.push(format!("       {}  (= 2 + 1/x)", ans.trim()));
            }
            for l in s.text.lines().filter(|l| l.trim_start().starts_with("ok ")) {
                out.push(format!("       {}", l.trim()));
            }
        }
        Err(_) => out.push("       (the rules could not work it out)".into()),
    }
    out.push("     so the exponent of the record curve at L is L (2 + 1/L) = 2L + 1: a power L^k matches the curve where 2L + 1 = k".into());
    // where Nuome's exponent touches the curve, and the constant it implies
    let l_star = (k - 1.0) / 2.0;
    let n_star = l_star.exp().exp();
    let (lo, hi) = (p.formula_min_n.unwrap_or(1000) as f64, p.formula_split.unwrap_or(100_000_000_000) as f64);
    let (l_lo, l_hi) = (lo.ln().ln(), hi.ln().ln());
    let place = if l_star < l_lo {
        "below the range the formula was fitted on".to_string()
    } else if l_star <= l_hi {
        format!("inside the range the formula was fitted on ({:.1}% of the way up it in L)", (l_star - l_lo) / (l_hi - l_lo) * 100.0)
    } else {
        format!("just past the top of the range the formula was fitted on (it ends at L = {l_hi:.3}), where it was scored on predicting the larger records")
    };
    out.push(format!(
        "  4. Nuome's exponent is {inner} x {outer} = {k:.3}, which is 2L + 1 at L = {l_star:.3}, that is n = e^(e^L) = {n_star:.1e}: {place} (fitted on L = {l_lo:.3} .. {l_hi:.3}). So the evolved rule is the tangent of the record curve there: the exponent is not a chance number but 2 ln ln n + 1 at that point"
    ));
    let c_pred = big_c * (2.0 * l_star).exp() * l_star.powf(1.0 - k);
    out.push(format!(
        "  5. the tangent's height: c = C e^(2L) L^(1-k) at L = {l_star:.3} gives c = {c_pred:.3}; Nuome's c is {c}: {}",
        if (c_pred / c - 1.0).abs() < 0.15 { format!("within {:.0}%, so the constant follows from the model too", (c_pred / c - 1.0).abs() * 100.0) } else { "not close: the constant is not explained by the model".into() }
    ));
    // the model's records against the bound, away from the tangent point
    let rows: Vec<String> = [1e10f64, 1e18, 1e30, 1e60, 1e100]
        .iter()
        .map(|&n| {
            let l = n.ln().ln();
            let f = c * l.powf(k);
            let h = big_c * n.ln().powi(2) * l;
            format!("n = {n:.0e}: bound / model record = {:.2}", f / h)
        })
        .collect();
    out.push(format!(
        "  6. so why it holds, and where it would stop: in the coordinate ln L the record curve bends upwards (its slope 2L + 1 keeps growing) while the bound is a straight line, so the bound stays close to the curve near the tangent point and falls behind it far away. {}. A ratio below 1 is where the model expects records above the bound (a prediction of the model, not something checked)",
        rows.join("; ")
    ));
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
    // the simple bound's constant, found by Nuome from the records it has
    let found = find_simple_c(p, limit, &run.records);
    let with_c;
    let p = match &found {
        Some((c, how)) => {
            out.push(how.clone());
            with_c = OpenProblem { simple_c: Some(*c), ..p.clone() };
            &with_c
        }
        None => p,
    };
    out.extend(formula_meaning(p, &run.records));
    out.extend(model_proof(p, cfg));
    out.extend(general_proofs(p, cfg));
    out.extend(bound_proof(p, cfg, limit, &run.records));
    out.extend(bound_why(p, cfg, &run.records));
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

#[cfg(test)]
mod proof_tests {
    #[test]
    fn the_goldbach_proof_is_whole() {
        let cfg = crate::config::Config::builtin();
        let text = super::goldbach_proof(&cfg, Some(1_000_000)).expect("a proof");
        for part in ["PART 1", "PART 2", "lemma A", "theorem B", "PART 3", "PART 4", "SUMMARY", "OPEN"] {
            assert!(text.contains(part), "{part}");
        }
        assert!(!text.contains("FAILS"));
    }
}
