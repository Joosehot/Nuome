//! Golden Boy as a HotMotor engine: the raw answer (every legal move played
//! once, the forcing lines after it followed to their end, the end judged by
//! Golden heart and the tempo) is a chain of rules, each offering variants:
//!
//!   forcing  — how many plies the forcing lines may run (4, 6, 8)
//!   checks   — whether the mover's checks count as forcing (no, yes)
//!   tempo    — centipawns a legal move more than the other side (0, 3, 6)
//!   heart    — how strongly Golden heart speaks (x0.5, x1, x1.5)
//!
//! The judges: a move after which the other side mates at once is never
//! played; a chain without Golden heart cannot be right (not offered).
//!
//! The seeds — the variant each rule speaks with — are chosen only after
//! Stockfish's games have been heard: a beam search over the rules, every
//! partial chain scored by how often the copy plays Stockfish's move on
//! moves sampled from the games of the voice; the chain is then checked on
//! games never heard, and frozen in out/golden/boy_rules.toml.
//!
//! Everything runs on the fast board (src/fast).

use crate::fast::movegen::{gen_moves, legal_moves, uci_to_move, MAX_MOVES};
use crate::fast::position::{m_promo, m_to, move_to_uci, Position};

pub const RULES_FILE: &str = "out/golden/boy_rules.toml";
const MATE: i64 = 100_000;
const NODES: u64 = 5_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cfg {
    pub plies: u32,
    pub checks: bool,
    pub tempo: i64,
    pub heart: f64,
}

impl Default for Cfg {
    fn default() -> Cfg {
        Cfg { plies: 8, checks: true, tempo: 3, heart: 1.0 }
    }
}

/// The rules in their order, each with its variants (applied to a chain).
pub const RULES: [(&str, &[f64]); 4] = [("forcing", &[4.0, 6.0, 8.0]), ("checks", &[0.0, 1.0]), ("tempo", &[0.0, 3.0, 6.0]), ("heart", &[0.5, 1.0, 1.5])];

fn apply(mut c: Cfg, rule: usize, v: f64) -> Cfg {
    match rule {
        0 => c.plies = v as u32,
        1 => c.checks = v > 0.5,
        2 => c.tempo = v as i64,
        _ => c.heart = v,
    }
    c
}

fn legal_list(p: &mut Position) -> Vec<u32> {
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, false);
    list[..n]
        .iter()
        .copied()
        .filter(|&m| {
            p.make(m);
            let ok = !p.in_check_c(1 - p.turn);
            p.unmake();
            ok
        })
        .collect()
}

/// A quiet end's worth for the side to move: Golden heart (scaled) plus the tempo.
fn stand(p: &mut Position, my_moves: usize, c: &Cfg, w: Option<&[f64]>) -> i64 {
    let heart = match w {
        Some(w) => (crate::golden_heart::eval_fast(w, p) as f64 * c.heart) as i64,
        None => 0,
    };
    if c.tempo == 0 {
        return heart;
    }
    // the tempo counts the moves as generated (pseudo-legal), both sides alike
    let mut list = [0u32; MAX_MOVES];
    let mine = gen_moves(p, &mut list, false);
    p.make_null();
    let theirs = gen_moves(p, &mut list, false);
    p.unmake_null();
    let _ = my_moves;
    heart + c.tempo * (mine as i64 - theirs as i64) + 10
}

/// The forcing lines to their end (see golden_chess): the attacker's captures,
/// promotions and (when the rule says so) checks; the defender's captures,
/// promotions and, in check, every reply; each side free to stop.
fn forced(p: &mut Position, plies: u32, attacker: bool, nodes: &mut u64, c: &Cfg, w: Option<&[f64]>) -> i64 {
    *nodes += 1;
    let ms = legal_list(p);
    let check = p.in_check();
    if ms.is_empty() {
        return if check { -MATE } else { 0 };
    }
    if check && plies == 0 {
        return -MATE / 2; // in check at the end of the line: not quiet, counted as bad, not as lost
    }
    let here = if check { -MATE } else { stand(p, ms.len(), c, w) };
    if plies == 0 || *nodes > NODES {
        return here;
    }
    let mut best = here;
    for m in ms {
        let capture = p.board[m_to(m) as usize] != 0 || crate::fast::position::m_flag(m) == 2;
        let promo = m_promo(m) != 0;
        p.make(m);
        let gives_check = c.checks && attacker && p.in_check();
        if check || capture || promo || gives_check {
            let v = -forced(p, plies - 1, !attacker, nodes, c, w);
            if v > best {
                best = v;
            }
        }
        p.unmake();
    }
    best
}

/// The judge: after this move (made on `p`), can the other side mate at once?
fn allows_mate(p: &mut Position) -> bool {
    for r in legal_list(p) {
        p.make(r);
        let mated = p.in_check() && legal_list(p).is_empty();
        p.unmake();
        if mated {
            return true;
        }
    }
    false
}

/// Every legal move's worth (for the mover), best first; ties keep the rules' order.
pub fn rank(p: &mut Position, c: &Cfg, w: Option<&[f64]>) -> Vec<(u32, i64)> {
    let mut rows = Vec::new();
    for m in legal_list(p) {
        p.make(m);
        let mut nodes = 0;
        let mut v = if p.in_check() && legal_list(p).is_empty() { MATE } else { -forced(p, c.plies - 1, false, &mut nodes, c, w) };
        if v < MATE && allows_mate(p) {
            v = -MATE + 1;
        }
        p.unmake();
        rows.push((m, v));
    }
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    rows
}

/// The frozen chain (or the default before any tuning).
pub fn frozen() -> Cfg {
    let mut c = Cfg::default();
    if let Ok(t) = std::fs::read_to_string(RULES_FILE) {
        for l in t.lines() {
            if let Some((k, v)) = l.split_once('=') {
                let v: f64 = match v.trim().parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(i) = RULES.iter().position(|r| r.0 == k.trim()) {
                    c = apply(c, i, v);
                }
            }
        }
    }
    c
}

/// Stockfish's own moves sampled from the games `lines` (every `every`-th), as (position, Stockfish's move).
fn heard(lines: &[&str], want: usize) -> Vec<(Position, u32)> {
    let mut out = Vec::new();
    let total: usize = lines.len();
    let stride = (total / want.max(1)).max(1);
    let mut k = 0usize;
    for (gi, line) in lines.iter().enumerate().step_by(stride) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() != 3 {
            continue;
        }
        let (sw, sb) = (parts[0].contains('W'), parts[0].contains('B'));
        let moves: Vec<&str> = parts[2].split_whitespace().collect();
        // one of Stockfish's moves from this game, spread over the game by its index
        let sf_plies: Vec<usize> = (0..moves.len()).filter(|&i| if i % 2 == 0 { sw } else { sb }).collect();
        if sf_plies.is_empty() {
            continue;
        }
        let at = sf_plies[(gi * 7919 + k) % sf_plies.len()];
        k += 1;
        let mut p = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let mut ok = true;
        for u in &moves[..at] {
            match uci_to_move(&mut p, u) {
                Some(m) => p.make(m),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        if let Some(m) = uci_to_move(&mut p, moves[at]) {
            if legal_moves(&mut p).len() > 1 {
                let fresh = Position::from_fen(&p.to_fen()).unwrap();
                out.push((fresh, m));
            }
        }
        if out.len() >= want {
            break;
        }
    }
    out
}

/// How often the chain plays Stockfish's move, on every core.
fn agreement(set: &[(Position, u32)], c: &Cfg, w: Option<&[f64]>) -> f64 {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = set.len().div_ceil(threads).max(1);
    let hits: usize = std::thread::scope(|sc| {
        set.chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    part.iter()
                        .filter(|(p, m)| {
                            let mut q = p.clone();
                            rank(&mut q, c, w).first().map(|r| r.0) == Some(*m)
                        })
                        .count()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .sum()
    });
    hits as f64 / set.len().max(1) as f64
}

/// Hear the games, choose the seeds by a beam search over the rules, check on the games never heard, freeze.
pub fn tune(path: &str, voice_n: usize, check_n: usize, beam: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let cut = lines.len() * 95 / 100;
    let voice = heard(&lines[..cut], voice_n);
    let check = heard(&lines[cut..], check_n);
    let w = crate::golden_heart::frozen();
    let mut out = format!("Golden Boy, HotMotor: {} of Stockfish's moves heard from the voice games, {} from games never heard\n", voice.len(), check.len());
    let default = Cfg::default();
    out.push_str(&format!("  default chain {default:?}: Stockfish's move {:.1}% (voice)\n", 100.0 * agreement(&voice, &default, w)));
    // the beam: the rules in order, undecided rules at their default
    let mut partial: Vec<(Cfg, f64)> = vec![(default, 0.0)];
    for (r, (name, variants)) in RULES.iter().enumerate() {
        let mut next = Vec::new();
        for (c, _) in &partial {
            for &v in variants.iter() {
                let c2 = apply(*c, r, v);
                if next.iter().any(|(x, _): &(Cfg, f64)| *x == c2) {
                    continue;
                }
                next.push((c2, agreement(&voice, &c2, w)));
            }
        }
        next.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        next.truncate(beam);
        out.push_str(&format!("  rule {name}: best so far {:?} at {:.1}%\n", next[0].0, 100.0 * next[0].1));
        partial = next;
    }
    let (best, on_voice) = partial[0];
    let (c_best, c_default) = (agreement(&check, &best, w), agreement(&check, &default, w));
    out.push_str(&format!("chosen chain {best:?}: Stockfish's move {:.1}% of the voice's moves; on the games never heard {:.1}% (the default chain {:.1}%)\n", 100.0 * on_voice, 100.0 * c_best, 100.0 * c_default));
    let keep = c_best >= c_default;
    let chain = if keep { best } else { default };
    let toml = format!("# Golden Boy's chain, chosen on {} of Stockfish's moves, checked on {} from games never heard\nforcing = {}\nchecks = {}\ntempo = {}\nheart = {}\n", voice.len(), check.len(), chain.plies, chain.checks as u8, chain.tempo, chain.heart);
    std::fs::write(RULES_FILE, toml).map_err(|e| e.to_string())?;
    out.push_str(&format!("{} -> frozen in {RULES_FILE} ({:.0} s)\n", if keep { "kept" } else { "not better on the unheard games: the default is kept" }, t0.elapsed().as_secs_f64()));
    Ok(out)
}

/// Golden Boy's move in a FEN with the frozen chain: (move, worth, the next best).
pub fn best(fen: &str) -> Option<(String, i64, Option<(String, i64)>)> {
    let mut p = Position::from_fen(fen).ok()?;
    let rows = rank(&mut p, &frozen(), crate::golden_heart::frozen());
    let first = rows.first()?;
    Some((move_to_uci(first.0), first.1, rows.get(1).map(|r| (move_to_uci(r.0), r.1))))
}
