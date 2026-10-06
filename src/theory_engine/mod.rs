//! The theory engine: the chess move by Nuome's own architecture, the one
//! the mathematics is solved with.
//!
//!   sentence -> lexicon (tokens) -> parser (the position, the profile the
//!   words ask for, the facts of the position) -> rules (each principle of
//!   the known theory offers the moves that follow it) -> beam search over
//!   plans (profile and judges score them) -> checks (every finalist is
//!   proven by the rules of the game alone: no forced mate, no forced loss)
//!   -> render (the move, its rule, the fact that asked for it, the line,
//!   the judges, the proof)
//!
//! No model and no learned weight in the loop: the same sentence and the
//! same data/chess_rules.toml give the same move. The known positions (the
//! opening table, the data, the tables) are answered before any of this
//! runs, as logical moves.
//!
//! - lexicon.rs: closed vocabulary (a FEN, the grandmasters, a few words)
//! - parser.rs:  Request (profile) and Situation (facts)
//! - rules.rs:   the principles as rules offering moves (their measures and
//!               tests are src/golden_theory.rs)
//! - search.rs:  the beam over plans, the opponent answering by the rules
//! - judges.rs:  breakable preferences, forgiven by the profile
//! - checks.rs:  the proof, by the rules-only search
//! - data/chess_rules.toml: every number

pub mod checks;
pub mod config;
pub mod judges;
pub mod lexicon;
pub mod parser;
pub mod rules;
pub mod search;

use crate::fast::position::{move_to_uci, Position};
use config::Config;

pub struct Outcome {
    pub mv: String,
    pub lines: Vec<String>,
}

/// The move `sentence` asks for ("best move in FEN [like Tal]").
pub fn think(sentence: &str) -> Result<Outcome, String> {
    let cfg = Config::load()?;
    let req = parser::parse(sentence, &cfg)?;
    think_fen(&req.fen, &req.profile, &req.words, &cfg)
}

pub fn think_fen(fen: &str, profile: &config::Axes, words: &[String], cfg: &Config) -> Result<Outcome, String> {
    let p = Position::from_fen(fen)?;
    let mut lines = vec![format!(
        "request: profile initiative {:.1} safety {:.1} simplicity {:.1}{}",
        profile.initiative,
        profile.safety,
        profile.simplicity,
        if words.is_empty() { String::new() } else { format!(" ({})", words.join(", ")) }
    )];
    let sit = parser::Situation::read(&p);
    lines.push(format!("facts: {}", sit.describe()));
    let plans = search::search(&p, &sit, profile, cfg);
    if plans.is_empty() || plans[0].steps.is_empty() {
        return Err("no legal move".into());
    }
    for (i, pl) in plans.iter().take(3).enumerate() {
        lines.push(format!("plan {}: {} | material {:+} judges {} = {}", i + 1, pl.line(), pl.material, pl.notes.iter().map(|n| n.text.clone()).collect::<Vec<_>>().join(", "), pl.score()));
    }
    let reference = checks::reference(fen, cfg).ok_or("no reference")?;
    lines.push(format!("reference by the rules alone: {} {:+}", reference.mv, reference.value));
    let mut finalists: Vec<u32> = Vec::new();
    for pl in &plans {
        if !finalists.contains(&pl.first()) {
            finalists.push(pl.first());
        }
    }
    finalists.truncate(cfg.checks.finalists.max(1));
    for &f in &finalists {
        let c = checks::check(&p, f, &reference, cfg);
        lines.push(format!("check {}", c.note));
        if c.passed {
            let plan = plans.iter().find(|pl| pl.first() == f).unwrap();
            let v = plan.steps[0].variant.as_ref().unwrap();
            lines.push(format!("move {} by rule {} (asked by: {}), proven", move_to_uci(f), v.rule, v.why));
            return Ok(Outcome { mv: move_to_uci(f), lines });
        }
    }
    lines.push(format!("no plan passed the checks; the reference move {} is played", reference.mv));
    Ok(Outcome { mv: reference.mv, lines })
}

/// The smoke test: (fen TAB move) lines, reversed when `reverse`, each reported at once; the full lines in `log`.
pub fn smoke(file: &str, words: &str, reverse: bool, log: &str) -> Result<(), String> {
    use std::io::Write;
    let cfg = Config::load()?;
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    if reverse {
        rows.reverse();
    }
    let mut out = std::fs::File::create(log).map_err(|e| e.to_string())?;
    let mut hit = 0;
    for (i, (fen, sf)) in rows.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let req = parser::parse(&format!("best move in {fen} {words}"), &cfg)?;
        let o = think_fen(fen, &req.profile, &req.words, &cfg)?;
        let right = o.mv == *sf;
        hit += right as usize;
        let _ = writeln!(out, "{}/{} {fen} - Stockfish {sf}\n{}\n", i + 1, rows.len(), o.lines.join("\n"));
        println!("  {}/{}: played {}, Stockfish {sf} {} ({:.1} s) - {} - {hit}/{} so far", i + 1, rows.len(), o.mv, if right { "RIGHT" } else { "miss" }, t0.elapsed().as_secs_f64(), o.lines.last().cloned().unwrap_or_default(), i + 1);
        let _ = std::io::stdout().flush();
    }
    println!("theory engine: {hit}/{}", rows.len());
    Ok(())
}
