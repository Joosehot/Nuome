//! Golden Move: Golden Answer's one answer kind for a chess position, "what
//! is the right move here". The answer is derived from what is known, in
//! this order, and says which it was:
//!
//!   1. the opening table: the position is a known line;
//!   2. a known position: the exact position has Stockfish's move in the data;
//!   3. the theory engine: the known theory (data/golden_theory.toml), weighted
//!      by Texel, as the evaluation inside StarNet's search; the answer names
//!      the line it saw and the principles that weigh most in the position
//!      it reaches.
//!
//! Nothing is cut away and nothing is guessed: a move the engine plays comes
//! with its depth, its value and its line, so it can be checked.

use crate::fast::position::Position;
use crate::golden_grandmasters::{theory_eval, Known};
use crate::star_search::Eval as _;

/// Where the answer came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    OpeningTable,
    KnownPosition,
    TheoryEngine,
}

pub struct GoldenMove {
    pub fen: String,
    pub mv: String,
    pub source: Source,
    /// the engine's depth, nodes and value (cp, from the mover) when it searched
    pub search: Option<(usize, u64, i32)>,
    /// the line the engine saw, the move first
    pub line: Vec<String>,
    /// the principles weighing most in the position the line reaches, (id, cp from the mover)
    pub principles: Vec<(String, i32)>,
}

impl GoldenMove {
    pub fn explain(&self) -> String {
        match self.source {
            Source::OpeningTable => format!("{}: the opening table, a known line", self.mv),
            Source::KnownPosition => format!("{}: a known position, Stockfish's move from the data", self.mv),
            Source::TheoryEngine => {
                let (d, n, v) = self.search.unwrap_or((0, 0, 0));
                format!(
                    "{}: the theory engine, depth {d}, {:.1}M nodes, {v:+} cp; line {}; weighing most: {}",
                    self.mv,
                    n as f64 / 1e6,
                    self.line.join(" "),
                    self.principles.iter().map(|(id, cp)| format!("{id} {cp:+}")).collect::<Vec<_>>().join(", ")
                )
            }
        }
    }
}

/// The principles weighing most in `p` from side `me`: (id, cp), largest first, at most `n`.
fn principles(p: &Position, me: usize, n: usize) -> Result<Vec<(String, i32)>, String> {
    let ev = theory_eval()?;
    let ph = crate::golden_theory::phase(p);
    let mut parts: Vec<(String, i32)> = ev
        .weights
        .iter()
        .map(|&(i, w)| {
            let (id, st) = crate::golden_theory::STATES[i];
            let we = ev.endgame.iter().find(|x| x.0 == i).map(|x| x.1).unwrap_or(w);
            let v = (ph * w + (1.0 - ph) * we) * (st(p, me) - st(p, 1 - me)) * ev.scale;
            (id.to_string(), v.round() as i32)
        })
        .filter(|x| x.1 != 0)
        .collect();
    parts.sort_by(|a, b| b.1.abs().cmp(&a.1.abs()));
    parts.truncate(n);
    Ok(parts)
}

/// The right move in `fen`, in `ms` milliseconds when the engine has to search.
pub fn golden_move(known: &Known, fen: &str, ms: u64) -> Result<GoldenMove, String> {
    let p = Position::from_fen(fen)?;
    if let Some(m) = crate::golden_grow::table_move_pub(fen) {
        return Ok(GoldenMove { fen: fen.into(), mv: m, source: Source::OpeningTable, search: None, line: Vec::new(), principles: Vec::new() });
    }
    if let Some(m) = known.get(&p) {
        return Ok(GoldenMove { fen: fen.into(), mv: m, source: Source::KnownPosition, search: None, line: Vec::new(), principles: Vec::new() });
    }
    let ev = theory_eval()?;
    let (mv, depth, nodes, value) = crate::star_search::search_root_pub(&ev, fen, ms).ok_or("no legal move")?;
    // the line: the move, then the engine's best replies (a short search each)
    let me = p.turn as usize;
    let mut q = p.clone();
    let mut line = vec![mv.clone()];
    let mut cur = crate::fast::movegen::uci_to_move(&mut q, &mv).ok_or("illegal")?;
    q.make(cur);
    for _ in 0..5 {
        if crate::fast::movegen::legal_moves(&mut q).is_empty() {
            break;
        }
        let Some((r, _)) = crate::star_search::analyse(&ev, None, &q.to_fen(), (ms / 10).max(50)) else { break };
        let Some(rm) = crate::fast::movegen::uci_to_move(&mut q, &r) else { break };
        line.push(r);
        cur = rm;
        q.make(cur);
    }
    let _ = ev.eval(&q);
    let principles = principles(&q, me, 5)?;
    Ok(GoldenMove { fen: fen.into(), mv, source: Source::TheoryEngine, search: Some((depth, nodes, value)), line, principles })
}
