//! Golden Answer's chess: "best move in <FEN>" in the kind the asker chooses.
//!
//! - logical: only what Nuome derives from the rules itself — checkmate and
//!   stalemate; the retrograde tables Nuome computes for three pieces
//!   (retro.rs); a forced mate found by exact search over every reply
//!   (neuro::supergenius). A move is called best only when that is proved.
//! - theoretical: the Syzygy tablebases (Ronald de Man), results computed by
//!   others and named: for up to five pieces, a move that keeps the
//!   position's win, draw or loss, probed through python-chess.
//! - abstract: Golden Boy's move by the patterns it discovered in
//!   Stockfish's choices; it claims nothing.

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

/// The answer: short (the move in UCI, or the verdict), lines, sources; None with a reason.
pub fn answer(kind: Kind3, fen: &str) -> Result<(String, Vec<String>, Vec<String>), String> {
    let b = Board::from_fen(fen).map_err(|e| format!("not a position: {e}"))?;
    let ms = b.moves();
    let side = if b.white { "white" } else { "black" };
    if ms.is_empty() {
        let v = if b.in_check() { format!("checkmate: {side} has no legal move and is in check") } else { format!("stalemate: {side} has no legal move and is not in check") };
        return match kind {
            Kind3::Logical => Ok(("no move".into(), vec![v], vec![])),
            _ => Err("the rules decide it: ask logical".into()),
        };
    }
    match kind {
        Kind3::Logical => {
            // the three-piece tables Nuome computes from the rules
            if let Some((m, v, rows)) = crate::retro::exact_move(&b) {
                let key = crate::retro::key_of(&crate::retro::pieces_of(&b));
                let mut lines = vec![format!("{key} is solved whole by Nuome's retrograde analysis from the rules (every position, every reply)")];
                lines.push(format!("{}: {}", m.uci(), crate::retro::show(v)));
                for (m2, v2) in rows.iter().skip(1).take(4) {
                    lines.push(format!("{}: {}", m2.uci(), crate::retro::show(*v2)));
                }
                return Ok((m.uci(), lines, vec![]));
            }
            // a forced mate, every reply answered
            let (_, _, rows, nodes) = crate::neuro::supergenius(&b, &[], 400_000).ok_or("no legal move")?;
            use crate::neuro::Proof;
            let wins: Vec<(String, u32)> = rows.iter().filter_map(|(m, p, _)| if let Proof::Win(n) = p { Some((m.uci(), *n)) } else { None }).collect();
            if let Some((m, n)) = wins.iter().min_by_key(|w| w.1) {
                let lines = vec![
                    format!("{m} mates by force in {} (an exact search answered every reply, {nodes} positions)", (n + 1) / 2),
                    if wins.len() > 1 { format!("{} moves mate by force; {m} is the fastest found", wins.len()) } else { "it is the only move found that mates by force".into() },
                ];
                return Ok((m.clone(), lines, vec![]));
            }
            let losses = rows.iter().filter(|r| matches!(r.1, Proof::Loss(_))).count();
            let draws: Vec<String> = rows.iter().filter(|r| matches!(r.1, Proof::Draw)).map(|r| r.0.uci()).collect();
            if !draws.is_empty() && draws.len() + losses == rows.len() {
                return Ok((draws[0].clone(), vec![format!("{} holds a draw by force, and every other move is a forced draw or loss ({nodes} positions)", draws[0])], vec![]));
            }
            if losses == rows.len() {
                let slow = rows.iter().max_by_key(|r| if let Proof::Loss(n) = r.1 { n } else { 0 }).map(|r| r.0.uci()).unwrap_or_default();
                return Ok((slow.clone(), vec![format!("every move loses by force; {slow} lasts longest ({nodes} positions)")], vec![]));
            }
            Err(format!("no move is proved best: no forced mate within the search ({nodes} positions), and the position has {} pieces (Nuome's own tables cover 3)", b.sq.iter().filter(|&&p| p != 0).count()))
        }
        Kind3::Theoretical => {
            let pieces = b.sq.iter().filter(|&&p| p != 0).count();
            if pieces > 5 {
                return Err(format!("no known result covers it: the Syzygy tables here reach 5 pieces, the position has {pieces}"));
            }
            let script = format!(
                "import chess, chess.syzygy\nb = chess.Board('{fen}')\nwith chess.syzygy.open_tablebase('{SYZYGY}') as tb:\n    for m in b.legal_moves:\n        b.push(m)\n        print(m.uci(), -tb.probe_wdl(b))\n        b.pop()\n"
            );
            let out = std::process::Command::new("python").arg("-c").arg(&script).output().map_err(|e| format!("python-chess could not be run: {e}"))?;
            if !out.status.success() {
                return Err(format!("the Syzygy probe failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
            }
            let rows: Vec<(String, i32)> = String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_once(' ').and_then(|(m, v)| v.trim().parse().ok().map(|v| (m.to_string(), v)))).collect();
            let best = rows.iter().map(|r| r.1).max().ok_or("no moves probed")?;
            let keep: Vec<&String> = rows.iter().filter(|r| r.1 == best).map(|r| &r.0).collect();
            let what = match best {
                2 => "win",
                1 => "win (cursed: drawn by the 50-move rule with best defence)",
                0 => "draw",
                -1 => "loss (blessed: saved by the 50-move rule)",
                _ => "loss",
            };
            let lines = vec![
                format!("the position is a {what} for {side} with best play"),
                format!("moves that keep it: {}", keep.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")),
                "the tables give win, draw or loss only (WDL), not the fastest way".into(),
            ];
            Ok((keep[0].clone(), lines, vec![format!("Syzygy endgame tablebases, 3-5 pieces (Ronald de Man), probed with python-chess from {SYZYGY}")]))
        }
        Kind3::Abstract => {
            let rosters = crate::patterns::load();
            let (m, w, rows) = crate::patterns::play(&b, &[], &rosters).ok_or("no legal move")?;
            let mut lines = vec![format!("Golden Boy's move by the patterns it discovered in Stockfish's choices ({} patterns)", rosters.len())];
            if let Some((_, _, hits)) = rows.first() {
                if !hits.is_empty() {
                    lines.push(format!("because: {}", hits.iter().take(3).map(|p| p.words()).collect::<Vec<_>>().join("; ")));
                }
            }
            if w.is_infinite() {
                lines.push("its calculation found a forced mate first".into());
            }
            Ok((m.uci(), lines, vec![]))
        }
    }
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
    fn logical_mates_and_tables() {
        // KRvK mate in 1: the table
        let (m, _, _) = answer(Kind3::Logical, "6k1/8/6K1/8/8/8/8/R7 w - - 0 1").unwrap();
        assert_eq!(m, "a1a8");
        // a back-rank mate in 1 on a full board: the exact search
        let (m, lines, _) = answer(Kind3::Logical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").unwrap();
        assert_eq!(m, "d1d8");
        assert!(lines[0].contains("mates by force"));
        // checkmate: no move
        assert_eq!(answer(Kind3::Logical, "R5k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1").unwrap().0, "no move");
    }
}
