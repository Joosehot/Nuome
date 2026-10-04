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
                -Network::write(&a).worth()
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
    out.push_str(&format!("  worth for the side to move: {}\n", net.worth()));
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
