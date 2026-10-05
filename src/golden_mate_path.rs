//! The mate path: every root move grows its tree; from the second move on
//! the lumberjack fells every other tree at every ply — the side to move,
//! the opponent too, keeps only the move the eval likes best (Steldens'
//! classic evaluation, src/fast/eval, after the captures that do not lose
//! material are traded to quiet) — and the branch goes on until a
//! theoretical mate. Every root move is followed so; the shortest path that
//! ends in our mate is chosen, else the move the eval likes best.

use crate::fast::movegen::legal_moves;
use crate::fast::position::{move_to_uci, Position};

/// How many plies of captures are traded before the classic eval is read.
const QS_PLIES: usize = 4;

/// The captures that do not lose material traded to quiet, each side free
/// to stand; the classic eval at the quiet ends, for the side to move.
fn quiet(p: &mut Position, plies: usize) -> i32 {
    use crate::fast::movegen::{gen_moves, MAX_MOVES};
    let stand = crate::fast::eval::evaluate_f64(p).round() as i32;
    if plies == 0 {
        return stand;
    }
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, true);
    let mut best = stand;
    for &m in &list[..n] {
        if crate::fast::see::see(p, m) < 0 {
            continue;
        }
        p.make(m);
        if !p.in_check_c(me) {
            best = best.max(-quiet(p, plies - 1));
        }
        p.unmake();
    }
    best
}

/// The value of move `m` for the mover: the classic eval after it, captures traded to quiet.
fn value_of(p: &mut Position, m: u32) -> i32 {
    p.make(m);
    let v = -quiet(p, QS_PLIES);
    p.unmake();
    v
}

/// The longest branch followed.
pub const MAX_PLIES: usize = 60;

/// How a branch ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    /// we mate: the moves from the root
    Mate(Vec<String>),
    /// we are mated
    Mated(Vec<String>),
    /// stalemate, repetition, or MAX_PLIES without a mate
    NoMate(Vec<String>),
}

/// The eval's best move in `p` (the one kept; the lumberjack fells the rest):
/// a mate at once first, otherwise the classic eval after the captures; ties
/// by the rules' order.
fn kept(p: &mut Position) -> Option<u32> {
    let ms = legal_moves(p);
    let mut best: Option<(i32, u32)> = None;
    for m in ms {
        p.make(m);
        let mate = p.in_check() && legal_moves(p).is_empty();
        p.unmake();
        if mate {
            return Some(m);
        }
        let v = value_of(p, m);
        if best.map_or(true, |b| v > b.0) {
            best = Some((v, m));
        }
    }
    best.map(|b| b.1)
}

/// The branch of root move `m`: from the second move on, only the kept move at every ply.
pub fn branch(fen: &str, m: u32) -> Option<End> {
    let mut p = Position::from_fen(fen).ok()?;
    let us = p.turn;
    let mut path = vec![move_to_uci(m)];
    p.make(m);
    let mut seen = vec![p.key64()];
    for _ in 1..MAX_PLIES {
        if legal_moves(&mut p).is_empty() {
            return Some(if p.in_check() {
                if p.turn == us { End::Mated(path) } else { End::Mate(path) }
            } else {
                End::NoMate(path)
            });
        }
        let k = kept(&mut p)?;
        path.push(move_to_uci(k));
        p.make(k);
        let key = p.key64();
        if seen.contains(&key) {
            return Some(End::NoMate(path));
        }
        seen.push(key);
    }
    if legal_moves(&mut p).is_empty() && p.in_check() {
        return Some(if p.turn == us { End::Mated(path) } else { End::Mate(path) });
    }
    Some(End::NoMate(path))
}

/// The move when no branch ends in our mate: the classic eval's best
/// (captures traded to quiet); ties by the rules' order. (move, centipawns)
pub fn best_by_eval(fen: &str) -> Option<(String, i32)> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut best: Option<(i32, u32)> = None;
    for m in legal_moves(&mut p) {
        let v = value_of(&mut p, m);
        if best.map_or(true, |b| v > b.0) {
            best = Some((v, m));
        }
    }
    best.map(|(v, m)| (move_to_uci(m), v))
}

/// Every root move's branch (on all cores), and the shortest mate path among them.
pub fn mate_path(fen: &str) -> Result<(Option<Vec<String>>, Vec<End>), String> {
    let mut p = Position::from_fen(fen)?;
    let roots = legal_moves(&mut p);
    let ends: Vec<End> = std::thread::scope(|s| {
        let hs: Vec<_> = roots.iter().map(|&m| s.spawn(move || branch(fen, m))).collect();
        hs.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
    });
    let best = ends.iter().filter_map(|e| if let End::Mate(p) = e { Some(p.clone()) } else { None }).min_by_key(|p| p.len());
    Ok((best, ends))
}

pub fn report(fen: &str) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let (best, ends) = mate_path(fen)?;
    let mates = ends.iter().filter(|e| matches!(e, End::Mate(_))).count();
    let mated = ends.iter().filter(|e| matches!(e, End::Mated(_))).count();
    Ok(format!(
        "{fen}\n  {} root moves in {:.1} s: {mates} branches end in our mate, {mated} in theirs, {} in no mate\n  mate path: {}\n",
        ends.len(),
        t0.elapsed().as_secs_f64(),
        ends.len() - mates - mated,
        match best {
            Some(p) => format!("{} ({} plies)\n  move: {}", p.join(" "), p.len(), p[0]),
            None => match best_by_eval(fen) {
                Some((m, v)) => format!("none found\n  move: {m} (the eval's best: {v:+} cp)"),
                None => "none found".to_string(),
            },
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mate_in_one_is_the_shortest_path() {
        let (best, _) = mate_path("6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").unwrap();
        assert_eq!(best.unwrap(), vec!["d1d8".to_string()]);
    }
}
