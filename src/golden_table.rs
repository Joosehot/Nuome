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
                let h = Heard { key: p.key64(), fen: p.to_fen(), sf: u.to_string() };
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
            let h = Heard { key, fen, sf: tied[0].clone() };
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

/// The second report: every position's reference is Stockfish's move at a
/// fixed depth (tools/golden_table_sf.py), one per position. The table is
/// written from the train positions' references, the grid net learns from
/// them (epochs by validate) and is frozen; holdout: the familiar positions
/// (the table) and the unfamiliar ones (the net; the table itself answers
/// "not familiar" there).
pub fn report_sf(max_epochs: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(SF_LABELS).map_err(|e| format!("{SF_LABELS}: {e}"))?;
    let label: HashMap<u64, String> = text.lines().filter_map(|l| l.split_once('\t')).filter_map(|(k, m)| Some((k.parse().ok()?, m.trim().to_string()))).collect();
    let [train, validate, holdout] = hear(usize::MAX)?;
    let relabel = |set: Vec<Heard>| -> Vec<Heard> { set.into_iter().filter_map(|h| label.get(&h.key).map(|m| Heard { key: h.key, fen: h.fen, sf: m.clone() })).collect() };
    let (train, validate, holdout) = (relabel(train), relabel(validate), relabel(holdout));
    let mut out = format!("Golden Answer's table, openings, reference = Stockfish's move at a fixed depth ({} positions analysed): train {} / validate {} / holdout {} occurrences\n", label.len(), train.len(), validate.len(), holdout.len());
    // the net learns from distinct train positions (each once)
    let mut seen = std::collections::HashSet::new();
    let tr: Vec<_> = train.iter().filter(|h| seen.insert(h.key)).filter_map(choices).collect();
    let mut seen_v = std::collections::HashSet::new();
    let va: Vec<_> = validate.iter().filter(|h| seen_v.insert(h.key)).filter_map(choices).collect();
    let mut w = vec![0f32; NW];
    let mut best = (net_agreement(&w, &va), w.clone(), 0);
    for e in 1..=max_epochs {
        epoch(&mut w, &tr, 0.05 / e as f32);
        let a = net_agreement(&w, &va);
        out.push_str(&format!("  grid net epoch {e}: Stockfish's move {:.1}% of the distinct validate positions\n", 100.0 * a));
        if a > best.0 {
            best = (a, w.clone(), e);
        }
    }
    let w = best.1;
    out.push_str(&format!("  frozen at epoch {} ({} distinct train positions; {:.0} s)\n", best.2, tr.len(), t0.elapsed().as_secs_f64()));
    let table = write_table(&train, &w);
    // holdout, by occurrence and by distinct position
    let mut stats = [[0usize; 4]; 2]; // [occurrence, distinct] x [all, familiar, table hits, net hits on unfamiliar]
    let mut unfamiliar = [0usize; 2];
    let mut seen_h = std::collections::HashSet::new();
    for h in &holdout {
        let first = seen_h.insert(h.key);
        let Some((xs, c, _)) = choices(h) else { continue };
        for (k, count) in [(0usize, true), (1, first)] {
            if !count {
                continue;
            }
            stats[k][0] += 1;
            match table.answer(h.key) {
                Some(m) => {
                    stats[k][1] += 1;
                    stats[k][2] += (*m == h.sf) as usize;
                }
                None => {
                    unfamiliar[k] += 1;
                    stats[k][3] += (net_pick(&w, &xs) == c) as usize;
                }
            }
        }
    }
    for (k, name) in [(0, "occurrences in the games"), (1, "distinct positions")] {
        let [all, fam, hits, net] = stats[k];
        let pc = |a: usize, b: usize| 100.0 * a as f64 / b.max(1) as f64;
        out.push_str(&format!(
            "holdout, {name} ({all}):\n  familiar {:.1}%; the table's move = Stockfish's in {:.1}% of them (the same analysed position, so by construction)\n  unfamiliar {:.1}% (the table says \"not familiar\"); the frozen grid net would give Stockfish's move in {:.1}% of them\n  the table alone: {:.1}% of all; with the net where the table is not familiar: {:.1}% of all\n",
            pc(fam, all),
            pc(hits, fam),
            pc(unfamiliar[k], all),
            pc(net, unfamiliar[k]),
            pc(hits, all),
            pc(hits + net, all)
        ));
    }
    out.push_str(&format!("({:.0} s)\n", t0.elapsed().as_secs_f64()));
    Ok(out)
}
