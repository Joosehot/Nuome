//! Golden Answer's chess: "best move in <FEN>", answered theoretically, raw:
//! no search tree (no minimax, no alpha-beta).
//!
//! - up to five pieces: the Syzygy tablebases (Ronald de Man), probed through
//!   python-chess: a move that keeps the position's win, draw or loss;
//! - otherwise: every legal move played once; after it the forcing lines
//!   (captures, promotions, the mover's checks) are followed to their end,
//!   each side free to stop, so material won or lost by force shows; the end
//!   is judged as it stands — checkmate and stalemate by the rules, the rest
//!   by Golden heart (measured from 124 707 Stockfish games) plus the tempo
//!   (3 cp a legal move more than the other side, 10 cp for the move).
//!   No search over quiet moves.
//!
//! The logical and abstract kinds do not answer chess.

use crate::golden::Board;

pub const SYZYGY: &str = "C:/Users/joose/Tools/syzygy/345-wdl";

/// Read "best move in <FEN>" (and its other phrasings) whole; the FEN, if the question is that.
pub fn read(q: &str) -> Option<String> {
    let t = q.trim().trim_end_matches(['?', '.']).trim();
    let low = t.to_lowercase();
    const ASK: &[&str] = &[
        "what is the best move in ",
        "what is the best move for ",
        "find the best move in ",
        "best move in ",
        "best move for ",
        "solve the position ",
        "solve ",
        "mikä on paras siirto asemassa ",
        "paras siirto asemassa ",
        "paras siirto ",
        "ratkaise asema ",
    ];
    let p = ASK.iter().find(|p| low.starts_with(*p))?;
    let fen = t[p.len()..].trim().trim_matches(['"', '\'']).to_string();
    Board::from_fen(&fen).ok().map(|_| fen)
}

pub enum Kind3 {
    Logical,
    Theoretical,
    Abstract,
}

const MATE: i64 = 100_000;

/// Golden Answer answers chess only theoretically.
const ONLY: &str = "Golden Answer answers chess only theoretically (from known results: the Syzygy tables, and Golden heart measured from Stockfish's games); ask theoretical";

/// A quiet position's worth for the side to move: Golden heart (or material
/// without it) plus the tempo — 3 cp a legal move more than the other side
/// has, and 10 cp for having the move.
fn stand(b: &Board, my_moves: usize) -> i64 {
    let heart = match crate::golden_heart::frozen() {
        Some(w) => crate::golden_heart::eval(w, b),
        None => crate::supergenius_eval::material(b),
    };
    heart + crate::supergenius_eval::tempo(b, my_moves)
}

/// How long the forcing lines may run, and how many positions they may take a move.
const FORCED_PLIES: u32 = 8;
const FORCED_NODES: u64 = 40_000;

/// The forcing lines followed to their end: the attacker (the side that made
/// the move asked about) plays its captures, promotions and checks, the
/// defender its captures and promotions (every reply when in check), each
/// side free to stop where going on would cost it; at the end, the position
/// as it stands with the tempo. Material won or lost by force shows here.
/// The worth for the side to move.
fn forced(b: &Board, plies: u32, attacker: bool, nodes: &mut u64) -> i64 {
    *nodes += 1;
    let ms = b.moves();
    let check = b.in_check();
    if ms.is_empty() {
        return if check { -MATE } else { 0 };
    }
    let here = stand(b, ms.len());
    if plies == 0 || *nodes > FORCED_NODES {
        return here;
    }
    let mut best = if check { -MATE } else { here };
    for m in ms {
        let capture = b.sq[m.to as usize] != 0 || (b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep);
        let a = b.play(m);
        let gives_check = attacker && a.in_check();
        if !(check || capture || m.promo != 0 || gives_check) {
            continue;
        }
        let v = -forced(&a, plies - 1, !attacker, nodes);
        if v > best {
            best = v;
        }
    }
    best
}

/// The worth of the position a move leaves, for the side that made it: the forcing lines from there to their end.
fn judged(after: &Board) -> i64 {
    let mut nodes = 0;
    -forced(after, FORCED_PLIES - 1, false, &mut nodes)
}

/// Every legal move with the judgement of the position it leaves, best first (ties in the rules' order).
fn raw_rows(b: &Board) -> Vec<(String, i64)> {
    let mut rows: Vec<(String, i64)> = b.moves().into_iter().map(|m| (m.uci(), judged(&b.play(m)))).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    rows
}

fn heart_source() -> String {
    if crate::golden_heart::frozen().is_some() {
        "Golden heart: the static eval measured from 124 707 Stockfish games (CCRL 40/15, computerchess.org.uk), out/golden/heart_weights.txt".into()
    } else {
        "Kaufman's piece values (1999): 100/325/325/500/975".into()
    }
}

/// The answer: short (the move in UCI, or the verdict), lines, sources; Err with a reason.
pub fn answer(kind: Kind3, fen: &str) -> Result<(String, Vec<String>, Vec<String>), String> {
    let b = Board::from_fen(fen).map_err(|e| format!("not a position: {e}"))?;
    if !matches!(kind, Kind3::Theoretical) {
        return Err(ONLY.into());
    }
    let side = if b.white { "white" } else { "black" };
    if b.moves().is_empty() {
        let v = if b.in_check() { format!("checkmate: {side} has no legal move and is in check") } else { format!("stalemate: {side} has no legal move and is not in check") };
        return Ok(("no move".into(), vec![v], vec!["the rules of chess (FIDE Laws of Chess, articles 5.1 and 5.2)".into()]));
    }
    if b.sq.iter().filter(|&&p| p != 0).count() <= 5 {
        let rows = syzygy_rows(fen)?;
        let best = rows.iter().map(|r| r.1).max().ok_or("no moves probed")?;
        let keep: Vec<&String> = rows.iter().filter(|r| r.1 == best).map(|r| &r.0).collect();
        // among the moves that keep the result, the one that leaves the best position as it stands
        let raw = raw_rows(&b);
        let pick = raw.iter().map(|r| &r.0).find(|m| keep.contains(m)).cloned().unwrap_or_else(|| keep[0].clone());
        let what = match best {
            2 => "win",
            1 => "win (cursed: drawn by the 50-move rule with best defence)",
            0 => "draw",
            -1 => "loss (blessed: saved by the 50-move rule)",
            _ => "loss",
        };
        let lines = vec![
            format!("the position is a {what} for {side} with best play"),
            format!("{} moves keep it: {}", keep.len(), keep.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")),
            format!("among them {pick} is the best by the forcing lines, Golden heart and the tempo"),
        ];
        return Ok((pick, lines, vec![format!("Syzygy endgame tablebases, 3-5 pieces (Ronald de Man), probed with python-chess from {SYZYGY}"), heart_source()]));
    }
    let rows = raw_rows(&b);
    let (m, v) = rows[0].clone();
    let lines = vec![
        format!("{m} is the best move: {side} {v:+} (every legal move played once; after it the forcing lines — captures, promotions, checks — followed to their end; the end judged by Golden heart and the tempo)"),
        rows.get(1).map_or(String::new(), |(m2, v2)| format!("the next is {m2} at {v2:+}")),
    ];
    Ok((m, lines.into_iter().filter(|l| !l.is_empty()).collect(), vec![heart_source()]))
}

/// Every legal move with its Syzygy WDL for the mover (2 win, 0 draw, -2 loss; ±1 cursed/blessed).
fn syzygy_rows(fen: &str) -> Result<Vec<(String, i32)>, String> {
    let script = format!(
        "import chess, chess.syzygy\nb = chess.Board('{fen}')\nwith chess.syzygy.open_tablebase('{SYZYGY}') as tb:\n    for m in b.legal_moves:\n        b.push(m)\n        v = -2 if b.is_checkmate() else (0 if b.is_stalemate() or len(b.piece_map()) == 2 else tb.probe_wdl(b))\n        print(m.uci(), -v)\n        b.pop()\n"
    );
    let out = std::process::Command::new("python").arg("-c").arg(&script).output().map_err(|e| format!("python-chess could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!("the Syzygy probe failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_once(' ').and_then(|(m, v)| v.trim().parse().ok().map(|v| (m.to_string(), v)))).collect())
}

/// Golden Boy plays Golden Answer's raw theoretical answer. (move, kind, lines)
pub fn play(fen: &str) -> Option<(String, &'static str, Vec<String>)> {
    let (m, lines, _) = answer(Kind3::Theoretical, fen).ok()?;
    (m != "no move").then_some((m, "theoretical", lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_question() {
        assert!(read("best move in 6k1/8/6K1/8/8/8/8/R7 w - - 0 1").is_some());
        assert!(read("paras siirto asemassa 6k1/8/6K1/8/8/8/8/R7 w - - 0 1").is_some());
        assert!(read("best move in the opening").is_none());
    }

    #[test]
    fn only_theoretical_and_raw() {
        // a back-rank mate in 1: the rules see the mate in the position the move leaves
        let (m, _, src) = answer(Kind3::Theoretical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").unwrap();
        assert_eq!(m, "d1d8");
        assert!(!src.is_empty());
        // checkmate: no move, by the rules
        assert_eq!(answer(Kind3::Theoretical, "R5k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1").unwrap().0, "no move");
        // the other kinds: no answer
        assert!(answer(Kind3::Logical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").is_err());
        assert!(answer(Kind3::Abstract, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").is_err());
    }
}
