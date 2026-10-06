//! StarNet's search core (from Steldens' steldens-rs/src/pvs.rs, negamax and
//! qsearch), with the evaluation pluggable: iterative deepening, PVS
//! alpha-beta, a transposition table, check extension, null move (R = 3),
//! move ordering (the table's move, promotions, good captures by victim and
//! attacker, killers, the countermove, quiet moves into the enemy king's zone,
//! history plus, where the tree is still deep, the evaluation's gain from a
//! silent move), late
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
    /// centipawns per unit of this evaluation (to blend it with StarNNUE)
    fn cp(&self) -> f32 {
        1.0
    }
}

/// StarNNUE inside the search, updated move by move: a frame per ply with the
/// board, the kings and both perspectives' accumulators; `mix` is StarNNUE's
/// share of the value (the rest is the evaluation's, in centipawns).
pub struct Nn<'n> {
    pub net: &'n crate::fast::starnnue::Net,
    pub mix: f32,
    frames: Vec<(Box<[u8; 120]>, [usize; 2], Vec<f32>, Vec<f32>)>,
    top: usize,
}

fn kings(p: &Position) -> [usize; 2] {
    use crate::fast::bitboard::SQ120_TO_64;
    [SQ120_TO_64[p.king_sq[0]] as usize, SQ120_TO_64[p.king_sq[1]] as usize]
}

impl<'n> Nn<'n> {
    pub fn new(net: &'n crate::fast::starnnue::Net, mix: f32) -> Nn<'n> {
        let h = net.h;
        Nn { net, mix, frames: (0..MAX_PLY + 4).map(|_| (Box::new([0u8; 120]), [0usize; 2], vec![0f32; h], vec![0f32; h])).collect(), top: 0 }
    }
    fn root(&mut self, p: &Position) {
        let k = kings(p);
        let f = &mut self.frames[0];
        f.0.copy_from_slice(&p.board);
        f.1 = k;
        self.net.refresh(p, 0, k, &mut f.2);
        self.net.refresh(p, 1, k, &mut f.3);
        self.top = 0;
    }
    /// After a move (or a null move): the child's frame from its parent's, only the changed squares (a king move refreshes its side).
    fn push(&mut self, p: &Position) {
        use crate::fast::bitboard::SQ64_TO_120;
        let (lo, hi) = self.frames.split_at_mut(self.top + 1);
        let (parent, child) = (&lo[self.top], &mut hi[0]);
        let k = kings(p);
        child.0.copy_from_slice(&p.board);
        child.1 = k;
        for c in 0..2 {
            let acc = if c == 0 { &mut child.2 } else { &mut child.3 };
            if k[c] != parent.1[c] {
                self.net.refresh(p, c, k, acc);
                continue;
            }
            acc.copy_from_slice(if c == 0 { &parent.2 } else { &parent.3 });
            for s in 0..64 {
                let sq = SQ64_TO_120[s];
                let (a, b) = (parent.0[sq], p.board[sq]);
                if a != b {
                    if a != 0 {
                        self.net.update(acc, c, k, a, s, -1.0);
                    }
                    if b != 0 {
                        self.net.update(acc, c, k, b, s, 1.0);
                    }
                }
            }
        }
        self.top += 1;
    }
    fn pop(&mut self) {
        self.top -= 1;
    }
    fn cp(&self, p: &Position) -> f32 {
        let f = &self.frames[self.top];
        if p.turn == 0 { self.net.forward(&f.2, &f.3) } else { self.net.forward(&f.3, &f.2) }
    }
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
    nn: Option<Nn<'a>>,
    tt: Vec<Entry>,
    killers: [[u32; 2]; MAX_PLY],
    /// the move that last refuted each move (by its from and to squares)
    counter: Vec<u32>,
    /// the move made at each ply of the current line (0 for a null move)
    line: [u32; MAX_PLY + 1],
    history: Vec<i32>,
    pub nodes: u64,
    deadline: Instant,
    stopped: bool,
    /// root moves left out (to find the next best)
    exclude: Vec<u32>,
}

impl<'a> Searcher<'a> {
    /// The value of the position for the side to move: the evaluation, or its blend with StarNNUE.
    fn value(&self, p: &Position) -> i32 {
        match &self.nn {
            None => self.ev.eval(p),
            Some(nn) => {
                let own = if nn.mix < 1.0 { self.ev.eval(p) as f32 * self.ev.cp() } else { 0.0 };
                (nn.mix * nn.cp(p) + (1.0 - nn.mix) * own).clamp(-1.0e8, 1.0e8) as i32
            }
        }
    }
    fn nn_push(&mut self, p: &Position) {
        if let Some(nn) = self.nn.as_mut() {
            nn.push(p);
        }
    }
    fn nn_pop(&mut self) {
        if let Some(nn) = self.nn.as_mut() {
            nn.pop();
        }
    }
    pub fn new(ev: &'a dyn Eval) -> Searcher<'a> {
        Searcher { ev, nn: None, tt: vec![Entry::default(); 1 << TT_BITS], killers: [[0; 2]; MAX_PLY], counter: vec![0; 120 * 120], line: [0; MAX_PLY + 1], history: vec![0; 2 * 120 * 120], nodes: 0, deadline: Instant::now(), stopped: false, exclude: Vec::new() }
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

fn score_moves(s: &Searcher, p: &mut Position, moves: &[u32], tt_mv: Option<u32>, ply: usize, depth: usize) -> Vec<(u32, i32)> {
    let ek = p.king_sq[1 - p.turn as usize];
    let (ekf, ekr) = (file_of120(ek), rank_of120(ek));
    let killers = s.killers[ply.min(MAX_PLY - 1)];
    let prev = if ply > 0 { s.line[ply - 1] } else { 0 };
    let counter = if prev != 0 { s.counter[m_from(prev) as usize * 120 + m_to(prev) as usize] } else { 0 };
    // the silent moves judged by the net where the tree is still deep: its gain from the move
    let here = if depth >= 3 { Some(s.ev.eval(p)) } else { None };
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
                } else if m == counter {
                    1 << 22
                } else {
                    let gain = match here {
                        Some(h) => {
                            p.make(m);
                            let after = -s.ev.eval(p);
                            p.unmake();
                            ((after - h) as i64).clamp(-(1 << 18), 1 << 18) as i32
                        }
                        None => 0,
                    };
                    let base = s.hist(p.turn, m).min((1 << 19) - 1) + gain;
                    if king_zone {
                        (1 << 21) + base.max(0)
                    } else {
                        base
                    }
                }
            };
            (m, sc)
        })
        .collect()
}

fn qsearch(s: &mut Searcher, p: &mut Position, mut alpha: i32, beta: i32, ply: usize, qleft: u8) -> i32 {
    s.nodes += 1;
    s.check_time();
    let stand = s.value(p);
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
        s.nn_push(p);
        let v = -qsearch(s, p, -beta, -alpha, ply + 1, qleft - 1);
        s.nn_pop();
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
        s.line[ply] = 0;
        s.nn_push(p);
        let v = -negamax(s, p, depth.saturating_sub(4), -beta, -beta + 1, ply + 1, false);
        s.nn_pop();
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
    let mut scored = score_moves(s, p, &list[..n], tt_mv, ply, depth);
    scored.sort_by_key(|&(_, sc)| -sc);
    let (mut best, mut best_mv, mut legal) = (-INF, 0u32, 0usize);
    for &(m, _) in &scored {
        if ply == 0 && s.exclude.contains(&m) {
            continue;
        }
        let capture = m_flag(m) == 1 || m_flag(m) == 2;
        let promo = m_promo(m) != 0;
        p.make(m);
        if p.in_check_c(1 - p.turn) {
            p.unmake();
            continue;
        }
        legal += 1;
        s.line[ply] = m;
        s.nn_push(p);
        let gives_check = p.in_check();
        let quiet = !capture && !promo;
        // late move pruning: the quiet tail at low depth
        if !is_pv && !check && depth <= 3 && quiet && !gives_check && legal > 6 + 2 * depth * depth {
            s.nn_pop();
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
        s.nn_pop();
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
                    if ply > 0 && s.line[ply - 1] != 0 {
                        let prev = s.line[ply - 1];
                        s.counter[m_from(prev) as usize * 120 + m_to(prev) as usize] = m;
                    }
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
    best_move_with(ev, None, fen, ms)
}

/// The same, with StarNNUE blended in at share `mix` when `net` is given.
pub fn best_move_with<'a>(ev: &'a dyn Eval, net: Option<(&'a crate::fast::starnnue::Net, f32)>, fen: &str, ms: u64) -> Option<(String, usize, u64)> {
    best_move_excluding(ev, net, fen, ms, &[])
}

/// The best `k` moves: the search again and again, each time without the moves already found; `ms` shared out.
pub fn best_moves<'a>(ev: &'a dyn Eval, net: Option<(&'a crate::fast::starnnue::Net, f32)>, fen: &str, ms: u64, k: usize) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut ex: Vec<u32> = Vec::new();
    let Ok(mut p) = Position::from_fen(fen) else { return found };
    let n = legal_moves(&mut p).len();
    for _ in 0..k.min(n) {
        match best_move_excluding(ev, net, fen, ms / k as u64, &ex) {
            Some((m, _, _)) if !found.contains(&m) => {
                if let Some(mv) = crate::fast::movegen::uci_to_move(&mut p, &m) {
                    ex.push(mv);
                }
                found.push(m);
            }
            _ => break,
        }
    }
    found
}

fn best_move_excluding<'a>(ev: &'a dyn Eval, net: Option<(&'a crate::fast::starnnue::Net, f32)>, fen: &str, ms: u64, exclude: &[u32]) -> Option<(String, usize, u64)> {
    search_root(ev, net, fen, ms, exclude).map(|r| (r.0, r.1, r.2))
}

/// The best move and its value for the side to move (the last whole depth).
pub fn analyse<'a>(ev: &'a dyn Eval, net: Option<(&'a crate::fast::starnnue::Net, f32)>, fen: &str, ms: u64) -> Option<(String, i32)> {
    search_root(ev, net, fen, ms, &[]).map(|r| (r.0, r.3))
}

/// The root search with an evaluation alone: (move, depth, nodes, value from the mover).
pub fn search_root_pub(ev: &dyn Eval, fen: &str, ms: u64) -> Option<(String, usize, u64, i32)> {
    search_root(ev, None, fen, ms, &[])
}

fn search_root<'a>(ev: &'a dyn Eval, net: Option<(&'a crate::fast::starnnue::Net, f32)>, fen: &str, ms: u64, exclude: &[u32]) -> Option<(String, usize, u64, i32)> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut s = Searcher::new(ev);
    s.exclude = exclude.to_vec();
    if let Some((n, mix)) = net {
        let mut nn = Nn::new(n, mix);
        nn.root(&p);
        s.nn = Some(nn);
    }
    s.deadline = Instant::now() + Duration::from_millis(ms);
    let roots = legal_moves(&mut p);
    if roots.is_empty() {
        return None;
    }
    let roots: Vec<u32> = roots.into_iter().filter(|m| !exclude.contains(m)).collect();
    if roots.is_empty() {
        return None;
    }
    let mut best = (roots[0], 0usize);
    let mut value = 0i32;
    for depth in 1..60 {
        let v = negamax(&mut s, &mut p, depth, -INF, INF, 0, true);
        if s.stopped {
            break;
        }
        value = v;
        if let Some(e) = s.probe(p.key64()) {
            if e.mv != 0 {
                best = (e.mv, depth);
            }
        }
        if v.abs() > MATE_ZONE {
            break;
        }
    }
    Some((move_to_uci(best.0), best.1, s.nodes, value))
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
/// The judge as an engine: the theory's state rules, weighted, as the
/// evaluation inside the search (pawns x 100 = cp), from the side to move.
pub struct TheoryEval {
    /// the middlegame weights (state index, weight)
    pub weights: Vec<(usize, f32)>,
    /// the endgame weights; empty = not tapered, `weights` alone
    pub endgame: Vec<(usize, f32)>,
    pub bias: f32,
    /// cp per unit of the fitted value: 100 / the material weight, so a pawn is 100 cp
    pub scale: f32,
}

impl Eval for TheoryEval {
    /// From the side to move. The value was fitted from the mover's side (the
    /// side not to move), so its bias enters with the opposite sign.
    fn eval(&self, p: &Position) -> i32 {
        let v = if self.endgame.is_empty() {
            crate::golden_theory::state_score(&self.weights, p, p.turn as usize)
        } else {
            crate::golden_theory::tapered_score(&self.weights, &self.endgame, p, p.turn as usize)
        };
        ((v - self.bias) * self.scale).clamp(-1.0e8, 1.0e8) as i32
    }
}

/// Several evaluations added in cp, each with a weight (the judge's theory and Goldennet together; StarNNUE joins by the search's mix).
pub struct BlendEval(pub Vec<(Box<dyn Eval>, f32)>);

impl Eval for BlendEval {
    fn eval(&self, p: &Position) -> i32 {
        self.0.iter().map(|(e, w)| w * e.eval(p) as f32 * e.cp()).sum::<f32>().clamp(-1.0e8, 1.0e8) as i32
    }
}

/// The rules alone: material by the classic values (1, 3, 3, 5, 9 pawns, 100 cp
/// each), from the side to move; everything else (captures, checks, mates) the
/// search knows from the rules of the game. No theory, no learned weight.
pub struct RulesEval;

impl Eval for RulesEval {
    fn eval(&self, p: &Position) -> i32 {
        let me = p.turn as usize;
        let v: i32 = [100, 300, 300, 500, 900].iter().enumerate().map(|(t, w)| w * (p.bb[me * 6 + t].count_ones() as i32 - p.bb[(1 - me) * 6 + t].count_ones() as i32)).sum();
        v
    }
}

impl Eval for GoldennetEval {
    fn eval(&self, p: &Position) -> i32 {
        let x = crate::golden_table::points(p, 1 - p.turn as usize);
        let v: f32 = x.iter().map(|&i| self.0[i as usize]).sum();
        (-v * 1000.0).clamp(-1.0e8, 1.0e8) as i32
    }
    fn cp(&self) -> f32 {
        0.2
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

/// The same test on a file of (fen TAB Stockfish's move), in the file's order, each reported at once.
pub fn smoke_file(path: &str, ms: u64) -> Result<(), String> {
    let w: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    smoke_file_with(&GoldennetEval(w), path, ms, false)
}

/// The same with any evaluation alone in the search; `reverse` reads the file backwards (the smoke set's order).
pub fn smoke_file_with(ev: &dyn Eval, path: &str, ms: u64, reverse: bool) -> Result<(), String> {
    smoke_file_mix(ev, None, path, ms, reverse)
}

/// The same with StarNNUE mixed in (`mix` = StarNNUE's share).
pub fn smoke_file_mix(ev: &dyn Eval, mix: Option<f32>, path: &str, ms: u64, reverse: bool) -> Result<(), String> {
    let net = if mix.is_some() { Some(crate::fast::starnnue::net()?) } else { None };
    use std::io::Write;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    if reverse {
        rows.reverse();
    }
    let mut hit = 0;
    for (i, (fen, sf)) in rows.iter().enumerate() {
        let t0 = Instant::now();
        let (m, d, k) = match (net, mix) { (Some(n), Some(x)) => best_move_with(ev, Some((n, x)), fen, ms), _ => best_move(ev, fen, ms) }.unwrap_or_default();
        hit += (m == *sf) as usize;
        println!("  {}/{}: played {m}, Stockfish {sf} {} (depth {d}, {:.1}M nodes, {:.1} s) — {hit}/{} so far\n    {fen}", i + 1, rows.len(), if m == *sf { "RIGHT" } else { "miss" }, k as f64 / 1e6, t0.elapsed().as_secs_f64(), i + 1);
        let _ = std::io::stdout().flush();
    }
    Ok(())
}

/// Ordinary game positions (CCRL holdout games, plies 20-80, Stockfish's move in the game) with StarNet's search, each net file compared.
pub fn quiet_test(n: usize, ms: u64, files: &[&str]) -> Result<(), String> {
    use std::io::Write;
    let rows = crate::golden_table::ccrl_quiet_positions(n)?;
    println!("ordinary game positions: {} from CCRL holdout games (plies 20-80), Stockfish's move in the game, {ms} ms each", rows.len());
    for f in files {
        let w: Vec<f32> = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?.lines().filter_map(|l| l.trim().parse().ok()).collect();
        if w.len() != crate::golden_table::NW {
            println!("  {f}: not a Goldennet file");
            continue;
        }
        let ev = GoldennetEval(w);
        let t0 = Instant::now();
        let (mut hit, mut depth) = (0, 0);
        for (fen, sf) in &rows {
            if let Some((m, d, _)) = best_move(&ev, fen, ms) {
                hit += (m == *sf) as usize;
                depth += d;
            }
        }
        println!("  {f}: Stockfish's move {hit}/{} ({:.0}%), average depth {:.1} ({:.0} s)", rows.len(), 100.0 * hit as f64 / rows.len().max(1) as f64, depth as f64 / rows.len().max(1) as f64, t0.elapsed().as_secs_f64());
        let _ = std::io::stdout().flush();
    }
    Ok(())
}

/// Goldennet with the best ChessEngine eval (StarNNUE, nnue1024_sib) blended in: on the
/// smoke positions (each reported at once) and the check set, StarNNUE's share `mixes`.
pub fn blend_test(mixes: &[f32], ms: u64, n_check: usize) -> Result<(), String> {
    use std::io::Write;
    let net = crate::fast::starnnue::net()?;
    let w: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    let ev = GoldennetEval(w);
    let smoke = std::fs::read_to_string("out/golden/smoke_reverse.tsv").map_err(|e| e.to_string())?;
    let mut smoke: Vec<(String, String)> = smoke.lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.to_string())).collect();
    smoke.reverse();
    let check = std::fs::read_to_string("out/golden/grow_check.tsv").map_err(|e| e.to_string())?;
    let check: Vec<(String, String)> = check.lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.to_string())).take(n_check).collect();
    for &mix in mixes {
        let name = if mix == 0.0 { "Goldennet alone".to_string() } else if mix == 1.0 { "StarNNUE alone (nnue1024_sib)".to_string() } else { format!("StarNNUE {:.0}% + Goldennet {:.0}%", 100.0 * mix, 100.0 * (1.0 - mix)) };
        println!("{name}:");
        let _ = std::io::stdout().flush();
        let mut hit = 0;
        for (i, (fen, sf)) in smoke.iter().enumerate() {
            let (m, d, k) = best_move_with(&ev, if mix > 0.0 { Some((net, mix)) } else { None }, fen, ms).unwrap_or_default();
            hit += (m == *sf) as usize;
            println!("  smoke {}/{}: played {m}, Stockfish {sf} {} (depth {d}, {:.1}M nodes)", i + 1, smoke.len(), if m == *sf { "RIGHT" } else { "miss" }, k as f64 / 1e6);
            let _ = std::io::stdout().flush();
        }
        let mut ch = 0;
        for (fen, sf) in &check {
            if best_move_with(&ev, if mix > 0.0 { Some((net, mix)) } else { None }, fen, 1000).map(|r| r.0 == *sf).unwrap_or(false) {
                ch += 1;
            }
        }
        println!("  {name}: smoke {hit}/{} at {ms} ms; check set {ch}/{} at 1 s", smoke.len(), check.len());
        let _ = std::io::stdout().flush();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_starnnue_matches_from_scratch() {
        let Ok(net) = crate::fast::starnnue::net() else { return };
        for fen in ["rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R2QKB1R w KQ - 0 8"] {
            let mut p = Position::from_fen(fen).unwrap();
            let mut nn = Nn::new(net, 1.0);
            nn.root(&p);
            let mut r = 0x1234_5678u64;
            for _ in 0..100 {
                let ms = legal_moves(&mut p);
                if ms.is_empty() {
                    break;
                }
                r ^= r << 13;
                r ^= r >> 7;
                r ^= r << 17;
                p.make(ms[(r % ms.len() as u64) as usize]);
                nn.push(&p);
                let a = nn.cp(&p);
                let b = net.eval(&p) as f32;
                assert!((a - b).abs() <= 1.0, "{a} vs {b} at {}", p.to_fen());
            }
        }
    }
}
