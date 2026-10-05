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
