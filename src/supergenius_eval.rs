//! The golden heart: the supergenius's eval, what a position is worth when
//! nothing in it is decided yet. Two things only, as the supergenius's
//! intuition: material and tempo.
//!
//! - material: the pieces' worth as measured over millions of master games
//!   (Kaufman 1999): pawn 100, knight 325, bishop 325, rook 500, queen 975,
//!   the bishop pair +50;
//! - tempo: the legal moves the side to move has minus the other side's,
//!   3 cp a move (the order of Kaufman's mobility measurements), and 10 cp
//!   for having the move.
//!
//! In centipawns for the side to move. Mate, stalemate, the draw rules and
//! the classes the supergenius has solved whole are the rules' verdicts
//! and come before the eval; goldenboy applies those itself. Nothing here
//! needs the network: counts from the board, a few microseconds.

use crate::golden::Board;

pub const PAWN: i64 = 100;
pub const KNIGHT: i64 = 325;
pub const BISHOP: i64 = 325;
pub const ROOK: i64 = 500;
pub const QUEEN: i64 = 975;
pub const PAIR: i64 = 50;
pub const TEMPO_MOVE: i64 = 3;
pub const TEMPO_TURN: i64 = 10;

pub fn piece_worth(t: i8) -> i64 {
    match t.abs() {
        1 => PAWN,
        2 => KNIGHT,
        3 => BISHOP,
        4 => ROOK,
        5 => QUEEN,
        _ => 0,
    }
}

/// Material for the side to move, with the bishop pair.
pub fn material(b: &Board) -> i64 {
    let mut v = 0;
    let mut bishops = [0; 2];
    for &p in &b.sq {
        if p == 0 {
            continue;
        }
        let mine = if b.white { p > 0 } else { p < 0 };
        let s = if mine { 1 } else { -1 };
        v += s * piece_worth(p);
        if p.abs() == 3 {
            bishops[(!mine) as usize] += 1;
        }
    }
    if bishops[0] >= 2 {
        v += PAIR;
    }
    if bishops[1] >= 2 {
        v -= PAIR;
    }
    v
}

/// Tempo: my legal moves minus theirs (their moves counted with the turn
/// handed over), and the move itself.
pub fn tempo(b: &Board, my_moves: usize) -> i64 {
    let mut t = b.clone();
    t.white = !t.white;
    t.ep = None;
    let theirs = t.moves().len();
    TEMPO_MOVE * (my_moves as i64 - theirs as i64) + TEMPO_TURN
}

/// The worth of `b` for the side to move, in centipawns.
pub fn eval(b: &Board) -> i64 {
    let my_moves = b.moves().len();
    material(b) + tempo(b, my_moves)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_start_is_the_tempo_only_and_a_queen_up_is_a_queen() {
        assert_eq!(eval(&Board::start()), TEMPO_TURN);
        let b = Board::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").unwrap();
        assert!(eval(&b) > QUEEN);
    }

    #[test]
    fn eval_cost_per_node() {
        let b = Board::from_fen("r2q1rk1/5p2/p3p1pQ/1p2N3/2pPb3/2P2PR1/PP4PP/R5K1 b - - 0 1").unwrap();
        let n = 5000;
        let t = std::time::Instant::now();
        for _ in 0..n {
            std::hint::black_box(eval(&b));
        }
        eprintln!("eval: {:.1} us a node", t.elapsed().as_secs_f64() * 1e6 / n as f64);
    }
}
