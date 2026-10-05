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
    /// Golden Answer: answer the question as "logical" (derived, every step
    /// checked), "theoretical" (from known results, named) or "abstract"
    /// (probably right, with its reasons); "bench" runs the fixed test set.
    #[arg(long)]
    answer: Option<String>,
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
    if let Some(kind) = &cli.answer {
        if kind == "bench" {
            print!("{}", nuome::golden_answer::bench_report());
            return Ok(());
        }
        let Some(k) = nuome::golden_answer::Kind::parse(kind) else { bail!("--answer takes logical, theoretical, abstract or bench") };
        print!("{}", nuome::golden_answer::answer(k, &cli.words.join(" ")).render());
        return Ok(());
    }
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
        if which == "addmult6" {
            // addmult6 [MAX] [COUNT] [SECONDS] [LO] [HI]: Boyer's enigma #6a
            let n: Vec<f64> = cli.words.iter().filter_map(|w| w.replace(',', "").replace('_', "").parse().ok()).collect();
            let get = |i: usize, d: f64| n.get(i).copied().unwrap_or(d);
            let _ = std::fs::create_dir_all("out/addmult");
            println!("{}", nuome::addmult::campaign(get(3, 1e9) as u128, get(4, 1e12) as u128, get(0, 300.0) as u64, get(1, 20.0) as usize, get(2, 60.0), std::path::Path::new("out/addmult/log.txt")));
            return Ok(());
        }
        if which == "golden" {
            // golden "<fen>": the golden function's move, with the best moves' values (formula from out/golden/formula.txt);
            // a trailing number (an old depth) is ignored: the function is one formula, no search
            let text = cli.words.join(" ");
            let depth = cli.words.last().and_then(|w| w.parse::<usize>().ok()).filter(|d| *d <= 8);
            let fen = if depth.is_some() { cli.words[..cli.words.len() - 1].join(" ") } else { text.clone() };
            let fen = if fen.trim().is_empty() || fen.trim() == "startpos" { nuome::golden::Board::start().fen() } else { fen };
            let b = nuome::golden::Board::from_fen(fen.trim()).map_err(|e| anyhow::anyhow!(e))?;
            // FEN -> NEURO (the position as a network, by the rules) -> GOLDEN (the move that leaves the best network)
            let t0 = std::time::Instant::now();
            // Golden Boy with Golden Answer's three kinds: proved (logical), known results (theoretical), patterns (abstract)
            match nuome::golden_chess::play(&b.fen()) {
                None => println!("{}", if b.in_check() { "checkmate: no move" } else { "stalemate: no move" }),
                Some((mv, kind, lines)) => {
                    println!("Golden Boy on {} ({} to move)", b.fen(), if b.white { "white" } else { "black" });
                    for l in lines {
                        println!("  {l}");
                    }
                    println!("move: {mv} ({kind}, {:.3} s)", t0.elapsed().as_secs_f64());
                }
            }
            return Ok(());
        }
        if which == "chess-rules-data" {
            // chess-rules-data COUNT FILE [SEED]: random legal positions with the rules' exact answers (for Neuras)
            let count = cli.words.first().and_then(|w| w.replace('_', "").parse().ok()).unwrap_or(100_000);
            let path = cli.words.get(1).cloned().unwrap_or_else(|| "out/golden/rules_train.bin".into());
            let seed = cli.words.get(2).and_then(|w| w.parse().ok()).unwrap_or(1);
            let _ = std::fs::create_dir_all("out/golden");
            let t0 = std::time::Instant::now();
            println!("{} ({:.1} s)", nuome::chess_rules_data::write(&path, count, seed).map_err(|e| anyhow::anyhow!(e))?, t0.elapsed().as_secs_f64());
            return Ok(());
        }
        if which == "chess-network" {
            // chess-network [fool|scholar|legal|opera|all]: how chess works as a 64-node network, and famous games laid into it
            println!("{}", nuome::chess_network::report(cli.words.first().map(|s| s.as_str()).unwrap_or("all")).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-selfplay" {
            // golden-selfplay [ROUNDS] [GAMES]: train the golden function by self-play (a mutant must beat it)
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            let (rounds, games) = (n.first().copied().unwrap_or(200), n.get(1).copied().unwrap_or(24));
            let t0 = std::time::Instant::now();
            nuome::golden_terms::selfplay(rounds, games, 2026, &mut |l| println!("{l}")).map_err(|e| anyhow::anyhow!(e))?;
            println!("({:.0} s)", t0.elapsed().as_secs_f64());
            return Ok(());
        }
        if which == "neuro" {
            // neuro "<fen>": the position written as a network, its numbers, and GOLDEN's move
            println!("{}", nuome::neuro::report(&cli.words.join(" ")).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-lumberjack" {
            // golden-lumberjack FENS_FILE [SECONDS]: brute force proves, the lumberjack fells, Golden Answer chooses among the trees left
            let path = cli.words.first().context("golden-lumberjack takes a file of FENs")?;
            let secs = cli.words.get(1).and_then(|w| w.parse().ok()).unwrap_or(nuome::golden_brute::SECONDS);
            for fen in std::fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()) {
                print!("{}", nuome::golden_brute::golden_boy_report(fen.trim(), secs).map_err(|e| anyhow::anyhow!(e))?);
            }
            return Ok(());
        }
        if which == "golden-table-fens" {
            // golden-table-fens: every distinct opening position with its split, for Stockfish's analysis
            println!("{} positions written to out/golden/table_fens.txt", nuome::golden_table::write_fens("out/golden/table_fens.txt").map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldennet-profile" {
            // goldennet-profile "<fen>": where a leaf's time goes (moves, points, the frozen sum), one core
            print!("{}", nuome::golden_table::profile(&cli.words.join(" "), 4).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldenboy-net" {
            // goldenboy-net "<fen>" [SECONDS]: Golden Boy's move with the opening table and Goldennet
            let fen = cli.words.iter().filter(|w| w.parse::<f64>().is_err()).cloned().collect::<Vec<_>>().join(" ");
            let secs = cli.words.iter().filter_map(|w| w.parse::<f64>().ok()).last().unwrap_or(5.0);
            let (m, how) = nuome::golden_table::goldenboy_move(&fen, secs).map_err(|e| anyhow::anyhow!(e))?;
            println!("move: {m} ({how})");
            return Ok(());
        }
        if which == "goldennet-learn" {
            // goldennet-learn FILE [EPOCHS]: Goldennet learns Stockfish's moves (fen TAB move) and is frozen again
            let path = cli.words.first().context("goldennet-learn takes a file")?;
            let e = cli.words.get(1).and_then(|w| w.parse().ok()).unwrap_or(1);
            print!("{}", nuome::golden_table::learn_from(path, e).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldennet-speed" {
            // goldennet-speed "<fen>" [SECONDS]: how deep Goldennet gets, every line, in the time
            let fen = cli.words.iter().filter(|w| w.parse::<f64>().is_err()).cloned().collect::<Vec<_>>().join(" ");
            let secs = cli.words.iter().filter_map(|w| w.parse::<f64>().ok()).last().unwrap_or(10.0);
            print!("{}", nuome::golden_table::speed(&fen, secs).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldennet-value" {
            // goldennet-value [N_TRAIN] [EPOCHS] [MAX_DEPTH]: Goldennet as a value (Stockfish's win probability), then searched deeper
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::teach_value(n.first().copied().unwrap_or(500_000), n.get(1).copied().unwrap_or(3), n.get(2).copied().unwrap_or(3)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldennet-deep" {
            // goldennet-deep [N_MID] [N_END] [MAX_DEPTH]: Goldennet at the leaves of a full-width search, on a holdout sample
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::report_deep(n.first().copied().unwrap_or(1000), n.get(1).copied().unwrap_or(500), n.get(2).copied().unwrap_or(3)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "goldennet" {
            // goldennet [N_TRAIN] [EPOCHS]: the grid net taught the table's voice (middlegame and endgame)
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::teach_goldennet(n.first().copied().unwrap_or(1_000_000), n.get(1).copied().unwrap_or(5)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table-cover" {
            // golden-table-cover [MAX_GAMES]: how much of the CCRL middlegame and endgame the full table knows
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::coverage_ccrl(n.first().copied().unwrap_or(20_000)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table-learn" {
            // golden-table-learn: teach the table the middlegame and endgame (6+ pieces) positions with Stockfish's moves
            print!("{}", nuome::golden_table::learn_mid_end().map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table-all" {
            // golden-table-all: every known opening position into the table (out/golden/opening_table.tsv)
            print!("{}", nuome::golden_table::write_all().map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table-only" {
            // golden-table-only: the table written from Stockfish's analysed moves, its coverage on the holdout
            print!("{}", nuome::golden_table::report_table().map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table-sf" {
            // golden-table-sf [EPOCHS]: the table and net against Stockfish's analysis (tools/golden_table_sf.py first)
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::report_sf(n.first().copied().unwrap_or(4)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-table" {
            // golden-table [MAX_GAMES] [EPOCHS]: Golden Answer's opening table and grid net, reported on the holdout
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            print!("{}", nuome::golden_table::report(n.first().copied().unwrap_or(usize::MAX), n.get(1).copied().unwrap_or(3)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-matepath" {
            // golden-matepath FENS_FILE: the shortest mate path of every position in the file
            let path = cli.words.first().context("golden-matepath takes a file of FENs")?;
            for fen in std::fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()) {
                print!("{}", nuome::golden_mate_path::report(fen.trim()).map_err(|e| anyhow::anyhow!(e))?);
            }
            return Ok(());
        }
        if which == "golden-brute" {
            // golden-brute FENS_FILE [PLIES] [SECONDS]: every line, deepening to PLIES within SECONDS a position, no evaluation; what the rules prove
            let path = cli.words.first().context("golden-brute takes a file of FENs")?;
            let plies = cli.words.get(1).and_then(|w| w.parse().ok()).unwrap_or(nuome::golden_brute::PLIES);
            let secs = cli.words.get(2).and_then(|w| w.parse().ok()).unwrap_or(nuome::golden_brute::SECONDS);
            for fen in std::fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()) {
                print!("{}", nuome::golden_brute::report(fen.trim(), plies, secs).map_err(|e| anyhow::anyhow!(e))?);
            }
            return Ok(());
        }
        if which == "golden-formula" {
            // golden-formula FENS_FILE: the Golden formula's move for every FEN in the file, one per line
            let path = cli.words.first().context("golden-formula takes a file of FENs")?;
            for fen in std::fs::read_to_string(path)?.lines().filter(|l| !l.trim().is_empty()) {
                let m = nuome::golden_formula::rank(fen.trim()).and_then(|r| r.first().map(|x| x.0.clone())).unwrap_or_else(|| "none".into());
                println!("{m}");
            }
            return Ok(());
        }
        if which == "golden-heart" {
            // golden-heart [MAX_GAMES] [GAMES FILE]: the HotMotor heart cloned from Stockfish's games
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.replace('_', "").parse().ok()).collect();
            let path = cli.words.iter().find(|w| w.ends_with(".txt")).cloned().unwrap_or_else(|| "out/golden/ccrl_sf_games.txt".to_string());
            println!("{}", nuome::golden_heart::clone_heart(&path, n.first().copied().unwrap_or(usize::MAX)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-voice" {
            // golden-voice [ROUNDS] [GAMES FILE]: clone Stockfish's play from its games into a static network eval
            // golden-voice [ROUNDS] [MAX_GAMES] [GAMES FILE]
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.replace('_', "").parse().ok()).collect();
            let path = cli.words.iter().find(|w| w.ends_with(".txt")).cloned().unwrap_or_else(|| "out/golden/sf_games.txt".to_string());
            println!("{}", nuome::golden_voice::report(&path, n.first().copied().unwrap_or(20)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-weights" {
            // golden-weights [TRAIN] [VAL] [VAL_FROM] [FILE] (VAL_FROM past the discovery half): optimize the patterns' weights against Stockfish, kept only when better on unseen positions
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.replace('_', "").parse().ok()).collect();
            let path = cli.words.iter().find(|w| w.ends_with(".jsonl")).cloned().unwrap_or_else(|| "out/golden/discover_mix.jsonl".to_string());
            println!("{}", nuome::patterns::optimize(&path, n.first().copied().unwrap_or(200_000), n.get(1).copied().unwrap_or(100_000), n.get(2).copied().unwrap_or(600_000)).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-discover" {
            // golden-discover [N]: Golden Boy finds Stockfish's recurring patterns in N positions, tested on the next N
            let n = cli.words.first().and_then(|w| w.replace('_', "").parse().ok()).unwrap_or(50_000);
            // golden-discover [N] [FILE]: the file defaults to the million Stockfish-evaluated positions exported from the parquet shards
            let path = cli.words.get(1).cloned().unwrap_or_else(|| "out/golden/sf_positions_1m.jsonl".to_string());
            println!("{}", nuome::patterns::report(&path, n).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-learn-243" {
            // golden-learn-243 [SECONDS]: Golden learns the given 243-graph in its own language
            let secs = cli.words.first().and_then(|w| w.parse().ok()).unwrap_or(120.0);
            match nuome::golden_learn::learn_243(secs) {
                Some(l) => println!("learned the 243-graph's far adjacencies: {} of {} pairs wrong in {:.0} s
E243 = {}", l.wrong, l.pairs, l.seconds, l.expr.text()),
                None => println!("the 243-graph could not be built or written in the frame"),
            }
            return Ok(());
        }
        if which == "graph-orbits" {
            // graph-orbits v k l m p [SECONDS]: an srg with an automorphism of prime order p fixing one vertex
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            if n.len() < 5 {
                bail!("graph-orbits v k l m p [seconds]");
            }
            print!("{}", nuome::graph_orbits::report(n[0], n[1], n[2], n[3], n[4], n.get(5).copied().unwrap_or(30) as f64));
            return Ok(());
        }
        if which == "graph-function" {
            // graph-function [v k l m]... [SECONDS]: the graph function finds the known graphs again, then tries the given ones
            let nums: Vec<u64> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            let seconds = if nums.len() % 4 == 1 { nums[nums.len() - 1] as f64 } else { 60.0 };
            let targets: Vec<(u64, u64, u64, u64)> = nums.chunks_exact(4).map(|c| (c[0], c[1], c[2], c[3])).collect();
            println!("{}", nuome::graph_function::report(&targets, seconds));
            return Ok(());
        }
        if which == "golden-whole" {
            // golden-whole [TRUTH FILES...]: the supergenius writes the whole golden function at once
            let given: Vec<&str> = cli.words.iter().map(|s| s.as_str()).collect();
            let files: Vec<&str> = if given.is_empty() { vec!["out/golden/truth_cp_s3.txt", "out/golden/truth_cp_s4.txt"] } else { given };
            let t0 = std::time::Instant::now();
            let terms = nuome::golden_terms::write_whole(&files, &mut |l| println!("{l}")).map_err(|e| anyhow::anyhow!(e))?;
            let text = nuome::golden_terms::written(&nuome::golden::Board::start(), &terms);
            println!("\nTHE GOLDEN FUNCTION ({} terms, {} characters for the start position), saved to {} ({:.0} s)", terms.len(), text.len(), nuome::golden_terms::TERMS_FILE, t0.elapsed().as_secs_f64());
            std::fs::write("out/golden/golden_function.txt", &text)?;
            println!("written in full to out/golden/golden_function.txt");
            return Ok(());
        }
        if which == "golden-polish" {
            // golden-polish [MAX_TERMS]: every found term into one golden function, polished on all the exact truth
            // golden-polish [tune] [TRUTH FILES...]: add terms until none helps (no limit); "tune" instead tunes every weight once
            // golden-polish supergenius [FILES...]: only the supergenius's reasoned terms are tried
            let tune = cli.words.first().map(|w| w.as_str()) == Some("tune");
            let sg = cli.words.first().map(|w| w.as_str()) == Some("supergenius");
            let given: Vec<&str> = cli.words.iter().filter(|w| w.as_str() != "tune" && w.as_str() != "supergenius").map(|s| s.as_str()).collect();
            let only = sg.then(|| nuome::golden_terms::supergenius_terms().into_iter().map(|x| x.0).collect::<Vec<_>>());
            let files: Vec<&str> = if given.is_empty() { vec!["out/golden/truth_34.txt", "out/golden/truth_5.txt"] } else { given };
            let t0 = std::time::Instant::now();
            let terms = nuome::golden_terms::polish(&files, tune, only, &mut |l| println!("{l}")).map_err(|e| anyhow::anyhow!(e))?;
            let text = nuome::golden_terms::written(&nuome::golden::Board::start(), &terms);
            println!("\nTHE GOLDEN FUNCTION: the supergenius's formula for the position + {} terms Nuome found (written for the start position, {} characters):\n{text}", terms.len(), text.len());
            println!("saved to {} ({:.0} s)", nuome::golden_terms::TERMS_FILE, t0.elapsed().as_secs_f64());
            return Ok(());
        }
        if which == "golden-terms" {
            // golden-terms [MAX_TERMS]: Nuome looks for new terms of the golden function against the endgame tables
            let k = cli.words.first().and_then(|w| w.parse().ok()).unwrap_or(4);
            println!("{}", nuome::golden_terms::report("out/golden/truth_34.txt", "out/golden/truth_5.txt", k).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "supergenius-golden" {
            // supergenius-golden: the supergenius writes the golden function from the rules of chess (no evolution)
            let _ = std::fs::create_dir_all("out/golden");
            println!("{}", nuome::supergenius_golden::report(&cli.words.join(" ")).map_err(|e| anyhow::anyhow!(e))?);
            return Ok(());
        }
        if which == "golden-play" {
            // golden-play [GAMES]: games of the golden function against itself -> out/golden/games.txt
            let n: Vec<usize> = cli.words.iter().filter_map(|w| w.parse().ok()).collect();
            let games = n.first().copied().unwrap_or(8);
            let openings = nuome::golden::OPENINGS;
            let _ = std::fs::create_dir_all("out/golden");
            println!("the supergenius writes a new golden function for every position");
            let results: Vec<(usize, Vec<String>, String, String, f64)> = std::thread::scope(|sc| {
                let hs: Vec<_> = (0..games).map(|g| {
                    let op = openings[g % openings.len()];
                    sc.spawn(move || {
                        let t0 = std::time::Instant::now();
                        let (moves, result, how) = nuome::golden::self_play_supergenius(op, 300);
                        (g, moves, result, how, t0.elapsed().as_secs_f64())
                    })
                }).collect();
                hs.into_iter().map(|h| h.join().expect("game")).collect()
            });
            let mut text = String::new();
            for (g, moves, result, how, secs) in &results {
                println!("game {}: {} after {} plies ({how}, {secs:.0} s)", g + 1, result, moves.len());
                text.push_str(&format!("{result}	{how}	{}
", moves.join(" ")));
            }
            std::fs::write("out/golden/games.txt", text)?;
            println!("games written to out/golden/games.txt (one per line: result, how, UCI moves)");
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
