//! The golden function: a chess position in (FEN), the move to play out.
//! It does one thing, try to win, and it knows only the rules of chess.
//!
//! - rules: every legal move (castling through no check, en passant,
//!   the four promotions), mate, stalemate, the 50-move rule, threefold
//!   repetition and insufficient material; proved by perft (the move
//!   counts to fixed depths match the published ones);
//! - atoms: numbers the rules define about a position, seen by the side to
//!   move: legal moves of each side, in check, the kings' escape squares,
//!   enemy pieces attacked, captures available, how many of each piece each
//!   side has, the halfmove clock. No piece values, no centre, no king
//!   safety: nothing we wrote says what is good;
//! - the formula: + - * min max and small whole numbers over the atoms. It
//!   is not written by us: Nuome breeds formulas and keeps the ones that
//!   win games (against each other, a random mover and a "most legal
//!   moves" mover). Game results are the only fitness, and a small price
//!   per node makes the simpler formula win a tie;
//! - the move: a shallow alpha-beta search. Mate is a win, stalemate and
//!   the draw rules are 0 (the rules decide those), and the formula scores
//!   the positions at the horizon.
//!
//! "This function wins from every winnable position" is a claim like
//! Goldbach's: it stands until a game shows it wrong. The same position,
//! formula and depth always give the same move.

use crate::evolve::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mv {
    pub from: u8,
    pub to: u8,
    /// promotion piece type (2..5), 0 if none
    pub promo: i8,
}

impl Mv {
    pub fn uci(&self) -> String {
        let s = |q: u8| format!("{}{}", (b'a' + q % 8) as char, q / 8 + 1);
        let p = match self.promo {
            2 => "n",
            3 => "b",
            4 => "r",
            5 => "q",
            _ => "",
        };
        format!("{}{}{p}", s(self.from), s(self.to))
    }
}

/// A position: pieces 1..6 = P N B R Q K, positive white, negative black; a1 = 0.
#[derive(Clone, PartialEq, Eq)]
pub struct Board {
    pub sq: [i8; 64],
    pub white: bool,
    /// 1 = white O-O, 2 = white O-O-O, 4 = black O-O, 8 = black O-O-O
    pub castle: u8,
    pub ep: Option<u8>,
    pub half: u32,
    pub full: u32,
}

fn file(s: i32) -> i32 {
    s % 8
}
fn rank(s: i32) -> i32 {
    s / 8
}
fn on(f: i32, r: i32) -> Option<i32> {
    ((0..8).contains(&f) && (0..8).contains(&r)).then_some(r * 8 + f)
}

const KNIGHT: [(i32, i32); 8] = [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
const KING: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
const ROOK: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

impl Board {
    pub fn start() -> Board {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(format!("not a FEN: {fen}"));
        }
        let mut sq = [0i8; 64];
        let (mut f, mut r) = (0i32, 7i32);
        for c in parts[0].chars() {
            match c {
                '/' => {
                    r -= 1;
                    f = 0;
                }
                '1'..='8' => f += c as i32 - '0' as i32,
                _ => {
                    let t = match c.to_ascii_lowercase() {
                        'p' => 1,
                        'n' => 2,
                        'b' => 3,
                        'r' => 4,
                        'q' => 5,
                        'k' => 6,
                        _ => return Err(format!("bad piece {c} in {fen}")),
                    };
                    let i = on(f, r).ok_or_else(|| format!("bad board in {fen}"))?;
                    sq[i as usize] = if c.is_ascii_uppercase() { t } else { -t };
                    f += 1;
                }
            }
        }
        let white = parts[1] == "w";
        let mut castle = 0;
        for c in parts.get(2).copied().unwrap_or("-").chars() {
            castle |= match c {
                'K' => 1,
                'Q' => 2,
                'k' => 4,
                'q' => 8,
                _ => 0,
            };
        }
        let ep = parts.get(3).and_then(|e| {
            let b = e.as_bytes();
            (b.len() == 2).then(|| (b[1] - b'1') * 8 + (b[0] - b'a'))
        });
        let half = parts.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);
        let full = parts.get(5).and_then(|x| x.parse().ok()).unwrap_or(1);
        Ok(Board { sq, white, castle, ep, half, full })
    }

    pub fn fen(&self) -> String {
        let mut s = String::new();
        for r in (0..8).rev() {
            let mut empty = 0;
            for f in 0..8 {
                let p = self.sq[(r * 8 + f) as usize];
                if p == 0 {
                    empty += 1;
                    continue;
                }
                if empty > 0 {
                    s.push_str(&empty.to_string());
                    empty = 0;
                }
                let c = b" pnbrqk"[p.unsigned_abs() as usize] as char;
                s.push(if p > 0 { c.to_ascii_uppercase() } else { c });
            }
            if empty > 0 {
                s.push_str(&empty.to_string());
            }
            if r > 0 {
                s.push('/');
            }
        }
        let c: String = [(1, 'K'), (2, 'Q'), (4, 'k'), (8, 'q')].iter().filter(|x| self.castle & x.0 != 0).map(|x| x.1).collect();
        let ep = self.ep.map_or("-".to_string(), |e| format!("{}{}", (b'a' + e % 8) as char, e / 8 + 1));
        format!("{s} {} {} {ep} {} {}", if self.white { "w" } else { "b" }, if c.is_empty() { "-".into() } else { c }, self.half, self.full)
    }

    fn own(&self, p: i8) -> bool {
        if self.white { p > 0 } else { p < 0 }
    }

    /// Is square `s` attacked by the side `white`?
    pub fn attacked(&self, s: i32, white: bool) -> bool {
        let sign = if white { 1 } else { -1 };
        let (f, r) = (file(s), rank(s));
        // pawns attack diagonally forward: a white pawn on (f +- 1, r - 1)
        let pr = if white { r - 1 } else { r + 1 };
        for df in [-1, 1] {
            if let Some(t) = on(f + df, pr) {
                if self.sq[t as usize] == sign {
                    return true;
                }
            }
        }
        for (df, dr) in KNIGHT {
            if let Some(t) = on(f + df, r + dr) {
                if self.sq[t as usize] == 2 * sign {
                    return true;
                }
            }
        }
        for (df, dr) in KING {
            if let Some(t) = on(f + df, r + dr) {
                if self.sq[t as usize] == 6 * sign {
                    return true;
                }
            }
        }
        for (dirs, a, b) in [(ROOK, 4, 5), (BISHOP, 3, 5)] {
            for (df, dr) in dirs {
                let (mut x, mut y) = (f + df, r + dr);
                while let Some(t) = on(x, y) {
                    let p = self.sq[t as usize];
                    if p != 0 {
                        if p == a * sign || p == b * sign {
                            return true;
                        }
                        break;
                    }
                    x += df;
                    y += dr;
                }
            }
        }
        false
    }

    pub fn king(&self, white: bool) -> i32 {
        let k = if white { 6 } else { -6 };
        self.sq.iter().position(|&p| p == k).map_or(-1, |i| i as i32)
    }

    pub fn in_check(&self) -> bool {
        let k = self.king(self.white);
        k >= 0 && self.attacked(k, !self.white)
    }

    /// Moves that obey how the pieces move (the king may still be left in check).
    fn pseudo(&self, captures_only: bool) -> Vec<Mv> {
        let mut out = Vec::with_capacity(48);
        let push = |out: &mut Vec<Mv>, from: i32, to: i32, promo: bool| {
            if promo {
                for p in [5, 4, 3, 2] {
                    out.push(Mv { from: from as u8, to: to as u8, promo: p });
                }
            } else {
                out.push(Mv { from: from as u8, to: to as u8, promo: 0 });
            }
        };
        for s in 0..64i32 {
            let p = self.sq[s as usize];
            if p == 0 || !self.own(p) {
                continue;
            }
            let (f, r) = (file(s), rank(s));
            match p.abs() {
                1 => {
                    let dir = if self.white { 1 } else { -1 };
                    let last = if self.white { 7 } else { 0 };
                    let start = if self.white { 1 } else { 6 };
                    if let Some(t) = on(f, r + dir) {
                        if self.sq[t as usize] == 0 {
                            if !captures_only || r + dir == last {
                                push(&mut out, s, t, r + dir == last);
                            }
                            if r == start && !captures_only {
                                let t2 = on(f, r + 2 * dir).unwrap();
                                if self.sq[t2 as usize] == 0 {
                                    push(&mut out, s, t2, false);
                                }
                            }
                        }
                    }
                    for df in [-1, 1] {
                        if let Some(t) = on(f + df, r + dir) {
                            let q = self.sq[t as usize];
                            if (q != 0 && !self.own(q)) || self.ep == Some(t as u8) {
                                push(&mut out, s, t, r + dir == last);
                            }
                        }
                    }
                }
                2 | 6 => {
                    let steps = if p.abs() == 2 { KNIGHT } else { KING };
                    for (df, dr) in steps {
                        if let Some(t) = on(f + df, r + dr) {
                            let q = self.sq[t as usize];
                            if (q == 0 && !captures_only) || (q != 0 && !self.own(q)) {
                                push(&mut out, s, t, false);
                            }
                        }
                    }
                    if p.abs() == 6 && !captures_only {
                        self.castles(s, &mut out);
                    }
                }
                t => {
                    let dirs: Vec<(i32, i32)> = match t {
                        3 => BISHOP.to_vec(),
                        4 => ROOK.to_vec(),
                        _ => ROOK.iter().chain(BISHOP.iter()).copied().collect(),
                    };
                    for (df, dr) in dirs {
                        let (mut x, mut y) = (f + df, r + dr);
                        while let Some(t) = on(x, y) {
                            let q = self.sq[t as usize];
                            if q == 0 {
                                if !captures_only {
                                    push(&mut out, s, t, false);
                                }
                            } else {
                                if !self.own(q) {
                                    push(&mut out, s, t, false);
                                }
                                break;
                            }
                            x += df;
                            y += dr;
                        }
                    }
                }
            }
        }
        out
    }

    /// Castling: the right is kept, the squares between are empty, and the
    /// king is not in check and does not pass or land on an attacked square.
    fn castles(&self, k: i32, out: &mut Vec<Mv>) {
        let (base, short, long) = if self.white { (0, 1, 2) } else { (56, 4, 8) };
        if k != base + 4 || self.attacked(k, !self.white) {
            return;
        }
        let rook = if self.white { 4 } else { -4 };
        if self.castle & short != 0 && self.sq[(base + 7) as usize] == rook && self.sq[(base + 5) as usize] == 0 && self.sq[(base + 6) as usize] == 0
            && !self.attacked(base + 5, !self.white) && !self.attacked(base + 6, !self.white)
        {
            out.push(Mv { from: k as u8, to: (base + 6) as u8, promo: 0 });
        }
        if self.castle & long != 0 && self.sq[base as usize] == rook && (1..=3).all(|i| self.sq[(base + i) as usize] == 0)
            && !self.attacked(base + 3, !self.white) && !self.attacked(base + 2, !self.white)
        {
            out.push(Mv { from: k as u8, to: (base + 2) as u8, promo: 0 });
        }
    }

    /// The position after `m` (m must be one of this position's moves).
    pub fn play(&self, m: Mv) -> Board {
        let mut b = self.clone();
        let (from, to) = (m.from as usize, m.to as usize);
        let p = b.sq[from];
        let captured = b.sq[to];
        b.sq[from] = 0;
        // en passant: the captured pawn is beside, not on the target square
        if p.abs() == 1 && Some(m.to) == self.ep && captured == 0 {
            let victim = if self.white { to - 8 } else { to + 8 };
            b.sq[victim] = 0;
        }
        b.sq[to] = if m.promo != 0 { m.promo * p.signum() } else { p };
        // castling moves the rook too
        if p.abs() == 6 && (to as i32 - from as i32).abs() == 2 {
            let (rf, rt) = if to > from { (from + 3, from + 1) } else { (from - 4, from - 1) };
            b.sq[rt] = b.sq[rf];
            b.sq[rf] = 0;
        }
        // rights: lost when the king or a rook moves, or a rook is captured on its corner
        for (sqr, right) in [(4usize, 3u8), (60, 12), (0, 2), (7, 1), (56, 8), (63, 4)] {
            if from == sqr || to == sqr {
                b.castle &= !right;
            }
        }
        b.ep = (p.abs() == 1 && (to as i32 - from as i32).abs() == 16).then(|| ((from + to) / 2) as u8);
        b.half = if p.abs() == 1 || captured != 0 { 0 } else { self.half + 1 };
        if !self.white {
            b.full += 1;
        }
        b.white = !self.white;
        b
    }

    /// Every legal move.
    pub fn moves(&self) -> Vec<Mv> {
        self.pseudo(false).into_iter().filter(|&m| !self.play(m).leaves_king(self.white)).collect()
    }

    #[allow(dead_code)] // kept with the rules; the atoms count captures from the legal moves
    fn captures(&self) -> Vec<Mv> {
        self.pseudo(true).into_iter().filter(|&m| !self.play(m).leaves_king(self.white)).collect()
    }

    /// After a move by `white`: is that side's king attacked?
    fn leaves_king(&self, white: bool) -> bool {
        let k = self.king(white);
        k < 0 || self.attacked(k, !white)
    }

    /// Not enough material for anyone to mate: K v K, K+minor v K, K+B v K+B with bishops on one colour.
    pub fn insufficient(&self) -> bool {
        let pieces: Vec<(usize, i8)> = self.sq.iter().enumerate().filter(|(_, &p)| p != 0 && p.abs() != 6).map(|(i, &p)| (i, p)).collect();
        match pieces.len() {
            0 => true,
            1 => matches!(pieces[0].1.abs(), 2 | 3),
            _ => pieces.iter().all(|x| x.1.abs() == 3) && {
                let colour = |i: usize| (i % 8 + i / 8) % 2;
                pieces.iter().all(|x| colour(x.0) == colour(pieces[0].0))
            },
        }
    }

    /// The position part of the FEN (for repetitions).
    pub fn key(&self) -> String {
        let f = self.fen();
        f.rsplitn(3, ' ').nth(2).unwrap_or(&f).to_string()
    }

    pub fn parse(&self, uci: &str) -> Option<Mv> {
        self.moves().into_iter().find(|m| m.uci() == uci)
    }
}

/// Count the leaf positions `depth` moves deep (perft).
pub fn perft(b: &Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }
    let ms = b.moves();
    if depth == 1 {
        return ms.len() as u64;
    }
    ms.iter().map(|&m| perft(&b.play(m), depth - 1)).sum()
}


impl Board {
    /// A number for the position part (repetitions inside the search; FNV-1a).
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |x: u8| {
            h ^= x as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        for &p in &self.sq {
            eat(p as u8);
        }
        eat(self.white as u8);
        eat(self.castle);
        eat(self.ep.map_or(255, |e| e));
        h
    }

    /// The same position with the other side to move (no en passant): what they could do now.
    fn flipped(&self) -> Board {
        let mut t = self.clone();
        t.white = !t.white;
        t.ep = None;
        t
    }
}

// ── atoms: numbers the rules define about a position ──

/// The atoms' names, in order; "my" is the side to move.
pub const ATOMS: [&str; 29] = [
    "my_moves",
    "their_moves",
    "in_check",
    "my_escapes",
    "their_escapes",
    "i_attack",
    "they_attack",
    "my_captures",
    "their_captures",
    "my_pawns",
    "my_knights",
    "my_bishops",
    "my_rooks",
    "my_queens",
    "their_pawns",
    "their_knights",
    "their_bishops",
    "their_rooks",
    "their_queens",
    "halfmove",
    "my_pawn_power",
    "their_pawn_power",
    "my_best_capture",
    "my_nodes",
    "their_nodes",
    "my_unstoppable",
    "their_unstoppable",
    "my_key_squares",
    "their_key_squares",
];
const N_ATOMS: usize = ATOMS.len();

fn is_capture(b: &Board, m: &Mv) -> bool {
    b.sq[m.to as usize] != 0 || (b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep)
}

/// Squares next to `white`'s king it could step to: on the board, not its own
/// piece, and not attacked once the king has left its square.
fn escapes(b: &Board, white: bool) -> i64 {
    let k = b.king(white);
    if k < 0 {
        return 0;
    }
    let mut t = b.clone();
    t.sq[k as usize] = 0;
    let own = |p: i8| if white { p > 0 } else { p < 0 };
    KING.iter()
        .filter_map(|&(df, dr)| on(file(k) + df, rank(k) + dr))
        .filter(|&s| !own(b.sq[s as usize]) && !t.attacked(s, !white))
        .count() as i64
}

/// How many moves the rules give a lone piece of type `t` (2..6), averaged
/// over the 64 squares of an empty board: knight 5.25, bishop 8.75,
/// rook 14, queen 22.75, king 6.5625.
pub fn rule_mobility(t: i8) -> f64 {
    let mut total = 0;
    for s in 0..64 {
        let mut b = Board { sq: [0; 64], white: true, castle: 0, ep: None, half: 0, full: 1 };
        b.sq[s] = t;
        total += b.pseudo(false).len();
    }
    total as f64 / 64.0
}

/// The network: for every one of the 64 squares (nodes), how many edges of
/// each side point at it - the squares each piece attacks under the rules
/// (pawns diagonally forward, sliders until the first piece). [white, black].
pub fn attack_edges(b: &Board) -> [[u8; 64]; 2] {
    let mut e = [[0u8; 64]; 2];
    for s in 0..64i32 {
        let p = b.sq[s as usize];
        if p == 0 {
            continue;
        }
        let side = (p < 0) as usize;
        let (f, r) = (file(s), rank(s));
        let mut hit = |t: i32| e[side][t as usize] += 1;
        match p.abs() {
            1 => {
                let dr = if p > 0 { 1 } else { -1 };
                for df in [-1, 1] {
                    if let Some(t) = on(f + df, r + dr) {
                        hit(t);
                    }
                }
            }
            2 | 6 => {
                for (df, dr) in if p.abs() == 2 { KNIGHT } else { KING } {
                    if let Some(t) = on(f + df, r + dr) {
                        hit(t);
                    }
                }
            }
            t => {
                let dirs: Vec<(i32, i32)> = match t {
                    3 => BISHOP.to_vec(),
                    4 => ROOK.to_vec(),
                    _ => ROOK.iter().chain(BISHOP.iter()).copied().collect(),
                };
                for (df, dr) in dirs {
                    let (mut x, mut y) = (f + df, r + dr);
                    while let Some(t) = on(x, y) {
                        hit(t);
                        if b.sq[t as usize] != 0 {
                            break;
                        }
                        x += df;
                        y += dr;
                    }
                }
            }
        }
    }
    e
}

/// The edges of the one piece on node `s` in this network (its reach now).
pub fn reach(b: &Board, s: usize) -> usize {
    let p = b.sq[s];
    if p == 0 {
        return 0;
    }
    let t = b;
    let (f, r) = (file(s as i32), rank(s as i32));
    let mut n = 0;
    match p.abs() {
        1 => {
            let dr = if p > 0 { 1 } else { -1 };
            n += [-1, 1].iter().filter(|&&df| on(f + df, r + dr).is_some()).count();
            if on(f, r + dr).is_some_and(|x| t.sq[x as usize] == 0) {
                n += 1;
            }
        }
        2 | 6 => {
            for (df, dr) in if p.abs() == 2 { KNIGHT } else { KING } {
                if on(f + df, r + dr).is_some() {
                    n += 1;
                }
            }
        }
        k => {
            let dirs: Vec<(i32, i32)> = match k {
                3 => BISHOP.to_vec(),
                4 => ROOK.to_vec(),
                _ => ROOK.iter().chain(BISHOP.iter()).copied().collect(),
            };
            for (df, dr) in dirs {
                let (mut x, mut y) = (f + df, r + dr);
                while let Some(q) = on(x, y) {
                    n += 1;
                    if t.sq[q as usize] != 0 {
                        break;
                    }
                    x += df;
                    y += dr;
                }
            }
        }
    }
    n
}

/// The pawn on `s` has a clear path: no piece on any node ahead of it on its file.
fn clear_path(b: &Board, s: usize, white: bool) -> bool {
    let (f, r) = (s % 8, s / 8);
    if white { (r + 1..8).all(|y| b.sq[y * 8 + f] == 0) } else { (0..r).all(|y| b.sq[y * 8 + f] == 0) }
}

/// Steps a pawn on square `s` still needs to become a queen.
fn steps_left(s: usize, white: bool) -> i64 {
    if white { 7 - (s / 8) as i64 } else { (s / 8) as i64 }
}

/// A piece's worth in 1/256 moves: its rule mobility; a pawn's is the
/// queen's halved for every step it still needs (the rules promote it).
pub fn rule_value(p: i8, s: usize) -> i64 {
    match p.abs() {
        1 => (rule_mobility(5) * 256.0) as i64 >> steps_left(s, p > 0).clamp(1, 6),
        6 => 0,
        t => (rule_mobility(t) * 256.0) as i64,
    }
}

/// The atoms of `b` (side to move's view); `ms` are its legal moves, `mask`
/// says which atoms are needed (the others stay 0).
pub fn atoms(b: &Board, ms: &[Mv], mask: u32) -> [i64; N_ATOMS] {
    let mut a = [0i64; N_ATOMS];
    let want = |i: usize| mask & (1 << i) != 0;
    a[0] = ms.len() as i64;
    a[7] = ms.iter().filter(|m| is_capture(b, m)).count() as i64;
    if want(1) || want(8) {
        let t = b.flipped();
        let tm = t.moves();
        a[1] = tm.len() as i64;
        a[8] = tm.iter().filter(|m| is_capture(&t, m)).count() as i64;
    }
    a[2] = b.in_check() as i64;
    if want(3) {
        a[3] = escapes(b, b.white);
    }
    if want(4) {
        a[4] = escapes(b, !b.white);
    }
    let mine = |p: i8| if b.white { p > 0 } else { p < 0 };
    for s in 0..64 {
        let p = b.sq[s];
        if p == 0 {
            continue;
        }
        let t = p.unsigned_abs() as usize;
        if t <= 5 {
            a[if mine(p) { 8 + t } else { 13 + t }] += 1;
        }
        if t != 6 {
            if !mine(p) && want(5) && b.attacked(s as i32, b.white) {
                a[5] += 1;
            }
            if mine(p) && want(6) && b.attacked(s as i32, !b.white) {
                a[6] += 1;
            }
        }
    }
    a[19] = b.half as i64;
    if want(20) || want(21) {
        for (sq, &p) in b.sq.iter().enumerate() {
            if p.abs() == 1 {
                // 2^(6 - steps left): 32 one step from queening, 1 at the start; a blocked path counts 1
                let w = if clear_path(b, sq, p > 0) { 1i64 << (6 - steps_left(sq, p > 0).clamp(1, 6)) } else { 1 };
                a[if mine(p) { 20 } else { 21 }] += w;
            }
        }
    }
    if want(22) {
        // the most the side to move wins by one capture: the victim, less
        // the capturer when the square is defended (it can be taken back)
        let mut best = 0;
        for m in ms.iter().filter(|m| is_capture(b, m)) {
            let victim = if b.sq[m.to as usize] == 0 { rule_value(if b.white { -1 } else { 1 }, m.to as usize) } else { rule_value(b.sq[m.to as usize], m.to as usize) };
            let attacker = rule_value(b.sq[m.from as usize], m.from as usize);
            let defended = b.play(*m).attacked(m.to as i32, !b.white);
            best = best.max(if defended { victim - attacker } else { victim });
        }
        a[22] = best;
    }
    if want(23) || want(24) {
        // the nodes each side holds: more of its edges point there than the other side's
        let e = attack_edges(b);
        let (me, them) = if b.white { (0, 1) } else { (1, 0) };
        a[23] = (0..64).filter(|&s| e[me][s] > e[them][s]).count() as i64;
        a[24] = (0..64).filter(|&s| e[them][s] > e[me][s]).count() as i64;
    }
    if want(25) || want(26) || want(27) || want(28) {
        let cheb = |x: usize, y: usize| ((x % 8) as i64 - (y % 8) as i64).abs().max(((x / 8) as i64 - (y / 8) as i64).abs());
        for (sq, &p) in b.sq.iter().enumerate() {
            if p.abs() != 1 {
                continue;
            }
            let white = p > 0;
            let ours = mine(p);
            let enemy_king = b.king(!white);
            let own_king = b.king(white);
            if enemy_king < 0 || own_king < 0 {
                continue;
            }
            // the race: the pawn's path is clear and the enemy king cannot reach the queening node in time
            // (the side to move gains a tempo)
            let steps = steps_left(sq, white) - if (white && sq / 8 == 1) || (!white && sq / 8 == 6) { 1 } else { 0 };
            let queen_node = if white { 56 + sq % 8 } else { sq % 8 };
            let d = cheb(enemy_king as usize, queen_node);
            let tempo = if ours { 0 } else { 1 };
            if clear_path(b, sq, white) && d - tempo > steps {
                a[if ours { 25 } else { 26 }] += 1;
            }
            // key nodes in front of the pawn: the own king standing there escorts it home
            let (f, r) = ((sq % 8) as i32, (sq / 8) as i32);
            let rel = if white { r } else { 7 - r };
            let fwd = if white { 1 } else { -1 };
            let rows: Vec<i32> = if rel <= 3 { vec![2] } else { vec![1, 2] };
            let on_key = rows.iter().any(|&k| (f - 1..=f + 1).filter_map(|x| on(x, r + fwd * k)).any(|t| t == own_king));
            if on_key {
                a[if ours { 27 } else { 28 }] += 1;
            }
        }
    }
    a
}

// ── formulas ──

/// A formula over the atoms: the value of a position for the side to move.
#[derive(Clone, Debug, PartialEq)]
pub enum F {
    A(u8),
    C(i64),
    Add(Box<F>, Box<F>),
    Sub(Box<F>, Box<F>),
    Mul(Box<F>, Box<F>),
    Min(Box<F>, Box<F>),
    Max(Box<F>, Box<F>),
}

/// Formula values stay inside +-CLAMP, far below a mate.
const CLAMP: i64 = 1_000_000_000;
const MATE: i64 = 1_000_000_000_000;

impl F {
    pub fn eval(&self, a: &[i64; N_ATOMS]) -> i64 {
        let v = match self {
            F::A(i) => a[*i as usize],
            F::C(c) => *c,
            F::Add(x, y) => x.eval(a) + y.eval(a),
            F::Sub(x, y) => x.eval(a) - y.eval(a),
            F::Mul(x, y) => x.eval(a).saturating_mul(y.eval(a)),
            F::Min(x, y) => x.eval(a).min(y.eval(a)),
            F::Max(x, y) => x.eval(a).max(y.eval(a)),
        };
        v.clamp(-CLAMP, CLAMP)
    }

    /// Which atoms the formula reads (bit i = atom i).
    pub fn mask(&self) -> u32 {
        match self {
            F::A(i) => 1 << i,
            F::C(_) => 0,
            F::Add(x, y) | F::Sub(x, y) | F::Mul(x, y) | F::Min(x, y) | F::Max(x, y) => x.mask() | y.mask(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            F::A(_) | F::C(_) => 1,
            F::Add(x, y) | F::Sub(x, y) | F::Mul(x, y) | F::Min(x, y) | F::Max(x, y) => 1 + x.size() + y.size(),
        }
    }

    fn kids(&mut self) -> Vec<&mut F> {
        match self {
            F::A(_) | F::C(_) => vec![],
            F::Add(x, y) | F::Sub(x, y) | F::Mul(x, y) | F::Min(x, y) | F::Max(x, y) => vec![x, y],
        }
    }

    /// Node `k` in pre-order.
    fn at(&mut self, k: usize) -> &mut F {
        if k == 0 {
            return self;
        }
        let mut k = k - 1;
        let sizes: Vec<usize> = self.kids().iter().map(|c| c.size()).collect();
        for (i, s) in sizes.iter().enumerate() {
            if k < *s {
                return self.kids().into_iter().nth(i).expect("kid").at(k);
            }
            k -= s;
        }
        self
    }

    fn inner(&self) -> String {
        match self {
            F::A(i) => ATOMS[*i as usize].into(),
            F::C(c) => c.to_string(),
            F::Add(x, y) => format!("({} + {})", x.inner(), y.inner()),
            F::Sub(x, y) => format!("({} - {})", x.inner(), y.inner()),
            F::Mul(x, y) => format!("({} * {})", x.inner(), y.inner()),
            F::Min(x, y) => format!("min({}, {})", x.inner(), y.inner()),
            F::Max(x, y) => format!("max({}, {})", x.inner(), y.inner()),
        }
    }

    /// The formula in words, e.g. `my_moves - their_moves`; parse() reads it back.
    pub fn show(&self) -> String {
        let s = self.inner();
        match self {
            F::Add(..) | F::Sub(..) | F::Mul(..) => s[1..s.len() - 1].to_string(),
            _ => s,
        }
    }

    /// Read a formula: atoms, whole numbers, + - * (usual precedence), min(a, b), max(a, b), brackets.
    pub fn parse(text: &str) -> Result<F, String> {
        let toks = tokens(text)?;
        let mut i = 0;
        let f = expr(&toks, &mut i)?;
        if i != toks.len() {
            return Err(format!("unexpected '{}' in {text}", toks[i]));
        }
        Ok(f)
    }
}

fn tokens(text: &str) -> Result<Vec<String>, String> {
    let cs: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_alphanumeric() || c == '_' {
            let j = (i..cs.len()).find(|&j| !(cs[j].is_ascii_alphanumeric() || cs[j] == '_')).unwrap_or(cs.len());
            out.push(cs[i..j].iter().collect());
            i = j;
        } else if "()+-*,".contains(c) {
            out.push(c.to_string());
            i += 1;
        } else {
            return Err(format!("bad character '{c}' in {text}"));
        }
    }
    Ok(out)
}

fn expect(t: &[String], i: &mut usize, s: &str) -> Result<(), String> {
    if t.get(*i).map(|x| x.as_str()) == Some(s) {
        *i += 1;
        Ok(())
    } else {
        Err(format!("expected '{s}'"))
    }
}

fn expr(t: &[String], i: &mut usize) -> Result<F, String> {
    let mut f = term(t, i)?;
    while let Some(op) = t.get(*i).filter(|x| *x == "+" || *x == "-").cloned() {
        *i += 1;
        let g = term(t, i)?;
        f = if op == "+" { F::Add(Box::new(f), Box::new(g)) } else { F::Sub(Box::new(f), Box::new(g)) };
    }
    Ok(f)
}

fn term(t: &[String], i: &mut usize) -> Result<F, String> {
    let mut f = factor(t, i)?;
    while t.get(*i).is_some_and(|x| x == "*") {
        *i += 1;
        let g = factor(t, i)?;
        f = F::Mul(Box::new(f), Box::new(g));
    }
    Ok(f)
}

fn factor(t: &[String], i: &mut usize) -> Result<F, String> {
    let tok = t.get(*i).ok_or("the formula ends too early")?.clone();
    *i += 1;
    if tok == "(" {
        let f = expr(t, i)?;
        expect(t, i, ")")?;
        return Ok(f);
    }
    if tok == "-" {
        let n = t.get(*i).and_then(|x| x.parse::<i64>().ok()).ok_or("'-' must come before a number")?;
        *i += 1;
        return Ok(F::C(-n));
    }
    if let Ok(n) = tok.parse::<i64>() {
        return Ok(F::C(n));
    }
    if tok == "min" || tok == "max" {
        expect(t, i, "(")?;
        let x = expr(t, i)?;
        expect(t, i, ",")?;
        let y = expr(t, i)?;
        expect(t, i, ")")?;
        return Ok(if tok == "min" { F::Min(Box::new(x), Box::new(y)) } else { F::Max(Box::new(x), Box::new(y)) });
    }
    ATOMS.iter().position(|a| *a == tok).map(|p| F::A(p as u8)).ok_or_else(|| format!("unknown atom '{tok}'"))
}

/// The formula used when none has been evolved: my legal moves minus theirs.
pub fn default_formula() -> F {
    F::parse("my_moves - their_moves").expect("default formula")
}

/// The evolved formula from `path` if it is there and reads, else the default; and where it came from.
pub fn load_formula(path: &str) -> (F, String) {
    match std::fs::read_to_string(path).map(|t| F::parse(t.trim())) {
        Ok(Ok(f)) => (f, path.to_string()),
        Ok(Err(e)) => (default_formula(), format!("default ({path} does not read: {e})")),
        Err(_) => (default_formula(), "default (no evolved formula yet)".into()),
    }
}

// ── the golden function: one formula, no search ──

pub struct Answer {
    pub mv: Mv,
    pub value: i64,
    /// every move with its value, best first
    pub ranked: Vec<(Mv, i64)>,
}

/// The value of playing `m` in `b`, for the mover: the rules first (a move
/// that mates wins; stalemate, insufficient material, the 50-move rule and
/// a third repetition draw), else the formula on the position after it,
/// from the mover's side (the formula speaks for the side to move there,
/// the opponent, so its value is negated).
pub fn value_of(b: &Board, m: Mv, history: &[u64], f: &F) -> i64 {
    let a = b.play(m);
    let ms = a.moves();
    if ms.is_empty() {
        return if a.in_check() { MATE } else { 0 };
    }
    if a.half >= 100 || a.insufficient() {
        return 0;
    }
    // a third time here (b itself has the other side to move, so it never matches)
    let k = a.hash();
    if history.iter().filter(|h| **h == k).count() >= 2 {
        return 0;
    }
    -f.eval(&atoms(&a, &ms, f.mask()))
}

/// The golden function: in `b`, the legal move whose resulting position
/// the formula values best for the mover; `history` holds the earlier
/// positions' hashes (repetitions). Among equal values the first move in
/// the rules' order wins, so the same position always gives the same move.
pub fn golden(b: &Board, history: &[u64], f: &F) -> Option<Answer> {
    let mut ranked: Vec<(Mv, i64)> = b.moves().into_iter().map(|m| (m, value_of(b, m, history, f))).collect();
    if ranked.is_empty() {
        return None;
    }
    ranked.sort_by_key(|x| -x.1); // stable
    Some(Answer { mv: ranked[0].0, value: ranked[0].1, ranked })
}

/// The value in words: "+12", or "mate" for a move that mates.
pub fn show_value(v: i64) -> String {
    if v == MATE {
        "mate".into()
    } else {
        format!("{v:+}")
    }
}

// ── games ──

/// Who moves: the golden function with a formula, or a random legal mover (seeded).
pub enum Player<'a> {
    Golden(&'a F),
    /// the supergenius writes a new formula for every position
    Supergenius,
    Random(u64),
}

pub struct Game {
    pub moves: Vec<String>,
    /// +1 white won, -1 black won, 0 draw or unfinished
    pub score: i32,
    /// "1-0", "0-1", "1/2-1/2", "*"
    pub result: String,
    pub how: String,
}

/// A game from `opening` (UCI moves), at most `max_plies` plies in all.
pub fn play_game(white: &Player, black: &Player, opening: &[&str], max_plies: usize) -> Game {
    let mut b = Board::start();
    let mut history: Vec<u64> = Vec::new();
    let mut moves = Vec::new();
    for u in opening {
        let Some(m) = b.parse(u) else { break };
        history.push(b.hash());
        b = b.play(m);
        moves.push(u.to_string());
    }
    let seed = |p: &Player| if let Player::Random(s) = p { *s | 1 } else { 1 };
    let mut rngs = [Rng(seed(white)), Rng(seed(black))];
    let end = |moves: Vec<String>, score: i32, how: &str| Game {
        moves,
        score,
        result: match score {
            1 => "1-0".into(),
            -1 => "0-1".into(),
            _ if how.starts_with("stopped") => "*".into(),
            _ => "1/2-1/2".into(),
        },
        how: how.into(),
    };
    loop {
        if history.iter().filter(|h| **h == b.hash()).count() >= 2 {
            return end(moves, 0, "threefold repetition");
        }
        if b.half >= 100 {
            return end(moves, 0, "50-move rule");
        }
        if b.insufficient() {
            return end(moves, 0, "insufficient material");
        }
        let ms = b.moves();
        if ms.is_empty() {
            return if b.in_check() { end(moves, if b.white { -1 } else { 1 }, "checkmate") } else { end(moves, 0, "stalemate") };
        }
        if moves.len() >= max_plies {
            return end(moves, 0, &format!("stopped after {max_plies} plies"));
        }
        let side = if b.white { 0 } else { 1 };
        let m = match if b.white { white } else { black } {
            Player::Golden(f) => golden(&b, &history, f).expect("a legal move").mv,
            Player::Supergenius => crate::golden_terms::best_move(&b, &history).expect("a legal move"),
            Player::Random(_) => ms[rngs[side].below(ms.len())],
        };
        history.push(b.hash());
        b = b.play(m);
        moves.push(m.uci());
    }
}

/// A game of the supergenius's golden functions against themselves; (moves, result, how).
pub fn self_play_supergenius(opening: &[&str], max_plies: usize) -> (Vec<String>, String, String) {
    let g = play_game(&Player::Supergenius, &Player::Supergenius, opening, max_plies);
    (g.moves, g.result, g.how)
}

/// A game of the golden function against itself; (moves, result, how).
pub fn self_play(opening: &[&str], max_plies: usize, f: &F) -> (Vec<String>, String, String) {
    let p = Player::Golden(f);
    let g = play_game(&p, &p, opening, max_plies);
    (g.moves, g.result, g.how)
}

pub const OPENINGS: [&[&str]; 8] = [
    &["e2e4", "e7e5"],
    &["d2d4", "d7d5"],
    &["e2e4", "c7c5"],
    &["d2d4", "g8f6"],
    &["c2c4", "e7e5"],
    &["g1f3", "d7d5"],
    &["e2e4", "e7e6"],
    &["e2e4", "c7c6"],
];

// ── evolution: formulas that win games ──

pub struct Settings {
    pub generations: usize,
    pub population: usize,
    pub islands: usize,
    /// search depth in the games (plies)
    pub depth: usize,
    pub max_plies: usize,
    /// how many of OPENINGS each formula plays from, per baseline
    pub openings: usize,
    pub seed: u64,
}

/// The "most legal moves" baseline (played at depth 1): after its move it has as many legal moves as it can.
pub fn mobility_formula() -> F {
    F::parse("0 - their_moves").expect("mobility")
}

fn mix(a: u64, b: u64) -> u64 {
    let mut r = Rng((a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x632B_E59B_D9B4_E019)) | 1);
    r.next()
}

fn random_f(r: &mut Rng, depth: usize) -> F {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(3) == 0 { F::C(r.below(19) as i64 - 9) } else { F::A(r.below(N_ATOMS) as u8) };
    }
    let mut g = || Box::new(random_f(r, depth - 1));
    let (x, y) = (g(), g());
    match r.below(8) {
        0 | 1 => F::Add(x, y),
        2 | 3 => F::Sub(x, y),
        4 | 5 => F::Mul(x, y),
        6 => F::Min(x, y),
        _ => F::Max(x, y),
    }
}

const MAX_SIZE: usize = 25;
const NODE_PRICE: f64 = 0.001;

fn mutate(r: &mut Rng, f: &F, other: &F) -> F {
    let mut child = f.clone();
    let k = r.below(child.size());
    match r.below(10) {
        // crossover: a branch of the other parent
        0..=2 => {
            let mut o = other.clone();
            let j = r.below(o.size());
            *child.at(k) = o.at(j).clone();
        }
        3..=5 => match child.at(k) {
            F::C(c) => *c = (*c + [-3, -2, -1, 1, 2, 3][r.below(6)]).clamp(-100, 100),
            F::A(a) => *a = r.below(N_ATOMS) as u8,
            node => *node = random_f(r, 2),
        },
        6 => {
            // scale a branch by a small number
            let node = child.at(k);
            *node = F::Mul(Box::new(F::C(r.below(9) as i64 - 4)), Box::new(node.clone()));
        }
        _ => *child.at(k) = random_f(r, 3),
    }
    child
}

/// One formula's games in a generation, in points (win 1, draw 1/2).
#[derive(Clone, Copy, Default, Debug)]
pub struct Tally {
    pub random: f64,
    pub mobility: f64,
    pub peers: f64,
    pub games: [usize; 3],
}

impl Tally {
    pub fn total(&self) -> f64 {
        self.random + self.mobility + self.peers
    }
}

fn points(score: i32, white: bool) -> f64 {
    (if white { score } else { -score } as f64 + 1.0) / 2.0
}

/// Score a population: each formula against the random mover and the
/// mobility mover from `s.openings` openings, and against its neighbours on
/// the ring (one and two places along, both colours); the mobility mover always looks 1 ply ahead. Colours and random seeds change with `gen`.
fn tally(pop: &[F], s: &Settings, gen: usize) -> Vec<Tally> {
    let mob = mobility_formula();
    let n = pop.len();
    let mut t = vec![Tally::default(); n];
    for (i, f) in pop.iter().enumerate() {
        let me = Player::Golden(f);
        for o in 0..s.openings {
            let op = OPENINGS[(o + gen) % OPENINGS.len()];
            let white = (o + gen) % 2 == 0;
            let rnd = Player::Random(mix(s.seed ^ gen as u64, o as u64));
            let g = if white { play_game(&me, &rnd, op, s.max_plies) } else { play_game(&rnd, &me, op, s.max_plies) };
            t[i].random += points(g.score, white);
            let mp = Player::Golden(&mob);
            let white = !white;
            let g = if white { play_game(&me, &mp, op, s.max_plies) } else { play_game(&mp, &me, op, s.max_plies) };
            t[i].mobility += points(g.score, white);
        }
        t[i].games[0] += s.openings;
        t[i].games[1] += s.openings;
    }
    // neighbours one and two places along the ring (one place only in a tiny population)
    let offsets = if n >= 5 { 2 } else { (n > 1) as usize };
    for off in 1..=offsets {
        for i in 0..n {
            let j = (i + off) % n;
            if n == 2 && i == 1 {
                break;
            }
            let op = OPENINGS[(gen + i + off) % OPENINGS.len()];
            let (a, b) = (Player::Golden(&pop[i]), Player::Golden(&pop[j]));
            for (w, bl, iw) in [(&a, &b, true), (&b, &a, false)] {
                let g = play_game(w, bl, op, s.max_plies);
                t[i].peers += points(g.score, iw);
                t[j].peers += points(g.score, !iw);
                t[i].games[2] += 1;
                t[j].games[2] += 1;
            }
        }
    }
    t
}

fn fitness(t: &Tally, f: &F) -> f64 {
    t.total() - NODE_PRICE * f.size() as f64
}

/// One island's generation: score, keep the best, breed the rest.
fn step(pop: &mut Vec<F>, r: &mut Rng, s: &Settings, gen: usize) -> Vec<(f64, Tally, F)> {
    let t = tally(pop, s, gen);
    let mut scored: Vec<(f64, Tally, F)> = pop.drain(..).zip(t).map(|(f, t)| (fitness(&t, &f), t, f)).collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let keep = (s.population / 8).max(1);
    let mut next: Vec<F> = scored.iter().take(keep).map(|x| x.2.clone()).collect();
    let pick = |r: &mut Rng| {
        let mut best = r.below(scored.len());
        for _ in 0..2 {
            let c = r.below(scored.len());
            if scored[c].0 > scored[best].0 {
                best = c;
            }
        }
        best
    };
    let mut tries = 0;
    while next.len() < s.population {
        let (a, b) = (pick(r), pick(r));
        let child = mutate(r, &scored[a].2, &scored[b].2);
        tries += 1;
        if child.size() <= MAX_SIZE || tries > 1000 {
            next.push(child);
        }
    }
    *pop = next;
    scored
}

/// The final test: 2 games per opening (both colours) against each baseline, fixed seeds.
pub fn test_formula(f: &F, depth: usize, max_plies: usize) -> [[usize; 3]; 2] {
    let mob = mobility_formula();
    let me = Player::Golden(f);
    // [vs random, vs mobility] x [won, drawn, lost]
    let mut out = [[0usize; 3]; 2];
    for (o, op) in OPENINGS.iter().enumerate() {
        for white in [true, false] {
            let rnd = Player::Random(mix(0xC0FFEE, (2 * o + white as usize) as u64));
            let mp = Player::Golden(&mob);
            for (k, other) in [(0, &rnd), (1, &mp)] {
                let g = if white { play_game(&me, other, op, max_plies) } else { play_game(other, &me, op, max_plies) };
                let p = if white { g.score } else { -g.score };
                out[k][(1 - p) as usize] += 1;
            }
        }
    }
    out
}

fn show_wdl(x: &[usize; 3]) -> String {
    format!("+{} ={} -{} ({:.1}/{})", x[0], x[1], x[2], x[0] as f64 + x[1] as f64 / 2.0, x[0] + x[1] + x[2])
}

pub struct Outcome {
    pub best: F,
    pub report: String,
}

/// Breed formulas on islands (one thread each); `log` gets a line per generation.
pub fn evolve(s: &Settings, log: &mut dyn FnMut(&str)) -> Outcome {
    let t0 = std::time::Instant::now();
    let mut islands: Vec<(Vec<F>, Rng)> = (0..s.islands)
        .map(|i| {
            let mut r = Rng(mix(s.seed, i as u64 + 1));
            ((0..s.population).map(|_| random_f(&mut r, 3)).collect(), r)
        })
        .collect();
    let mut champions: Vec<(f64, Tally, F)> = Vec::new();
    for gen in 0..s.generations {
        let tg = std::time::Instant::now();
        let results: Vec<Vec<(f64, Tally, F)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = islands.iter_mut().map(|(pop, r)| sc.spawn(move || step(pop, r, s, gen))).collect();
            hs.into_iter().map(|h| h.join().expect("island")).collect()
        });
        // migration every 5 generations: each island's best replaces the next island's last child
        if s.islands > 1 && (gen + 1) % 5 == 0 {
            for i in 0..s.islands {
                let best = results[i][0].2.clone();
                let next = &mut islands[(i + 1) % s.islands].0;
                let last = next.len() - 1;
                next[last] = best;
            }
        }
        let mut tops: Vec<&(f64, Tally, F)> = results.iter().map(|r| &r[0]).collect();
        tops.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (fit, t, f) = tops[0];
        let mean = results.iter().flatten().map(|x| x.1.total()).sum::<f64>() / results.iter().map(|r| r.len()).sum::<usize>() as f64;
        log(&format!(
            "gen {:>3}: best {:.1} points (random {:.1}/{}, mobility {:.1}/{}, peers {:.1}/{}), mean {:.2}, {:.1} s: {}",
            gen + 1,
            fit,
            t.random,
            t.games[0],
            t.mobility,
            t.games[1],
            t.peers,
            t.games[2],
            mean,
            tg.elapsed().as_secs_f64(),
            f.show()
        ));
        if gen + 1 == s.generations {
            champions = results.into_iter().map(|mut r| r.swap_remove(0)).collect();
        }
    }
    // the final test: each island's best, plus the default and the baselines for comparison
    champions.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut cands: Vec<F> = Vec::new();
    for c in &champions {
        if !cands.contains(&c.2) {
            cands.push(c.2.clone());
        }
    }
    let tests: Vec<[[usize; 3]; 2]> = std::thread::scope(|sc| {
        let hs: Vec<_> = cands.iter().map(|f| sc.spawn(move || test_formula(f, s.depth, s.max_plies))).collect();
        hs.into_iter().map(|h| h.join().expect("test")).collect()
    });
    let pts = |x: &[[usize; 3]; 2]| (x[0][0] + x[1][0]) as f64 + (x[0][1] + x[1][1]) as f64 / 2.0;
    let mut order_: Vec<usize> = (0..cands.len()).collect();
    order_.sort_by(|&a, &b| (pts(&tests[b]) - NODE_PRICE * cands[b].size() as f64).total_cmp(&(pts(&tests[a]) - NODE_PRICE * cands[a].size() as f64)));
    let best = cands[order_[0]].clone();
    let mut out = String::new();
    out.push_str(&format!(
        "\nNuome breeds the golden function: formulas over {} atoms the rules define ({})\n{} islands x {} formulas, {} generations; games at depth {}, at most {} plies (unfinished = draw), {} openings per baseline per generation\nfitness: game points only (win 1, draw 1/2) minus {} per node\n\n",
        N_ATOMS,
        ATOMS.join(", "),
        s.islands,
        s.population,
        s.generations,
        s.depth,
        s.max_plies,
        s.openings,
        NODE_PRICE
    ));
    out.push_str("final test, the islands' best (8 openings x both colours, fixed seeds; won/drawn/lost):\n");
    for &i in order_.iter().take(8) {
        out.push_str(&format!("  vs random {}  vs mobility {}   {}\n", show_wdl(&tests[i][0]), show_wdl(&tests[i][1]), cands[i].show()));
    }
    let d = test_formula(&default_formula(), s.depth, s.max_plies);
    out.push_str(&format!("for comparison, the default {}: vs random {}  vs mobility {}\n", default_formula().show(), show_wdl(&d[0]), show_wdl(&d[1])));
    out.push_str(&format!("\nbest: {}\n({:.1} s)\n", best.show(), t0.elapsed().as_secs_f64()));
    Outcome { best, report: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    // published perft numbers (chessprogramming.org)
    #[test]
    fn perft_start() {
        let b = Board::start();
        assert_eq!([perft(&b, 1), perft(&b, 2), perft(&b, 3), perft(&b, 4)], [20, 400, 8902, 197281]);
    }

    #[test]
    fn perft_kiwipete() {
        let b = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!([perft(&b, 1), perft(&b, 2), perft(&b, 3)], [48, 2039, 97862]);
    }

    #[test]
    fn perft_position3_en_passant_and_checks() {
        let b = Board::from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1").unwrap();
        assert_eq!([perft(&b, 1), perft(&b, 2), perft(&b, 3), perft(&b, 4)], [14, 191, 2812, 43238]);
    }

    #[test]
    fn perft_position4_promotions_and_castling() {
        let b = Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1").unwrap();
        assert_eq!([perft(&b, 1), perft(&b, 2), perft(&b, 3)], [6, 264, 9467]);
    }

    #[test]
    fn perft_position5() {
        let b = Board::from_fen("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8").unwrap();
        assert_eq!([perft(&b, 1), perft(&b, 2), perft(&b, 3)], [44, 1486, 62379]);
    }

    #[test]
    fn fen_round_trip() {
        let f = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        assert_eq!(Board::from_fen(f).unwrap().fen(), f);
    }

    #[test]
    fn formulas_read_back_what_they_show() {
        for text in ["my_moves - their_moves", "max(3 * my_queens, -2) + min(i_attack, they_attack * (halfmove - 1))", "my_pawns - (their_pawns - -4)", "0 - their_moves", "7"] {
            let f = F::parse(text).unwrap();
            assert_eq!(F::parse(&f.show()).unwrap(), f, "{text}");
        }
        assert_eq!(F::parse("a + b * c").is_err(), true);
        let f = F::parse("my_knights + 2 * their_rooks").unwrap();
        assert_eq!(f.show(), "my_knights + (2 * their_rooks)");
    }

    #[test]
    fn atoms_count_what_the_rules_say() {
        let b = Board::start();
        let a = atoms(&b, &b.moves(), u32::MAX);
        let get = |n: &str| a[ATOMS.iter().position(|x| *x == n).unwrap()];
        assert_eq!((get("my_moves"), get("their_moves"), get("my_pawns"), get("their_knights"), get("my_escapes"), get("i_attack")), (20, 20, 8, 2, 0, 0));
        // black king on g8 boxed in by its pawns, white rook checks from a8: no escapes, mate
        let b = Board::from_fen("R5k1/5ppp/8/8/8/8/8/6K1 b - - 0 1").unwrap();
        let a = atoms(&b, &b.moves(), u32::MAX);
        assert_eq!((a[0], a[2], a[3]), (0, 1, 0));
    }

    #[test]
    fn takes_a_mate_in_one() {
        let f = default_formula();
        let b = Board::from_fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
        assert_eq!(golden(&b, &[], &f).unwrap().mv.uci(), "a1a8");
        // with any formula: the rules score the mate, not the formula
        let g = F::parse("their_queens * 9").unwrap();
        assert_eq!(golden(&b, &[], &g).unwrap().mv.uci(), "a1a8");
    }

    #[test]
    fn evolution_smoke_test() {
        let s = Settings { generations: 2, population: 4, islands: 2, depth: 1, max_plies: 16, openings: 1, seed: 7 };
        let a = evolve(&s, &mut |_| {});
        let b = evolve(&s, &mut |_| {});
        assert_eq!(a.best, b.best);
        assert_eq!(F::parse(&a.best.show()).unwrap(), a.best);
    }
}
