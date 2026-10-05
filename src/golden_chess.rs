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
            // 6-32 pieces: a proof with its scope said: the best move of a bounded game, every move searched
            let depth = std::env::var("GOLDEN_PROOF_PLIES").ok().and_then(|x| x.parse().ok()).unwrap_or(4);
            let (m, v, n, second) = bounded(&b, depth);
            let who = if b.white { "white" } else { "black" };
            let mut lines = vec![
                format!("no forced mate and no solved class: what is proved is the bounded game (scope: {depth} plies of every legal move by both sides, then captures traded until quiet, each side free to stop; scored by material, Kaufman 100/325/325/500/975, mate ±100000)"),
                format!("in that game {m} gives {who} {v:+} centipawns, and no other move gives more ({n} positions, every reply searched)"),
            ];
            if let Some((m2, v2)) = second {
                lines.push(format!("the next best is {m2} at {v2:+}"));
            }
            lines.push("this proves the move best in the bounded game, not best in chess".into());
            Ok((m, lines, vec![]))
        }
        Kind3::Theoretical => {
            let pieces = b.sq.iter().filter(|&&p| p != 0).count();
            if pieces > 5 {
                return Err(format!("no known result covers it: the Syzygy tables here reach 5 pieces, the position has {pieces}"));
            }
            let rows = syzygy_rows(fen)?;
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

const MATE: i64 = 100_000;

/// The bounded game's value for the side to move: every move for `depth`
/// plies, then captures (and promotions) until quiet with standing pat;
/// material at the leaves. Alpha-beta cuts only lines that cannot change
/// the value, so the value is the full minimax value of this game.
fn game(b: &Board, depth: u32, mut alpha: i64, beta: i64, nodes: &mut u64) -> i64 {
    *nodes += 1;
    let ms = b.moves();
    if ms.is_empty() {
        return if b.in_check() { -MATE } else { 0 };
    }
    let quiet = depth == 0;
    if quiet {
        let stand = crate::supergenius_eval::material(b);
        if stand >= beta {
            return stand;
        }
        alpha = alpha.max(stand);
    }
    // captures first: the order changes only the speed, never the value
    let mut order: Vec<(i64, crate::golden::Mv)> = ms
        .into_iter()
        .map(|m| {
            let victim = b.sq[m.to as usize].unsigned_abs() as i64;
            let ep = b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep;
            ((victim + ep as i64) * 10 + (m.promo != 0) as i64 * 50, m)
        })
        .collect();
    if quiet {
        order.retain(|(score, _)| *score > 0);
    }
    order.sort_by_key(|o| std::cmp::Reverse(o.0));
    let mut best = if quiet { alpha } else { -MATE - 1 };
    for (_, m) in order {
        let v = -game(&b.play(m), depth.saturating_sub(1), -beta, -alpha.max(best), nodes);
        if v > best {
            best = v;
        }
        if best >= beta {
            break;
        }
    }
    best
}

/// Every root move's value in the bounded game (full window each, best first), and the positions searched.
fn bounded_rows(b: &Board, depth: u32) -> (Vec<(String, i64)>, u64) {
    let mut nodes = 0u64;
    let mut rows: Vec<(String, i64)> = b
        .moves()
        .into_iter()
        .map(|m| (m.uci(), -game(&b.play(m), depth.saturating_sub(1), -MATE - 1, MATE + 1, &mut nodes)))
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    (rows, nodes)
}

/// The best move of the bounded game: (move, value, positions, next best).
fn bounded(b: &Board, depth: u32) -> (String, i64, u64, Option<(String, i64)>) {
    let (rows, nodes) = bounded_rows(b, depth);
    let second = rows.get(1).cloned();
    (rows[0].0.clone(), rows[0].1, nodes, second)
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

/// Golden Boy plays with Golden Answer's kinds, each move saying what it
/// rests on: a strong proof (mate, solved class) first; in positions of up
/// to five pieces, among the moves that keep the Syzygy result
/// (theoretical), the one its patterns rank first; otherwise the bounded
/// game's best (logical, proved in its scope), the patterns choosing among
/// equals. (move, kind, lines)
pub fn play(fen: &str) -> Option<(String, &'static str, Vec<String>)> {
    let b = Board::from_fen(fen).ok()?;
    if b.moves().is_empty() {
        return None;
    }
    // 1. a strong proof: a forced mate, a solved class, a forced draw or loss
    if let Ok((m, lines, _)) = answer(Kind3::Logical, fen) {
        if m != "no move" && !lines[0].starts_with("no forced mate and no solved class") {
            return Some((m, "logical", lines));
        }
    }
    let rosters = crate::patterns::load();
    let ranked: Vec<String> = crate::patterns::play(&b, &[], &rosters).map(|(_, _, rows)| rows.iter().map(|r| r.0.uci()).collect()).unwrap_or_default();
    if b.sq.iter().filter(|&&p| p != 0).count() <= 5 {
        if let Ok(rows) = syzygy_rows(fen) {
            if let Some(best) = rows.iter().map(|r| r.1).max() {
                let keep: Vec<&String> = rows.iter().filter(|r| r.1 == best).map(|r| &r.0).collect();
                let pick = ranked.iter().find(|m| keep.contains(m)).cloned().unwrap_or_else(|| keep[0].clone());
                let what = match best {
                    2 | 1 => "win",
                    0 => "draw",
                    _ => "loss",
                };
                let lines = vec![format!("Syzygy: the position is a {what}; {} moves keep it, and Golden Boy's patterns rank {pick} first among them", keep.len())];
                return Some((pick, "theoretical", lines));
            }
        }
    }
    // 3. the bounded game, proved; among its equally best moves the patterns choose
    let depth = std::env::var("GOLDEN_PROOF_PLIES").ok().and_then(|x| x.parse().ok()).unwrap_or(4);
    let (rows, nodes) = bounded_rows(&b, depth);
    let top = rows.first()?.1;
    let equal: Vec<&String> = rows.iter().filter(|r| r.1 == top).map(|r| &r.0).collect();
    let pick = ranked.iter().find(|m| equal.contains(m)).cloned().unwrap_or_else(|| equal[0].clone());
    let lines = vec![format!(
        "the bounded game ({depth} plies of every move, captures to quiet, material) gives {top:+} at best ({nodes} positions); {} moves reach it, and Golden Boy's patterns choose {pick} among them",
        equal.len()
    )];
    Some((pick, "logical (bounded) + patterns among equals", lines))
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
