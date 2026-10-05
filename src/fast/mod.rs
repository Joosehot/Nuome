//! Steldens' fast board, copied in for Golden Answer's search: magic
//! bitboards, a 120-square mailbox, pseudo-legal move generation with
//! captures-only mode, and make/unmake with an undo stack (no copying of
//! boards per node). Steldens' NNUE accumulators are not used here: the two
//! modules below are empty stand-ins so the copied files compile unchanged.

pub mod bitboard;
pub mod movegen;
pub mod position;
pub mod see;
pub mod starnnue;
pub mod zobrist;

/// Stand-in: no NNUE in Nuome.
pub mod nnue {
    pub const H1: usize = 1;
    pub type Acc = [[f32; H1]; 2];
    pub fn active() -> bool {
        false
    }
    pub fn refresh(_: &mut super::position::Position) {}
    pub struct Net;
    impl Net {
        pub fn remove_piece(&self, _: &mut Acc, _: u8, _: usize) {}
        pub fn add_piece(&self, _: &mut Acc, _: u8, _: usize) {}
    }
    pub fn net() -> Option<&'static Net> {
        None
    }
}

/// Stand-in: no lazy NNUE stack in Nuome.
pub mod nnue2 {
    pub struct Dirty {
        pub king: bool,
        pub n_rem: u8,
        pub rem: [(u8, usize); 3],
        pub add: (u8, usize),
        pub ks: [usize; 2],
    }
    #[derive(Default, Clone)]
    pub struct Nn2;
    impl Nn2 {
        pub fn push(&mut self, _: Dirty) {}
        pub fn pop(&mut self) {}
    }
    #[derive(Default, Clone)]
    pub struct Nn2Cell(Nn2);
    impl Nn2Cell {
        pub fn get_mut(&mut self) -> &mut Nn2 {
            &mut self.0
        }
    }
    pub fn live() -> Option<()> {
        None
    }
}

use position::{color_of, type_of, Position, EMPTY, OFF};

/// Nuome's board from the fast one.
pub fn board_of(p: &Position) -> crate::golden::Board {
    let mut sq = [0i8; 64];
    for (s, x) in sq.iter_mut().enumerate() {
        let pc = p.board[bitboard::SQ64_TO_120[s]];
        if pc != EMPTY && pc != OFF {
            let t = type_of(pc) as i8;
            *x = if color_of(pc) == 0 { t } else { -t };
        }
    }
    let ep = if p.ep >= 0 {
        let s = bitboard::SQ120_TO_64[p.ep as usize];
        (s >= 0).then_some(s as u8)
    } else {
        None
    };
    crate::golden::Board { sq, white: p.turn == 0, castle: p.castle, ep, half: p.halfmove, full: 1 }
}

/// How many pieces of each side strike each square ([white, black]), from the
/// bitboards: the same counts as golden::attack_edges (sliders up to and
/// including the first piece in their way).
pub fn edges_of(p: &Position) -> [[u8; 64]; 2] {
    let t = bitboard::tables();
    let mut e = [[0u8; 64]; 2];
    for c in 0..2 {
        for ty in 0..6 {
            let mut bb = p.bb[c * 6 + ty];
            while bb != 0 {
                let s = bb.trailing_zeros() as usize;
                bb &= bb - 1;
                let mut a = match ty {
                    0 => if c == 0 { t.att.pawn_w[s] } else { t.att.pawn_b[s] },
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, p.occ),
                    3 => t.rook_attacks(s, p.occ),
                    4 => t.queen_attacks(s, p.occ),
                    _ => t.att.king[s],
                };
                while a != 0 {
                    let q = a.trailing_zeros() as usize;
                    a &= a - 1;
                    e[c][q] += 1;
                }
            }
        }
    }
    e
}

#[cfg(test)]
mod tests {
    #[test]
    fn edges_match_nuomes_own() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            "2kr1bnr/ppqb2pp/2n1p3/3pp3/2pP4/P1P2N2/1PB1QPPP/RNB2RK1 w - - 0 11",
        ] {
            let p = super::position::Position::from_fen(fen).unwrap();
            let b = crate::golden::Board::from_fen(fen).unwrap();
            assert!(super::board_of(&p) == b || super::board_of(&p).sq == b.sq);
            assert_eq!(super::edges_of(&p), crate::golden::attack_edges(&b), "{fen}");
        }
    }
}
