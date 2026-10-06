//! Known patterns: a move is played when it can be proved a known pattern —
//! Stockfish has chosen the same kind of move before in the same style of
//! position.
//!
//! A pattern is a move seen from the mover, at three levels of detail:
//!   A  the piece, from, to, what it takes, the promotion, whether it checks
//!   B  A + whether each side attacks the from square and the to square
//!   C  B + the material balance (in pawns, -3..+3) and the phase (pieces left)
//! The table counts, for every pattern, how often it was a legal choice in a
//! position where Stockfish moved and how often Stockfish chose it (CCRL games
//! and the Hugging Face train positions; nothing of the held-out positions).
//!
//! The judge: a move is proved when, at the most detailed level with enough
//! evidence (seen at least MIN_SEEN times), Stockfish chose it at least
//! MIN_RATE of the time.
//!
//! Golden Boy's move: the opening table when the position is familiar; the
//! Syzygy tables at five pieces or fewer (the caller); otherwise the move of
//! Golden (Goldennet with StarNet's search) and of StarNNUE (the same search),
//! each sent to the judge; a proved one is played (both proved: the one
//! Stockfish chose more often). Neither proved: the move is marked unproved
//! and, for now, StarNNUE's is played.

use crate::fast::movegen::{legal_moves, uci_to_move};
use crate::fast::position::{color_of, m_from, m_promo, m_to, move_to_uci, type_of, Position};
use std::collections::HashMap;

pub const TABLE: &str = "out/golden/pattern_table.bin";
pub const MIN_SEEN: u32 = 30;
pub const MIN_RATE: f32 = 0.5;
const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn mix(h: u64, x: u64) -> u64 {
    (h ^ x).wrapping_mul(0x100_0000_01b3)
}

fn attackers(p: &Position, s64: usize) -> (u8, u8) {
    use crate::fast::bitboard::SQ64_TO_120;
    let me = p.turn;
    let (mut a, mut b) = (0u8, 0u8);
    // the cheap way: whether each side attacks the square, and a second look without the cheapest attacker is left out
    if p.attacked(SQ64_TO_120[s64], me) {
        a = 1;
    }
    if p.attacked(SQ64_TO_120[s64], 1 - me) {
        b = 1;
    }
    (a, b)
}

/// The pattern keys of move `m` in `p` (before it is made): [A, B, C].
pub fn keys(p: &mut Position, m: u32) -> [u64; 3] {
    use crate::fast::bitboard::SQ120_TO_64;
    let me = p.turn as usize;
    let rel = |s120: u32| -> u64 {
        let s = SQ120_TO_64[s120 as usize] as usize;
        (if me == 0 { s } else { s ^ 56 }) as u64
    };
    let piece = type_of(p.board[m_from(m) as usize]) as u64;
    let taken = { let t = p.board[m_to(m) as usize]; if t == 0 { 0 } else { type_of(t) as u64 } };
    let promo = if m_promo(m) != 0 { type_of(m_promo(m) as u8) as u64 } else { 0 };
    let (fa, fb) = attackers(p, SQ120_TO_64[m_from(m) as usize] as usize);
    let (ta, tb) = attackers(p, SQ120_TO_64[m_to(m) as usize] as usize);
    p.make(m);
    let check = p.in_check() as u64;
    p.unmake();
    let mut a = 0xcbf2_9ce4_8422_2325u64;
    for x in [1, piece, rel(m_from(m)), rel(m_to(m)), taken, promo, check] {
        a = mix(a, x);
    }
    let mut b = a;
    for x in [2, fa as u64, fb as u64, ta as u64, tb as u64] {
        b = mix(b, x);
    }
    let worth = [0i64, 1, 3, 3, 5, 9, 0];
    let mut bal = 0i64;
    let mut pieces = 0u64;
    for c in 0..2 {
        for t in 0..6 {
            let n = p.bb[c * 6 + t].count_ones() as i64;
            bal += if c == me { 1 } else { -1 } * worth[t + 1] * n;
            if t > 0 && t < 5 {
                pieces += n as u64;
            }
        }
    }
    let mut cc = b;
    for x in [3, (bal.clamp(-3, 3) + 3) as u64, pieces.min(14) / 3] {
        cc = mix(cc, x);
    }
    let _ = color_of;
    [a, b, cc]
}

/// The pattern table: for each key, (seen as a legal choice, chosen by Stockfish).
pub struct Table {
    pub map: HashMap<u64, (u32, u32)>,
}

impl Table {
    pub fn load() -> Option<Table> {
        let b = std::fs::read(TABLE).ok()?;
        let mut map = HashMap::with_capacity(b.len() / 16);
        for c in b.chunks_exact(16) {
            let k = u64::from_le_bytes(c[0..8].try_into().ok()?);
            let s = u32::from_le_bytes(c[8..12].try_into().ok()?);
            let ch = u32::from_le_bytes(c[12..16].try_into().ok()?);
            map.insert(k, (s, ch));
        }
        Some(Table { map })
    }

    /// The judge: the most detailed level with enough evidence; proved when Stockfish chose it often enough. (proved, rate, seen, level)
    pub fn judge(&self, p: &mut Position, m: u32) -> (bool, f32, u32, char) {
        let k = keys(p, m);
        for (lv, name) in [(2usize, 'C'), (1, 'B'), (0, 'A')] {
            if let Some(&(seen, chosen)) = self.map.get(&k[lv]) {
                if seen >= MIN_SEEN {
                    let rate = chosen as f32 / seen as f32;
                    return (rate >= MIN_RATE, rate, seen, name);
                }
            }
        }
        (false, 0.0, 0, '-')
    }
}

/// Build the table from Stockfish's moves: the CCRL games (its moves only, every
/// game but the holdout tenth) and the first `hf` Hugging Face train positions.
pub fn build(hf: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let mut map: HashMap<u64, (u32, u32)> = HashMap::new();
    let mut n = 0usize;
    let mut add = |p: &mut Position, sf: &str, map: &mut HashMap<u64, (u32, u32)>| {
        for m in legal_moves(p) {
            let chosen = move_to_uci(m) == sf;
            for k in keys(p, m) {
                let e = map.entry(k).or_insert((0, 0));
                e.0 += 1;
                e.1 += chosen as u32;
            }
        }
    };
    let games = std::fs::read_to_string(crate::golden_table::GAMES).map_err(|e| e.to_string())?;
    for (g, line) in games.lines().enumerate() {
        if g % 10 == 9 {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let (w, b) = (parts[0].contains('W'), parts[0].contains('B'));
        let mut p = Position::from_fen(START)?;
        for (ply, u) in parts[2].split_whitespace().enumerate() {
            let Some(m) = uci_to_move(&mut p, u) else { break };
            if (ply % 2 == 0 && w) || (ply % 2 == 1 && b) {
                add(&mut p, u, &mut map);
                n += 1;
            }
            p.make(m);
        }
    }
    let ccrl = n;
    let text = std::fs::read_to_string(crate::golden_table::HF_FILE).map_err(|e| e.to_string())?;
    let mut k = 0;
    for line in text.lines() {
        if k >= hf {
            break;
        }
        let mut it = line.splitn(3, '\t');
        let (Some(_), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        let Ok(mut p) = Position::from_fen(fen) else { continue };
        if p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 >= 18 {
            continue; // validate and holdout: never learned from
        }
        add(&mut p, mv, &mut map);
        k += 1;
    }
    let mut out = Vec::with_capacity(map.len() * 16);
    for (key, (s, c)) in &map {
        out.extend_from_slice(&key.to_le_bytes());
        out.extend_from_slice(&s.to_le_bytes());
        out.extend_from_slice(&c.to_le_bytes());
    }
    std::fs::write(TABLE, out).map_err(|e| e.to_string())?;
    Ok(format!("pattern table: {ccrl} CCRL positions (Stockfish's moves) + {k} Hugging Face positions; {} patterns ({:.0} s), {TABLE}\n", map.len(), t0.elapsed().as_secs_f64()))
}

/// Golden Boy's move by the known patterns: (move, where it came from, the judge's word).
pub fn move_by_patterns(t: &Table, fen: &str, ms: u64) -> Result<(String, String), String> {
    if let Some(m) = crate::golden_grow::table_move_pub(fen) {
        return Ok((m, "the opening table: a known line".into()));
    }
    let gw: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    let ev = crate::star_search::GoldennetEval(gw);
    let net = crate::fast::starnnue::net()?;
    let (golden, _, _) = crate::star_search::best_move(&ev, fen, ms / 2).ok_or("no move")?;
    let (star, _, _) = crate::star_search::best_move_with(&ev, Some((net, 1.0)), fen, ms / 2).ok_or("no move")?;
    let mut p = Position::from_fen(fen)?;
    let mut judged = Vec::new();
    for (who, mv) in [("Golden", &golden), ("StarNNUE", &star)] {
        let m = uci_to_move(&mut p, mv).ok_or("illegal")?;
        let (ok, rate, seen, lv) = t.judge(&mut p, m);
        judged.push((who, mv.clone(), ok, rate, seen, lv));
    }
    let words: Vec<String> = judged.iter().map(|(w, m, ok, r, s, lv)| format!("{w} {m}: {} (level {lv}, Stockfish chose it {:.0}% of {s})", if *ok { "proved" } else { "not proved" }, 100.0 * r)).collect();
    let proved: Vec<&(&str, String, bool, f32, u32, char)> = judged.iter().filter(|j| j.2).collect();
    let pick = match proved.iter().max_by(|a, b| a.3.total_cmp(&b.3)) {
        Some(j) => (j.1.clone(), format!("{} (proved); {}", j.0, words.join("; "))),
        None => (star.clone(), format!("unproved, StarNNUE for now; {}", words.join("; "))),
    };
    Ok(pick)
}

/// The smoke test by patterns: each position reported at once.
pub fn smoke(file: &str, ms: u64) -> Result<(), String> {
    use std::io::Write;
    let t = Table::load().ok_or("no pattern table: run golden-patterns first")?;
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    rows.reverse();
    let mut hit = 0;
    for (i, (fen, sf)) in rows.iter().enumerate() {
        let (m, how) = move_by_patterns(&t, fen, ms)?;
        hit += (m == *sf) as usize;
        println!("  {}/{}: played {m}, Stockfish {sf} {} — {how} — {hit}/{} so far", i + 1, rows.len(), if m == *sf { "RIGHT" } else { "miss" }, i + 1);
        let _ = std::io::stdout().flush();
    }
    println!("patterns smoke test: {hit}/{}", rows.len());
    Ok(())
}
