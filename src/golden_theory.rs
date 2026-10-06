//! The Golden Theory rules: one rule for each principle of
//! `data/golden_theory.toml` marked `measure = "rule"`, under the same id.
//!
//! A rule reads what a move does by its principle, from the mover's side: a
//! number, larger when the move follows the principle better. Most rules are a
//! state of the board measured after the move minus the same before it (what
//! the move changes); the events (a fork, a check, a sacrifice) are read from
//! the position after the move alone. A pawn is about one unit.
//!
//! The rules only read the board; which of them matter, and how much, is each
//! grandmaster's (data/golden_grandmasters.toml).

use crate::fast::bitboard::{tables, SQ120_TO_64, SQ64_TO_120};
use crate::fast::movegen::legal_moves;
use crate::fast::position::{m_flag, m_from, m_promo, m_to, mk_move, Position};

/// Piece values by type (pawn, knight, bishop, rook, queen, king), in pawns.
const VAL: [f32; 6] = [1.0, 3.25, 3.25, 5.0, 9.75, 0.0];
const FILE_A: u64 = 0x0101_0101_0101_0101;
const CENTER: u64 = 0x0000_0018_1800_0000;
const DARK: u64 = 0xAA55_AA55_AA55_AA55;

/// A move seen by the rules: the board before and after it.
pub struct Seen {
    pub before: Position,
    pub after: Position,
    pub me: usize,
    pub from: usize,
    pub to: usize,
    /// the moved piece's type (0 pawn .. 5 king)
    pub ty: usize,
    pub flag: u32,
    pub promo: bool,
}

impl Seen {
    pub fn new(p: &Position, m: u32) -> Seen {
        let before = p.clone();
        let mut after = p.clone();
        let from = SQ120_TO_64[m_from(m) as usize] as usize;
        let to = SQ120_TO_64[m_to(m) as usize] as usize;
        let ty = before.type_at64(from).map(|t| t as usize - 1).unwrap_or(0);
        after.make(m);
        Seen { me: p.turn as usize, before, after, from, to, ty, flag: m_flag(m), promo: m_promo(m) != 0 }
    }
    /// What the move changes in a state, both sides counted: (mine - theirs) after minus the same before.
    /// Symmetric, so a threat counts for the one who makes it as much as parrying it counts for the other.
    fn delta(&self, f: impl Fn(&Position, usize) -> f32) -> f32 {
        (f(&self.after, self.me) - f(&self.after, 1 - self.me)) - (f(&self.before, self.me) - f(&self.before, 1 - self.me))
    }
}

fn piece_attacks(p: &Position, c: usize, ty: usize, s: usize) -> u64 {
    let t = tables();
    match ty {
        0 => if c == 0 { t.att.pawn_w[s] } else { t.att.pawn_b[s] },
        1 => t.att.knight[s],
        2 => t.bishop_attacks(s, p.occ),
        3 => t.rook_attacks(s, p.occ),
        4 => t.queen_attacks(s, p.occ),
        _ => t.att.king[s],
    }
}

fn squares(mut b: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || if b == 0 { None } else { let s = b.trailing_zeros() as usize; b &= b - 1; Some(s) })
}

/// Squares side `c` attacks, and those it attacks twice or more.
fn attacks2(p: &Position, c: usize) -> (u64, u64) {
    let (mut a1, mut a2) = (0u64, 0u64);
    for ty in 0..6 {
        for s in squares(p.bb[c * 6 + ty]) {
            let a = piece_attacks(p, c, ty, s);
            a2 |= a1 & a;
            a1 |= a;
        }
    }
    (a1, a2)
}

fn attacks(p: &Position, c: usize) -> u64 {
    attacks2(p, c).0
}

fn type_on(p: &Position, s: usize) -> usize {
    p.type_at64(s).map(|t| t as usize - 1).unwrap_or(6)
}

/// The cheapest piece of side `c` attacking square `s`: (its square, its type).
fn cheapest_attacker(p: &Position, c: usize, s: usize) -> Option<(usize, usize)> {
    (0..6).find_map(|ty| squares(p.bb[c * 6 + ty]).find(|&f| piece_attacks(p, c, ty, f) & (1u64 << s) != 0).map(|f| (f, ty)))
}

fn material(p: &Position, me: usize) -> f32 {
    (0..5).map(|t| VAL[t] * (p.bb[me * 6 + t].count_ones() as f32 - p.bb[(1 - me) * 6 + t].count_ones() as f32)).sum()
}

/// Knights, bishops, rooks and queens of both sides.
fn pieces(p: &Position) -> u32 {
    (1..5).map(|t| (p.bb[t] | p.bb[6 + t]).count_ones()).sum()
}

fn endgame(p: &Position) -> bool {
    pieces(p) <= 6 && (p.bb[4] | p.bb[10]).count_ones() <= 1
}

/// Rank counted from side `c`'s own back rank (0..7).
fn rel_rank(c: usize, s: usize) -> usize {
    if c == 0 { s / 8 } else { 7 - s / 8 }
}

fn adjacent_files(f: usize) -> u64 {
    (if f > 0 { FILE_A << (f - 1) } else { 0 }) | (if f < 7 { FILE_A << (f + 1) } else { 0 })
}

/// The squares in front of `s` for side `c` (same file), toward promotion.
fn ahead(c: usize, s: usize) -> u64 {
    let file = FILE_A << (s % 8);
    let r = s / 8;
    if c == 0 { file & !((1u64 << ((r + 1) * 8)) - 1) } else { file & ((1u64 << (r * 8)) - 1) }
}

fn passed(p: &Position, c: usize) -> Vec<usize> {
    let theirs = p.bb[(1 - c) * 6];
    squares(p.bb[c * 6])
        .filter(|&s| {
            let front = ahead(c, s);
            let span = front | squares(adjacent_files(s % 8) & 0xFF).fold(0, |a, f| a | ahead(c, (s / 8) * 8 + f % 8));
            theirs & span == 0
        })
        .collect()
}

fn king(p: &Position, c: usize) -> Option<usize> {
    let b = p.bb[c * 6 + 5];
    (b != 0).then(|| b.trailing_zeros() as usize)
}

fn ring(p: &Position, c: usize) -> u64 {
    king(p, c).map(|k| tables().att.king[k] | (1u64 << k)).unwrap_or(0)
}

fn their_legal(p: &Position, me: usize) -> usize {
    let mut q = p.clone();
    if q.turn as usize == me {
        q.make_null();
    }
    legal_moves(&mut q).len()
}

fn cheb(a: usize, b: usize) -> usize {
    ((a % 8) as i32 - (b % 8) as i32).unsigned_abs().max(((a / 8) as i32 - (b / 8) as i32).unsigned_abs()) as usize
}

/// The pawns in front of side `c`'s king (its file and the next, two ranks).
fn shield(p: &Position, c: usize) -> u32 {
    let Some(k) = king(p, c) else { return 0 };
    let files = (FILE_A << (k % 8)) | adjacent_files(k % 8);
    let r = k / 8;
    let rows: u64 = if c == 0 { (0xFFFFu64 << ((r + 1).min(7) * 8)) & !(if r + 3 <= 7 { !0u64 << ((r + 3) * 8) } else { 0 }) } else { let lo = r.saturating_sub(2); (0xFFFFu64 << (lo * 8)) & ((1u64 << (r * 8)) - 1) };
    (p.bb[c * 6] & files & rows).count_ones()
}

// ── the states (side `c`'s view of a board) ──

fn st_space(p: &Position, c: usize) -> f32 {
    let half = if c == 0 { 0xFFFF_FFFF_0000_0000u64 } else { 0x0000_0000_FFFF_FFFFu64 };
    (attacks(p, c) & half).count_ones() as f32 / 4.0
}

fn st_center(p: &Position, c: usize) -> f32 {
    (attacks(p, c) & CENTER).count_ones() as f32 / 2.0 + (p.by_color[c] & CENTER).count_ones() as f32 / 2.0
}

fn st_undefended(p: &Position, c: usize) -> f32 {
    let mine = attacks(p, c);
    let theirs = attacks(p, 1 - c);
    squares(p.by_color[c] & !p.bb[c * 6 + 5] & theirs & !mine).map(|s| VAL[type_on(p, s)]).sum::<f32>()
}

fn st_structure(p: &Position, c: usize) -> f32 {
    let pawns = p.bb[c * 6];
    let mut bad = 0.0;
    for f in 0..8 {
        let n = (pawns & (FILE_A << f)).count_ones();
        if n > 1 {
            bad += (n - 1) as f32 * 0.5;
        }
        if n > 0 && pawns & adjacent_files(f) == 0 {
            bad += n as f32 * 0.5;
        }
    }
    -bad
}

fn st_outposts(p: &Position, c: usize) -> f32 {
    let my_pawn_att = squares(p.bb[c * 6]).fold(0, |a, s| a | piece_attacks(p, c, 0, s));
    squares(p.bb[c * 6 + 1])
        .filter(|&s| {
            let r = rel_rank(c, s);
            let attackable = squares(adjacent_files(s % 8) & 0xFF).fold(0, |a, f| a | ahead(c, (s / 8) * 8 + f % 8)) & p.bb[(1 - c) * 6];
            (3..=5).contains(&r) && attackable == 0 && my_pawn_att & (1u64 << s) != 0
        })
        .count() as f32
}

fn st_bishops_open(p: &Position, c: usize) -> f32 {
    let open = 1.0 - (p.bb[0] | p.bb[6]).count_ones() as f32 / 16.0;
    let pair = (p.bb[c * 6 + 2].count_ones() >= 2) as u8 as f32;
    let reach: f32 = squares(p.bb[c * 6 + 2]).map(|s| piece_attacks(p, c, 2, s).count_ones() as f32).sum();
    pair * (0.5 + open) + reach / 8.0
}

fn st_bad_bishop(p: &Position, c: usize) -> f32 {
    -squares(p.bb[c * 6 + 2]).map(|s| { let colour = if DARK & (1u64 << s) != 0 { DARK } else { !DARK }; (p.bb[c * 6] & colour).count_ones() as f32 / 4.0 }).sum::<f32>()
}

fn st_rook_seventh(p: &Position, c: usize) -> f32 {
    let seventh = if c == 0 { 0x00FF_0000_0000_0000u64 } else { 0xFF00u64 };
    let eighth = if c == 0 { 0xFF00_0000_0000_0000u64 } else { 0xFFu64 };
    let worth = p.bb[(1 - c) * 6] & seventh != 0 || p.bb[(1 - c) * 6 + 5] & eighth != 0;
    if worth { ((p.bb[c * 6 + 3] | p.bb[c * 6 + 4]) & seventh).count_ones() as f32 } else { 0.0 }
}

fn st_rook_behind_passer(p: &Position, c: usize) -> f32 {
    let mut n = 0.0;
    for side in [c, 1 - c] {
        for s in passed(p, side) {
            let behind = ahead(1 - side, s);
            let rooks = squares(p.bb[c * 6 + 3] & behind).filter(|&r| tables().rook_attacks(r, p.occ) & (1u64 << s) != 0).count();
            n += rooks as f32;
        }
    }
    n
}

fn st_king_safety(p: &Position, c: usize) -> f32 {
    if p.bb[(1 - c) * 6 + 4] == 0 {
        return 0.0;
    }
    let danger = (attacks(p, 1 - c) & ring(p, c)).count_ones() as f32;
    let k = king(p, c).unwrap_or(0);
    let home = rel_rank(c, k) == 0 && (k % 8 <= 2 || k % 8 >= 6);
    shield(p, c) as f32 * 0.5 + if home { 1.0 } else { 0.0 } - danger / 2.0
}

fn st_development(p: &Position, c: usize) -> f32 {
    let back = if c == 0 { 0xFFu64 } else { 0xFF00_0000_0000_0000 };
    let minors = p.bb[c * 6 + 1] | p.bb[c * 6 + 2];
    (minors & !back).count_ones() as f32
}

fn st_shield(p: &Position, c: usize) -> f32 {
    let k = king(p, c).unwrap_or(0);
    if rel_rank(c, k) <= 1 && (k % 8 <= 2 || k % 8 >= 5) { shield(p, c) as f32 } else { 0.0 }
}

fn st_coordination(p: &Position, c: usize) -> f32 {
    let half = if c == 0 { 0xFFFF_FFFF_0000_0000u64 } else { 0x0000_0000_FFFF_FFFFu64 };
    (attacks2(p, c).1 & half).count_ones() as f32 / 2.0
}

fn st_hypermodern(p: &Position, c: usize) -> f32 {
    let big = 0x0000_3C3C_3C3C_0000u64;
    let pawns = (p.bb[c * 6] & CENTER).count_ones() as f32;
    let afar: u32 = (1..5).flat_map(|ty| squares(p.bb[c * 6 + ty] & !big).map(move |s| (ty, s))).map(|(ty, s)| (piece_attacks(p, c, ty, s) & CENTER).count_ones()).sum();
    pawns + afar as f32 / 2.0
}

fn st_attack_king(p: &Position, c: usize) -> f32 {
    let zone = ring(p, 1 - c);
    let mut near = 0;
    let mut hit = 0u64;
    for ty in 1..5 {
        for s in squares(p.bb[c * 6 + ty]) {
            let a = piece_attacks(p, c, ty, s) & zone;
            if a != 0 {
                near += 1;
                hit |= a;
            }
        }
    }
    near as f32 + hit.count_ones() as f32 / 2.0
}

fn st_king_active(p: &Position, c: usize) -> f32 {
    if !endgame(p) {
        return 0.0;
    }
    let k = king(p, c).unwrap_or(0);
    let (f, r) = ((k % 8) as f32, (k / 8) as f32);
    (7.0 - ((f - 3.5).abs() + (r - 3.5).abs())) / 2.0
}

fn st_passed(p: &Position, c: usize) -> f32 {
    let own: f32 = passed(p, c).iter().map(|&s| rel_rank(c, s) as f32 / 2.0).sum();
    let theirs: f32 = passed(p, 1 - c).iter().map(|&s| rel_rank(1 - c, s) as f32 / 2.0).sum();
    own - theirs
}

fn st_outside(p: &Position, c: usize) -> f32 {
    if !endgame(p) {
        return 0.0;
    }
    let theirs: Vec<usize> = squares(p.bb[(1 - c) * 6]).map(|s| s % 8).collect();
    let (Some(lo), Some(hi)) = (theirs.iter().min(), theirs.iter().max()) else { return 0.0 };
    passed(p, c).iter().filter(|&&s| s % 8 + 2 <= *lo || s % 8 >= hi + 2).count() as f32
}

fn st_connected(p: &Position, c: usize) -> f32 {
    let ps = passed(p, c);
    ps.iter().filter(|&&a| ps.iter().any(|&b| (a % 8) + 1 == b % 8)).map(|&a| 1.0 + rel_rank(c, a) as f32 / 4.0).sum()
}

/// Passed pawns of `c` the enemy king cannot catch (the rule of the square), in pawn endings.
fn runaways(p: &Position, c: usize) -> f32 {
    if pieces(p) != 0 {
        return 0.0;
    }
    let Some(k) = king(p, 1 - c) else { return 0.0 };
    // the defender to move steps into the square at once
    let slack = if p.turn as usize == 1 - c { 1 } else { 0 };
    passed(p, c)
        .iter()
        .filter(|&&s| {
            let r = rel_rank(c, s);
            let steps = (7 - r).min(5);
            let queen = if c == 0 { 56 + s % 8 } else { s % 8 };
            cheb(k, queen) > steps + slack
        })
        .count() as f32
}

fn st_square(p: &Position, c: usize) -> f32 {
    runaways(p, c) - runaways(p, 1 - c)
}

fn st_pawn_ending(p: &Position, c: usize) -> f32 {
    if pieces(p) != 0 {
        return 0.0;
    }
    p.bb[c * 6].count_ones() as f32 - p.bb[(1 - c) * 6].count_ones() as f32
}

fn st_rook_ending(p: &Position, c: usize) -> f32 {
    let only_rooks = (1..5).all(|t| t == 3 || (p.bb[t] | p.bb[6 + t]) == 0) && p.bb[3] != 0 && p.bb[9] != 0;
    if !only_rooks {
        return 0.0;
    }
    let reach: u32 = squares(p.bb[c * 6 + 3]).map(|s| piece_attacks(p, c, 3, s).count_ones()).sum();
    let hits = (attacks(p, c) & p.bb[(1 - c) * 6]).count_ones();
    reach as f32 / 4.0 + hits as f32
}

fn st_pins(p: &Position, c: usize) -> f32 {
    crate::fast::see::pinned(p, 1 - c).count_ones() as f32 - crate::fast::see::pinned(p, c).count_ones() as f32
}

fn st_demolition(p: &Position, c: usize) -> f32 {
    -(shield(p, 1 - c) as f32)
}

/// Side `c`'s king boxed in on its back rank (its own pieces or attacks cover every square forward).
fn boxed(p: &Position, c: usize) -> bool {
    let Some(k) = king(p, c) else { return false };
    if rel_rank(c, k) != 0 {
        return false;
    }
    let forward = tables().att.king[k] & if c == 0 { 0xFF00u64 } else { 0x00FF_0000_0000_0000 };
    forward & !p.by_color[c] & !attacks(p, 1 - c) == 0
}

fn st_back_rank(p: &Position, c: usize) -> f32 {
    let heavy = |s: usize| (p.bb[s * 6 + 3] | p.bb[s * 6 + 4]).count_ones() as f32;
    (if boxed(p, 1 - c) { heavy(c) } else { 0.0 }) - (if boxed(p, c) { heavy(1 - c) } else { 0.0 })
}

fn st_hanging(p: &Position, c: usize) -> f32 {
    let defended = attacks(p, 1 - c);
    let mut won = 0.0;
    for s in squares(p.by_color[1 - c] & !p.bb[(1 - c) * 6 + 5]) {
        if let Some((_, a)) = cheapest_attacker(p, c, s) {
            let v = VAL[type_on(p, s)];
            if defended & (1u64 << s) == 0 { won += v } else if VAL[a] < v { won += v - VAL[a] }
        }
    }
    won
}

// ── the events ──

fn checkers(s: &Seen) -> u32 {
    let p = &s.after;
    let Some(k) = king(p, 1 - s.me) else { return 0 };
    (0..6).map(|ty| squares(p.bb[s.me * 6 + ty]).filter(|&f| piece_attacks(p, s.me, ty, f) & (1u64 << k) != 0).count() as u32).sum()
}

/// The material the move gives up at once: what their cheapest capture of the moved piece wins, less what it took.
fn given(s: &Seen) -> f32 {
    let took = material(&s.before, 1 - s.me) - material(&s.after, 1 - s.me);
    let p = &s.after;
    let Some((f, _)) = cheapest_attacker(p, 1 - s.me, s.to) else { return 0.0 };
    let m = mk_move(SQ64_TO_120[f], SQ64_TO_120[s.to], 0, 1);
    let loss = crate::fast::see::see_side(p, m, 1 - s.me) as f32 / 100.0;
    (loss - took).max(0.0)
}

fn ev_initiative(s: &Seen) -> f32 {
    let (g, _) = crate::fast::see::threats_against(&s.after, 1 - s.me);
    g as f32 / 100.0 + if s.after.in_check() { 1.0 } else { 0.0 }
}

fn ev_exchanges(s: &Seen) -> f32 {
    let traded = pieces(&s.before) as f32 - pieces(&s.after) as f32;
    let ahead = material(&s.before, s.me) >= 1.5;
    let cramped = st_space(&s.before, s.me) < st_space(&s.before, 1 - s.me);
    if ahead || cramped { traded } else { 0.0 }
}

fn ev_queen_early(s: &Seen) -> f32 {
    let back = if s.me == 0 { 0xFFu64 } else { 0xFF00_0000_0000_0000 };
    let undeveloped = ((s.before.bb[s.me * 6 + 1] | s.before.bb[s.me * 6 + 2]) & back).count_ones();
    if s.ty == 4 && undeveloped >= 3 { -1.0 } else { 0.0 }
}

fn ev_castle(s: &Seen) -> f32 {
    let rights = if s.me == 0 { 3 } else { 12 };
    if s.flag == 3 {
        1.0
    } else if s.before.castle & rights != 0 && s.after.castle & rights == 0 {
        -1.0
    } else {
        0.0
    }
}

fn ev_combinations(s: &Seen) -> f32 {
    // a new threat: larger than the one already standing before the move
    let mut q = s.before.clone();
    q.make_null();
    let standing = crate::fast::see::threats_against(&q, 1 - s.me).0;
    let threat = crate::fast::see::threats_against(&s.after, 1 - s.me).0;
    let forcing = s.after.in_check() || s.flag == 1 || s.flag == 2 || (threat >= 100 && threat > standing);
    if forcing && pieces(&s.after) >= 8 { 1.0 } else { 0.0 }
}

fn ev_promotion(s: &Seen) -> f32 {
    if !endgame(&s.before) || s.ty != 0 {
        return 0.0;
    }
    if s.promo { 5.0 } else { 0.5 + rel_rank(s.me, s.to) as f32 / 4.0 }
}

fn ev_fork(s: &Seen) -> f32 {
    let p = &s.after;
    let a = piece_attacks(p, s.me, s.ty, s.to);
    let defended = attacks(p, 1 - s.me);
    let targets = squares(a & p.by_color[1 - s.me])
        .filter(|&t| { let ty = type_on(p, t); ty == 5 || VAL[ty] > VAL[s.ty] || defended & (1u64 << t) == 0 })
        .count();
    if targets >= 2 && given(s) < VAL[s.ty] { targets as f32 } else { 0.0 }
}

fn ev_double_check(s: &Seen) -> f32 {
    if checkers(s) >= 2 { 1.0 } else { 0.0 }
}

fn ev_sacrifice(s: &Seen) -> f32 {
    if given(s) >= 2.0 { 1.0 } else { 0.0 }
}

/// A rule: what the move does by one principle.
pub type Rule = fn(&Seen) -> f32;

/// Every rule, under its principle's id in data/golden_theory.toml.
pub const RULES: [(&str, Rule); 37] = [
    ("material", |s| s.delta(material)),
    ("space", |s| s.delta(st_space)),
    ("center", |s| s.delta(st_center)),
    ("initiative", ev_initiative),
    ("defend_in_advance", |s| -s.delta(st_undefended)),
    ("exchanges", ev_exchanges),
    ("restriction", |s| -(their_legal(&s.after, s.me) as f32 - their_legal(&s.before, s.me) as f32) / 4.0),
    ("pawn_structure", |s| s.delta(st_structure)),
    ("knight_outpost", |s| s.delta(st_outposts)),
    ("bishops_open", |s| s.delta(st_bishops_open)),
    ("bad_bishop", |s| s.delta(st_bad_bishop)),
    ("rook_seventh", |s| s.delta(st_rook_seventh)),
    ("rook_behind_passer", |s| s.delta(st_rook_behind_passer)),
    ("queen_not_early", ev_queen_early),
    ("king_safety_middlegame", |s| s.delta(st_king_safety)),
    ("development", |s| s.delta(st_development)),
    ("castle_early", ev_castle),
    ("pawn_shield", |s| s.delta(st_shield)),
    ("coordination", |s| s.delta(st_coordination)),
    ("classical_hypermodern", |s| s.delta(st_hypermodern)),
    ("combinations", ev_combinations),
    ("attack_king", |s| s.delta(st_attack_king)),
    ("king_active", |s| s.delta(st_king_active)),
    ("passed_pawn", |s| s.delta(st_passed)),
    ("outside_passer", |s| s.delta(st_outside)),
    ("connected_passers", |s| s.delta(st_connected)),
    ("promotion_goal", ev_promotion),
    ("rule_of_square", |s| s.delta(st_square)),
    ("extra_pawn_decides", |s| s.delta(st_pawn_ending)),
    ("rook_endgame_initiative", |s| s.delta(st_rook_ending)),
    ("fork", ev_fork),
    ("pin", |s| s.delta(st_pins)),
    ("double_check", ev_double_check),
    ("sacrifice", ev_sacrifice),
    ("pawn_demolition", |s| s.delta(st_demolition)),
    ("back_rank", |s| s.delta(st_back_rank)),
    ("hanging", |s| s.delta(st_hanging)),
];

/// A state of the board for one side, in pawns.
pub type State = fn(&Position, usize) -> f32;

/// The state rules as states, under their ids: a rule `s.delta(f)` is
/// `f(after) - f(before)` (both sides counted), so along a line the deltas
/// sum to the state at its end. The event rules (a fork, a check, a
/// sacrifice) have no state; a search sees those itself.
pub const STATES: [(&str, State); 28] = [
    ("material", material),
    ("space", st_space),
    ("center", st_center),
    ("defend_in_advance", |p, c| -st_undefended(p, c)),
    // the state is the cheap mobility (attacked squares not held by one's own side), not the legal count the move rule uses: a search evaluates millions of positions
    ("restriction", |p, c| (attacks(p, c) & !p.by_color[c]).count_ones() as f32 / 4.0),
    ("pawn_structure", st_structure),
    ("knight_outpost", st_outposts),
    ("bishops_open", st_bishops_open),
    ("bad_bishop", st_bad_bishop),
    ("rook_seventh", st_rook_seventh),
    ("rook_behind_passer", st_rook_behind_passer),
    ("king_safety_middlegame", st_king_safety),
    ("development", st_development),
    ("pawn_shield", st_shield),
    ("coordination", st_coordination),
    ("classical_hypermodern", st_hypermodern),
    ("attack_king", st_attack_king),
    ("king_active", st_king_active),
    ("passed_pawn", st_passed),
    ("outside_passer", st_outside),
    ("connected_passers", st_connected),
    ("rule_of_square", st_square),
    ("extra_pawn_decides", st_pawn_ending),
    ("rook_endgame_initiative", st_rook_ending),
    ("pin", st_pins),
    ("pawn_demolition", st_demolition),
    ("back_rank", st_back_rank),
    ("hanging", st_hanging),
];

/// The weighted states of `p` from `me`, both sides counted: sum of w x (state(me) - state(them)).
pub fn state_score(ws: &[(usize, f32)], p: &Position, me: usize) -> f32 {
    ws.iter().map(|&(i, w)| { let f = STATES[i].1; w * (f(p, me) - f(p, 1 - me)) }).sum()
}

pub fn rule(id: &str) -> Option<Rule> {
    RULES.iter().find(|r| r.0 == id).map(|r| r.1)
}

/// What move `uci` in `fen` does by principle `id`.
pub fn measure(fen: &str, uci: &str, id: &str) -> Result<f32, String> {
    let mut p = Position::from_fen(fen)?;
    let m = crate::fast::movegen::uci_to_move(&mut p, uci).ok_or(format!("illegal {uci}"))?;
    Ok(rule(id).ok_or(format!("no rule {id}"))?(&Seen::new(&p, m)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (principle, position, the move that follows it, a move that does not)
    const CASES: &[(&str, &str, &str, &str)] = &[
        ("material", "4k3/8/8/3p4/4P3/8/8/4K3 w - - 0 1", "e4d5", "e4e5"),
        ("space", "4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1", "e2e4", "h2h3"),
        ("center", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "e2e4", "a2a3"),
        ("initiative", "4k3/8/8/3n4/8/8/8/R3K3 w - - 0 1", "a1a5", "a1a2"),
        ("defend_in_advance", "4k3/8/8/3r4/8/8/3N4/4K3 w - - 0 1", "e1e2", "e1f1"),
        ("exchanges", "4k3/8/8/3r4/8/8/3R4/R3K3 w - - 0 1", "d2d5", "d2d3"),
        ("restriction", "7k/8/8/8/8/8/8/R3K3 w - - 0 1", "a1a7", "a1a2"),
        ("pawn_structure", "4k3/8/8/8/8/2p5/PP1P4/4K3 w - - 0 1", "d2c3", "b2c3"),
        ("knight_outpost", "4k3/8/8/8/3P4/5N2/8/4K3 w - - 0 1", "f3e5", "f3h4"),
        ("bishops_open", "4k3/8/8/8/8/8/8/2B1KB2 w - - 0 1", "f1c4", "f1e2"),
        ("bad_bishop", "4k3/8/8/8/8/3P1P2/4B3/4K3 w - - 0 1", "d3d4", "e1d1"),
        ("rook_seventh", "4k3/p7/8/8/8/8/8/R3K3 w - - 0 1", "a1a7", "a1a2"),
        ("rook_behind_passer", "4k3/8/8/8/P7/8/8/1R2K3 w - - 0 1", "b1a1", "b1b4"),
        ("queen_not_early", "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2", "g1f3", "d1h5"),
        ("king_safety_middlegame", "r2qk2r/pppppppp/8/8/8/8/PPPPPPPP/R2QK2R w KQkq - 0 1", "e1g1", "e1f1"),
        ("development", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "g1f3", "a2a3"),
        ("castle_early", "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1", "e1g1", "e1f1"),
        ("pawn_shield", "6k1/8/8/8/8/8/5PPP/6K1 w - - 0 1", "h2h3", "g2g4"),
        ("coordination", "4k3/8/8/8/8/8/R7/3RK3 w - - 0 1", "a2a5", "a2a3"),
        ("classical_hypermodern", "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "e2e4", "h2h3"),
        ("combinations", "r1b1kbnr/8/8/8/8/8/8/RNBQKBNR w - - 0 1", "a1a8", "b1c3"),
        ("attack_king", "6k1/8/8/8/8/8/8/3QK3 w - - 0 1", "d1d5", "d1d2"),
        ("king_active", "8/8/8/4k3/8/8/8/K7 w - - 0 1", "a1b2", "a1a2"),
        ("passed_pawn", "4k3/8/8/8/P7/8/8/4K3 w - - 0 1", "a4a5", "e1d1"),
        ("outside_passer", "4k3/8/8/1p3ppp/2P5/8/5PPP/4K3 w - - 0 1", "c4b5", "f2f3"),
        ("connected_passers", "4k3/8/8/1p6/P1P5/8/8/4K3 w - - 0 1", "a4b5", "c4c5"),
        ("promotion_goal", "4k3/8/8/8/P7/8/8/4K3 w - - 0 1", "a4a5", "e1d2"),
        ("rule_of_square", "8/8/8/8/p7/8/8/4K2k w - - 0 1", "e1d2", "e1f2"),
        ("extra_pawn_decides", "4k3/4p3/8/8/8/8/3PP3/3rK3 w - - 0 1", "e1d1", "e1f2"),
        ("rook_endgame_initiative", "4k2r/pp6/8/8/8/8/8/R3K3 w - - 0 1", "a1a5", "a1b1"),
        ("fork", "r3k3/8/8/1N6/8/8/8/4K3 w - - 0 1", "b5c7", "b5a3"),
        ("pin", "4k3/8/8/4n3/8/8/8/K2R4 w - - 0 1", "d1e1", "d1f1"),
        ("double_check", "4k3/8/8/8/8/8/4B3/K3R3 w - - 0 1", "e2b5", "e2d3"),
        ("sacrifice", "6k1/5ppp/8/8/2B5/8/5PPP/6K1 w - - 0 1", "c4f7", "c4d3"),
        ("pawn_demolition", "6k1/5ppp/8/8/8/8/8/R2B2KR w - - 0 1", "h1h7", "h1h2"),
        ("back_rank", "r5k1/5ppp/8/8/8/8/5PPP/6K1 w - - 0 1", "h2h3", "g1h1"),
        ("hanging", "4k3/8/8/2n5/8/8/8/4K2R w - - 0 1", "h1h5", "h1h2"),
    ];

    #[test]
    fn every_rule_has_a_case_and_every_case_holds() {
        for (id, _) in RULES.iter() {
            assert!(CASES.iter().any(|c| c.0 == *id), "no case for {id}");
        }
        let mut bad = Vec::new();
        for &(id, fen, good, worse) in CASES {
            let (a, b) = (measure(fen, good, id), measure(fen, worse, id));
            match (a, b) {
                (Ok(a), Ok(b)) if a > b => {}
                (a, b) => bad.push(format!("{id}: {good} {a:?} vs {worse} {b:?}")),
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    #[test]
    fn every_measured_principle_has_its_rule() {
        let text = std::fs::read_to_string("data/golden_theory.toml").unwrap();
        let t: toml::Value = toml::from_str(&text).unwrap();
        for p in t["principle"].as_array().unwrap() {
            if p["measure"].as_str() == Some("rule") {
                let id = p["id"].as_str().unwrap();
                assert!(rule(id).is_some(), "{id} is marked rule but has none");
            }
        }
    }
}
