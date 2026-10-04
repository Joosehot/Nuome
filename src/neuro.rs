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
    pub edges: Vec<Edge>,
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
    /// The network of `b`, seen from the side to move.
    pub fn write(b: &Board) -> Network {
        let mine = |p: i8| if b.white { p > 0 } else { p < 0 };
        let mut piece = [0i8; 64];
        for n in 0..64 {
            let p = b.sq[n];
            piece[n] = if p == 0 { 0 } else if mine(p) { p.abs() } else { -p.abs() };
        }
        let mut edges = Vec::with_capacity(96);
        let mut struck = [[0u8; 64]; 2];
        let e = crate::golden::attack_edges(b);
        let (me, them) = if b.white { (0, 1) } else { (1, 0) };
        for n in 0..64 {
            struck[0][n] = e[me][n];
            struck[1][n] = e[them][n];
        }
        for from in 0..64usize {
            let p = piece[from];
            if p == 0 {
                continue;
            }
            let my = p > 0;
            let targets = crate::golden::strikes_of(b, from) | if p.abs() == 1 { crate::golden::pawn_pushes(b, from) } else { 0 };
            let mut t = targets;
            while t != 0 {
                let to = t.trailing_zeros() as usize;
                t &= t - 1;
                let q = piece[to];
                let kind = if q == 0 { Kind::Move } else if (q > 0) == my { Kind::Guard } else { Kind::Capture };
                // a pawn's diagonal edge onto an empty node is a strike, not a move
                if p.abs() == 1 && q == 0 && (to % 8) != (from % 8) {
                    continue;
                }
                edges.push(Edge { from: from as u8, to: to as u8, kind, mine: my });
            }
        }
        let kings = |my: bool| piece.iter().position(|&p| p == if my { 6 } else { -6 }).map_or(0u64, |k| 1 << k);
        let empty: u64 = (0..64).filter(|&n| piece[n] == 0).fold(0, |a, n| a | 1 << n);
        let region = |my: bool| {
            let k = kings(my);
            let other = if my { 1 } else { 0 };
            let allowed: u64 = (0..64).filter(|&n| struck[other][n] == 0).fold(0, |a, n| a | 1 << n) & (empty | k);
            let mut r = k;
            loop {
                let n = r | (grow(r) & allowed);
                if n == r {
                    return r;
                }
                r = n;
            }
        };
        Network { edges, struck, region: [region(true), region(false)], piece, my_turn_white: b.white }
    }

    fn count(&self, my: bool, kind: Kind) -> i64 {
        self.edges.iter().filter(|e| e.mine == my && e.kind == kind).count() as i64
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
        let open = |t: usize| (rule_mobility(t as i8) * 256.0) as i64;
        let q = open(5);
        let mut v = 0i64;
        // pieces: their open-board worth and their edges now (reach), pawns as queens in waiting along their path
        let mut edges_of = [0i64; 64];
        for e in &self.edges {
            edges_of[e.from as usize] += 1;
        }
        for n in 0..64 {
            let p = self.piece[n];
            if p == 0 || p.abs() == 6 {
                continue;
            }
            let s = if p > 0 { 1 } else { -1 };
            let t = p.unsigned_abs() as usize;
            let w = if t == 1 {
                let white_pawn = (p > 0) == self.my_turn_white;
                let steps = if white_pawn { 7 - (n / 8) as i64 } else { (n / 8) as i64 };
                q >> steps.clamp(1, 6)
            } else {
                (open(t) + edges_of[n] * unit) / 2
            };
            v += s * w;
        }
        // edges: a move is one unit; a capture is worth what it takes (the victim's worth, in units) - the side to move may take now
        let victim = |n: usize| {
            let t = self.piece[n].unsigned_abs() as usize;
            if t == 1 { q / 64 } else if t == 6 { 0 } else { open(t) }
        };
        v += unit * (self.count(true, Kind::Move) - self.count(false, Kind::Move));
        let best_capture = self.edges.iter().filter(|e| e.mine && e.kind == Kind::Capture).map(|e| victim(e.to as usize)).max().unwrap_or(0);
        v += best_capture;
        // the goal: the enemy king's room counts against, mine for
        v += unit * (self.region[0].count_ones() as i64 - self.region[1].count_ones() as i64);
        // my pieces under strike lose, theirs under my strike gain
        v -= (0..64).filter(|&n| self.piece[n] > 0 && self.piece[n] != 6 && self.struck[1][n] > 0 && self.struck[0][n] == 0).map(victim).sum::<i64>();
        v
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
                -(n.worth() + n.vision_worth(&a))
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
    let mut out = format!("NEURO writes {} as a network: {} edges\n", b.fen(), net.edges.len());
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
        let occ: u64 = (0..64).filter(|&n| self.piece[n] != 0).fold(0, |a, n| a | 1 << n);
        let side_bb = |side: usize| (0..64).filter(|&n| (self.piece[n] > 0) == (side == 0) && self.piece[n] != 0).fold(0u64, |a, n| a | 1 << n);
        let kind_bb = |side: usize, t: i8| (0..64).filter(|&n| self.piece[n] == if side == 0 { t } else { -t }).fold(0u64, |a, n| a | 1 << n);
        let king_of = |side: usize| self.piece.iter().position(|&p| p == if side == 0 { 6 } else { -6 });
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
