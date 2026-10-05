//! Golden invents the neural net's tables for every phase of the game.
//!
//! The question (QUESTION): invent tables for the neural net, for each
//! phase of the game (opening, middlegame, endgame of more than five
//! pieces): parts of a position that recur from game to game and help
//! determine the move; the regular table answers when the whole position is
//! already familiar.
//!
//! A part (Golden's invention) is a shape: a few squares at fixed offsets
//! from an anchor square; its key is what stands on those squares (empty or
//! one of the twelve pieces, seen from the mover), at every anchor where the
//! shape fits. Each key has a value in its table; the net's value of a
//! position = the grid points (golden_table::points: content and attacks of
//! every square) + the value of every key of every chosen shape. Learned
//! from Stockfish's moves (a conditional logit over the legal moves).
//!
//! Golden draws CANDIDATES connected shapes of 2-4 squares, tries each with
//! a short training, keeps the KEEP that raise validate the most, then the
//! final net; holdout by phase: the regular table where the position is
//! familiar, the net elsewhere.

use crate::fast::movegen::{legal_moves, uci_to_move};
use crate::fast::position::{move_to_uci, Position};
use crate::golden_table::{points, NW};
use std::collections::HashMap;

pub const QUESTION: &str = "Invent the tables for the neural net, for each phase of the game: parts of a position that recur and help determine the move. Use the regular table when the position is already familiar.";
pub const CANDIDATES: usize = 16;
pub const KEEP: usize = 4;

pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// A shape: offsets (file, rank) from the anchor; the first is (0, 0).
#[derive(Clone, Debug)]
pub struct Shape(pub Vec<(i8, i8)>);

impl Shape {
    fn anchors(&self) -> Vec<usize> {
        (0..64)
            .filter(|&a| {
                let (f, r) = ((a % 8) as i8, (a / 8) as i8);
                self.0.iter().all(|&(df, dr)| (0..8).contains(&(f + df)) && (0..8).contains(&(r + dr)))
            })
            .collect()
    }
    fn size(&self) -> usize {
        64 * 13usize.pow(self.0.len() as u32)
    }
    pub fn text(&self) -> String {
        self.0.iter().map(|(f, r)| format!("({f},{r})")).collect::<Vec<_>>().join(" ")
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// Golden draws connected shapes of 2-4 squares.
fn draw_shapes(n: usize, seed: u64) -> Vec<Shape> {
    let mut r = Rng(seed | 1);
    let steps = [(1i8, 0i8), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];
    let mut out: Vec<Shape> = Vec::new();
    while out.len() < n {
        let k = 2 + (r.next() % 3) as usize;
        let mut cells = vec![(0i8, 0i8)];
        while cells.len() < k {
            let base = cells[(r.next() % cells.len() as u64) as usize];
            let s = steps[(r.next() % 8) as usize];
            let c = (base.0 + s.0, base.1 + s.1);
            if !cells.contains(&c) && c.0.abs() <= 3 && c.1.abs() <= 3 {
                cells.push(c);
            }
        }
        let mut norm = cells.clone();
        norm.sort();
        if !out.iter().any(|o| {
            let mut x = o.0.clone();
            x.sort();
            x == norm
        }) {
            out.push(Shape(cells));
        }
    }
    out
}

/// The features of a position seen from `me`: the grid points, then every key of every shape (offsets after the grid's NW).
fn features(p: &Position, me: usize, shapes: &[(Shape, Vec<usize>, usize)]) -> Vec<u32> {
    let mut f: Vec<u32> = points(p, me).iter().map(|&x| x as u32).collect();
    // contents seen from me: 0 empty, 1..6 mine, 7..12 theirs, squares flipped for black
    let mut c = [0u8; 64];
    for col in 0..2 {
        for ty in 0..6 {
            let mut b = p.bb[col * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let rel = if me == 0 { s } else { s ^ 56 };
                c[rel] = 1 + if col == me { 0 } else { 6 } + ty as u8;
            }
        }
    }
    for (sh, anchors, base) in shapes {
        for &a in anchors {
            let (af, ar) = ((a % 8) as i8, (a / 8) as i8);
            let mut code = 0usize;
            for &(df, dr) in &sh.0 {
                code = code * 13 + c[((ar + dr) * 8 + af + df) as usize] as usize;
            }
            f.push((base + a * 13usize.pow(sh.0.len() as u32) + code) as u32);
        }
    }
    f
}

/// One position: the features after every legal move, Stockfish's move's index.
fn choices(fen: &str, sf: &str, shapes: &[(Shape, Vec<usize>, usize)]) -> Option<(Vec<Vec<u32>>, usize)> {
    let mut p = Position::from_fen(fen).ok()?;
    let me = p.turn as usize;
    let mut xs = Vec::new();
    let mut chosen = None;
    for (i, m) in legal_moves(&mut p).into_iter().enumerate() {
        if move_to_uci(m) == sf {
            chosen = Some(i);
        }
        p.make(m);
        xs.push(features(&p, me, shapes));
        p.unmake();
    }
    Some((xs, chosen?))
}

fn score(w: &[f32], x: &[u32]) -> f32 {
    x.iter().map(|&i| w[i as usize]).sum()
}

fn pick(w: &[f32], xs: &[Vec<u32>]) -> usize {
    let mut best = (f32::NEG_INFINITY, 0);
    for (i, x) in xs.iter().enumerate() {
        let s = score(w, x);
        if s > best.0 {
            best = (s, i);
        }
    }
    best.1
}

fn layout(chosen: &[Shape]) -> (Vec<(Shape, Vec<usize>, usize)>, usize) {
    let mut base = NW;
    let mut v = Vec::new();
    for s in chosen {
        v.push((s.clone(), s.anchors(), base));
        base += s.size();
    }
    (v, base)
}

/// Train on `train` for `epochs` (streamed), return the weights.
fn train(set: &[(String, String)], shapes: &[(Shape, Vec<usize>, usize)], n: usize, epochs: usize) -> Vec<f32> {
    let mut w = vec![0f32; n];
    for e in 1..=epochs {
        let lr = 0.05 / e as f32;
        for (fen, sf) in set {
            let Some((xs, c)) = choices(fen, sf, shapes) else { continue };
            let s: Vec<f32> = xs.iter().map(|x| score(&w, x)).collect();
            let mx = s.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let ex: Vec<f32> = s.iter().map(|v| (v - mx).exp()).collect();
            let z: f32 = ex.iter().sum();
            for (i, x) in xs.iter().enumerate() {
                let g = if i == c { 1.0 } else { 0.0 } - ex[i] / z;
                if g.abs() < 1e-4 {
                    continue;
                }
                // the grid's 64 and each shape's keys share the step
                for &f in x {
                    w[f as usize] += lr * g;
                }
            }
        }
    }
    w
}

fn agreement(w: &[f32], set: &[(String, String)], shapes: &[(Shape, Vec<usize>, usize)]) -> f64 {
    let mut n = 0;
    let mut hit = 0;
    for (fen, sf) in set {
        if let Some((xs, c)) = choices(fen, sf, shapes) {
            n += 1;
            hit += (pick(w, &xs) == c) as usize;
        }
    }
    hit as f64 / n.max(1) as f64
}

/// The data of one phase: (train, validate, holdout) as (fen, Stockfish's move), and the regular table's keys (train only).
fn phase_data(phase: &str, n_train: usize) -> Result<(Vec<(String, String)>, Vec<(String, String)>, Vec<(String, String)>), String> {
    let (mut tr, mut va, mut ho) = (Vec::new(), Vec::new(), Vec::new());
    if phase == "opening" {
        let labels = std::fs::read_to_string(crate::golden_table::SF_LABELS).map_err(|e| e.to_string())?;
        let label: HashMap<&str, &str> = labels.lines().filter_map(|l| l.split_once('\t')).collect();
        let fens = std::fs::read_to_string("out/golden/table_fens.txt").map_err(|e| e.to_string())?;
        for l in fens.lines() {
            let f: Vec<&str> = l.splitn(3, '\t').collect();
            if f.len() < 3 {
                continue;
            }
            let Some(m) = label.get(f[1]) else { continue };
            let row = (f[2].to_string(), m.to_string());
            match f[0] {
                "train" if tr.len() < n_train => tr.push(row),
                "validate" if va.len() < 10_000 => va.push(row),
                "holdout" if ho.len() < 20_000 => ho.push(row),
                _ => {}
            }
        }
    } else {
        let text = std::fs::read_to_string(crate::golden_table::HF_FILE).map_err(|e| e.to_string())?;
        for line in text.lines() {
            if tr.len() >= n_train && va.len() >= 10_000 && ho.len() >= 20_000 {
                break;
            }
            let mut it = line.splitn(3, '\t');
            let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
            if ph != phase {
                continue;
            }
            let Ok(p) = Position::from_fen(fen) else { continue };
            let row = (fen.to_string(), mv.to_string());
            match p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 {
                19 if ho.len() < 20_000 => ho.push(row),
                18 if va.len() < 10_000 => va.push(row),
                k if k < 18 && tr.len() < n_train => tr.push(row),
                _ => {}
            }
        }
    }
    Ok((tr, va, ho))
}

/// Golden invents the tables for one phase and reports it.
fn one_phase(phase: &str, n_train: usize, seed: u64, out: &mut String) -> Result<(), String> {
    let t0 = std::time::Instant::now();
    let (tr, va, ho) = phase_data(phase, n_train)?;
    let small: Vec<(String, String)> = tr.iter().take(30_000).cloned().collect();
    let (grid_only, n0) = layout(&[]);
    let w0 = train(&small, &grid_only, n0, 1);
    let base = agreement(&w0, &va, &grid_only);
    out.push_str(&format!("{phase}: {} train / {} validate / {} holdout; the grid alone (short training) {:.1}% on validate\n", tr.len(), va.len(), ho.len(), 100.0 * base));
    // Golden draws shapes and tries each with the grid, a short training
    let cands = draw_shapes(CANDIDATES, seed);
    let mut tried: Vec<(f64, Shape)> = cands
        .into_iter()
        .map(|s| {
            let (l, n) = layout(std::slice::from_ref(&s));
            let w = train(&small, &l, n, 1);
            (agreement(&w, &va, &l), s)
        })
        .collect();
    tried.sort_by(|a, b| b.0.total_cmp(&a.0));
    let chosen: Vec<Shape> = tried.iter().take(KEEP).map(|t| t.1.clone()).collect();
    for (a, s) in tried.iter().take(KEEP) {
        out.push_str(&format!("  kept shape {} : {:.1}% on validate with the grid ({:+.1})\n", s.text(), 100.0 * a, 100.0 * (a - base)));
    }
    // the final net: the grid and the chosen tables
    let (l, n) = layout(&chosen);
    let w = train(&tr, &l, n, 2);
    let va_final = agreement(&w, &va, &l);
    // the fair comparison: the grid alone with the same training
    let wg = train(&tr, &grid_only, n0, 2);
    // holdout: the regular table where familiar (the train positions), the net elsewhere
    let familiar: std::collections::HashSet<u64> = tr.iter().filter_map(|(f, _)| Position::from_fen(f).ok().map(|p| p.key64())).collect();
    let table: HashMap<u64, &str> = tr.iter().filter_map(|(f, m)| Position::from_fen(f).ok().map(|p| (p.key64(), m.as_str()))).collect();
    let (mut fam, mut fam_hit, mut net_n, mut net_hit, mut grid_hit) = (0, 0, 0, 0, 0);
    for (fen, sf) in &ho {
        let Ok(p) = Position::from_fen(fen) else { continue };
        if familiar.contains(&p.key64()) {
            fam += 1;
            fam_hit += (table.get(&p.key64()) == Some(&sf.as_str())) as usize;
        } else if let Some((xs, c)) = choices(fen, sf, &l) {
            net_n += 1;
            net_hit += (pick(&w, &xs) == c) as usize;
            if let Some((gx, gc)) = choices(fen, sf, &grid_only) {
                grid_hit += (pick(&wg, &gx) == gc) as usize;
            }
        }
    }
    let all = (fam + net_n).max(1) as f64;
    out.push_str(&format!(
        "  final net (grid + {} tables, {} weights): validate {:.1}%\n  holdout: familiar {:.1}% (the regular table: {:.1}% right), the rest by the net: {:.1}% right (the grid alone, same training: {:.1}%); all: {:.1}% ({:.0} s)\n",
        chosen.len(),
        n,
        100.0 * va_final,
        100.0 * fam as f64 / all,
        100.0 * fam_hit as f64 / fam.max(1) as f64,
        100.0 * net_hit as f64 / net_n.max(1) as f64,
        100.0 * grid_hit as f64 / net_n.max(1) as f64,
        100.0 * (fam_hit + net_hit) as f64 / all,
        t0.elapsed().as_secs_f64()
    ));
    let _ = uci_to_move;
    Ok(())
}

/// Golden Answer's answer to QUESTION: the tables for every phase.
pub fn answer(n_train: usize) -> Result<(String, Vec<String>, Vec<String>), String> {
    let mut out = String::new();
    for (i, phase) in ["opening", "middlegame", "endgame"].iter().enumerate() {
        one_phase(phase, n_train, 0x7AB1_E000 + i as u64, &mut out)?;
    }
    Ok(("the tables for the net, by phase (below)".into(), out.lines().map(|l| l.to_string()).collect(), vec!["Stockfish's moves: its analysis of the CCRL opening positions (depth 14); Hugging Face agi-noobs/chess-sft-10m-stockfish for the middlegame and endgame".into()]))
}
