//! The supergenius's eval: what a position is worth when nothing in it is
//! decided yet. Goldenboy solves positions and never writes functions; when
//! its tree reaches a node the rules do not decide, it asks this eval.
//!
//! The eval is the supergenius's work, from the rules and the network:
//! - the golden function the supergenius writes for the position itself
//!   (supergenius_golden::function_for: a piece's worth is its reach, a
//!   pawn a queen in waiting along a clear path, the race, the key nodes,
//!   one move one unit, the enemy king's free edges against), and
//! - the terms Nuome found the Goldbach way against the exact truth of the
//!   endgame tables (golden_terms, out/golden/terms.txt), if any are there.
//!
//! Mate, stalemate, the draw rules and the classes the supergenius has
//! solved whole are the rules' verdicts and come before the eval; goldenboy
//! applies those itself.

use crate::golden::{atoms, Board};

/// The worth of `b` for the side to move, in 1/256 moves.
pub fn eval(b: &Board) -> i64 {
    let ms = b.moves();
    let f = crate::supergenius_golden::function_for(b).0;
    let mut v = f.eval(&atoms(b, &ms, f.mask()));
    let terms = crate::golden_terms::terms();
    if !terms.is_empty() {
        // the terms file holds weights as seen after a move (the opponent to move), so here they add
        v += crate::golden_terms::term_sum(b, terms);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_is_even_and_a_queen_up_is_a_lot() {
        assert_eq!(eval(&Board::start()), 0);
        let b = Board::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();
        assert!(eval(&b) > 4000);
    }
}
