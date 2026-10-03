use anyhow::{bail, Context, Result};
use clap::Parser;
use nuome::config::Config;
use nuome::print::Style;
use nuome::{explain, lexicon, solve, Options};
use std::path::PathBuf;

/// Nuome: ask a math question in plain words, get a worked solution where
/// every step names the rule that took it and every answer is checked.
///
///   nuome "solve 2x + 3 = 7"
///   nuome "factor x^2 - 5x + 6"
///   nuome "differentiate x^3 sin x step by step"
///   nuome "solve x^2 - 4x + 1 = 0 using the quadratic formula"
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The question. Several words are joined; omit to read --file.
    words: Vec<String>,
    /// Solve every line of a file (# starts a comment).
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Print math with √, ·, ² and ± instead of plain ASCII.
    #[arg(long, conflicts_with = "latex")]
    unicode: bool,
    /// Print math as LaTeX.
    #[arg(long)]
    latex: bool,
    /// Show the tokens, the request, the profile, every finalist and every check (stderr).
    #[arg(long)]
    explain: bool,
    /// Skip unknown words instead of failing.
    #[arg(long)]
    lenient: bool,
    /// Use this rules.toml instead of the built-in one.
    #[arg(long)]
    rules: Option<PathBuf>,
    /// List every word and phrase Nuome knows.
    #[arg(long)]
    vocabulary: bool,
    /// Check a graph (one line "a-b" per edge, # starts a comment) against
    /// Conway's 99-graph conditions, pair by pair, and draw it.
    #[arg(long)]
    graph: Option<PathBuf>,
    /// With --graph: repair it by the fitness search (line swaps that keep
    /// every degree), then check and draw the best graph found.
    #[arg(long, requires = "graph")]
    repair: bool,
    /// Search for the 99-graph for this many hours, round by round, from
    /// Joose's 3 x 33 grid and his 7-network graph; progress in out/overnight.log.
    #[arg(long)]
    overnight: Option<f64>,
    /// Print a whole proof as an answer: "goldbach" (the Goldbach bound, with
    /// what is proved and what is open). The words, if any, may say "up to 10^10".
    #[arg(long)]
    proof: Option<String>,
    /// Language of a whole proof: en (default) or fi.
    #[arg(long, default_value = "en")]
    lang: String,
    /// Make up new statements about a problem, test them on unseen numbers,
    /// keep the survivors: "goldbach".
    #[arg(long)]
    ideas: Option<String>,
    /// List the first N prime exponents p with 2^p - 1 of 100 million digits,
    /// each with a factor found by trial factoring or as a survivor; written
    /// to out/eff-candidates.tsv (factor depth: factor_bits in rules.toml).
    #[arg(long)]
    eff_list: Option<usize>,
    /// Deepen trial factoring of every survivor in out/eff-candidates.tsv to
    /// factors below 2^BITS (at most 62), all together; each result is written
    /// to out/eff-deepen.log at once and the list is updated at the end.
    #[arg(long)]
    eff_deepen: Option<u32>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.vocabulary {
        for w in lexicon::vocabulary() {
            println!("{w}");
        }
        return Ok(());
    }
    let cfg = match &cli.rules {
        Some(p) => Config::load(p)?,
        None => Config::builtin(),
    };
    if let Some(bits) = cli.eff_deepen {
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("out");
        let path = out_dir.join("eff-candidates.tsv");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}; run --eff-list first", path.display()))?;
        // the depth already cleared is in the first line: "below 2^N"
        let from: u32 = text.lines().next().and_then(|l| l.split("below 2^").nth(1)).and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next()).and_then(|n| n.parse().ok()).context("no depth in the list's first line")?;
        let bits = bits.min(62);
        if bits <= from {
            bail!("the survivors are already cleared below 2^{from}");
        }
        let survivors: Vec<u64> = text.lines().filter(|l| l.contains("\tsurvivor")).filter_map(|l| l.split('\t').next()?.parse().ok()).collect();
        println!("{} survivors, factors from 2^{from} to 2^{bits}, all together on every core; results in out/eff-deepen.log as they come", survivors.len());
        let log = std::sync::Mutex::new(std::fs::OpenOptions::new().create(true).append(true).open(out_dir.join("eff-deepen.log"))?);
        let found = std::sync::Mutex::new(std::collections::HashMap::new());
        let finished = std::sync::atomic::AtomicUsize::new(0);
        let t0 = std::time::Instant::now();
        nuome::mersenne::deepen(&survivors, from, bits, &|p, f| {
            use std::io::Write;
            let n = finished.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            let line = match f {
                Some(q) => format!("{n}/{} {p}: COMPOSITE, factor {q} (rechecked), {:.0} s", survivors.len(), t0.elapsed().as_secs_f64()),
                None => format!("{n}/{} {p}: no factor below 2^{bits}, still a survivor, {:.0} s", survivors.len(), t0.elapsed().as_secs_f64()),
            };
            let mut g = log.lock().expect("no panics");
            let _ = writeln!(g, "{line}");
            let _ = g.flush();
            if let Some(q) = f {
                found.lock().expect("no panics").insert(p, q);
            }
        });
        let found = found.into_inner().expect("done");
        // rewrite the list: new factors, and the new depth for the survivors
        let mut lines = text.lines();
        let head = lines.next().unwrap_or("").replace(&format!("below 2^{from}"), &format!("below 2^{bits}"));
        let mut new = format!("{head}\n");
        for l in lines {
            let p: Option<u64> = l.split('\t').next().and_then(|x| x.parse().ok());
            match p.and_then(|p| found.get(&p).map(|q| (p, q))) {
                Some((p, q)) => {
                    let digits = l.split('\t').nth(1).unwrap_or("");
                    new.push_str(&format!("{p}\t{digits}\tcomposite\t{q}\n"));
                }
                None => new.push_str(&format!("{l}\n")),
            }
        }
        std::fs::write(&path, new)?;
        println!("{} of {} survivors now have a factor; {} survivors left, cleared below 2^{bits} ({:.0} s)", found.len(), survivors.len(), survivors.len() - found.len(), t0.elapsed().as_secs_f64());
        return Ok(());
    }
    if let Some(count) = cli.eff_list {
        let bits = cfg.open.get("eff_prime").and_then(|p| p.factor_bits).unwrap_or(50);
        let t0 = std::time::Instant::now();
        let list = nuome::mersenne::candidates(count, bits);
        let mut tsv = format!("# the first {count} prime exponents p with 2^p - 1 of 100 million digits; factor = smallest factor below 2^{bits} (2^p - 1 composite), survivor = none below 2^{bits} (needs a full test)\nexponent\tdigits\tstatus\tfactor\n");
        for (p, f) in &list {
            let digits = (*p as f64 * std::f64::consts::LOG10_2).floor() as u64 + 1;
            match f {
                Some(q) => tsv.push_str(&format!("{p}\t{digits}\tcomposite\t{q}\n")),
                None => tsv.push_str(&format!("{p}\t{digits}\tsurvivor\t\n")),
            }
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("out").join("eff-candidates.tsv");
        std::fs::create_dir_all(path.parent().expect("out"))?;
        std::fs::write(&path, tsv)?;
        let survivors = list.iter().filter(|x| x.1.is_none()).count();
        println!("{} candidates, {} composite, {survivors} survivors ({:.1} s); written to {}", list.len(), list.len() - survivors, t0.elapsed().as_secs_f64(), path.display());
        return Ok(());
    }
    if let Some(which) = &cli.ideas {
        if which == "conway99" {
            let p = cfg.open.get("conway99");
            println!(
                "{}",
                nuome::ideas::conway99(
                    p.and_then(|p| p.evolve_population).unwrap_or(120),
                    p.and_then(|p| p.evolve_generations).unwrap_or(100),
                    p.and_then(|p| p.evolve_seed).unwrap_or(2026)
                )
            );
            return Ok(());
        }
        if which == "prime-range" {
            let p = cfg.open.get("eff_prime");
            let digits = nuome::attempt::requested_limit(&cli.words.join(" ")).unwrap_or(100_000_000);
            println!(
                "{}",
                nuome::prime_formula::range(
                    &nuome::prime_formula::Settings { generations: p.and_then(|p| p.evolve_generations).unwrap_or(300), seed: p.and_then(|p| p.evolve_seed).unwrap_or(2026) },
                    digits
                )
            );
            return Ok(());
        }
        if which == "check-candidate" {
            // check-candidate DIGITS K LIMIT: trial factoring of 10^(DIGITS-1) + K from 10^6 to LIMIT
            let nums: Vec<u64> = cli.words.iter().filter_map(|w| w.replace(',', "").replace('_', "").parse().ok()).collect();
            let (digits, k, limit) = (nums.first().copied().unwrap_or(100_000_000), nums.get(1).copied().unwrap_or(13), nums.get(2).copied().unwrap_or(10_000_000_000));
            let t0 = std::time::Instant::now();
            println!("checking 10^{} + {k}: trial factoring by every prime from 1,000,000 to {limit} ...", digits - 1);
            match nuome::bigprime::deep_factor(digits, k, 1_000_000, limit) {
                Some(p) => println!("COMPOSITE: divisible by {p} (10^{} mod {p} = {}, plus {k} is a multiple of {p}) ({:.1} s)", digits - 1, (p - (k % p)) % p, t0.elapsed().as_secs_f64()),
                None => println!("no prime factor below {limit}: still a candidate (not a proof of anything) ({:.1} s)", t0.elapsed().as_secs_f64()),
            }
            return Ok(());
        }
        if which == "supergenius-conway" {
            // supergenius-conway [SECONDS]: the supergenius on Conway's 99-graph
            let seconds = cli.words.iter().find_map(|w| w.parse().ok()).unwrap_or(300.0);
            println!("{}", nuome::supergenius_conway::report(seconds));
            return Ok(());
        }
        if which == "three-cubes" {
            // three-cubes [BOUND] [K ...]: the open k below 1000 by default
            let nums: Vec<i64> = cli.words.iter().filter_map(|w| w.replace(',', "").replace('_', "").parse().ok()).collect();
            let bound = nums.first().copied().unwrap_or(1_000_000_000);
            let ks: Vec<i64> = if nums.len() > 1 { nums[1..].to_vec() } else { nuome::three_cubes::OPEN.to_vec() };
            println!("{}", nuome::three_cubes::report(&ks, bound));
            return Ok(());
        }
        if which == "supergenius-shortcut" {
            let generations = cli.words.iter().find_map(|w| w.parse().ok()).unwrap_or(300);
            println!("{}", nuome::supergenius_shortcut::report(100_000_000, &nuome::prime_shortcut::Settings { population: 800, generations, islands: 12, seed: 2026 }));
            return Ok(());
        }
        if which == "supergenius" {
            let digits = nuome::supergenius::digits_asked(&cli.words.join(" ")).unwrap_or(100_000_000);
            return supergenius(digits);
        }
        if which == "prime-shortcut" {
            println!("{}", nuome::prime_shortcut::report(&nuome::prime_shortcut::Settings { population: 800, generations: cli.words.iter().find_map(|w| w.parse().ok()).unwrap_or(300), islands: 12, seed: 2026 }));
            return Ok(());
        }
        if which == "prime-learn" {
            println!("{}", nuome::prime_learn::report(&nuome::prime_learn::Settings { population: 1000, generations: 400, seed: 2026, islands: 12, learn_to: 140, upto: 200 }));
            return Ok(());
        }
        if which == "prime-evolve" {
            println!(
                "{}",
                nuome::prime_evolve::report(&nuome::prime_evolve::Settings { generations: 150, population: 400, seed: 2026, band: 0.01 })
            );
            return Ok(());
        }
        if which == "prime-function" {
            println!(
                "{}",
                nuome::prime_function::report(&nuome::prime_function::Settings { count: 100_000, far: 1_000_000, max_terms: 6, price: 0.0001 })
            );
            return Ok(());
        }
        if which == "prime-formula" {
            let p = cfg.open.get("eff_prime");
            println!(
                "{}",
                nuome::prime_formula::report(&nuome::prime_formula::Settings {
                    generations: p.and_then(|p| p.evolve_generations).unwrap_or(300),
                    seed: p.and_then(|p| p.evolve_seed).unwrap_or(2026),
                })
            );
            return Ok(());
        }
        if which == "mersenne" {
            let p = cfg.open.get("eff_prime");
            println!(
                "{}",
                nuome::mersenne_ideas::report(&nuome::mersenne_ideas::Settings {
                    generations: p.and_then(|p| p.evolve_generations).unwrap_or(150),
                    seed: p.and_then(|p| p.evolve_seed).unwrap_or(2026),
                    limit: 10_000_000,
                })
            );
            return Ok(());
        }
        if which != "goldbach" {
            bail!("no idea machine for \"{which}\"; try --ideas goldbach, --ideas conway99 or --ideas mersenne");
        }
        let limit = nuome::attempt::requested_limit(&cli.words.join(" ")).unwrap_or(400_000) as usize;
        println!("{}", nuome::ideas::report(&nuome::ideas::Settings { limit, from: 1_000 }));
        return Ok(());
    }
    if let Some(which) = &cli.proof {
        if which != "goldbach" {
            bail!("no whole proof called \"{which}\"; try --proof goldbach");
        }
        let limit = nuome::attempt::requested_limit(&cli.words.join(" "));
        let text = match cli.lang.as_str() {
            "fi" => nuome::goldbach_fi::goldbach_proof(&cfg, limit),
            "en" => nuome::attempt::goldbach_proof(&cfg, limit),
            other => bail!("no language \"{other}\"; try --lang en or --lang fi"),
        }
        .context("rules.toml does not turn on the Goldbach bound (simple_bound)")?;
        println!("{text}");
        return Ok(());
    }
    if let Some(hours) = cli.overnight {
        let problem = cfg.open.get("conway99");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let seven_path = root.join(problem.and_then(|p| p.seed_graph.clone()).unwrap_or_else(|| "data/conway/joose-7-networks.txt".into()));
        let seven = nuome::conway::parse_edges(&std::fs::read_to_string(&seven_path).with_context(|| format!("reading {}", seven_path.display()))?).context("the 7-network graph")?;
        let s = nuome::conway::Evolve { population: 200, generations: 300, seed: problem.and_then(|p| p.evolve_seed).unwrap_or(2026), climb: 600 };
        println!("{}", nuome::conway::overnight(hours, &root.join("out"), &seven, &s));
        return Ok(());
    }
    if let Some(f) = &cli.graph {
        let text = std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?;
        let problem = cfg.open.get("conway99");
        let pictures = problem.and_then(|p| p.pictures.clone());
        let text = if cli.repair {
            let Some(g) = nuome::conway::parse_edges(&text) else { bail!("{} is not a list of \"a-b\" lines on at most 128 points", f.display()) };
            let s = nuome::conway::Evolve {
                population: problem.and_then(|p| p.evolve_population).unwrap_or(120),
                generations: problem.and_then(|p| p.evolve_generations).unwrap_or(100),
                seed: problem.and_then(|p| p.evolve_seed).unwrap_or(2026),
                climb: problem.and_then(|p| p.evolve_climb).unwrap_or(300),
            };
            let e = nuome::conway::evolve_from(&g, &s);
            let steps: Vec<String> = e.history.iter().map(|h| h.to_string()).collect();
            println!("repair by the fitness search ({} graphs, {} generations, seed {}, line swaps that keep every degree)", s.population, s.generations, s.seed);
            println!("fitness (common-neighbour counts off, summed over all pairs; 0 = solved): {}", steps.join(" -> "));
            let repaired = nuome::conway::edge_list(&e.best_graph);
            let saved = f.with_file_name(format!("{}-repaired.txt", f.file_stem().map_or("graph".into(), |s| s.to_string_lossy())));
            std::fs::write(&saved, &repaired).with_context(|| format!("writing {}", saved.display()))?;
            println!("the repaired graph: {}", saved.display());
            println!("its check:");
            repaired
        } else {
            text
        };
        let (lines, ok) = nuome::conway::check_text(&text, pictures.as_deref());
        for l in lines {
            println!("{l}");
        }
        if !ok {
            bail!("not a solution");
        }
        return Ok(());
    }
    let style = if cli.latex {
        Style::Latex
    } else if cli.unicode {
        Style::Unicode
    } else {
        Style::Ascii
    };
    let opts = Options { lenient: cli.lenient, style };
    let mut questions = Vec::new();
    if let Some(f) = &cli.file {
        let text = std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?;
        questions.extend(text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from));
    }
    if !cli.words.is_empty() {
        questions.push(cli.words.join(" "));
    }
    if questions.is_empty() {
        bail!("ask something, e.g. nuome \"solve 2x + 3 = 7\"");
    }
    // "find the first 100 million digit prime": the supergenius answers
    if let [q] = &questions[..] {
        if let Some(digits) = nuome::supergenius::asks(q) {
            return supergenius(digits);
        }
    }
    let mut failed = 0;
    for (i, q) in questions.iter().enumerate() {
        if i > 0 {
            println!("\n{}\n", "-".repeat(60));
        }
        match solve(q, &cfg, &opts) {
            Ok(s) => {
                if cli.explain {
                    eprint!("{}", explain::explain(&s.request, &s.outcome, &cfg));
                }
                print!("{}", s.text);
            }
            Err(diags) => {
                failed += 1;
                if questions.len() > 1 {
                    eprintln!("{q}");
                }
                for d in &diags {
                    eprintln!("{d}");
                }
            }
        }
    }
    if failed > 0 {
        bail!("{failed} question{} not answered", if failed == 1 { "" } else { "s" });
    }
    Ok(())
}

/// The supergenius's report, with the whole number written to a file when
/// it is too long to print.
fn supergenius(digits: u64) -> Result<()> {
    let found = nuome::supergenius::answer(digits);
    println!("{}", nuome::supergenius::report_with(digits, &found));
    if let Some(n) = &found.number {
        let s = n.to_string();
        if s.len() > 300 {
            let path = std::path::PathBuf::from("out").join("supergenius").join(format!("first_prime_{digits}_digits.txt"));
            std::fs::create_dir_all(path.parent().expect("dir"))?;
            std::fs::write(&path, format!("{s}
"))?;
            println!("the whole number: {}", path.display());
        }
    }
    Ok(())
}
