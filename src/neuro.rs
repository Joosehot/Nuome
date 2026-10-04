//! NEURO: a chess position written as a network of numbers, by the rules
//! alone (nothing learned). 64 nodes; an edge from a piece's node to every
//! node the rules let it reach, typed: a move (empty node), a capture (an
//! enemy there), a guard (a friend there). From the edges, the numbers the
//! golden function reads: each side's edges by type, each node's in-degree
//! from each side, the kings' regions (the nodes a king can walk through
//! without being struck), the pawns' paths and the shortest walks between
//! what matters (king to king, kings to the queening nodes).
//!
//! GOLDEN then solves the puzzle the network poses: which legal move leaves
//! the network best for the mover. The function over the network is the
//! supergenius's: a piece is worth its edges (its reach now, with what the
//! rules give it on an open board), a pawn is a queen in waiting, a move is
//! one unit, the enemy king's room counts against, and a mating move wins.

use crate::golden::{rule_mobility, Board, Mv};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
static DEADLINE_MS: AtomicU64 = AtomicU64::new(0);

/// Stop every calculation after `ms` milliseconds from now (0 = no time limit).
pub fn set_deadline(ms: u64) {
    let start = START.get_or_init(Instant::now);
    DEADLINE_MS.store(if ms == 0 { 0 } else { start.elapsed().as_millis() as u64 + ms }, Ordering::Relaxed);
}

fn time_is_up() -> bool {
    let d = DEADLINE_MS.load(Ordering::Relaxed);
    d != 0 && START.get().map_or(false, |s| s.elapsed().as_millis() as u64 > d)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Move,
    Capture,
    Guard,
}

pub struct Edge {
    pub from: u8,
    pub to: u8,
    pub kind: Kind,
    /// true = the side to move
    pub mine: bool,
}

pub struct Network {
    /// edges of each side by kind: [side][move, capture, guard]
    pub counts: [[i64; 3]; 2],
    /// the pieces' worth (open-board reach and reach now), mine minus theirs, pawns as queens in waiting
    pub pieces_worth: i64,
    /// the most the side to move takes with one capture
    pub best_capture: i64,
    /// my pieces struck and unguarded, all but the largest (the side to move answers one)
    pub hanging_rest: i64,
    /// in-degree of every node from each side: [mine, theirs]
    pub struck: [[u8; 64]; 2],
    /// nodes each king can walk through without being struck, king included
    pub region: [u64; 2],
    /// pieces on the board by node (the side to move positive)
    pub piece: [i8; 64],
    pub my_turn_white: bool,
}

const NOT_A: u64 = 0xfefe_fefe_fefe_fefe;
const NOT_H: u64 = 0x7f7f_7f7f_7f7f_7f7f;

fn grow(s: u64) -> u64 {
    let h = s | ((s << 1) & NOT_A) | ((s >> 1) & NOT_H);
    h | (h << 8) | (h >> 8)
}

/// King steps from set `a` to set `b` through `allowed` nodes (64 if unreachable).
pub fn walk(a: u64, b: u64, allowed: u64) -> i64 {
    if a == 0 || b == 0 {
        return 64;
    }
    let mut r = a;
    for k in 0..64 {
        if r & b != 0 {
            return k;
        }
        let n = r | (grow(r) & allowed);
        if n == r {
            return 64;
        }
        r = n;
    }
    64
}

impl Network {
    /// The network of `b`, seen from the side to move, written with the
    /// bitboards: every piece's edges are a table lookup, every count a
    /// population count.
    pub fn write(b: &Board) -> Network {
        let t = crate::bitboard::tables();
        let w = b.white;
        let mut piece = [0i8; 64];
        let (mut occ, mut mine, mut theirs) = (0u64, 0u64, 0u64);
        for n in 0..64 {
            let p = b.sq[n];
            if p == 0 {
                continue;
            }
            let my = if w { p > 0 } else { p < 0 };
            piece[n] = if my { p.abs() } else { -p.abs() };
            occ |= 1 << n;
            if my {
                mine |= 1 << n;
            } else {
                theirs |= 1 << n;
            }
        }
        let empty = !occ;
        let unit = 256i64;
        let open = |k: usize| (rule_mobility(k as i8) * 256.0) as i64;
        let q = open(5);
        let mut counts = [[0i64; 3]; 2];
        let mut struck = [[0u8; 64]; 2];
        let mut hit = [0u64; 2];
        let mut pieces_worth = 0i64;
        let mut best_capture = 0i64;
        let mut hanging: Vec<i64> = Vec::new();
        let victim = |k: i8| -> i64 {
            match k.abs() {
                1 => q / 64,
                6 => 0,
                k => open(k as usize),
            }
        };
        let mut bits = occ;
        while bits != 0 {
            let n = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let p = piece[n];
            let my = p > 0;
            let side = (!my) as usize;
            let (own, other) = if my { (mine, theirs) } else { (theirs, mine) };
            let white_piece = my == w;
            let attacks = match p.abs() {
                1 => {
                    if white_piece {
                        t.att.pawn_w[n]
                    } else {
                        t.att.pawn_b[n]
                    }
                }
                2 => t.att.knight[n],
                3 => t.bishop_attacks(n, occ),
                4 => t.rook_attacks(n, occ),
                5 => t.queen_attacks(n, occ),
                _ => t.att.king[n],
            };
            hit[side] |= attacks;
            let mut a = attacks;
            while a != 0 {
                let m = a.trailing_zeros() as usize;
                a &= a - 1;
                struck[side][m] += 1;
            }
            let moves = if p.abs() == 1 {
                let one = (if white_piece { (1u64 << n) << 8 } else { (1u64 << n) >> 8 }) & empty;
                let start = if white_piece { n / 8 == 1 } else { n / 8 == 6 };
                let two = if one != 0 && start { (if white_piece { one << 8 } else { one >> 8 }) & empty } else { 0 };
                one | two
            } else {
                attacks & empty
            };
            counts[side][0] += moves.count_ones() as i64;
            counts[side][1] += (attacks & other).count_ones() as i64;
            counts[side][2] += (attacks & own).count_ones() as i64;
            let sign = if my { 1 } else { -1 };
            match p.abs() {
                1 => {
                    let steps = if white_piece { 7 - (n / 8) as i64 } else { (n / 8) as i64 };
                    pieces_worth += sign * (q >> steps.clamp(1, 6));
                }
                6 => {}
                k => pieces_worth += sign * (open(k as usize) + attacks.count_ones() as i64 * unit) / 2,
            }
            if my {
                let mut c = attacks & theirs;
                while c != 0 {
                    let m = c.trailing_zeros() as usize;
                    c &= c - 1;
                    best_capture = best_capture.max(victim(piece[m]));
                }
            }
        }
        // my pieces struck by them and not guarded by me
        let mut m = mine;
        while m != 0 {
            let n = m.trailing_zeros() as usize;
            m &= m - 1;
            if piece[n] != 6 && hit[1] >> n & 1 == 1 && hit[0] >> n & 1 == 0 {
                hanging.push(victim(piece[n]));
            }
        }
        hanging.sort_unstable_by(|a, b| b.cmp(a));
        let hanging_rest: i64 = hanging.iter().skip(1).sum();
        // the kings' regions: the nodes a king walks through unstruck by the other side
        let king = |my: bool| piece.iter().position(|&p| p == if my { 6 } else { -6 }).map_or(0u64, |k| 1 << k);
        let region = |k: u64, other_hit: u64| {
            let allowed = !other_hit & (empty | k);
            let mut r = k;
            loop {
                let n = r | (grow(r) & allowed);
                if n == r {
                    return r;
                }
                r = n;
            }
        };
        let region = [region(king(true), hit[1]), region(king(false), hit[0])];
        Network { counts, pieces_worth, best_capture, hanging_rest, struck, region, piece, my_turn_white: w }
    }

    fn count(&self, my: bool, kind: Kind) -> i64 {
        let k = match kind {
            Kind::Move => 0,
            Kind::Capture => 1,
            Kind::Guard => 2,
        };
        self.counts[(!my) as usize][k]
    }

    /// The numbers of the network, named (for the explanation).
    pub fn numbers(&self) -> Vec<(&'static str, i64)> {
        let my = |k| self.count(true, k);
        let th = |k| self.count(false, k);
        let kings: Vec<i64> = [6i8, -6].iter().map(|&k| self.piece.iter().position(|&p| p == k).map_or(0, |n| n as i64)).collect();
        let kd = ((kings[0] % 8 - kings[1] % 8).abs()).max((kings[0] / 8 - kings[1] / 8).abs());
        vec![
            ("my move edges", my(Kind::Move)),
            ("their move edges", th(Kind::Move)),
            ("my capture edges", my(Kind::Capture)),
            ("their capture edges", th(Kind::Capture)),
            ("my guard edges", my(Kind::Guard)),
            ("their guard edges", th(Kind::Guard)),
            ("my king's region", self.region[0].count_ones() as i64),
            ("their king's region", self.region[1].count_ones() as i64),
            ("my pieces they strike", (0..64).filter(|&n| self.piece[n] > 0 && self.piece[n] != 6 && self.struck[1][n] > 0).count() as i64),
            ("their pieces I strike", (0..64).filter(|&n| self.piece[n] < 0 && self.piece[n] != -6 && self.struck[0][n] > 0).count() as i64),
            ("king distance", kd),
        ]
    }

    /// The worth of the network for the side to move, in 1/256 moves: the
    /// supergenius's function over the network's numbers.
    pub fn worth(&self) -> i64 {
        let unit = 256i64;
        self.pieces_worth
            + unit * (self.counts[0][0] - self.counts[1][0])
            + self.best_capture
            + unit / 8 * (self.region[0].count_ones() as i64 - self.region[1].count_ones() as i64)
            - self.hanging_rest
    }
}

const MATE: i64 = 1_000_000_000_000;

/// GOLDEN: the legal move after which the network is best for the mover;
/// a mating move wins, the draw rules give 0 (the rules decide those).
pub fn golden(b: &Board, history: &[u64]) -> Option<(Mv, i64, Vec<(Mv, i64)>)> {
    let mut ranked: Vec<(Mv, i64)> = b
        .moves()
        .into_iter()
        .map(|m| {
            let a = b.play(m);
            let ms = a.moves();
            let v = if ms.is_empty() {
                if a.in_check() { MATE } else { 0 }
            } else if a.half >= 100 || a.insufficient() || history.iter().filter(|h| **h == a.hash()).count() >= 2 {
                0
            } else {
                let n = Network::write(&a);
                -n.worth()
            };
            (m, v)
        })
        .collect();
    if ranked.is_empty() {
        return None;
    }
    ranked.sort_by_key(|x| -x.1);
    Some((ranked[0].0, ranked[0].1, ranked))
}

pub fn report(fen: &str) -> Result<String, String> {
    let b = if fen.trim().is_empty() || fen.trim() == "startpos" { Board::start() } else { Board::from_fen(fen.trim())? };
    let net = Network::write(&b);
    let mut out = format!("NEURO writes {} as a network: {} edges\n", b.fen(), net.counts.iter().flatten().sum::<i64>());
    for (k, v) in net.numbers() {
        out.push_str(&format!("  {k:<24} {v}\n"));
    }
    out.push_str(&format!("  worth for the side to move: {}\n", net.worth() + net.vision_worth(&b)));
    match golden(&b, &[]) {
        Some((m, v, ranked)) => {
            out.push_str("GOLDEN solves the puzzle: the move after which the network is best\n");
            for (mv, val) in ranked.iter().take(5) {
                out.push_str(&format!("  {:<6} {}\n", mv.uci(), if *val == MATE { "mate".into() } else { format!("{val:+}") }));
            }
            out.push_str(&format!("move: {} ({})\n", m.uci(), if v == MATE { "mate".into() } else { format!("{v:+}") }));
        }
        None => out.push_str("no move\n"),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_network_is_balanced_and_has_the_right_edges() {
        let n = Network::write(&Board::start());
        assert_eq!(n.worth(), 0);
        assert_eq!(n.count(true, Kind::Move), 20);
        assert_eq!(n.count(true, Kind::Capture), 0);
    }

    #[test]
    fn golden_mates_and_takes_the_queen() {
        let b = Board::from_fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
        assert_eq!(golden(&b, &[]).unwrap().0.uci(), "a1a8");
        let b = Board::from_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1").unwrap();
        assert_eq!(golden(&b, &[]).unwrap().0.uci(), "d1d5");
    }
}


// ── board vision, written into the network ──

/// Everything the board vision sees, as numbers of the network: per node
/// the strikes of each side and the cheapest striker; the pins; what each
/// piece stands to lose where it stands; safe mobility; the king's zone
/// under strike and its pawn shield; the pawns' structure and the passed
/// pawns with their stop node; the centre; the rooks' files; development
/// and castling; pieces a pawn can kick; and the walks that decide races.
/// Side 0 is the side to move.
pub struct Vision {
    pub cheapest: [[i64; 64]; 2],
    pub pinned: [u64; 2],
    /// what a side's pieces stand to lose, largest first (value, pinned)
    pub losses: [Vec<(i64, bool)>; 2],
    pub safe_mobility: [i64; 2],
    pub zone_struck: [i64; 2],
    pub shield: [i64; 2],
    pub king_struck: [bool; 2],
    pub doubled: [i64; 2],
    pub isolated: [i64; 2],
    /// passed pawns: (steps left, stop node blocked, stop node only they strike, the pawn itself in danger)
    pub passed: [Vec<(i64, bool, bool, bool)>; 2],
    pub centre_pawns_safe: [i64; 2],
    pub centre_struck: [i64; 2],
    pub bishop_pair: [bool; 2],
    pub rooks_open: [i64; 2],
    pub rooks_half: [i64; 2],
    pub rooks_seventh: [i64; 2],
    pub undeveloped: [i64; 2],
    pub castled: [bool; 2],
    pub can_castle: [bool; 2],
    pub knights_on_rim: [i64; 2],
    pub kickable: [i64; 2],
    /// walks: own king to its pawns' stop nodes, enemy king to the queening nodes of my pawns
    pub king_to_own_pawn_front: [i64; 2],
    pub enemy_king_to_my_queening: [i64; 2],
    /// pawns the enemy king cannot catch (the race, with the side to move's tempo)
    pub unstoppable: [i64; 2],
    pub phase: i64,
}

fn rule_worth(t: i8) -> i64 {
    match t.abs() {
        1 => (rule_mobility(5) * 256.0) as i64 / 64,
        6 => 100_000,
        t => (rule_mobility(t) * 256.0) as i64,
    }
}

impl Network {
    /// What a piece of `side` worth `val` on `n` stands to lose there: the
    /// board vision's rule (unstruck 0; struck and unguarded all; else the
    /// difference when a cheaper piece strikes it).
    fn loss_on(&self, v: &Vision, side: usize, n: usize, val: i64) -> i64 {
        let o = 1 - side;
        if self.struck[o][n] == 0 {
            0
        } else if self.struck[side][n] == 0 {
            val
        } else if v.cheapest[o][n] < val {
            val - v.cheapest[o][n]
        } else {
            0
        }
    }

    pub fn vision(&self, b: &Board) -> Vision {
        let w = self.my_turn_white;
        let mut v = Vision {
            cheapest: [[i64::MAX; 64]; 2],
            pinned: [0; 2],
            losses: [Vec::new(), Vec::new()],
            safe_mobility: [0; 2],
            zone_struck: [0; 2],
            shield: [0; 2],
            king_struck: [false; 2],
            doubled: [0; 2],
            isolated: [0; 2],
            passed: [Vec::new(), Vec::new()],
            centre_pawns_safe: [0; 2],
            centre_struck: [0; 2],
            bishop_pair: [false; 2],
            rooks_open: [0; 2],
            rooks_half: [0; 2],
            rooks_seventh: [0; 2],
            undeveloped: [0; 2],
            castled: [false; 2],
            can_castle: [false; 2],
            knights_on_rim: [0; 2],
            kickable: [0; 2],
            king_to_own_pawn_front: [64; 2],
            enemy_king_to_my_queening: [64; 2],
            unstoppable: [0; 2],
            phase: 0,
        };
        // the cheapest striker of every node, per side (from the pieces' strike sets)
        for n in 0..64 {
            let p = self.piece[n];
            if p == 0 {
                continue;
            }
            let side = (p < 0) as usize;
            let val = rule_worth(p);
            let mut s = crate::golden::strikes_of(b, n);
            while s != 0 {
                let t = s.trailing_zeros() as usize;
                s &= s - 1;
                v.cheapest[side][t] = v.cheapest[side][t].min(val);
            }
            v.phase += match p.abs() {
                2 | 3 => 1,
                4 => 2,
                5 => 4,
                _ => 0,
            };
        }
        // the piece sets once, not in every loop
        let mut occ = 0u64;
        let mut sides = [0u64; 2];
        let mut kinds = [[0u64; 7]; 2];
        let mut kings: [Option<usize>; 2] = [None, None];
        for n in 0..64 {
            let p = self.piece[n];
            if p == 0 {
                continue;
            }
            let side = (p < 0) as usize;
            occ |= 1 << n;
            sides[side] |= 1 << n;
            kinds[side][p.unsigned_abs() as usize] |= 1 << n;
            if p.abs() == 6 {
                kings[side] = Some(n);
            }
        }
        let side_bb = |side: usize| sides[side];
        let kind_bb = |side: usize, t: i8| kinds[side][t as usize];
        let king_of = |side: usize| kings[side];
        let opening = v.phase >= 18;
        for side in 0..2 {
            let o = 1 - side;
            let Some(k) = king_of(side) else { continue };
            // pins: a slider of theirs behind exactly one piece of ours on a line to our king
            let (kf, kr) = ((k % 8) as i32, (k / 8) as i32);
            for (dirs, line_kinds) in [([(1, 0), (-1, 0), (0, 1), (0, -1)], [4i8, 5]), ([(1, 1), (1, -1), (-1, 1), (-1, -1)], [3i8, 5])] {
                for (df, dr) in dirs {
                    let (mut x, mut y, mut own, mut own_n) = (kf + df, kr + dr, 0u32, 0usize);
                    while (0..8).contains(&x) && (0..8).contains(&y) {
                        let n = (y * 8 + x) as usize;
                        let p = self.piece[n];
                        if p != 0 {
                            let mine = (p > 0) == (side == 0);
                            if mine {
                                own += 1;
                                own_n = n;
                                if own > 1 {
                                    break;
                                }
                            } else {
                                if own == 1 && line_kinds.contains(&p.abs()) {
                                    v.pinned[side] |= 1 << own_n;
                                }
                                break;
                            }
                        }
                        x += df;
                        y += dr;
                    }
                }
            }
            // losses and safe mobility
            for n in 0..64 {
                let p = self.piece[n];
                if p == 0 || (p > 0) != (side == 0) || p.abs() == 6 {
                    continue;
                }
                let l = self.loss_on(&v, side, n, rule_worth(p));
                if l > 0 {
                    v.losses[side].push((l, v.pinned[side] >> n & 1 == 1));
                }
                if p.abs() != 1 {
                    let mut s = crate::golden::strikes_of(b, n) & !side_bb(side);
                    while s != 0 {
                        let t = s.trailing_zeros() as usize;
                        s &= s - 1;
                        if v.cheapest[o][t] >= rule_worth(p) {
                            v.safe_mobility[side] += 1;
                        }
                    }
                }
            }
            v.losses[side].sort_by(|a, b| b.0.cmp(&a.0));
            // the king
            let zone = grow(1u64 << k) & !(1u64 << k);
            v.zone_struck[side] = (0..64).filter(|&n| zone >> n & 1 == 1 && self.struck[o][n] > 0).count() as i64;
            v.king_struck[side] = self.struck[o][k] > 0;
            let fwd: i32 = if (side == 0) == w { 1 } else { -1 };
            for df in [-1, 0, 1] {
                let (f, r) = (kf + df, kr + fwd);
                if (0..8).contains(&f) && (0..8).contains(&r) && self.piece[(r * 8 + f) as usize] == if side == 0 { 1 } else { -1 } {
                    v.shield[side] += 1;
                }
            }
            let white_side = (side == 0) == w;
            let home = if white_side { 0 } else { 7 };
            v.castled[side] = kr == home && (kf == 6 || kf == 2);
            v.can_castle[side] = b.castle & if white_side { 3 } else { 12 } != 0;
            // pawns
            let pawns = kind_bb(side, 1);
            let opp_pawns = kind_bb(o, 1);
            let files: Vec<i32> = (0..64).filter(|&n| pawns >> n & 1 == 1).map(|n| (n % 8) as i32).collect();
            let ofiles: Vec<i32> = (0..64).filter(|&n| opp_pawns >> n & 1 == 1).map(|n| (n % 8) as i32).collect();
            let mut stop_nodes = 0u64;
            let mut queening = 0u64;
            for n in (0..64).filter(|&n| pawns >> n & 1 == 1) {
                let (f, r) = ((n % 8) as i32, (n / 8) as i32);
                if files.iter().filter(|&&x| x == f).count() > 1 {
                    v.doubled[side] += 1;
                }
                if !files.contains(&(f - 1)) && !files.contains(&(f + 1)) {
                    v.isolated[side] += 1;
                }
                let ahead = |tr: i32| if white_side { tr > r } else { tr < r };
                let passed = !(0..64).any(|t| opp_pawns >> t & 1 == 1 && ((t % 8) as i32 - f).abs() <= 1 && ahead((t / 8) as i32));
                let stop = r + fwd;
                let stop_n = ((stop * 8 + f) as usize).min(63);
                if (0..8).contains(&stop) {
                    stop_nodes |= 1 << stop_n;
                }
                queening |= 1 << (if white_side { 56 + f } else { f } as usize);
                if passed {
                    let steps = if white_side { 7 - r } else { r } as i64;
                    let blocked = (0..8).contains(&stop) && occ >> stop_n & 1 == 1;
                    let contested = (0..8).contains(&stop) && self.struck[o][stop_n] > 0 && self.struck[side][stop_n] == 0;
                    let danger = self.loss_on(&v, side, n, rule_worth(1)) > 0;
                    v.passed[side].push((steps, blocked, contested, danger));
                    // the race: a clear path and the enemy king too far from the queening node
                    if let Some(ek) = king_of(o) {
                        let qn = if white_side { 56 + f } else { f } as usize;
                        let d = ((ek % 8) as i64 - (qn % 8) as i64).abs().max(((ek / 8) as i64 - (qn / 8) as i64).abs());
                        let clear = !blocked && (1..steps).all(|s| self.piece[((r + fwd * s as i32) * 8 + f) as usize] == 0);
                        let steps_eff = steps - if (white_side && r == 1) || (!white_side && r == 6) { 1 } else { 0 };
                        let tempo = if side == 0 { 0 } else { 1 };
                        if clear && d - tempo > steps_eff {
                            v.unstoppable[side] += 1;
                        }
                    }
                }
                if (27..=28).contains(&n) || (35..=36).contains(&n) {
                    if self.loss_on(&v, side, n, rule_worth(1)) == 0 {
                        v.centre_pawns_safe[side] += 1;
                    }
                }
            }
            let empty: u64 = !occ;
            let allowed_k = (0..64).filter(|&n| self.struck[o][n] == 0).fold(0u64, |a, n| a | 1 << n) & (empty | 1 << k);
            v.king_to_own_pawn_front[side] = walk(1 << k, stop_nodes, allowed_k);
            if let Some(ek) = king_of(o) {
                let allowed_e = (0..64).filter(|&n| self.struck[side][n] == 0).fold(0u64, |a, n| a | 1 << n) & (empty | 1 << ek);
                v.enemy_king_to_my_queening[side] = walk(1 << ek, queening, allowed_e);
            }
            v.centre_struck[side] = [27usize, 28, 35, 36].iter().filter(|&&n| self.struck[side][n] > 0).count() as i64;
            v.bishop_pair[side] = kind_bb(side, 3).count_ones() >= 2;
            for n in (0..64).filter(|&n| kind_bb(side, 4) >> n & 1 == 1) {
                let (f, r) = ((n % 8) as i32, (n / 8) as i32);
                if !files.contains(&f) {
                    if ofiles.contains(&f) {
                        v.rooks_half[side] += 1;
                    } else {
                        v.rooks_open[side] += 1;
                    }
                }
                if r == if white_side { 6 } else { 1 } {
                    v.rooks_seventh[side] += 1;
                }
            }
            if opening {
                let minors = kind_bb(side, 2) | kind_bb(side, 3);
                v.undeveloped[side] = (0..64).filter(|&n| minors >> n & 1 == 1 && (n / 8) as i32 == home).count() as i64;
                v.knights_on_rim[side] = (0..64).filter(|&n| kind_bb(side, 2) >> n & 1 == 1 && (n % 8 == 0 || n % 8 == 7)).count() as i64;
                let heavy = minors | kind_bb(side, 4) | kind_bb(side, 5);
                for n in (0..64).filter(|&n| heavy >> n & 1 == 1) {
                    // a pawn of theirs can strike n after one step
                    let (f, r) = ((n % 8) as i32, (n / 8) as i32);
                    let ofwd = -fwd;
                    for df in [-1, 1] {
                        let (af, ar) = (f + df, r - ofwd);
                        if !(0..8).contains(&af) || !(0..8).contains(&ar) || occ >> (ar * 8 + af) & 1 == 1 {
                            continue;
                        }
                        let p1 = ar - ofwd;
                        let start = if white_side { 6 } else { 1 };
                        if (0..8).contains(&p1) && opp_pawns >> (p1 * 8 + af) & 1 == 1 {
                            v.kickable[side] += 1;
                            break;
                        }
                        let p2 = ar - 2 * ofwd;
                        if (0..8).contains(&p2) && p2 == start && opp_pawns >> (p2 * 8 + af) & 1 == 1 && occ >> (p1 * 8 + af) & 1 == 0 {
                            v.kickable[side] += 1;
                            break;
                        }
                    }
                }
            }
        }
        v
    }

    /// The board vision's part of the worth, for the side to move, in 1/256
    /// moves (the vision's centipawns, 1 cp = 2.56 units, each number mine
    /// minus theirs; the side to move may answer one threat).
    pub fn vision_worth(&self, b: &Board) -> i64 {
        let v = self.vision(b);
        let cp = |x: f64| (x * 2.56) as i64;
        let unit = 256i64;
        let mut total = 0i64;
        let opening = v.phase >= 18;
        for side in 0..2 {
            let sg = if side == 0 { 1 } else { -1 };
            let mut pos = 0i64;
            pos += v.safe_mobility[side] * cp(if opening { 3.0 } else { 4.0 });
            let mut losses = v.losses[side].clone();
            if side == 0 {
                if let Some(i) = losses.iter().position(|x| !x.1) {
                    losses.remove(i); // the side to move can answer one threat
                }
            }
            if let Some(first) = losses.first() {
                pos -= first.0 + losses.get(1).map_or(0, |x| x.0 / 2);
            }
            pos -= (v.pinned[side].count_ones() as i64) * cp(15.0);
            pos -= v.zone_struck[side] * cp(12.0) + if opening { (3 - v.shield[side]) * cp(10.0) } else { 0 } + if v.king_struck[side] { cp(20.0) } else { 0 };
            pos -= (v.doubled[side] + v.isolated[side]) * cp(12.0);
            for &(steps, blocked, contested, danger) in &v.passed[side] {
                let adv = 7 - steps;
                let mut bonus = cp(10.0 + (adv * adv) as f64 * 2.0);
                if blocked {
                    bonus = bonus * 3 / 10;
                } else if contested {
                    bonus /= 2;
                }
                if danger {
                    bonus = bonus * 3 / 10;
                }
                pos += bonus;
            }
            // the race and the escort: walks in the network
            pos += v.unstoppable[side] * rule_worth(5);
            pos -= v.king_to_own_pawn_front[side].min(8) * unit / 2;
            pos += v.enemy_king_to_my_queening[side].min(8) * unit / 4;
            pos += v.centre_pawns_safe[side] * cp(18.0) + v.centre_struck[side] * cp(4.0);
            if v.bishop_pair[side] {
                pos += cp(25.0);
            }
            pos += v.rooks_open[side] * cp(18.0) + v.rooks_half[side] * cp(10.0) + v.rooks_seventh[side] * cp(12.0);
            if opening {
                pos -= v.undeveloped[side] * cp(12.0) + v.knights_on_rim[side] * cp(10.0) + v.kickable[side] * cp(20.0);
                if v.castled[side] {
                    pos += cp(40.0);
                } else if !v.can_castle[side] {
                    pos -= cp(35.0);
                }
            }
            total += sg * pos;
        }
        total
    }
}


// ── the supergenius solves the position as a puzzle ──

/// What the calculation proved about a move: a forced win in n plies, a
/// forced draw, a forced loss in n plies, or nothing within the budget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Proof {
    Win(u32),
    Draw,
    Loss(u32),
    Unknown,
}

struct Solver {
    nodes: u64,
    budget: u64,
    history: Vec<u64>,
}

const WIN: i64 = 1_000_000;
const UNKNOWN: i64 = 0;

impl Solver {
    /// Exact value from the side to move: +WIN - ply for a forced mate, 0 for
    /// a forced draw, and UNKNOWN (0, marked) when the budget runs out.
    /// Only the rules decide: mate, stalemate, repetition, 50 moves,
    /// insufficient material. Returns (value, fully proven).
    fn solve(&mut self, b: &Board, depth: u32, mut alpha: i64, beta: i64, ply: u32) -> (i64, bool) {
        self.nodes += 1;
        let ms = b.moves();
        if ms.is_empty() {
            return (if b.in_check() { -WIN + ply as i64 } else { 0 }, true);
        }
        let h = b.hash();
        if b.half >= 100 || b.insufficient() || self.history.iter().filter(|x| **x == h).count() >= 2 {
            return (0, true);
        }
        // a class the supergenius has solved whole: the exact verdict
        if let Some(t) = crate::retro::table(&crate::retro::pieces_of(b)) {
            if let Some(i) = t.index(b) {
                let v = t.val[i];
                if v != crate::retro::UNSET && v != crate::retro::NONE {
                    return (if v > 0 { WIN - ply as i64 - v as i64 } else if v < 0 { -WIN + ply as i64 + (-v as i64 - 1) } else { 0 }, true);
                }
            }
        }
        if depth == 0 || (self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up())) {
            return (UNKNOWN, false);
        }
        // checks and captures first: proofs are found sooner
        let mut ordered: Vec<(i32, Mv)> = ms
            .into_iter()
            .map(|m| {
                let a = b.play(m);
                let key = (a.in_check() as i32) * 2 + (b.sq[m.to as usize] != 0) as i32;
                (-key, m)
            })
            .collect();
        ordered.sort_by_key(|x| x.0);
        self.history.push(h);
        let mut best = -WIN - 1;
        let mut all_proven = true;
        for (_, m) in ordered {
            let (v, proven) = self.solve(&b.play(m), depth - 1, -beta, -alpha, ply + 1);
            let v = -v;
            if !proven {
                all_proven = false;
            }
            if v > best {
                best = v;
            }
            if v > alpha {
                alpha = v;
            }
            if alpha >= beta {
                // a cut: the value is proven at least this good when the cutting line is proven
                self.history.pop();
                return (best, proven || best >= WIN - 1000);
            }
        }
        self.history.pop();
        // a forced mate found is a proof on its own: no other move needs to be known
        (best, all_proven || best >= WIN - 1000)
    }
}

/// The supergenius on a position: every root move calculated to the end
/// within `budget` nodes (deepening), proofs first; unproven moves are
/// judged by the network they leave (NEURO + the board vision).
pub fn supergenius(b: &Board, history: &[u64], budget: u64) -> Option<(Mv, Proof, Vec<(Mv, Proof, i64)>, u64)> {
    let ms = b.moves();
    if ms.is_empty() {
        return None;
    }
    let mut proofs: Vec<Proof> = vec![Proof::Unknown; ms.len()];
    let mut s = Solver { nodes: 0, budget, history: history.to_vec() };
    s.history.push(b.hash());
    let mut depth = 1;
    while s.nodes < budget && !time_is_up() && depth <= 40 {
        for (i, &m) in ms.iter().enumerate() {
            if proofs[i] != Proof::Unknown {
                continue;
            }
            let a = b.play(m);
            let (v, proven) = s.solve(&a, depth, -WIN - 1, WIN + 1, 1);
            let v = -v;
            if proven {
                proofs[i] = if v >= WIN - 100 { Proof::Win((WIN - v) as u32) } else if v <= -WIN + 100 { Proof::Loss((v + WIN) as u32) } else { Proof::Draw };
            }
            if s.nodes > budget {
                break;
            }
        }
        if proofs.iter().all(|p| *p != Proof::Unknown) {
            break;
        }
        depth += 1;
    }
    // the network's judgement for what is not proven
    // no function: an unproven move carries no number (0); proofs decide, then the rules' order
    let worth = |_m: Mv| 0i64;
    // proofs first; an unproven move stands above a proven draw only when the network sees it ahead
    let rank = |p: Proof, w: i64| match p {
        Proof::Win(n) => (4i64, -(n as i64)),
        Proof::Unknown if w >= 0 => (3, w),
        Proof::Draw => (2, 0),
        Proof::Unknown => (1, w),
        Proof::Loss(n) => (0, n as i64),
    };
    let mut rows: Vec<(Mv, Proof, i64)> = ms.iter().enumerate().map(|(i, &m)| (m, proofs[i], worth(m))).collect();
    rows.sort_by(|x, y| rank(y.1, y.2).cmp(&rank(x.1, x.2)));
    let nodes = s.nodes;
    Some((rows[0].0, rows[0].1, rows, nodes))
}

pub fn show_proof(p: Proof) -> String {
    match p {
        Proof::Win(n) => format!("proved: mate in {}", (n + 1) / 2),
        Proof::Draw => "proved: draw".into(),
        Proof::Loss(n) => format!("proved: mated in {}", (n + 1) / 2),
        Proof::Unknown => "not proved: judged by the network".into(),
    }
}


// ── proofs over every reply: winning, equal, losing ──

/// The horizon's verdict on a position, for the side to move, from the
/// rules alone: the pieces' worth is their reach on an open board (a pawn a
/// queen in waiting); ahead by at least a knight is winning, behind by that
/// is losing, else equal. Mate and the draw rules decide before any horizon.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Winning,
    Equal,
    Losing,
}

fn material(b: &Board) -> i64 {
    let worth = |p: i8, n: usize| match p.abs() {
        1 => {
            let steps = if p > 0 { 7 - (n / 8) as i64 } else { (n / 8) as i64 };
            ((rule_mobility(5) * 256.0) as i64) >> steps.clamp(1, 6)
        }
        6 => 0,
        t => (rule_mobility(t) * 256.0) as i64,
    };
    let mut v = 0;
    for n in 0..64 {
        let p = b.sq[n];
        if p != 0 {
            let w = worth(p, n);
            v += if (p > 0) == b.white { w } else { -w };
        }
    }
    v
}

pub fn horizon(b: &Board) -> Class {
    let knight = (rule_mobility(2) * 256.0) as i64;
    let m = material(b);
    if m >= knight {
        Class::Winning
    } else if m <= -knight {
        Class::Losing
    } else {
        Class::Equal
    }
}

/// Plies the calculation goes by the pieces on the board: 10 with a full
/// board, deeper as it empties (the exact tables take over at 3 pieces).
pub fn depth_for(b: &Board) -> u32 {
    match b.sq.iter().filter(|&&p| p != 0).count() {
        0..=5 => 24,
        6..=11 => 16,
        12..=19 => 12,
        _ => 10,
    }
}

struct Prover {
    nodes: u64,
    budget: u64,
    history: Vec<u64>,
}

impl Prover {
    /// The rules' verdict on `b` if they give one now: mate (-), stalemate
    /// / draw rules (0), a solved class (its sign), else None.
    fn verdict(&self, b: &Board, ms: &[Mv]) -> Option<Class> {
        if ms.is_empty() {
            return Some(if b.in_check() { Class::Losing } else { Class::Equal });
        }
        if b.half >= 100 || b.insufficient() || self.history.iter().filter(|h| **h == b.hash()).count() >= 2 {
            return Some(Class::Equal);
        }
        if let Some(t) = crate::retro::table(&crate::retro::pieces_of(b)) {
            if let Some(i) = t.index(b) {
                let v = t.val[i];
                if v != crate::retro::UNSET && v != crate::retro::NONE {
                    return Some(if v > 0 { Class::Winning } else if v < 0 { Class::Losing } else { Class::Equal });
                }
            }
        }
        None
    }

    /// Can the side to move reach at least `goal` whatever the other side
    /// answers, within `depth` plies? A proof over every reply: some move
    /// of ours after which every reply of theirs leaves us a move that
    /// still reaches the goal. None when the budget ran out.
    fn can_reach(&mut self, b: &Board, goal: Class, depth: u32) -> Option<bool> {
        self.nodes += 1;
        if (self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up())) {
            return None;
        }
        let ms = b.moves();
        if let Some(c) = self.verdict(b, &ms) {
            return Some(at_least(c, goal));
        }
        if depth == 0 {
            return self.settled(b, goal, QUIET);
        }
        self.history.push(b.hash());
        let mut undecided = false;
        for m in ordered(b, ms) {
            match self.holds_against_every_reply(&b.play(m), goal, depth - 1) {
                Some(true) => {
                    self.history.pop();
                    return Some(true);
                }
                Some(false) => {}
                None => undecided = true,
            }
        }
        self.history.pop();
        if undecided { None } else { Some(false) }
    }

    /// After our move: does every reply of theirs leave us able to reach the goal?
    fn holds_against_every_reply(&mut self, b: &Board, goal: Class, depth: u32) -> Option<bool> {
        self.nodes += 1;
        if (self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up())) {
            return None;
        }
        let ms = b.moves();
        if let Some(c) = self.verdict(b, &ms) {
            // their verdict, seen from us
            return Some(at_least(opposite(c), goal));
        }
        if depth == 0 {
            return self.settled_reply(b, goal, QUIET);
        }
        self.history.push(b.hash());
        let mut undecided = false;
        for r in ordered(b, ms) {
            match self.can_reach(&b.play(r), goal, depth - 1) {
                Some(false) => {
                    self.history.pop();
                    return Some(false);
                }
                Some(true) => {}
                None => undecided = true,
            }
        }
        self.history.pop();
        if undecided { None } else { Some(true) }
    }
}

/// Plies of captures and promotions finished before a horizon verdict.
const QUIET: u32 = 8;

fn forcing(b: &Board, m: &Mv) -> bool {
    b.sq[m.to as usize] != 0 || m.promo != 0 || (b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep)
}

impl Prover {
    /// At the horizon the forced business is finished first: can the side
    /// to move reach `goal` by standing pat, or by a capture or promotion
    /// that holds against every forcing reply (up to `q` more plies)?
    fn settled(&mut self, b: &Board, goal: Class, q: u32) -> Option<bool> {
        self.nodes += 1;
        if (self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up())) {
            return None;
        }
        let ms = b.moves();
        if let Some(c) = self.verdict(b, &ms) {
            return Some(at_least(c, goal));
        }
        if at_least(horizon(b), goal) {
            return Some(true);
        }
        if q == 0 {
            return Some(false);
        }
        let mut undecided = false;
        for m in ms.into_iter().filter(|m| forcing(b, m)) {
            match self.settled_reply(&b.play(m), goal, q - 1) {
                Some(true) => return Some(true),
                Some(false) => {}
                None => undecided = true,
            }
        }
        if undecided { None } else { Some(false) }
    }

    /// After our move at the horizon: does every reply - declining, or any
    /// capture or promotion - still leave us the goal?
    fn settled_reply(&mut self, b: &Board, goal: Class, q: u32) -> Option<bool> {
        self.nodes += 1;
        if (self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up())) {
            return None;
        }
        let ms = b.moves();
        if let Some(c) = self.verdict(b, &ms) {
            return Some(at_least(opposite(c), goal));
        }
        if !at_least(opposite(horizon(b)), goal) {
            return Some(false);
        }
        if q == 0 {
            return Some(true);
        }
        let mut undecided = false;
        for r in ms.into_iter().filter(|m| forcing(b, m)) {
            match self.settled(&b.play(r), goal, q - 1) {
                Some(false) => return Some(false),
                Some(true) => {}
                None => undecided = true,
            }
        }
        if undecided { None } else { Some(true) }
    }
}

fn opposite(c: Class) -> Class {
    match c {
        Class::Winning => Class::Losing,
        Class::Losing => Class::Winning,
        Class::Equal => Class::Equal,
    }
}

fn at_least(c: Class, goal: Class) -> bool {
    let rank = |c: Class| match c {
        Class::Winning => 2,
        Class::Equal => 1,
        Class::Losing => 0,
    };
    rank(c) >= rank(goal)
}

/// checks and captures first: proofs are found sooner
fn ordered(b: &Board, ms: Vec<Mv>) -> Vec<Mv> {
    let mut v: Vec<(i32, Mv)> = ms
        .into_iter()
        .map(|m| {
            let a = b.play(m);
            (-((a.in_check() as i32) * 2 + (b.sq[m.to as usize] != 0) as i32), m)
        })
        .collect();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| x.1).collect()
}

/// What is proved about a root move: the best class it reaches against
/// every reply (Some), or nothing within the budget (None).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Claim {
    Mate(u32),
    Holds(Class),
    Uncovered,
    Mated(u32),
}

/// The supergenius decides the position: proven mates first (the exact
/// calculation), then for every move the strongest class it holds against
/// every reply to the depth the pieces allow; ties by the rules' order.
pub fn decide(b: &Board, history: &[u64], budget: u64) -> Option<(Mv, Claim, Vec<(Mv, Claim)>, u64)> {
    let (_, _, rows, nodes) = supergenius(b, history, budget / 4)?;
    let depth = depth_for(b);
    let mut p = Prover { nodes: 0, budget: budget - budget / 4, history: history.to_vec() };
    p.history.push(b.hash());
    let mut out: Vec<(Mv, Claim)> = Vec::new();
    for (m, proof, _) in &rows {
        let claim = match proof {
            Proof::Win(n) => Claim::Mate(*n),
            Proof::Loss(n) => Claim::Mated(*n),
            Proof::Draw => Claim::Holds(Class::Equal),
            Proof::Unknown => {
                let a = b.play(*m);
                let mut best = Claim::Uncovered;
                for goal in [Class::Winning, Class::Equal, Class::Losing] {
                    match p.holds_against_every_reply(&a, goal, depth - 1) {
                        Some(true) => {
                            best = Claim::Holds(goal);
                            break;
                        }
                        Some(false) => continue,
                        None => break,
                    }
                }
                best
            }
        };
        out.push((*m, claim));
    }
    let rank = |c: Claim| match c {
        Claim::Mate(n) => (6i64, -(n as i64)),
        Claim::Holds(Class::Winning) => (5, 0),
        Claim::Holds(Class::Equal) => (4, 0),
        Claim::Uncovered => (3, 0),
        Claim::Holds(Class::Losing) => (2, 0),
        Claim::Mated(n) => (1, n as i64),
    };
    // stable: the rules' order breaks ties
    out.sort_by(|x, y| rank(y.1).cmp(&rank(x.1)));
    let nodes = nodes + p.nodes;
    Some((out[0].0, out[0].1, out, nodes))
}

pub fn show_claim(c: Claim) -> String {
    match c {
        Claim::Mate(n) => format!("proved: mate in {}", (n + 1) / 2),
        Claim::Mated(n) => format!("proved: mated in {}", (n + 1) / 2),
        Claim::Holds(Class::Winning) => "proved winning against every reply".into(),
        Claim::Holds(Class::Equal) => "proved at least equal against every reply".into(),
        Claim::Holds(Class::Losing) => "losing: some reply wins against every continuation".into(),
        Claim::Uncovered => "not covered within the budget".into(),
    }
}


// ── the lumberjack ──

/// Every root move grows its own tree, ply by ply, all together. A tree
/// leads somewhere when it proves its move at least equal (or winning, or a
/// mate) against every reply. A move proved losing is felled at once; a
/// tree that leads nowhere after `fell_at` plies is felled too, and the
/// move leaves the list. The rest grow until the budget ends. The move is
/// the best proof left (mate, winning, equal), ties by the rules' order;
/// if every tree falls, the last one felled stays.
pub const FELL_AT: u32 = 20;

pub struct Felled {
    pub mv: Mv,
    pub at_depth: u32,
    pub why: &'static str,
}

pub fn lumberjack(b: &Board, history: &[u64], budget: u64) -> Option<(Mv, Claim, Vec<(Mv, Claim, u32)>, Vec<Felled>, u64)> {
    // a time budget: a quarter of it for the exact mate calculation, the rest for the trees
    let time_ms = std::env::var("GOLDEN_TIME_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let budget = if time_ms > 0 { u64::MAX / 4 } else { budget };
    set_deadline(if time_ms > 0 { time_ms / 4 } else { 0 });
    let ms = b.moves();
    if ms.is_empty() {
        return None;
    }
    // mates first: the exact calculation (a quarter of the budget)
    let (_, _, rows, nodes0) = supergenius(b, history, budget / 4)?;
    let mut standing: Vec<(Mv, Claim, u32)> = Vec::new();
    let mut felled: Vec<Felled> = Vec::new();
    for (m, proof, _) in &rows {
        match proof {
            Proof::Win(n) => standing.push((*m, Claim::Mate(*n), 0)),
            Proof::Draw => standing.push((*m, Claim::Holds(Class::Equal), 0)),
            Proof::Loss(n) => felled.push(Felled { mv: *m, at_depth: 0, why: "mated by force" }),
            Proof::Unknown => standing.push((*m, Claim::Uncovered, 0)),
        }
        let _ = Claim::Mated(0);
    }
    if standing.iter().any(|s| matches!(s.1, Claim::Mate(_))) {
        standing.sort_by_key(|s| match s.1 { Claim::Mate(n) => n as i64, _ => i64::MAX });
        return Some((standing[0].0, standing[0].1, standing, felled, nodes0));
    }
    let mut p = Prover { nodes: 0, budget: budget - budget / 4, history: history.to_vec() };
    p.history.push(b.hash());
    set_deadline(if time_ms > 0 { time_ms - time_ms / 4 } else { 0 });
    let mut depth = 1u32;
    while p.nodes < p.budget && !time_is_up() && standing.len() > 1 && depth <= 64 {
        let mut next: Vec<(Mv, Claim, u32)> = Vec::new();
        let mut out_of_budget = false;
        for (m, claim, _) in standing.iter().copied() {
            if out_of_budget || matches!(claim, Claim::Mate(_)) {
                next.push((m, claim, depth));
                continue;
            }
            let a = b.play(m);
            // the strongest class this tree holds at this depth
            let mut held: Option<Class> = None;
            let mut undecided = false;
            for goal in [Class::Winning, Class::Equal] {
                match p.holds_against_every_reply(&a, goal, depth) {
                    Some(true) => {
                        held = Some(goal);
                        break;
                    }
                    Some(false) => {}
                    None => {
                        undecided = true;
                        break;
                    }
                }
            }
            if p.nodes > p.budget {
                out_of_budget = true;
                next.push((m, claim, depth));
                continue;
            }
            match held {
                Some(c) => next.push((m, Claim::Holds(c), depth)),
                None if undecided => next.push((m, claim, depth)),
                None => {
                    // not even equal against every reply at this depth: losing here
                    if depth >= 4 {
                        felled.push(Felled { mv: m, at_depth: depth, why: "a reply beats every continuation" });
                    } else {
                        next.push((m, Claim::Holds(Class::Losing), depth));
                    }
                }
            }
        }
        standing = next;
        if depth >= FELL_AT {
            let (keep, fall): (Vec<_>, Vec<_>) = standing.iter().copied().partition(|s| matches!(s.1, Claim::Holds(Class::Winning) | Claim::Holds(Class::Equal) | Claim::Mate(_)));
            if !keep.is_empty() {
                for s in fall {
                    felled.push(Felled { mv: s.0, at_depth: depth, why: "led nowhere" });
                }
                standing = keep;
            }
        }
        depth += 1;
    }
    let rank = |c: Claim| match c {
        Claim::Mate(n) => (6i64, -(n as i64)),
        Claim::Holds(Class::Winning) => (5, 0),
        Claim::Holds(Class::Equal) => (4, 0),
        Claim::Uncovered => (3, 0),
        Claim::Holds(Class::Losing) => (2, 0),
        Claim::Mated(n) => (1, n as i64),
    };
    // among the trees left standing in the same class, goldenboy chooses: the
    // supergenius's function over NEURO's network of the position after the move
    let golden = |m: Mv| {
        let a = b.play(m);
        let n = Network::write(&a);
        -n.worth()
    };
    let mut keyed: Vec<((i64, i64), i64, (Mv, Claim, u32))> = standing.iter().map(|s| (rank(s.1), golden(s.0), *s)).collect();
    keyed.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)));
    standing = keyed.into_iter().map(|k| k.2).collect();
    let nodes = nodes0 + p.nodes;
    match standing.first() {
        Some(s) => Some((s.0, s.1, standing.clone(), felled, nodes)),
        None => {
            let last = felled.last()?;
            Some((last.mv, Claim::Uncovered, Vec::new(), felled, nodes))
        }
    }
}


// ── goldenboy looks through every tree first; then the lumberjack fells ──

/// Goldenboy's view of a position after a move: the supergenius's function
/// over NEURO's network, from the mover's side. Mate and the draw rules
/// speak first; a class the supergenius has solved whole gives its verdict.
fn goldenboy_value(b: &Board, ply: u32) -> i64 {
    let ms = b.moves();
    if ms.is_empty() {
        return if b.in_check() { -WIN + ply as i64 } else { 0 };
    }
    if b.half >= 100 || b.insufficient() {
        return 0;
    }
    if let Some(t) = crate::retro::table(&crate::retro::pieces_of(b)) {
        if let Some(i) = t.index(b) {
            let v = t.val[i];
            if v != crate::retro::UNSET && v != crate::retro::NONE {
                return if v > 0 { WIN - ply as i64 - v as i64 } else if v < 0 { -WIN + ply as i64 + (-v as i64 - 1) } else { 0 };
            }
        }
    }
    // nothing decided here: goldenboy asks the supergenius's eval
    let _ = ms;
    crate::supergenius_eval::eval(b)
}

/// One root move's tree as goldenboy sees it: grown full width to `depth`
/// plies, every node valued; what comes back is the value the line holds
/// against every reply (the worst reply, our best answer), and whether the
/// budget ran out.
struct Walker {
    nodes: u64,
    budget: u64,
}

impl Walker {
    fn look(&mut self, b: &Board, depth: u32, ply: u32) -> Option<i64> {
        self.nodes += 1;
        if self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up()) {
            return None;
        }
        let ms = b.moves();
        if depth == 0 || ms.is_empty() || b.half >= 100 || b.insufficient() {
            return Some(goldenboy_value(b, ply));
        }
        // goldenboy sees every move here; the lumberjack lets only the
        // forcing ones (checks, captures, promotions) and the best few quiet
        // ones by its own look grow deeper
        let mut seen: Vec<(i64, bool, Mv)> = Vec::new();
        for m in ms {
            let a = b.play(m);
            let force = forcing(b, &m) || a.in_check();
            seen.push((-goldenboy_value(&a, ply + 1), force, m));
        }
        if depth == 1 {
            return Some(seen.iter().map(|x| x.0).max().unwrap());
        }
        seen.sort_by(|x, y| y.0.cmp(&x.0));
        let mut best = i64::MIN;
        let mut quiet_kept = 0;
        for (shallow, force, m) in seen {
            if !force {
                if quiet_kept >= BEAM {
                    // not grown: its shallow look stands
                    if shallow > best {
                        best = shallow;
                    }
                    continue;
                }
                quiet_kept += 1;
            }
            let v = -self.look(&b.play(m), depth - 1, ply + 1)?;
            if v > best {
                best = v;
            }
        }
        Some(best)
    }
}

/// Quiet moves a node grows deeper (the forcing ones always do).
const BEAM: usize = 6;

pub struct Tree {
    pub mv: Mv,
    /// goldenboy's value of the tree at each depth it reached (mover's side)
    pub seen: Vec<i64>,
    pub felled: Option<(u32, &'static str)>,
}

/// Goldenboy grows every tree together, a ply at a time, and values every
/// node. After each ply the lumberjack fells: a tree whose value fell to a
/// forced mate against us, a tree that is a piece or more behind every other
/// standing tree for three plies running (it leads nowhere), and after
/// FELL_AT plies every tree not among the best. Goldenboy plays the best
/// tree left: a proven mate first, else the highest value at the deepest
/// ply every standing tree reached.
pub fn goldenboy(b: &Board, history: &[u64], budget: u64) -> Option<(Mv, Vec<Tree>, u64, u32)> {
    let time_ms = std::env::var("GOLDEN_TIME_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let budget = if time_ms > 0 { u64::MAX / 4 } else { budget };
    set_deadline(time_ms);
    let ms = b.moves();
    if ms.is_empty() {
        return None;
    }
    let _ = history;
    let mut trees: Vec<Tree> = ms.iter().map(|&m| Tree { mv: m, seen: Vec::new(), felled: None }).collect();
    let mut w = Walker { nodes: 0, budget };
    let knight = (rule_mobility(2) * 256.0) as i64;
    let mut depth = 1u32;
    let mut reached = 0u32;
    'grow: while depth <= 64 {
        let mut values: Vec<Option<i64>> = Vec::new();
        for t in trees.iter() {
            if t.felled.is_some() {
                values.push(None);
                continue;
            }
            // our move is made; the reply side moves at depth - 1
            match w.look(&b.play(t.mv), depth - 1, 1) {
                Some(v) => values.push(Some(-v)),
                None => break 'grow,
            }
        }
        for (t, v) in trees.iter_mut().zip(&values) {
            if let Some(v) = v {
                t.seen.push(*v);
            }
        }
        reached = depth;
        // the lumberjack
        let standing: Vec<usize> = (0..trees.len()).filter(|&i| trees[i].felled.is_none()).collect();
        if standing.len() > 1 {
            let best_now = standing.iter().map(|&i| *trees[i].seen.last().unwrap()).max().unwrap();
            for &i in &standing {
                let s = &trees[i].seen;
                let v = *s.last().unwrap();
                if v <= -WIN + 1000 {
                    trees[i].felled = Some((depth, "mated by force"));
                } else if s.len() >= 3 && s[s.len() - 3..].iter().all(|&x| x <= best_now - knight) && best_now > -WIN + 1000 {
                    trees[i].felled = Some((depth, "a piece behind the best for three plies: leads nowhere"));
                } else if depth >= 4 && standing.len() > 3 && s[s.len() - 2..].iter().all(|&x| x <= best_now - knight / 2) && best_now > -WIN + 1000 {
                    trees[i].felled = Some((depth, "half a piece behind the best for two plies: felled"));
                } else if depth >= FELL_AT && v < best_now {
                    trees[i].felled = Some((depth, "not among the best after 20 plies"));
                }
            }
            // never fell the last one
            if trees.iter().all(|t| t.felled.is_some()) {
                let i = standing.iter().copied().max_by_key(|&i| *trees[i].seen.last().unwrap()).unwrap();
                trees[i].felled = None;
            }
        }
        if trees.iter().filter(|t| t.felled.is_none()).count() <= 1 {
            break;
        }
        depth += 1;
    }
    // goldenboy plays: the best standing tree at the deepest ply they all reached
    let standing: Vec<&Tree> = trees.iter().filter(|t| t.felled.is_none()).collect();
    let pick: &Tree = standing.iter().copied().max_by_key(|t| *t.seen.last().unwrap_or(&i64::MIN)).or_else(|| trees.iter().max_by_key(|t| *t.seen.last().unwrap_or(&i64::MIN)))?;
    let mv = pick.mv;
    let nodes = w.nodes;
    Some((mv, trees, nodes, reached))
}

pub fn show_tree_value(v: i64) -> String {
    if v >= WIN - 1000 {
        format!("mate in {}", (WIN - v + 1) / 2)
    } else if v <= -WIN + 1000 {
        format!("mated in {}", (v + WIN + 1) / 2)
    } else {
        format!("{v:+}")
    }
}

#[cfg(test)]
mod speed {
    use super::*;

    #[test]
    fn where_the_time_goes_per_node() {
        let b = Board::from_fen("r2q1rk1/5p2/p3p1pQ/1p2N3/2pPb3/2P2PR1/PP4PP/R5K1 b - - 0 1").unwrap();
        let n = 2000;
        let t = std::time::Instant::now();
        for _ in 0..n {
            std::hint::black_box(b.moves());
        }
        let moves_us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        let t = std::time::Instant::now();
        for _ in 0..n {
            std::hint::black_box(Network::write(&b).worth());
        }
        let net_us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let net = Network::write(&b);
            std::hint::black_box(net.vision_worth(&b));
        }
        let vis_us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        let t = std::time::Instant::now();
        for _ in 0..n {
            std::hint::black_box(crate::retro::pieces_of(&b));
        }
        let pieces_us = t.elapsed().as_secs_f64() * 1e6 / n as f64;
        eprintln!("per node: moves {moves_us:.1} us, network+worth {net_us:.1} us, network+vision {vis_us:.1} us, pieces_of {pieces_us:.1} us");
    }
}


// ── depth: alpha-beta, the lossless lumberjack ──

/// Goldenboy's deep search. The same search as looking through every tree,
/// minus the branches that cannot change the choice (alpha-beta): the
/// result is identical, the depth far greater. Deepening ply by ply inside
/// the time; the best move of the last ply tried first; captures first by
/// their victim; positions seen once are remembered (Zobrist key). At the
/// horizon the captures and promotions are finished before the supergenius's
/// eval speaks. Mates and the solved classes are exact.
struct Deep {
    nodes: u64,
    budget: u64,
    history: Vec<u64>,
    seen: HashMap<u64, (u32, i64, u8, Option<Mv>)>,
    stopped: bool,
}

const QUIET_PLIES: u32 = 8;

impl Deep {
    fn out(&mut self) -> bool {
        if self.stopped {
            return true;
        }
        if self.nodes > self.budget || (self.nodes % 256 == 0 && time_is_up()) {
            self.stopped = true;
        }
        self.stopped
    }

    /// Mate, the draw rules, a solved class: the rules' verdict, if any.
    fn verdict(&self, b: &Board, ms: &[Mv], ply: u32) -> Option<i64> {
        if ms.is_empty() {
            return Some(if b.in_check() { -WIN + ply as i64 } else { 0 });
        }
        if b.half >= 100 || b.insufficient() || self.history.iter().filter(|h| **h == b.hash()).count() >= 2 {
            return Some(0);
        }
        if let Some(t) = crate::retro::table(&crate::retro::pieces_of(b)) {
            if let Some(i) = t.index(b) {
                let v = t.val[i];
                if v != crate::retro::UNSET && v != crate::retro::NONE {
                    return Some(if v > 0 { WIN - ply as i64 - v as i64 } else if v < 0 { -WIN + ply as i64 + (-v as i64 - 1) } else { 0 });
                }
            }
        }
        None
    }

    fn order(b: &Board, ms: Vec<Mv>, first: Option<Mv>) -> Vec<Mv> {
        let victim = |m: &Mv| match b.sq[m.to as usize].abs() {
            0 => if m.promo != 0 { 9 } else { 0 },
            t => t as i32 * 10 - b.sq[m.from as usize].abs() as i32,
        };
        let mut v: Vec<(i32, Mv)> = ms.into_iter().map(|m| (if Some(m) == first { 1000 } else { victim(&m) }, m)).collect();
        v.sort_by_key(|x| -x.0);
        v.into_iter().map(|x| x.1).collect()
    }

    /// The horizon: stand pat on the eval, or finish a capture / promotion.
    fn settle(&mut self, b: &Board, mut alpha: i64, beta: i64, ply: u32, q: u32) -> i64 {
        self.nodes += 1;
        if self.out() {
            return 0;
        }
        let ms = b.moves();
        if let Some(v) = self.verdict(b, &ms, ply) {
            return v;
        }
        let stand = crate::supergenius_eval::eval(b);
        if q == 0 || stand >= beta {
            return stand;
        }
        if stand > alpha {
            alpha = stand;
        }
        let forcing: Vec<Mv> = ms.into_iter().filter(|m| forcing(b, m)).collect();
        for m in Self::order(b, forcing, None) {
            let v = -self.settle(&b.play(m), -beta, -alpha, ply + 1, q - 1);
            if v >= beta {
                return v;
            }
            if v > alpha {
                alpha = v;
            }
        }
        alpha
    }

    fn search(&mut self, b: &Board, depth: u32, mut alpha: i64, beta: i64, ply: u32) -> i64 {
        self.nodes += 1;
        if self.out() {
            return 0;
        }
        let ms = b.moves();
        if let Some(v) = self.verdict(b, &ms, ply) {
            return v;
        }
        if depth == 0 {
            return self.settle(b, alpha, beta, ply, QUIET_PLIES);
        }
        let key = b.hash();
        let mut first = None;
        if let Some(&(d, v, kind, m)) = self.seen.get(&key) {
            first = m;
            if d >= depth && (kind == 0 || (kind == 1 && v >= beta) || (kind == 2 && v <= alpha)) {
                return v;
            }
        }
        let alpha0 = alpha;
        self.history.push(key);
        let mut best = -WIN - 1;
        let mut best_move = None;
        for m in Self::order(b, ms, first) {
            let v = -self.search(&b.play(m), depth - 1, -beta, -alpha, ply + 1);
            if self.stopped {
                self.history.pop();
                return 0;
            }
            if v > best {
                best = v;
                best_move = Some(m);
            }
            if v > alpha {
                alpha = v;
            }
            if alpha >= beta {
                break;
            }
        }
        self.history.pop();
        let kind = if best <= alpha0 { 2 } else if best >= beta { 1 } else { 0 };
        self.seen.insert(key, (depth, best, kind, best_move));
        best
    }
}

/// Goldenboy with depth: iterative deepening inside the time, every root
/// move's value at the deepest complete ply.
pub fn deep(b: &Board, history: &[u64], budget: u64) -> Option<(Mv, i64, Vec<(Mv, i64)>, u64, u32)> {
    let time_ms = std::env::var("GOLDEN_TIME_MS").ok().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let budget = if time_ms > 0 { u64::MAX / 4 } else { budget };
    set_deadline(time_ms);
    let ms = b.moves();
    if ms.is_empty() {
        return None;
    }
    let mut d = Deep { nodes: 0, budget, history: history.to_vec(), seen: HashMap::new(), stopped: false };
    d.history.push(b.hash());
    let mut values: Vec<(Mv, i64)> = ms.iter().map(|&m| (m, 0)).collect();
    let mut reached = 0;
    for depth in 1..=64 {
        let mut this: Vec<(Mv, i64)> = Vec::new();
        let mut alpha = -WIN - 1;
        // the last ply's best first
        let order: Vec<Mv> = values.iter().map(|x| x.0).collect();
        for m in order {
            let v = -d.search(&b.play(m), depth - 1, -WIN - 1, -alpha, 1);
            if d.stopped {
                break;
            }
            this.push((m, v));
            if v > alpha {
                alpha = v;
            }
        }
        if d.stopped {
            break;
        }
        this.sort_by(|x, y| y.1.cmp(&x.1));
        values = this;
        reached = depth;
        if values[0].1 >= WIN - 1000 {
            break;
        }
    }
    let nodes = d.nodes;
    Some((values[0].0, values[0].1, values, nodes, reached))
}
