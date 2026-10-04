//! The supergenius writes the golden function, a new one for every
//! position, from that position's own network: the board is a closed
//! network of 64 nodes, the rules draw its edges, and every move changes
//! it. Nobody tells the supergenius what kind of position it is looking at;
//! the formula's numbers come out of the network itself:
//!
//! 1. a piece is worth its reach: the edges it has now in this network,
//!    together with what the rules give it on an open board (lines open as
//!    the game goes on) - so a shut-in bishop is worth less than an active one;
//! 2. a pawn is a queen in waiting: with a clear path, worth the queen's
//!    worth halved per step still needed; a blocked path is worth little;
//! 3. a pawn the enemy king cannot reach in time (its path clear, the king
//!    too far from the queening node) is as good as a queen;
//! 4. a king standing on a key node in front of its pawn escorts it home;
//! 5. one legal move is one unit of the same currency (mobility);
//! 6. the side to move may take now (its best capture);
//! 7. mate is the enemy king's node cut off: every free edge it keeps counts against.
//!
//! The golden function: in a position, the formula written for it, and
//! the legal move after which that formula is best for the mover (a mating
//! move wins, the draw rules give 0). In a pawn ending the paths and the race
//! carry the weight, because that is what the network holds; in a middle
//! game the reaches do. "It plays correctly in every position" stands until
//! a counterexample (tools/golden_tablebase.py, tools/golden_games.py).

use crate::golden::{reach, rule_mobility, Board, F};

/// The golden function for `b`: its formula, and how each number came out of `b`'s network.
pub fn function_for(b: &Board) -> (F, Vec<String>) {
    let unit = 256.0;
    let mut sum = [0usize; 7];
    let mut cnt = [0usize; 7];
    for s in 0..64 {
        let p = b.sq[s];
        if p != 0 {
            let t = p.unsigned_abs() as usize;
            sum[t] += reach(b, s);
            cnt[t] += 1;
        }
    }
    let names = ["", "pawn", "knight", "bishop", "rook", "queen", "king"];
    let mut lines = Vec::new();
    let mut worth = [0i64; 7];
    for t in 2..=5 {
        let open = rule_mobility(t as i8);
        let (now, note) = if cnt[t] > 0 {
            let r = sum[t] as f64 / cnt[t] as f64;
            (r, format!("{} on the board reach {:.2} edges now", cnt[t], r))
        } else {
            (open, "none on the board".to_string())
        };
        worth[t] = (unit * (now + open) / 2.0).round() as i64;
        lines.push(format!("  {:<7} {note}; open board {open:.2}; worth {}", names[t], worth[t]));
    }
    let q = worth[5];
    let pawn = (q / 64).max(1);
    let race = q;
    let key = q / 8;
    let u = unit as i64;
    lines.push(format!("  pawn    clear path: {pawn} x 2^(6 - steps left) (the queen's worth halved per step); blocked: {pawn}"));
    lines.push(format!("  a pawn the enemy king cannot catch: {race} (a queen); own king on a key node in front of its pawn: {key}"));
    lines.push(format!("  one legal move: {u}; best capture now: its worth; every free edge of the enemy king: -{u}"));
    let text = format!(
        "{u} * (my_moves - their_moves) + {} * (my_knights - their_knights) + {} * (my_bishops - their_bishops) + {} * (my_rooks - their_rooks) + {q} * (my_queens - their_queens) + {pawn} * (my_pawn_power - their_pawn_power) + {race} * (my_unstoppable - their_unstoppable) + {key} * (my_key_squares - their_key_squares) + my_best_capture - {u} * their_escapes",
        worth[2], worth[3], worth[4]
    );
    (F::parse(&text).expect("the supergenius writes formulas that read"), lines)
}

/// The supergenius on one position: its network, its formula, and the move.
pub fn report(fen: &str) -> Result<String, String> {
    let b = if fen.trim().is_empty() || fen.trim() == "startpos" { Board::start() } else { Board::from_fen(fen.trim())? };
    let (f, lines) = function_for(&b);
    let mut out = format!("Nuome, the supergenius, writes the golden function for {}\n\nfrom this position's network:\n", b.fen());
    for l in &lines {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&format!("\nTHE GOLDEN FUNCTION for this position:\n  {}\n", f.show()));
    match crate::golden::golden(&b, &[], &f) {
        Some(a) => out.push_str(&format!("the move: {}\n", a.mv.uci())),
        None => out.push_str(if b.in_check() { "no move: checkmate\n" } else { "no move: stalemate\n" }),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play(fen: &str) -> String {
        let b = Board::from_fen(fen).unwrap();
        crate::golden::golden(&b, &[], &function_for(&b).0).unwrap().mv.uci()
    }

    #[test]
    fn it_mates_and_takes_the_free_queen() {
        assert_eq!(play("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1"), "a1a8");
        assert_eq!(play("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1"), "d1d5");
    }

    #[test]
    fn the_formula_changes_with_the_position() {
        let a = function_for(&Board::start()).0;
        let b = function_for(&Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap()).0;
        assert_ne!(a, b);
    }
}
