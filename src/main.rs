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
