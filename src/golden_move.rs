//! Golden Move: Golden Answer's one answer kind for a chess position, "what
//! is the right move here". Two steps:
//!
//!   1. the logical move, automatic, from what is known exactly: the opening
//!      table (a known line), the known positions (the exact position with
//!      Stockfish's move in the data), the Syzygy tables (five pieces or
//!      fewer);
//!   2. else the theory move: the candidates come from a search on the
//!      evaluation (the theory engine's best moves, each with its value), and
//!      the choice among them is made by what Golden Answer knows of chess:
//!      the ten grandmasters, every one the known theory
//!      (data/golden_theory.toml) in its own style, voting.
//!
//! Nothing is cut away: every candidate goes to the judges, and the answer
//! names the candidates, their values, each grandmaster's choice and why.

use crate::fast::position::Position;
use crate::golden_grandmasters::{theory_eval, Known};

/// Where the answer came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    OpeningTable,
    KnownPosition,
    Tablebase,
    TheoryMove,
}

pub struct GoldenMove {
    pub fen: String,
    pub mv: String,
    pub source: Source,
    /// the candidates with their search values (cp, from the mover), the theory move's
    pub candidates: Vec<(String, i32)>,
    /// the grandmasters' verdict: votes and each one's choice with its principle
    pub verdict: String,
}

impl GoldenMove {
    pub fn explain(&self) -> String {
        match self.source {
            Source::OpeningTable => format!("{}: the logical move, the opening table (a known line)", self.mv),
            Source::KnownPosition => format!("{}: the logical move, a known position (Stockfish's move in the data)", self.mv),
            Source::Tablebase => format!("{}: the logical move, the Syzygy tables", self.mv),
            Source::TheoryMove => format!(
                "{}: the theory move; candidates {}; {}",
                self.mv,
                self.candidates.iter().map(|(c, v)| format!("{c} {v:+}")).collect::<Vec<_>>().join(", "),
                self.verdict
            ),
        }
    }
}

/// The right move in `fen`, in `ms` milliseconds when it has to be searched for.
pub fn golden_move(known: &Known, fen: &str, ms: u64) -> Result<GoldenMove, String> {
    let p = Position::from_fen(fen)?;
    let logical = |mv: String, source: Source| GoldenMove { fen: fen.into(), mv, source, candidates: Vec::new(), verdict: String::new() };
    if let Some(m) = crate::golden_grow::table_move_pub(fen) {
        return Ok(logical(m, Source::OpeningTable));
    }
    if let Some(m) = known.get(&p) {
        return Ok(logical(m, Source::KnownPosition));
    }
    if p.occ.count_ones() <= 5 {
        if let Some((m, _, _)) = crate::golden_chess::play(fen) {
            return Ok(logical(m, Source::Tablebase));
        }
    }
    // the theory move: the evaluation's search names the candidates and values them, the grandmasters choose
    let ev = theory_eval()?;
    let k = 3;
    let cands = crate::star_search::best_moves(&ev, None, fen, ms * 6 / 10, k);
    if cands.is_empty() {
        return Err("no legal move".into());
    }
    let each = (ms * 4 / 10) / cands.len() as u64;
    let mut q = p.clone();
    let mut vals = Vec::new();
    for c in &cands {
        let m = crate::fast::movegen::uci_to_move(&mut q, c).ok_or("illegal")?;
        q.make(m);
        let v = if crate::fast::movegen::legal_moves(&mut q).is_empty() {
            if q.in_check() { 1_000_000 } else { 0 }
        } else {
            -crate::star_search::analyse(&ev, None, &q.to_fen(), each).map(|r| r.1).unwrap_or(0)
        };
        q.unmake();
        vals.push((c.clone(), v));
    }
    let (mv, verdict) = crate::golden_grandmasters::judge_candidates(fen, &vals, &[], &[])?;
    Ok(GoldenMove { fen: fen.into(), mv, source: Source::TheoryMove, candidates: vals, verdict })
}
