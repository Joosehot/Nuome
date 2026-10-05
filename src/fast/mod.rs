//! Steldens' fast board, copied in for Golden Answer's search: magic
//! bitboards, a 120-square mailbox, pseudo-legal move generation with
//! captures-only mode, and make/unmake with an undo stack (no copying of
//! boards per node). Steldens' NNUE accumulators are not used here: the two
//! modules below are empty stand-ins so the copied files compile unchanged.

pub mod bitboard;
pub mod movegen;
pub mod position;
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
