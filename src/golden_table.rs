//! Golden Answer's approximate answer table for chess (first slice: openings).
//!
//! The table is the answer: a position maps to a move, read straight from
//! it; nothing is searched at answer time. Outside what it knows it says
//! "not familiar". The target is about 90 % agreement with Stockfish.
//!
//! - The voice: Stockfish's moves in CCRL games (out/golden/ccrl_sf_games.txt),
//!   only its moves, no evaluation. Fixed split by game number: n % 10 < 8
//!   train, 8 validate, 9 holdout. Openings: the first OPENING_PLIES plies.
//! - The grid net: a fixed 64-square board; only the points' values change.
//!   A square's point depends on what stands on it (empty, or one of the
//!   mover's / the opponent's six pieces) with the attack on it folded in
//!   (how many of the mover's and of the opponent's pieces attack it: 0, 1,
//!   2+). A position's value = the sum of its 64 points, seen from the mover.
//!   Learned from which move Stockfish chose (a conditional logit over the
//!   legal moves), epochs chosen on validate, then frozen.
//! - The table: written by Golden Answer from the train games: every
//!   position heard maps to the move Stockfish played there most often; ties
//!   go to the grid net.

use crate::fast::movegen::{legal_moves, uci_to_move};
use crate::fast::position::{move_to_uci, Position};
use std::collections::HashMap;

pub const GAMES: &str = "out/golden/ccrl_sf_games.txt";
pub const OPENING_PLIES: usize = 20;
const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
/// 13 contents x 9 attack states x 64 squares
pub const NW: usize = 13 * 9 * 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Split {
    Train,
    Validate,
    Holdout,
}

fn split_of(game: usize) -> Split {
    match game % 10 {
        8 => Split::Validate,
        9 => Split::Holdout,
        _ => Split::Train,
    }
}

/// One heard opening position: its key, the position, and Stockfish's move.
pub struct Heard {
    pub key: u64,
    pub fen: String,
    pub sf: String,
    /// the ply in its game (0 = the first move)
    pub ply: usize,
}

/// Every Stockfish opening move of the first `max_games` games, by split.
pub fn hear(max_games: usize) -> Result<[Vec<Heard>; 3], String> {
    let text = std::fs::read_to_string(GAMES).map_err(|e| format!("{GAMES}: {e}"))?;
    let mut out: [Vec<Heard>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (g, line) in text.lines().take(max_games).enumerate() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let (w, b) = (parts[0].contains('W'), parts[0].contains('B'));
        let mut p = Position::from_fen(START)?;
        for (ply, u) in parts[2].split_whitespace().take(OPENING_PLIES).enumerate() {
            let sf_moves = if ply % 2 == 0 { w } else { b };
            let Some(m) = uci_to_move(&mut p, u) else { break };
            if sf_moves {
                let h = Heard { key: p.key64(), fen: p.to_fen(), sf: u.to_string(), ply };
                out[split_of(g) as usize].push(h);
            }
            p.make(m);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- the grid net

/// The 64 points of position `p` seen from `me`: feature indices.
pub fn points(p: &Position, me: usize) -> [u16; 64] {
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    let mut cnt = [[0u8; 64]; 2];
    for c in 0..2 {
        for ty in 0..6 {
            let mut b = p.bb[c * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let mut at = match ty {
                    0 => {
                        if c == 0 {
                            t.att.pawn_w[s]
                        } else {
                            t.att.pawn_b[s]
                        }
                    }
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, occ),
                    3 => t.rook_attacks(s, occ),
                    4 => t.queen_attacks(s, occ),
                    _ => t.att.king[s],
                };
                while at != 0 {
                    let q = at.trailing_zeros() as usize;
                    at &= at - 1;
                    cnt[c][q] = cnt[c][q].saturating_add(1);
                }
            }
        }
    }
    let mut out = [0u16; 64];
    for s in 0..64 {
        let mut content = 0usize;
        for c in 0..2 {
            for ty in 0..6 {
                if p.bb[c * 6 + ty] >> s & 1 == 1 {
                    content = 1 + if c == me { 0 } else { 6 } + ty;
                }
            }
        }
        let am = cnt[me][s].min(2) as usize;
        let at = cnt[1 - me][s].min(2) as usize;
        let rel = if me == 0 { s } else { s ^ 56 };
        out[s] = (((content * 9 + am * 3 + at) * 64) + rel) as u16;
    }
    out
}

/// The points after every legal move (seen from the mover), and the index of Stockfish's move.
fn choices(h: &Heard) -> Option<(Vec<[u16; 64]>, usize, Vec<String>)> {
    let mut p = Position::from_fen(&h.fen).ok()?;
    let me = p.turn as usize;
    let ms = legal_moves(&mut p);
    let mut xs = Vec::with_capacity(ms.len());
    let mut names = Vec::with_capacity(ms.len());
    let mut chosen = None;
    for (i, &m) in ms.iter().enumerate() {
        let u = move_to_uci(m);
        if u == h.sf {
            chosen = Some(i);
        }
        p.make(m);
        xs.push(points(&p, me));
        p.unmake();
        names.push(u);
    }
    Some((xs, chosen?, names))
}

fn score(w: &[f32], x: &[u16; 64]) -> f32 {
    x.iter().map(|&i| w[i as usize]).sum()
}

/// The net's move index among the choices.
fn net_pick(w: &[f32], xs: &[[u16; 64]]) -> usize {
    let mut best = (f32::NEG_INFINITY, 0);
    for (i, x) in xs.iter().enumerate() {
        let s = score(w, x);
        if s > best.0 {
            best = (s, i);
        }
    }
    best.1
}

/// One epoch of the conditional logit: Stockfish's choice against the softmax over the legal moves.
fn epoch(w: &mut [f32], set: &[(Vec<[u16; 64]>, usize, Vec<String>)], lr: f32) {
    for (xs, c, _) in set {
        let s: Vec<f32> = xs.iter().map(|x| score(w, x)).collect();
        let mx = s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let e: Vec<f32> = s.iter().map(|v| (v - mx).exp()).collect();
        let z: f32 = e.iter().sum();
        for (i, x) in xs.iter().enumerate() {
            let g = if i == *c { 1.0 } else { 0.0 } - e[i] / z;
            if g.abs() < 1e-4 {
                continue;
            }
            for &f in x {
                w[f as usize] += lr * g;
            }
        }
    }
}

fn net_agreement(w: &[f32], set: &[(Vec<[u16; 64]>, usize, Vec<String>)]) -> f64 {
    set.iter().filter(|(xs, c, _)| net_pick(w, xs) == *c).count() as f64 / set.len().max(1) as f64
}

// ---------------------------------------------------------------- the table

pub struct Table {
    pub moves: HashMap<u64, String>,
}

impl Table {
    /// The answer: the move, or None ("not familiar").
    pub fn answer(&self, key: u64) -> Option<&String> {
        self.moves.get(&key)
    }
}

/// Golden Answer writes the table from the train games; ties go to the frozen net.
fn write_table(train: &[Heard], w: &[f32]) -> Table {
    let mut counts: HashMap<u64, (String, HashMap<String, u32>)> = HashMap::new();
    for h in train {
        let e = counts.entry(h.key).or_insert_with(|| (h.fen.clone(), HashMap::new()));
        *e.1.entry(h.sf.clone()).or_default() += 1;
    }
    let mut moves = HashMap::new();
    for (key, (fen, c)) in counts {
        let top = *c.values().max().unwrap();
        let tied: Vec<&String> = c.iter().filter(|x| *x.1 == top).map(|x| x.0).collect();
        let pick = if tied.len() == 1 {
            tied[0].clone()
        } else {
            let h = Heard { key, fen, sf: tied[0].clone(), ply: 0 };
            match choices(&h) {
                Some((xs, _, names)) => {
                    let best = tied.iter().copied().max_by(|a, b| {
                        let sa = names.iter().position(|n| n == *a).map_or(f32::NEG_INFINITY, |i| score(w, &xs[i]));
                        let sb = names.iter().position(|n| n == *b).map_or(f32::NEG_INFINITY, |i| score(w, &xs[i]));
                        sa.total_cmp(&sb)
                    });
                    best.unwrap().clone()
                }
                None => tied[0].clone(),
            }
        };
        moves.insert(key, pick);
    }
    Table { moves }
}

/// The first slice: hear, learn the net (epochs by validate), freeze, write the table, report on holdout.
pub fn report(max_games: usize, max_epochs: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let [train, validate, holdout] = hear(max_games)?;
    let mut out = format!(
        "Golden Answer's table, openings (the first {OPENING_PLIES} plies), Stockfish's moves from {} CCRL games: train {} / validate {} / holdout {} positions (heard in {:.0} s)\n",
        max_games.min(124_707),
        train.len(),
        validate.len(),
        holdout.len(),
        t0.elapsed().as_secs_f64()
    );
    // the grid net: points from Stockfish's choices, epochs chosen on validate, frozen
    let tr: Vec<_> = train.iter().filter_map(choices).collect();
    let va: Vec<_> = validate.iter().filter_map(choices).collect();
    let mut w = vec![0f32; NW];
    let mut best = (net_agreement(&w, &va), w.clone(), 0);
    for e in 1..=max_epochs {
        epoch(&mut w, &tr, 0.05 / e as f32);
        let a = net_agreement(&w, &va);
        out.push_str(&format!("  grid net epoch {e}: Stockfish's move {:.1}% of validate\n", 100.0 * a));
        if a > best.0 {
            best = (a, w.clone(), e);
        }
    }
    let w = best.1;
    out.push_str(&format!("  frozen at epoch {} ({:.0} s)\n", best.2, t0.elapsed().as_secs_f64()));
    // the table
    let table = write_table(&train, &w);
    // holdout
    let mut familiar = 0;
    let mut table_hits = 0;
    let mut net_hits = 0;
    let ho: Vec<_> = holdout.iter().filter_map(|h| choices(h).map(|c| (h, c))).collect();
    for (h, (xs, c, _)) in &ho {
        if let Some(m) = table.answer(h.key) {
            familiar += 1;
            if *m == h.sf {
                table_hits += 1;
            }
        }
        if net_pick(&w, xs) == *c {
            net_hits += 1;
        }
    }
    let n = ho.len().max(1) as f64;
    // the ceiling: how often Stockfish plays its own most frequent move in a position (holdout, familiar positions)
    let mut by_key: HashMap<u64, HashMap<&str, u32>> = HashMap::new();
    for (h, _) in &ho {
        if table.answer(h.key).is_some() {
            *by_key.entry(h.key).or_default().entry(h.sf.as_str()).or_default() += 1;
        }
    }
    let own_top: u32 = by_key.values().map(|c| *c.values().max().unwrap()).sum();
    out.push_str(&format!("ceiling: in the familiar holdout positions Stockfish plays its own most frequent move {:.1}% of the time (no table can agree with it more)
", 100.0 * own_top as f64 / familiar.max(1) as f64));
    out.push_str(&format!(
        "table: {} positions written\nholdout ({} positions):\n  familiar: {:.1}% ({familiar}); the rest answered \"not familiar\"\n  table's move = Stockfish's: {:.1}% of the familiar, {:.1}% of all holdout\n  grid net alone (frozen), for comparison: {:.1}% of all holdout\n({:.0} s)\n",
        table.moves.len(),
        ho.len(),
        100.0 * familiar as f64 / n,
        100.0 * table_hits as f64 / familiar.max(1) as f64,
        100.0 * table_hits as f64 / n,
        100.0 * net_hits as f64 / n,
        t0.elapsed().as_secs_f64()
    ));
    Ok(out)
}

/// Every distinct opening position of the games, with its split: out/golden/table_fens.txt (split TAB key TAB fen).
pub fn write_fens(path: &str) -> Result<usize, String> {
    let [train, validate, holdout] = hear(usize::MAX)?;
    let mut seen = std::collections::HashSet::new();
    let mut text = String::new();
    for (name, set) in [("train", &train), ("validate", &validate), ("holdout", &holdout)] {
        for h in set.iter() {
            if seen.insert((name, h.key)) {
                text.push_str(&format!("{name}	{}	{}
", h.key, h.fen));
            }
        }
    }
    std::fs::write(path, text).map_err(|e| e.to_string())?;
    Ok(seen.len())
}

// ---------------------------------------------------------------- Stockfish's analysis as the reference

pub const SF_LABELS: &str = "out/golden/table_sf.tsv";

/// The net's answer to a position: the move and how sure it is (the softmax
/// share of its best move among the legal ones).
fn net_answer(w: &[f32], xs: &[[u16; 64]]) -> (usize, f32) {
    let s: Vec<f32> = xs.iter().map(|x| score(w, x)).collect();
    let mx = s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let z: f32 = s.iter().map(|v| (v - mx).exp()).sum();
    let i = s.iter().position(|&v| v == mx).unwrap_or(0);
    (i, 1.0 / z)
}

/// The second report: the reference is Stockfish's move at a fixed depth
/// (tools/golden_table_sf.py), one per position. The table is the frozen
/// grid net itself: an answer is read straight from it (its points summed
/// after every legal move), nothing searched; it says "not familiar" when
/// its best move's share is below a threshold chosen on validate. Learned
/// from the distinct train positions, epochs chosen on validate.
pub fn report_sf(max_epochs: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(SF_LABELS).map_err(|e| format!("{SF_LABELS}: {e}"))?;
    let label: HashMap<u64, String> = text.lines().filter_map(|l| l.split_once('\t')).filter_map(|(k, m)| Some((k.parse().ok()?, m.trim().to_string()))).collect();
    let [train, validate, holdout] = hear(usize::MAX)?;
    let distinct = |set: Vec<Heard>| -> Vec<Heard> {
        let mut seen = std::collections::HashSet::new();
        set.into_iter().filter(|h| seen.insert(h.key)).filter_map(|h| label.get(&h.key).map(|m| Heard { key: h.key, fen: h.fen, sf: m.clone(), ply: h.ply })).collect()
    };
    let (train, validate, holdout) = (distinct(train), distinct(validate), distinct(holdout));
    let mut out = format!("Golden Answer's table = the frozen grid net, openings; reference = Stockfish's move at a fixed depth ({} positions analysed); distinct positions: train {} / validate {} / holdout {}\n", label.len(), train.len(), validate.len(), holdout.len());
    let tr: Vec<_> = train.iter().filter_map(choices).collect();
    let va: Vec<_> = validate.iter().filter_map(choices).collect();
    let mut w = vec![0f32; NW];
    let mut best = (net_agreement(&w, &va), w.clone(), 0);
    for e in 1..=max_epochs {
        epoch(&mut w, &tr, 0.05 / e as f32);
        let a = net_agreement(&w, &va);
        out.push_str(&format!("  epoch {e}: Stockfish's move {:.1}% of validate\n", 100.0 * a));
        if a > best.0 {
            best = (a, w.clone(), e);
        }
    }
    let w = best.1;
    out.push_str(&format!("  frozen at epoch {} ({:.0} s)\n", best.2, t0.elapsed().as_secs_f64()));
    // "not familiar": the threshold on the net's own sureness, chosen on validate
    let sure = |set: &[(Vec<[u16; 64]>, usize, Vec<String>)]| -> Vec<(f32, bool)> { set.iter().map(|(xs, c, _)| { let (i, p) = net_answer(&w, xs); (p, i == *c) }).collect() };
    let va_s = sure(&va);
    out.push_str("  validate, by threshold (answered share, agreement where answered):\n");
    for th in [0.0f32, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8] {
        let ans: Vec<&(f32, bool)> = va_s.iter().filter(|x| x.0 >= th).collect();
        out.push_str(&format!("    {th:.1}: {:.1}% answered, {:.1}% agree\n", 100.0 * ans.len() as f64 / va_s.len().max(1) as f64, 100.0 * ans.iter().filter(|x| x.1).count() as f64 / ans.len().max(1) as f64));
    }
    // the threshold: the lowest at which the answered ones agree 90 % on validate (the target), else none
    let th = [0.0f32, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95]
        .into_iter()
        .find(|&th| {
            let ans: Vec<&(f32, bool)> = va_s.iter().filter(|x| x.0 >= th).collect();
            !ans.is_empty() && ans.iter().filter(|x| x.1).count() as f64 / ans.len() as f64 >= 0.9
        });
    let ho = sure(&holdout.iter().filter_map(choices).collect::<Vec<_>>());
    let all = ho.len().max(1) as f64;
    out.push_str(&format!("holdout ({} distinct positions), never used before this line:\n  every position answered: Stockfish's move {:.1}%\n", ho.len(), 100.0 * ho.iter().filter(|x| x.1).count() as f64 / all));
    match th {
        Some(th) => {
            let ans: Vec<&(f32, bool)> = ho.iter().filter(|x| x.0 >= th).collect();
            out.push_str(&format!("  threshold {th:.2} (the lowest reaching 90 % on validate): answers {:.1}% of the positions, Stockfish's move in {:.1}% of those; the rest \"not familiar\"\n", 100.0 * ans.len() as f64 / all, 100.0 * ans.iter().filter(|x| x.1).count() as f64 / ans.len().max(1) as f64));
        }
        None => out.push_str("  no threshold reaches 90 % on validate\n"),
    }
    out.push_str(&format!("({:.0} s)\n", t0.elapsed().as_secs_f64()));
    Ok(out)
}

/// The table itself, against Stockfish's analysis: written from the train
/// positions' analysed moves; on the holdout, how much it knows (familiar),
/// and the table's move = Stockfish's where it knows; the rest "not familiar".
pub fn report_table() -> Result<String, String> {
    let text = std::fs::read_to_string(SF_LABELS).map_err(|e| format!("{SF_LABELS}: {e}"))?;
    let label: HashMap<u64, String> = text.lines().filter_map(|l| l.split_once('\t')).filter_map(|(k, m)| Some((k.parse().ok()?, m.trim().to_string()))).collect();
    let [train, _validate, holdout] = hear(usize::MAX)?;
    let table: HashMap<u64, &String> = train.iter().filter_map(|h| label.get(&h.key).map(|m| (h.key, m))).collect();
    let mut occ = [0usize; 3]; // all, familiar, hit
    let mut dis = [0usize; 3];
    let mut seen = std::collections::HashSet::new();
    let mut by_ply = vec![[0usize; 2]; OPENING_PLIES];
    for h in &holdout {
        let Some(sf) = label.get(&h.key) else { continue };
        let fam = table.get(&h.key);
        let hit = fam.map_or(false, |m| *m == sf);
        occ[0] += 1;
        occ[1] += fam.is_some() as usize;
        occ[2] += hit as usize;
        if seen.insert(h.key) {
            dis[0] += 1;
            dis[1] += fam.is_some() as usize;
            dis[2] += hit as usize;
        }
        let ply = h.ply;
        if ply < OPENING_PLIES {
            by_ply[ply][0] += 1;
            by_ply[ply][1] += fam.is_some() as usize;
        }
    }
    let pc = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
    let mut out = format!(
        "Golden Answer's table (openings), entries = Stockfish's analysed move for every train position: {} entries\nholdout by occurrence ({}): familiar {:.1}%, table = Stockfish {:.1}% of the familiar, {:.1}% of all\nholdout by distinct position ({}): familiar {:.1}%, table = Stockfish {:.1}% of the familiar, {:.1}% of all\nfamiliar by ply (occurrences):",
        table.len(), occ[0], pc(occ[1], occ[0]), pc(occ[2], occ[1]), pc(occ[2], occ[0]), dis[0], pc(dis[1], dis[0]), pc(dis[2], dis[1]), pc(dis[2], dis[0])
    );
    for (i, b) in by_ply.iter().enumerate() {
        out.push_str(&format!(" {}:{:.0}%", i + 1, pc(b[1], b[0])));
    }
    out.push('\n');
    Ok(out)
}

pub const TABLE_FILE: &str = "out/golden/opening_table.tsv";

/// The table with every known position: every analysed opening position
/// (train, validate and holdout) with Stockfish's move, written to
/// TABLE_FILE (key TAB move TAB fen). The holdout is inside it now, so it
/// measures nothing; a measure needs games the table has not seen.
pub fn write_all() -> Result<String, String> {
    let text = std::fs::read_to_string(SF_LABELS).map_err(|e| format!("{SF_LABELS}: {e}"))?;
    let label: HashMap<u64, String> = text.lines().filter_map(|l| l.split_once('\t')).filter_map(|(k, m)| Some((k.parse().ok()?, m.trim().to_string()))).collect();
    let [train, validate, holdout] = hear(usize::MAX)?;
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    let mut by_ply = vec![0usize; OPENING_PLIES];
    for h in train.iter().chain(&validate).chain(&holdout) {
        if let Some(m) = label.get(&h.key) {
            if seen.insert(h.key) {
                out.push_str(&format!("{}\t{m}\t{}\n", h.key, h.fen));
                by_ply[h.ply.min(OPENING_PLIES - 1)] += 1;
            }
        }
    }
    std::fs::write(TABLE_FILE, out).map_err(|e| e.to_string())?;
    Ok(format!(
        "Golden Answer's opening table: {} positions, every known one (train + validate + holdout), Stockfish's move at depth 14 each, written to {TABLE_FILE}\nentries by ply: {}\nthe holdout is inside the table now: measuring it needs games the table has not seen\n",
        seen.len(),
        by_ply.iter().enumerate().map(|(i, n)| format!("{}:{n}", i + 1)).collect::<Vec<_>>().join(" ")
    ))
}

pub const HF_FILE: &str = "out/golden/hf_mid_end.tsv";
pub const FULL_TABLE: &str = "out/golden/golden_table.tsv";

/// The middlegame and endgame (more than five pieces) positions taught to
/// the table: Stockfish's best move for each (Hugging Face
/// agi-noobs/chess-sft-10m-stockfish, extracted to HF_FILE). One in ten
/// (by the position's key) is held out first to see how much of it the
/// table knows; then every position goes in: FULL_TABLE (phase TAB key TAB move).
pub fn learn_mid_end() -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(HF_FILE).map_err(|e| format!("{HF_FILE}: {e}"))?;
    let mut rows: Vec<(&str, u64, &str)> = Vec::new();
    for line in text.lines() {
        let mut it = line.splitn(3, '\t');
        let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        let Ok(p) = Position::from_fen(fen) else { continue };
        rows.push((ph, p.key64(), mv));
    }
    let held = |k: u64| k.wrapping_mul(0x9E37_79B9_7F4A_7C15) % 10 == 9;
    let table: HashMap<u64, &str> = rows.iter().filter(|r| !held(r.1)).map(|r| (r.1, r.2)).collect();
    let mut out = format!("taught: {} middlegame and endgame (6+ pieces) positions read ({:.0} s)\n", rows.len(), t0.elapsed().as_secs_f64());
    for phase in ["middlegame", "endgame"] {
        let ho: Vec<&(&str, u64, &str)> = rows.iter().filter(|r| r.0 == phase && held(r.1)).collect();
        let fam = ho.iter().filter(|r| table.contains_key(&r.1)).count();
        out.push_str(&format!("  held out {phase}: {} positions, familiar to the table {:.2}%\n", ho.len(), 100.0 * fam as f64 / ho.len().max(1) as f64));
    }
    // every known position into the table: the openings and these
    let mut all = String::new();
    let opening = std::fs::read_to_string(TABLE_FILE).unwrap_or_default();
    let mut n_open = 0;
    for l in opening.lines() {
        let mut it = l.splitn(3, '\t');
        if let (Some(k), Some(m)) = (it.next(), it.next()) {
            all.push_str(&format!("opening\t{k}\t{m}\n"));
            n_open += 1;
        }
    }
    for (ph, k, m) in &rows {
        all.push_str(&format!("{ph}\t{k}\t{m}\n"));
    }
    std::fs::write(FULL_TABLE, all).map_err(|e| e.to_string())?;
    out.push_str(&format!("the table now: {n_open} opening + {} middlegame and endgame positions, written to {FULL_TABLE} ({:.0} s)\n", rows.len(), t0.elapsed().as_secs_f64()));
    Ok(out)
}

/// How much of the CCRL games' middlegame and endgame (plies 21-..., more
/// than five pieces) the full table knows: a source the table was not built from.
pub fn coverage_ccrl(max_games: usize) -> Result<String, String> {
    let text = std::fs::read_to_string(FULL_TABLE).map_err(|e| format!("{FULL_TABLE}: {e}"))?;
    let keys: std::collections::HashSet<u64> = text.lines().filter_map(|l| l.split('\t').nth(1)?.parse().ok()).filter(|_| true).collect();
    let games = std::fs::read_to_string(GAMES).map_err(|e| e.to_string())?;
    let mut n = [0usize; 2];
    let mut fam = [0usize; 2];
    for line in games.lines().take(max_games) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let mut p = Position::from_fen(START)?;
        for (ply, u) in parts[2].split_whitespace().enumerate() {
            let Some(m) = uci_to_move(&mut p, u) else { break };
            let pieces = p.occ.count_ones();
            if ply >= OPENING_PLIES && pieces > 5 {
                let ph = if pieces > 12 { 0 } else { 1 };
                n[ph] += 1;
                fam[ph] += keys.contains(&p.key64()) as usize;
            }
            p.make(m);
        }
    }
    Ok(format!(
        "the full table ({} positions) on CCRL positions from ply 21 on, more than five pieces ({max_games} games):\n  more than 12 pieces: {} positions, familiar {:.2}%\n  6-12 pieces: {} positions, familiar {:.2}%\n",
        keys.len(),
        n[0],
        100.0 * fam[0] as f64 / n[0].max(1) as f64,
        n[1],
        100.0 * fam[1] as f64 / n[1].max(1) as f64
    ))
}

pub const GOLDENNET_FILE: &str = "out/golden/goldennet.txt";

/// Goldennet taught the table's voice: the grid net learns Stockfish's moves
/// of the middlegame and endgame positions in HF_FILE (split by the key:
/// 5 % validate, 5 % holdout, the rest train; the first `n_train` train
/// positions used), streamed one position at a time, epochs chosen on
/// validate, frozen to GOLDENNET_FILE; holdout by phase.
pub fn teach_goldennet(n_train: usize, epochs: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(HF_FILE).map_err(|e| format!("{HF_FILE}: {e}"))?;
    let mut train = Vec::new();
    let mut validate = Vec::new();
    let mut holdout = Vec::new();
    for line in text.lines() {
        let mut it = line.splitn(3, '\t');
        let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        let Ok(p) = Position::from_fen(fen) else { continue };
        let k = p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20;
        let h = (ph == "endgame", Heard { key: p.key64(), fen: fen.to_string(), sf: mv.to_string(), ply: 0 });
        match k {
            19 => holdout.push(h),
            18 => validate.push(h),
            _ if train.len() < n_train => train.push(h),
            _ => {}
        }
    }
    validate.truncate(20_000);
    holdout.truncate(40_000);
    let va: Vec<_> = validate.iter().filter_map(|h| choices(&h.1)).collect();
    let mut out = format!("Goldennet taught the table's voice: {} train / {} validate / {} holdout middlegame and endgame positions ({:.0} s to read)\n", train.len(), va.len(), holdout.len(), t0.elapsed().as_secs_f64());
    let mut w = vec![0f32; NW];
    let mut best = (net_agreement(&w, &va), w.clone(), 0);
    for e in 1..=epochs {
        let lr = 0.05 / e as f32;
        for h in &train {
            if let Some(c) = choices(&h.1) {
                epoch(&mut w, std::slice::from_ref(&c), lr);
            }
        }
        let a = net_agreement(&w, &va);
        out.push_str(&format!("  epoch {e}: Stockfish's move {:.1}% of validate ({:.0} s)\n", 100.0 * a, t0.elapsed().as_secs_f64()));
        if a > best.0 {
            best = (a, w.clone(), e);
        }
    }
    let w = best.1;
    std::fs::write(GOLDENNET_FILE, w.iter().map(|x| format!("{x}\n")).collect::<String>()).map_err(|e| e.to_string())?;
    for (end, name) in [(false, "middlegame"), (true, "endgame (6+ pieces)")] {
        let set: Vec<_> = holdout.iter().filter(|h| h.0 == end).filter_map(|h| choices(&h.1)).collect();
        out.push_str(&format!("  holdout {name}: {} positions, Stockfish's move {:.1}%\n", set.len(), 100.0 * net_agreement(&w, &set)));
    }
    out.push_str(&format!("frozen at epoch {} to {GOLDENNET_FILE} ({:.0} s)\n", best.2, t0.elapsed().as_secs_f64()));
    Ok(out)
}

/// The frozen Goldennet from GOLDENNET_FILE.
pub fn load_goldennet() -> Option<Vec<f32>> {
    let w: Vec<f32> = std::fs::read_to_string(GOLDENNET_FILE).ok()?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    (w.len() == NW).then_some(w)
}

const NET_MATE: f32 = 1.0e6;

/// Every line to `depth`, nothing cut; the value for the side to move.
/// Mate and stalemate by the rules; at the leaves Goldennet's value of the
/// position for the side that has just moved, so the side to move gets minus it.
fn deep(w: &[f32], p: &mut Position, depth: usize) -> f32 {
    if depth == 0 {
        // a leaf: only whether any legal move exists (stop at the first), then the net
        if !has_legal(p) {
            return if p.in_check() { -NET_MATE } else { 0.0 };
        }
        let moved = 1 - p.turn as usize;
        return -score(w, &points(p, moved));
    }
    let ms = legal_moves(p);
    if ms.is_empty() {
        return if p.in_check() { -NET_MATE } else { 0.0 };
    }
    let mut best = f32::NEG_INFINITY;
    for m in ms {
        p.make(m);
        let v = -deep(w, p, depth - 1);
        p.unmake();
        if v > best {
            best = v;
        }
    }
    best
}

/// Goldennet's move with every line searched to `depth` plies (depth 1 = the net alone), root moves on all cores.
pub fn deep_pick(w: &[f32], fen: &str, depth: usize) -> Option<String> {
    let mut p = Position::from_fen(fen).ok()?;
    let ms = legal_moves(&mut p);
    let vals: Vec<(f32, String)> = std::thread::scope(|s| {
        let hs: Vec<_> = ms
            .iter()
            .map(|&m| {
                s.spawn(move || {
                    let mut q = Position::from_fen(fen).unwrap();
                    q.make(m);
                    (-deep(w, &mut q, depth - 1), move_to_uci(m))
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    vals.into_iter().fold(None::<(f32, String)>, |b, x| match b { Some(b) if b.0 >= x.0 => Some(b), _ => Some(x) }).map(|x| x.1)
}

/// Goldennet with deeper search, on a holdout sample (the same split as teach_goldennet).
pub fn report_deep(n_mid: usize, n_end: usize, max_depth: usize) -> Result<String, String> {
    let w = load_goldennet().ok_or("no Goldennet yet: run goldennet first")?;
    let text = std::fs::read_to_string(HF_FILE).map_err(|e| format!("{HF_FILE}: {e}"))?;
    let (mut mid, mut end) = (Vec::new(), Vec::new());
    for line in text.lines() {
        if mid.len() >= n_mid && end.len() >= n_end {
            break;
        }
        let mut it = line.splitn(3, '\t');
        let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        let Ok(p) = Position::from_fen(fen) else { continue };
        if p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 != 19 {
            continue;
        }
        if ph == "endgame" && end.len() < n_end {
            end.push((fen.to_string(), mv.to_string()));
        } else if ph == "middlegame" && mid.len() < n_mid {
            mid.push((fen.to_string(), mv.to_string()));
        }
    }
    let mut out = format!("Goldennet with deeper search (every line, nothing cut; Goldennet at the leaves), holdout sample: {} middlegame, {} endgame\n", mid.len(), end.len());
    for d in 1..=max_depth {
        let t0 = std::time::Instant::now();
        let hit = |set: &[(String, String)]| set.iter().filter(|(f, m)| deep_pick(&w, f, d).as_deref() == Some(m.as_str())).count();
        let (hm, he) = (hit(&mid), hit(&end));
        let secs = t0.elapsed().as_secs_f64();
        out.push_str(&format!("  depth {d}: middlegame {:.1}%, endgame {:.1}% ({:.2} s a position)\n", 100.0 * hm as f64 / mid.len().max(1) as f64, 100.0 * he as f64 / end.len().max(1) as f64, secs / (mid.len() + end.len()).max(1) as f64));
    }
    Ok(out)
}

pub const HF_VALUES: &str = "out/golden/hf_values.tsv";
pub const GOLDENNET_VALUE_FILE: &str = "out/golden/goldennet_value.txt";

/// One position with Stockfish's win probability after each move: (endgame?, fen, best move, [(move, win prob)]).
type Valued = (bool, String, String, Vec<(String, f32)>);

fn read_values(n_train: usize, n_val: usize, n_hold: usize) -> Result<(Vec<Valued>, Vec<Valued>, Vec<Valued>), String> {
    let text = std::fs::read_to_string(HF_VALUES).map_err(|e| format!("{HF_VALUES}: {e}"))?;
    let (mut tr, mut va, mut ho) = (Vec::new(), Vec::new(), Vec::new());
    for line in text.lines() {
        if tr.len() >= n_train && va.len() >= n_val && ho.len() >= n_hold {
            break;
        }
        let f: Vec<&str> = line.splitn(4, '\t').collect();
        if f.len() < 4 {
            continue;
        }
        let Ok(p) = Position::from_fen(f[2]) else { continue };
        let vals: Vec<(String, f32)> = f[3].split_whitespace().filter_map(|t| t.split_once(':').and_then(|(m, v)| v.parse().ok().map(|v| (m.to_string(), v)))).collect();
        let row = (f[0] == "endgame", f[2].to_string(), f[1].to_string(), vals);
        match p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 {
            19 if ho.len() < n_hold => ho.push(row),
            18 if va.len() < n_val => va.push(row),
            k if k < 18 && tr.len() < n_train => tr.push(row),
            _ => {}
        }
    }
    Ok((tr, va, ho))
}

/// The positions after each valued move, seen from the mover, with the target logit of the win probability.
fn valued_children(row: &Valued) -> Vec<([u16; 64], f32)> {
    let Ok(mut p) = Position::from_fen(&row.1) else { return Vec::new() };
    let me = p.turn as usize;
    let mut out = Vec::new();
    for (u, wp) in &row.3 {
        let Some(m) = uci_to_move(&mut p, u) else { continue };
        p.make(m);
        let q = wp.clamp(0.01, 0.99);
        out.push((points(&p, me), (q / (1.0 - q)).ln()));
        p.unmake();
    }
    out
}

/// Goldennet as a value: its points summed after a move give the logit of
/// Stockfish's win probability for the mover (squared error, streamed),
/// epochs chosen on validate (move agreement), frozen; then searched deeper.
pub fn teach_value(n_train: usize, epochs: usize, max_depth: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let (tr, va, ho) = read_values(n_train, 5_000, 3_000)?;
    let mut out = format!("Goldennet taught the table's voice as a value (Stockfish's win probability after each move): {} train / {} validate / {} holdout positions ({:.0} s to read)\n", tr.len(), va.len(), ho.len(), t0.elapsed().as_secs_f64());
    let agree = |w: &[f32], set: &[Valued]| -> f64 {
        set.iter().filter(|r| {
            let c = valued_children(r);
            let best = c.iter().enumerate().max_by(|a, b| score(w, &a.1 .0).total_cmp(&score(w, &b.1 .0))).map(|x| x.0);
            best.map(|i| r.3[i].0 == r.2).unwrap_or(false)
        }).count() as f64 / set.len().max(1) as f64
    };
    let mut w = vec![0f32; NW];
    let mut best = (0.0, w.clone(), 0);
    for e in 1..=epochs {
        let lr = 0.002 / e as f32;
        let mut se = 0.0f64;
        let mut n = 0usize;
        for r in &tr {
            for (x, t) in valued_children(r) {
                let err = t - score(&w, &x);
                se += (err * err) as f64;
                n += 1;
                for &f in &x {
                    w[f as usize] += lr * err;
                }
            }
        }
        let a = agree(&w, &va);
        out.push_str(&format!("  epoch {e}: train error {:.3}, validate Stockfish's move {:.1}% ({:.0} s)\n", se / n.max(1) as f64, 100.0 * a, t0.elapsed().as_secs_f64()));
        if a > best.0 {
            best = (a, w.clone(), e);
        }
    }
    let w = best.1;
    std::fs::write(GOLDENNET_VALUE_FILE, w.iter().map(|x| format!("{x}\n")).collect::<String>()).map_err(|e| e.to_string())?;
    out.push_str(&format!("  frozen at epoch {} to {GOLDENNET_VALUE_FILE}\n", best.2));
    let (mid, end): (Vec<&Valued>, Vec<&Valued>) = ho.iter().partition(|r| !r.0);
    for d in 1..=max_depth {
        let t1 = std::time::Instant::now();
        let hit = |set: &[&Valued]| set.iter().filter(|r| deep_pick(&w, &r.1, d).as_deref() == Some(r.2.as_str())).count();
        let (hm, he) = (hit(&mid), hit(&end));
        out.push_str(&format!("  holdout, every line to depth {d}: middlegame {:.1}% ({}), endgame {:.1}% ({}); {:.3} s a position\n", 100.0 * hm as f64 / mid.len().max(1) as f64, mid.len(), 100.0 * he as f64 / end.len().max(1) as f64, end.len(), t1.elapsed().as_secs_f64() / ho.len().max(1) as f64));
    }
    Ok(out)
}

// ---------------------------------------------------------------- what a table is

pub const TABLE_QUESTION: &str = "What is a table, and when does one answer?";

pub fn read_table_question(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(TABLE_QUESTION).trim_end_matches(['.', '?', '!'])
}

/// What a key does as a table: of the questions asked, the share whose key
/// was met before (recurrence), and, of those, the share where the key's
/// most frequent answer before is the answer now (determination).
pub struct KeyMeasure {
    pub asked: usize,
    pub recurrence: f64,
    pub determination: f64,
}

/// Measure a key on (position, answer) pairs: `seen` is what the table was
/// built from, `asked` what it is asked; `key` turns a position into its key.
pub fn measure_key(seen: &[(Position, String)], asked: &[(Position, String)], key: &dyn Fn(&Position) -> u64) -> KeyMeasure {
    let mut table: HashMap<u64, HashMap<&str, u32>> = HashMap::new();
    for (p, a) in seen {
        *table.entry(key(p)).or_default().entry(a.as_str()).or_default() += 1;
    }
    let (mut met, mut right) = (0usize, 0usize);
    for (p, a) in asked {
        if let Some(c) = table.get(&key(p)) {
            met += 1;
            let top = c.iter().max_by_key(|x| *x.1).map(|x| *x.0).unwrap_or("");
            right += (top == a) as usize;
        }
    }
    KeyMeasure { asked: asked.len(), recurrence: met as f64 / asked.len().max(1) as f64, determination: right as f64 / met.max(1) as f64 }
}

/// Stockfish's moves in the CCRL games as (position, move), the plies in `plies`, split train / holdout by game.
fn ccrl_pairs(plies: std::ops::Range<usize>, max_games: usize) -> Result<(Vec<(Position, String)>, Vec<(Position, String)>), String> {
    let text = std::fs::read_to_string(GAMES).map_err(|e| format!("{GAMES}: {e}"))?;
    let (mut tr, mut ho) = (Vec::new(), Vec::new());
    for (g, line) in text.lines().take(max_games).enumerate() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let (w, b) = (parts[0].contains('W'), parts[0].contains('B'));
        let mut p = Position::from_fen(START)?;
        for (ply, u) in parts[2].split_whitespace().enumerate() {
            if ply >= plies.end {
                break;
            }
            let Some(m) = uci_to_move(&mut p, u) else { break };
            if plies.contains(&ply) && (if ply % 2 == 0 { w } else { b }) && p.occ.count_ones() > 5 {
                let mut q = Position::from_fen(&p.to_fen())?;
                q.ply = 0;
                let pair = (q, u.to_string());
                if split_of(g) == Split::Holdout { ho.push(pair) } else if split_of(g) == Split::Train { tr.push(pair) }
            }
            p.make(m);
        }
    }
    Ok((tr, ho))
}

/// Golden Answer's answer to TABLE_QUESTION, in its kind.
pub fn answer_table_question(logical: bool, abstract_: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let definition = "a table is a finite map from keys to answers: the key of the question is looked up and its answer read, nothing computed; a key not in it is answered \"not familiar\"".to_string();
    if abstract_ {
        return Ok(("a table answers where its keys come back".into(), vec![definition, "this is an answer that claims nothing".into()], vec![]));
    }
    if !logical {
        return Ok((
            "a table answers exactly where its keys recur; known tables do so by making the questions recur (opening books) or by holding every key (endgame tablebases)".into(),
            vec![definition],
            vec!["opening books (e.g. Polyglot); Syzygy endgame tablebases (de Man): every position of up to seven pieces".into()],
        ));
    }
    let t0 = std::time::Instant::now();
    let whole = |p: &Position| p.key64();
    let mut lines = vec![
        format!("from its own example (the opening table, {TABLE_FILE}): {definition}"),
        "so a table answers a question right exactly when two things hold, both measurable for any key: recurrence (the question's key was met before) and determination (the key's answer before is the answer now); its share right = recurrence x determination".into(),
    ];
    // the whole position as the key, on Stockfish's game moves: openings and later
    for (name, plies) in [("openings (plies 1-20)", 0..20), ("middlegame and endgame (plies 21-80)", 20..80)] {
        let (tr, ho) = ccrl_pairs(plies, 40_000)?;
        let m = measure_key(&tr, &ho, &whole);
        lines.push(format!(
            "the whole position as the key, {name}, Stockfish's moves in CCRL games (40 000 games, train / holdout by game; {} questions): recurrence {:.1}%, determination {:.1}%, so right {:.1}%",
            m.asked,
            100.0 * m.recurrence,
            100.0 * m.determination,
            100.0 * m.recurrence * m.determination
        ));
    }
    lines.push("so: the whole position is a key that recurs only in the opening; past it a table needs keys that are parts of a position — parts that recur from game to game and still determine the move; measure_key measures any such key".into());
    lines.push(format!("({:.0} s)", t0.elapsed().as_secs_f64()));
    Ok(("a table answers where its keys recur and determine the answer: right = recurrence x determination; the whole position recurs only in the opening".into(), lines, vec![]))
}

/// How deep Goldennet gets in `seconds` on one position: every line, the net at every leaf; each depth timed, leaves per second.
pub fn speed(fen: &str, seconds: f64) -> Result<String, String> {
    let w = load_goldennet().ok_or("no Goldennet yet")?;
    let mut out = format!("{fen}\n");
    let mut last = 0.0f64;
    let mut factor = 35.0f64;
    for d in 1..=12 {
        if last * factor > seconds && d > 1 {
            out.push_str(&format!("  depth {d}: not started (about {:.0} s expected, over {seconds:.0} s)\n", last * factor));
            break;
        }
        let t0 = std::time::Instant::now();
        let m = deep_pick(&w, fen, d).unwrap_or_default();
        let s = t0.elapsed().as_secs_f64();
        // leaves: every line to depth d
        let mut p = Position::from_fen(fen)?;
        let leaves = if d <= 5 { count_leaves(&mut p, d) } else { 0 };
        out.push_str(&format!("  depth {d}: {m} in {s:.3} s{}\n", if leaves > 0 { format!(", {leaves} leaves, {:.1} M leaves/s", leaves as f64 / s.max(1e-9) / 1e6) } else { String::new() }));
        if last > 0.0 {
            factor = (s / last).max(1.0);
        }
        last = s;
        if s > seconds {
            break;
        }
    }
    Ok(out)
}

fn count_leaves(p: &mut Position, d: usize) -> u64 {
    if d == 0 {
        return 1;
    }
    let ms = legal_moves(p);
    if ms.is_empty() {
        return 1;
    }
    let mut n = 0;
    for m in ms {
        p.make(m);
        n += count_leaves(p, d - 1);
        p.unmake();
    }
    n
}

/// Where a leaf's time goes: the moves alone (every line to `d`), the points alone, the frozen sum alone.
pub fn profile(fen: &str, d: usize) -> Result<String, String> {
    let w = load_goldennet().ok_or("no Goldennet yet")?;
    let mut p = Position::from_fen(fen)?;
    let t0 = std::time::Instant::now();
    let leaves = count_leaves(&mut p, d);
    let moves_s = t0.elapsed().as_secs_f64();
    let n = 1_000_000;
    let t1 = std::time::Instant::now();
    let mut acc = 0u64;
    for _ in 0..n {
        acc += points(&p, 0)[0] as u64;
    }
    let pts = t1.elapsed().as_secs_f64() / n as f64;
    let x = points(&p, 0);
    let t2 = std::time::Instant::now();
    let mut s = 0f32;
    for _ in 0..n {
        s += score(&w, std::hint::black_box(&x));
    }
    let sum = t2.elapsed().as_secs_f64() / n as f64;
    Ok(format!(
        "one core: the moves of every line to depth {d}: {leaves} leaves in {moves_s:.2} s ({:.0} ns a leaf)\n  the points of one position (content + attack counts of 64 squares): {:.0} ns\n  the frozen sum of 64 points: {:.0} ns\n  ({acc} {s})\n",
        moves_s / leaves as f64 * 1e9,
        pts * 1e9,
        sum * 1e9
    ))
}

/// Whether the side to move has any legal move (stops at the first).
fn has_legal(p: &mut Position) -> bool {
    use crate::fast::movegen::{gen_moves, MAX_MOVES};
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, false);
    for &m in &list[..n] {
        p.make(m);
        let ok = !p.in_check_c(me);
        p.unmake();
        if ok {
            return true;
        }
    }
    false
}
