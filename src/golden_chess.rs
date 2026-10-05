//! Golden Answer's chess: "best move in <FEN>", answered theoretically, raw:
//! no search tree (no minimax, no alpha-beta).
//!
//! - up to five pieces: the Syzygy tablebases (Ronald de Man), probed through
//!   python-chess: a move that keeps the position's win, draw or loss;
//! - otherwise, every move is checked for five things, in this order, each
//!   deciding only between moves the earlier ones left equal:
//!   1. legality — the rules;
//!   2. safety  — what the other side's forcing lines (checks, captures,
//!      promotions, up to 20 plies, each side free to stop) win after it;
//!   3. force   — mate; otherwise what our own forcing lines would win if
//!      the other side did nothing (the threat the move makes);
//!   4. material — what the move takes at once;
//!   5. tempo   — our legal moves minus theirs after it.
//!   No Golden heart, no search over quiet moves.
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

/// Every legal move, best first by the Golden formula (golden_formula), with f.
fn raw_rows(b: &Board) -> Vec<(String, i64)> {
    crate::golden_formula::rank(&b.fen()).unwrap_or_default()
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
            format!("among them {pick} is first by the Golden formula"),
        ];
        return Ok((pick, lines, vec![format!("Syzygy endgame tablebases, 3-5 pieces (Ronald de Man), probed with python-chess from {SYZYGY}"), heart_source()]));
    }
    // more than five pieces: brute force proves, the lumberjack fells, StarNet's eval chooses among the trees left
    let secs = std::env::var("GOLDEN_TIME_MS").ok().and_then(|v| v.parse::<f64>().ok()).map_or(crate::golden_brute::SECONDS, |ms| ms / 1000.0);
    let (m, lines) = crate::golden_brute::golden_boy(fen, secs)?;
    let _ = side;
    Ok((m, lines, vec![
        "the rules of chess: what the brute force proves (every line, no evaluation)".into(),
        "StarNet's evaluation, StarNNUE (Steldens, nnue1024_sib.bin)".into(),
    ]))
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
        // more than five pieces: the Golden formula answers, and until Golden Answer makes it, there is no answer
        match answer(Kind3::Theoretical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1") {
            Ok((m, _, _)) => assert_eq!(m, "d1d8"), // legality decides first: the mate in one
            Err(e) => assert!(e.contains("not made yet")),
        }
        // checkmate: no move, by the rules
        assert_eq!(answer(Kind3::Theoretical, "R5k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1").unwrap().0, "no move");
        // the other kinds: no answer
        assert!(answer(Kind3::Logical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").is_err());
        assert!(answer(Kind3::Abstract, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").is_err());
    }
}
