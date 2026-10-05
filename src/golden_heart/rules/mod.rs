//! The rules of Golden heart, one decision a file. Each rule reads its own
//! inputs from a position (seen from the side that just moved) into its own
//! range of the input vector; what each input is worth is not written here:
//! it is measured from Stockfish's games (listen.rs) after every game has
//! been heard. A rule offers variants of how strongly it speaks; the beam
//! search chooses among them and the judges veto what cannot be right.

use crate::golden::Board;

pub mod activity;
pub mod king;
pub mod material;
pub mod pawns;
pub mod placement;
pub mod safety;

/// One rule: its name, its input range, and its inputs for a position seen from `me`.
pub struct Rule {
    pub name: &'static str,
    pub start: usize,
    pub len: usize,
    pub read: fn(&Board, bool, &Shared, &mut Vec<(u16, f32)>),
    /// what each input is called, for the report and rules file
    pub label: fn(usize) -> String,
}

/// What several rules read, computed once a position: who strikes each square.
pub struct Shared {
    /// attackers on each square: [white, black]
    pub edges: [[u8; 64]; 2],
}

impl Shared {
    pub fn of(b: &Board) -> Shared {
        Shared { edges: crate::golden::attack_edges(b) }
    }
}

pub const RULES: [Rule; 6] = [material::RULE, placement::RULE, activity::RULE, safety::RULE, king::RULE, pawns::RULE];

/// The whole input vector's length.
pub fn width() -> usize {
    RULES.iter().map(|r| r.start + r.len).max().unwrap_or(0)
}

/// Every rule's inputs for the position `b` seen from `me` (sparse: index, value).
pub fn read_all(b: &Board, me: bool) -> Vec<(u16, f32)> {
    read_all_with(b, me, &Shared::of(b))
}

/// The same, with the strikes already counted (from the fast board's bitboards).
pub fn read_all_with(b: &Board, me: bool, sh: &Shared) -> Vec<(u16, f32)> {
    let mut out = Vec::with_capacity(48);
    for r in &RULES {
        (r.read)(b, me, sh, &mut out);
    }
    out
}

/// The variants a rule offers: how strongly it speaks (0 = silent).
pub const VARIANTS: [f32; 4] = [0.0, 0.5, 1.0, 2.0];

/// Kaufman's piece worth, for the rules that weigh what they count (pawn 1).
pub fn worth(t: i8) -> f32 {
    match t.abs() {
        1 => 1.0,
        2 | 3 => 3.25,
        4 => 5.0,
        5 => 9.75,
        _ => 0.0,
    }
}

/// The square seen from `me` (rank flipped for black).
pub fn seen(n: usize, me: bool) -> usize {
    if me {
        n
    } else {
        (7 - n / 8) * 8 + n % 8
    }
}

pub fn mine(p: i8, me: bool) -> bool {
    p != 0 && (p > 0) == me
}
