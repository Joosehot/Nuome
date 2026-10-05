//! Golden heart, cloned like a voice: Stockfish's games are the recording,
//! a static network is the voice profile. No Stockfish evaluation is used —
//! only which move it played.
//!
//!   games -> positions and Stockfish's moves
//!         -> measure: the static network's weights, one per (mine/theirs,
//!            piece, square) seen from the side that moved (768 inputs);
//!            weight = how much more often Stockfish's move leaves that
//!            feature than the average legal move does
//!         -> copy: replay every position, pick the move whose result the
//!            network scores highest; the distance to Stockfish's moves
//!            shows what the voice still misses
//!         -> refine where the copy differs (measure again on those
//!            positions), and keep a round only when the copy gets closer
//!            on games the measuring never saw
//!         -> freeze: out/golden/voice_eval.txt, a few microseconds a call
//!
//! The games are split once: the first 15 are the voice, the last 5 the check.

use crate::golden::{Board, Mv};

pub const N: usize = 768;
pub const VOICE_FILE: &str = "out/golden/voice_eval.txt";

/// The inputs of a position seen from `me`: (mine 0 / theirs 1, piece 1..6, square seen from me).
pub fn inputs(b: &Board, me_white: bool) -> Vec<u16> {
    let mut out = Vec::with_capacity(32);
    for n in 0..64 {
        let p = b.sq[n];
        if p == 0 {
            continue;
        }
        let theirs = (p > 0) != me_white;
        let sq = if me_white { n } else { (7 - n / 8) * 8 + n % 8 };
        out.push(((theirs as usize * 6 + (p.unsigned_abs() as usize - 1)) * 64 + sq) as u16);
    }
    out
}

/// One position of the voice: the inputs after every legal move (seen from the mover), and Stockfish's move.
pub struct Heard {
    pub after: Vec<Vec<u16>>,
    /// the positions after each legal move (the copy plays on from them)
    pub boards: Vec<Board>,
    pub chosen: usize,
    pub phase: &'static str,
}

fn phase(b: &Board) -> &'static str {
    let pieces = b.sq.iter().filter(|&&p| p != 0 && p.abs() != 1 && p.abs() != 6).count();
    match pieces {
        0 => "pawn endings",
        1..=4 => "endgames",
        _ if b.sq.iter().filter(|&&p| p != 0).count() >= 28 => "openings",
        _ => "middlegames",
    }
}

/// Every position of the games in `path` (result TAB uci moves), game by game.
pub fn listen(path: &str) -> Result<Vec<Vec<Heard>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut games = Vec::new();
    for line in text.lines() {
        let Some((_, moves)) = line.split_once('\t') else { continue };
        let mut b = Board::start();
        let mut heard = Vec::new();
        for u in moves.split_whitespace() {
            let ms: Vec<Mv> = b.moves();
            let Some(ci) = ms.iter().position(|m| m.uci() == u) else { break };
            if ms.len() > 1 {
                let me = b.white;
                let boards: Vec<Board> = ms.iter().map(|&m| b.play(m)).collect();
                heard.push(Heard { after: boards.iter().map(|a| inputs(a, me)).collect(), boards, chosen: ci, phase: phase(&b) });
            }
            b = b.play(ms[ci]);
        }
        games.push(heard);
    }
    Ok(games)
}

fn score(w: &[f64], x: &[u16]) -> f64 {
    x.iter().map(|&i| w[i as usize]).sum()
}

/// The move the network picks (ties: the first in the rules' order).
fn pick(w: &[f64], h: &Heard) -> usize {
    let mut best = (f64::NEG_INFINITY, 0);
    for (i, x) in h.after.iter().enumerate() {
        let s = score(w, x);
        if s > best.0 {
            best = (s, i);
        }
    }
    best.1
}

/// The value for the side to move once the captures are traded to quiet (each side may stop), the network at the quiet leaves.
fn quiet(w: &[f64], b: &Board, depth: u32, mut alpha: f64, beta: f64) -> f64 {
    let stand = score(w, &inputs(b, b.white));
    if depth == 0 || stand >= beta {
        return stand;
    }
    alpha = alpha.max(stand);
    let mut best = stand;
    for m in b.moves() {
        if b.sq[m.to as usize] == 0 && m.promo == 0 {
            continue;
        }
        let v = -quiet(w, &b.play(m), depth - 1, -beta, -alpha);
        if v > best {
            best = v;
            alpha = alpha.max(v);
            if best >= beta {
                break;
            }
        }
    }
    best
}

/// The copy that listens after the captures: each move's result traded to quiet, then the network.
fn pick_quiet(w: &[f64], h: &Heard) -> usize {
    let mut best = (f64::NEG_INFINITY, 0);
    for (i, a) in h.boards.iter().enumerate() {
        let v = -quiet(w, a, 6, f64::NEG_INFINITY, f64::INFINITY);
        if v > best.0 {
            best = (v, i);
        }
    }
    best.1
}

pub fn agreement_quiet(w: &[f64], set: &[&Heard]) -> f64 {
    set.iter().filter(|h| pick_quiet(w, h) == h.chosen).count() as f64 / set.len().max(1) as f64
}

/// The share of positions where the copy plays Stockfish's move.
pub fn agreement(w: &[f64], set: &[&Heard]) -> f64 {
    set.iter().filter(|h| pick(w, h) == h.chosen).count() as f64 / set.len().max(1) as f64
}

/// Measure: each weight = (the input after Stockfish's move) minus (its mean over all legal moves), averaged.
pub fn measure(set: &[&Heard]) -> Vec<f64> {
    let mut w = vec![0.0; N];
    for h in set {
        for &i in &h.after[h.chosen] {
            w[i as usize] += 1.0;
        }
        let k = h.after.len() as f64;
        for x in &h.after {
            for &i in x {
                w[i as usize] -= 1.0 / k;
            }
        }
    }
    let n = set.len().max(1) as f64;
    w.iter_mut().for_each(|x| *x /= n);
    w
}

pub struct Cloned {
    pub w: Vec<f64>,
    pub rounds: Vec<(usize, f64, f64)>,
    pub kept: usize,
    pub voice_n: usize,
    pub check_n: usize,
    pub by_phase: Vec<(&'static str, usize, f64)>,
}

/// Clone: measure on the voice games, then refine where the copy differs,
/// a round kept only when the copy gets closer on the check games.
pub fn clone_voice(games: &[Vec<Heard>], voice_games: usize, rounds: usize) -> Cloned {
    let voice: Vec<&Heard> = games.iter().take(voice_games).flatten().collect();
    let check: Vec<&Heard> = games.iter().skip(voice_games).flatten().collect();
    let mut w = measure(&voice);
    let mut best = (agreement(&w, &check), w.clone());
    let mut log = vec![(0, agreement(&w, &voice), best.0)];
    let mut kept = 0;
    for r in 1..=rounds {
        // where the copy differs: measure again, Stockfish's move against the copy's
        let mut d = vec![0.0; N];
        let mut wrong = 0;
        for h in &voice {
            let p = pick(&w, h);
            if p != h.chosen {
                wrong += 1;
                for &i in &h.after[h.chosen] {
                    d[i as usize] += 1.0;
                }
                for &i in &h.after[p] {
                    d[i as usize] -= 1.0;
                }
            }
        }
        if wrong == 0 {
            break;
        }
        let step = 1.0 / (voice.len() as f64 * (1.0 + r as f64 / 10.0));
        for i in 0..N {
            w[i] += step * d[i];
        }
        let (a_voice, a_check) = (agreement(&w, &voice), agreement(&w, &check));
        log.push((r, a_voice, a_check));
        if a_check > best.0 {
            best = (a_check, w.clone());
            kept = r;
        }
    }
    let w = best.1;
    let mut by_phase = Vec::new();
    for ph in ["openings", "middlegames", "endgames", "pawn endings"] {
        let set: Vec<&Heard> = check.iter().copied().filter(|h| h.phase == ph).collect();
        if !set.is_empty() {
            by_phase.push((ph, set.len(), agreement(&w, &set)));
        }
    }
    Cloned { w, rounds: log, kept, voice_n: voice.len(), check_n: check.len(), by_phase }
}

pub fn save(w: &[f64]) -> Result<(), String> {
    let text: String = w.iter().map(|x| format!("{x:.6}\n")).collect();
    std::fs::write(VOICE_FILE, text).map_err(|e| e.to_string())
}

pub fn load() -> Option<Vec<f64>> {
    let w: Vec<f64> = std::fs::read_to_string(VOICE_FILE).ok()?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    (w.len() == N).then_some(w)
}

/// The frozen voice as an eval: the position's value for the side to move, in centipawn-like units.
pub fn eval(w: &[f64], b: &Board) -> i64 {
    (score(w, &inputs(b, b.white)) * 1000.0).round() as i64
}

pub fn report(path: &str, rounds: usize) -> Result<String, String> {
    let games = listen(path)?;
    let voice_games = games.len() * 3 / 4;
    let c = clone_voice(&games, voice_games, rounds);
    save(&c.w)?;
    let mut out = format!("Golden heart cloned from Stockfish's games: {} games as the voice ({} positions), {} unseen as the check ({} positions); only Stockfish's moves are used\n", voice_games, c.voice_n, games.len() - voice_games, c.check_n);
    for (r, v, ch) in c.rounds.iter().filter(|x| x.0 == 0 || x.0 % 5 == 0 || x.0 == c.kept) {
        out.push_str(&format!("  {}: the copy plays Stockfish's move in {:.1}% of the voice positions, {:.1}% of the unseen ones\n", if *r == 0 { "measured".to_string() } else { format!("refined {r}") }, 100.0 * v, 100.0 * ch));
    }
    out.push_str(&format!("kept: round {} (the closest copy on the unseen games)\nby phase, unseen:\n", c.kept));
    for (ph, n, a) in &c.by_phase {
        out.push_str(&format!("  {ph}: {:.1}% of {n}\n", 100.0 * a));
    }
    let check: Vec<&Heard> = games.iter().skip(voice_games).flatten().collect();
    out.push_str(&format!("the copy listening after the captures (traded to quiet, the network at the quiet leaves): {:.1}% of the unseen positions\n", 100.0 * agreement_quiet(&c.w, &check)));
    out.push_str(&format!("frozen in {VOICE_FILE} ({N} weights)\n"));
    Ok(out)
}
