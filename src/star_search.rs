//! StarNet's search core (from Steldens' steldens-rs/src/pvs.rs, negamax and
//! qsearch), with the evaluation pluggable: iterative deepening, PVS
//! alpha-beta, a transposition table, check extension, null move (R = 3),
//! move ordering (the table's move, promotions, good captures by victim and
//! attacker, killers, quiet moves into the enemy king's zone, history), late
//! move pruning and reductions, and a capture search that skips losing
//! captures (SEE < 0). Futility and delta pruning are left out: their margins
//! are centipawns, and the evaluations here have their own scales. StarNet's
//! policy beams, jokers and extensions are not part of it.

use crate::fast::movegen::{gen_moves, legal_moves, MAX_MOVES};
use crate::fast::position::{file_of120, m_flag, m_from, m_promo, m_to, move_to_uci, rank_of120, Position, VAL};
use std::time::{Duration, Instant};

const MATE: i32 = 1_000_000_000;
const MATE_ZONE: i32 = MATE - 1_000;
const MAX_PLY: usize = 100;
const INF: i32 = i32::MAX - 1;
const TT_BITS: u32 = 20;

/// An evaluation: the value of `p` for the side to move, in its own units (|v| well below MATE_ZONE).
pub trait Eval: Sync {
    fn eval(&self, p: &Position) -> i32;
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    mv: u32,
    score: i32,
    depth: i8,
    flag: u8,
}

pub struct Searcher<'a> {
    ev: &'a dyn Eval,
    tt: Vec<Entry>,
    killers: [[u32; 2]; MAX_PLY],
    history: Vec<i32>,
    pub nodes: u64,
    deadline: Instant,
    stopped: bool,
}

impl<'a> Searcher<'a> {
    pub fn new(ev: &'a dyn Eval) -> Searcher<'a> {
        Searcher { ev, tt: vec![Entry::default(); 1 << TT_BITS], killers: [[0; 2]; MAX_PLY], history: vec![0; 2 * 120 * 120], nodes: 0, deadline: Instant::now(), stopped: false }
    }
    fn hist(&self, side: u8, m: u32) -> i32 {
        self.history[side as usize * 14400 + m_from(m) as usize * 120 + m_to(m) as usize]
    }
    fn bump(&mut self, side: u8, m: u32, by: i32) {
        let h = &mut self.history[side as usize * 14400 + m_from(m) as usize * 120 + m_to(m) as usize];
        *h = (*h + by).min(1 << 20);
    }
    fn check_time(&mut self) {
        if self.nodes & 2047 == 0 && Instant::now() >= self.deadline {
            self.stopped = true;
        }
    }
    fn probe(&self, key: u64) -> Option<Entry> {
        let e = self.tt[(key >> (64 - TT_BITS)) as usize];
        (e.key == key).then_some(e)
    }
    fn store(&mut self, key: u64, e: Entry) {
        let slot = &mut self.tt[(key >> (64 - TT_BITS)) as usize];
        if slot.key != key || e.depth >= slot.depth {
            *slot = e;
        }
    }
}

fn has_non_pawn_material(p: &Position) -> bool {
    let s = p.turn as usize;
    (1..5).any(|t| p.bb[s * 6 + t] != 0)
}

fn score_moves(s: &Searcher, p: &Position, moves: &[u32], tt_mv: Option<u32>, ply: usize) -> Vec<(u32, i32)> {
    let ek = p.king_sq[1 - p.turn as usize];
    let (ekf, ekr) = (file_of120(ek), rank_of120(ek));
    let killers = s.killers[ply.min(MAX_PLY - 1)];
    moves
        .iter()
        .map(|&m| {
            let to = m_to(m) as usize;
            let king_zone = (file_of120(to) - ekf).abs() <= 2 && (rank_of120(to) - ekr).abs() <= 2;
            let sc = if Some(m) == tt_mv {
                1 << 30
            } else {
                let flag = m_flag(m);
                if m_promo(m) != 0 {
                    (1 << 25) + VAL[m_promo(m) as usize]
                } else if flag == 1 || flag == 2 {
                    let see = crate::fast::see::see(p, m);
                    if see >= 0 {
                        (1 << 24) + VAL[p.board[m_to(m) as usize] as usize] * 16 - VAL[p.board[m_from(m) as usize] as usize]
                    } else {
                        (1 << 21) + see
                    }
                } else if flag == 3 {
                    1 << 23
                } else if killers.contains(&m) {
                    (1 << 22) + if killers[0] == m { 2 } else { 1 }
                } else if king_zone {
                    (1 << 21) + s.hist(p.turn, m).min((1 << 20) - 1)
                } else {
                    s.hist(p.turn, m)
                }
            };
            (m, sc)
        })
        .collect()
}

fn qsearch(s: &mut Searcher, p: &mut Position, mut alpha: i32, beta: i32, ply: usize, qleft: u8) -> i32 {
    s.nodes += 1;
    s.check_time();
    let stand = s.ev.eval(p);
    if s.stopped || ply >= MAX_PLY || stand >= beta || qleft == 0 {
        return stand;
    }
    if stand > alpha {
        alpha = stand;
    }
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, true);
    let mut scored: Vec<u32> = list[..n].to_vec();
    scored.sort_by_key(|&m| -(VAL[p.board[m_to(m) as usize] as usize] * 16 - VAL[p.board[m_from(m) as usize] as usize] + m_promo(m) as i32 * 1000));
    let mut best = stand;
    for m in scored {
        if m_promo(m) == 0 && (m_flag(m) == 1 || m_flag(m) == 2) && crate::fast::see::see(p, m) < 0 {
            continue;
        }
        p.make(m);
        if p.in_check_c(1 - p.turn) {
            p.unmake();
            continue;
        }
        let v = -qsearch(s, p, -beta, -alpha, ply + 1, qleft - 1);
        p.unmake();
        if s.stopped {
            return stand;
        }
        best = best.max(v);
        alpha = alpha.max(v);
        if alpha >= beta {
            break;
        }
    }
    best
}

#[allow(clippy::too_many_arguments)]
fn negamax(s: &mut Searcher, p: &mut Position, mut depth: usize, mut alpha: i32, beta: i32, ply: usize, is_pv: bool) -> i32 {
    s.nodes += 1;
    s.check_time();
    if s.stopped {
        return 0;
    }
    if ply > 0 && (p.halfmove >= 100 || (p.halfmove >= 8 && p.repetition())) {
        return 0;
    }
    let check = p.in_check();
    if check {
        depth += 1;
    }
    if depth == 0 || ply >= MAX_PLY {
        return qsearch(s, p, alpha, beta, ply, 8);
    }
    let key = p.key64();
    let alpha0 = alpha;
    let mut tt_mv = None;
    if let Some(e) = s.probe(key) {
        tt_mv = Some(e.mv);
        if !is_pv && e.depth as usize >= depth {
            let mut sc = e.score;
            if sc > MATE_ZONE {
                sc -= ply as i32;
            } else if sc < -MATE_ZONE {
                sc += ply as i32;
            }
            match e.flag {
                0 => return sc,
                1 if sc >= beta => return sc,
                2 if sc <= alpha => return sc,
                _ => {}
            }
        }
    }
    if !is_pv && !check && depth >= 3 && beta < MATE_ZONE && has_non_pawn_material(p) {
        p.make_null();
        let v = -negamax(s, p, depth.saturating_sub(4), -beta, -beta + 1, ply + 1, false);
        p.unmake_null();
        if s.stopped {
            return 0;
        }
        if v >= beta {
            return beta;
        }
    }
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, false);
    let mut scored = score_moves(s, p, &list[..n], tt_mv, ply);
    scored.sort_by_key(|&(_, sc)| -sc);
    let (mut best, mut best_mv, mut legal) = (-INF, 0u32, 0usize);
    for &(m, _) in &scored {
        let capture = m_flag(m) == 1 || m_flag(m) == 2;
        let promo = m_promo(m) != 0;
        p.make(m);
        if p.in_check_c(1 - p.turn) {
            p.unmake();
            continue;
        }
        legal += 1;
        let gives_check = p.in_check();
        let quiet = !capture && !promo;
        // late move pruning: the quiet tail at low depth
        if !is_pv && !check && depth <= 4 && quiet && !gives_check && legal > 4 + depth * depth {
            p.unmake();
            continue;
        }
        let mut v;
        if legal == 1 {
            v = -negamax(s, p, depth - 1, -beta, -alpha, ply + 1, is_pv);
        } else {
            let red = if depth >= 3 && legal > 4 && quiet && !gives_check { 1 + usize::from(depth >= 6 && legal > 12) } else { 0 };
            v = -negamax(s, p, depth.saturating_sub(1 + red), -alpha - 1, -alpha, ply + 1, false);
            if v > alpha && red > 0 {
                v = -negamax(s, p, depth - 1, -alpha - 1, -alpha, ply + 1, false);
            }
            if v > alpha && v < beta {
                v = -negamax(s, p, depth - 1, -beta, -alpha, ply + 1, true);
            }
        }
        p.unmake();
        if s.stopped {
            return 0;
        }
        if v > best {
            best = v;
            best_mv = m;
        }
        if v > alpha {
            alpha = v;
            if alpha >= beta {
                if quiet {
                    let k = &mut s.killers[ply.min(MAX_PLY - 1)];
                    if k[0] != m {
                        k[1] = k[0];
                        k[0] = m;
                    }
                    let side = p.turn;
                    s.bump(side, m, 1 << depth.min(20));
                }
                break;
            }
        }
    }
    if legal == 0 {
        return if check { -(MATE - ply as i32) } else { 0 };
    }
    let flag = if best >= beta { 1 } else if best > alpha0 { 0 } else { 2 };
    let stored = if best > MATE_ZONE { best + ply as i32 } else if best < -MATE_ZONE { best - ply as i32 } else { best };
    s.store(key, Entry { key, mv: best_mv, score: stored, depth: depth as i8, flag });
    best
}

/// The move in `ms` milliseconds: iterative deepening, the last whole depth stands. (move, depth, nodes)
pub fn best_move(ev: &dyn Eval, fen: &str, ms: u64) -> Option<(String, usize, u64)> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut s = Searcher::new(ev);
    s.deadline = Instant::now() + Duration::from_millis(ms);
    let roots = legal_moves(&mut p);
    if roots.is_empty() {
        return None;
    }
    let mut best = (roots[0], 0usize);
    for depth in 1..60 {
        let v = negamax(&mut s, &mut p, depth, -INF, INF, 0, true);
        if s.stopped {
            break;
        }
        if let Some(e) = s.probe(p.key64()) {
            if e.mv != 0 {
                best = (e.mv, depth);
            }
        }
        if v.abs() > MATE_ZONE {
            break;
        }
    }
    Some((move_to_uci(best.0), best.1, s.nodes))
}

// ---------------------------------------------------------------- the evaluations to test

/// The Golden Function (the net we designed, out/golden/grow_function.txt), for the side to move.
pub struct GoldenFunctionEval(pub crate::golden_grow::Function);
impl Eval for GoldenFunctionEval {
    fn eval(&self, p: &Position) -> i32 {
        let v = -crate::golden_grow::eval_from(&self.0, p, 1 - p.turn as usize);
        (v * 1000.0).clamp(-1.0e8, 1.0e8) as i32
    }
}

/// Goldennet (a grid net file), for the side to move: minus its value for the side that has just moved.
pub struct GoldennetEval(pub Vec<f32>);
impl Eval for GoldennetEval {
    fn eval(&self, p: &Position) -> i32 {
        let x = crate::golden_table::points(p, 1 - p.turn as usize);
        let v: f32 = x.iter().map(|&i| self.0[i as usize]).sum();
        (-v * 1000.0).clamp(-1.0e8, 1.0e8) as i32
    }
}

/// StarNet's search with each evaluation on the first `n` positions of the fixed check set: Stockfish's move, depth.
pub fn test(n: usize, ms: u64) -> Result<String, String> {
    let text = std::fs::read_to_string("out/golden/grow_check.tsv").map_err(|e| format!("out/golden/grow_check.tsv: {e}"))?;
    let rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).take(n).collect();
    let mut out = format!("StarNet's search, {ms} ms a position, on {} positions of the fixed check set (Stockfish's move known):\n", rows.len());
    let mut evals: Vec<(&str, Box<dyn Eval>)> = vec![("Golden Function (the net we designed)", Box::new(GoldenFunctionEval(crate::golden_grow::Function::load())))];
    let read_net = |path: &str| -> Option<Vec<f32>> {
        let w: Vec<f32> = std::fs::read_to_string(path).ok()?.lines().filter_map(|l| l.trim().parse().ok()).collect();
        (w.len() == crate::golden_table::NW).then_some(w)
    };
    if let Some(w) = read_net(crate::golden_table::GOLDENNET_VALUE_FILE) {
        evals.push(("Goldennet as a value", Box::new(GoldennetEval(w))));
    }
    if let Some(w) = read_net(crate::golden_table::GOLDENNET_FILE) {
        evals.push(("Goldennet as a choice", Box::new(GoldennetEval(w))));
    }
    for (name, ev) in &evals {
        let t0 = Instant::now();
        let (mut hit, mut depth, mut nodes) = (0, 0usize, 0u64);
        for (fen, sf) in &rows {
            if let Some((m, d, k)) = best_move(ev.as_ref(), fen, ms) {
                hit += (m == *sf) as usize;
                depth += d;
                nodes += k;
            }
        }
        let k = rows.len().max(1);
        out.push_str(&format!(
            "  {name}: Stockfish's move {hit}/{k} ({:.0}%), average depth {:.1}, {:.0}k nodes a position ({:.0} s)\n",
            100.0 * hit as f64 / k as f64,
            depth as f64 / k as f64,
            nodes as f64 / k as f64 / 1e3,
            t0.elapsed().as_secs_f64()
        ));
    }
    Ok(out)
}

/// A smoke test on positions nothing has seen: the held-out bucket of the
/// Hugging Face data past the fixed check set's first CHECK positions; each
/// position's result printed as soon as it is done.
pub fn smoke(n: usize, ms: u64, skip: usize) -> Result<(), String> {
    use std::io::Write;
    let w: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    if w.len() != crate::golden_table::NW {
        return Err("no Goldennet value file".into());
    }
    let ev = GoldennetEval(w);
    let text = std::fs::read_to_string(crate::golden_table::HF_FILE).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    let mut seen = 0;
    for line in text.lines() {
        let mut it = line.splitn(3, '\t');
        let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        let Ok(p) = Position::from_fen(fen) else { continue };
        if p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 == 19 {
            seen += 1;
            if seen > skip {
                rows.push((ph.to_string(), fen.to_string(), mv.to_string()));
                if rows.len() >= n {
                    break;
                }
            }
        }
    }
    println!("smoke test: {} positions never seen, Goldennet (Texel) with StarNet's search, {ms} ms each", rows.len());
    let _ = std::io::stdout().flush();
    let mut hit = 0;
    for (i, (ph, fen, sf)) in rows.iter().enumerate() {
        let t0 = Instant::now();
        let (m, d, k) = best_move(&ev, fen, ms).unwrap_or_default();
        hit += (m == *sf) as usize;
        println!("  {}/{} {ph}: played {m}, Stockfish {sf} {} (depth {d}, {:.1}M nodes, {:.1} s) — {hit}/{} so far\n    {fen}", i + 1, rows.len(), if m == *sf { "RIGHT" } else { "miss" }, k as f64 / 1e6, t0.elapsed().as_secs_f64(), i + 1);
        let _ = std::io::stdout().flush();
    }
    println!("smoke test: Stockfish's move {hit}/{}", rows.len());
    Ok(())
}
