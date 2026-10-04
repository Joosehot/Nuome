//! The supergenius writes the golden function: one formula for every chess
//! position, from the rules of chess alone, by reasoning, not by search
//! and not by evolution. It sees the board as a closed network of 64 nodes
//! (the squares); the rules draw the edges (where each piece may go or
//! strike), and every move changes the network. The golden function finds
//! the move that leaves the best network for the mover. Every number in it
//! is computed from the rules:
//!
//! 0. the nodes a side holds: more of its edges point at a node than the
//!    other side's; every node held is one node of the network won;
//! 1. a piece is worth what the rules let it do: the moves it has, on
//!    average, alone on an empty board (counted by the move generator);
//! 2. a pawn is a queen in waiting: the rules promote it on the last rank,
//!    and every step still needed is a step the opponent can stop, so its
//!    worth is the queen's halved per step left;
//! 3. a legal move is one unit of the same currency, so mobility and
//!    material add up;
//! 4. the rules let the side to move take an attacked piece now, and
//!    take back on a defended square;
//! 5. the game is won by mate: a king in check with no square to go to,
//!    so every escape square left to the enemy king counts against.
//!
//! The function: in a position, the legal move after which the formula is
//! best for the mover (a move that mates wins, the draw rules give 0).
//! "It plays correctly in every position" is a conjecture: it stands until
//! a counterexample (tools/golden_tablebase.py, tools/golden_games.py).

use crate::golden::{rule_mobility, Board, F};

/// The formula, built from the rule-derived numbers; and the reasoning.
pub fn derive() -> Result<(F, Vec<String>), String> {
    let unit = 256.0; // one move = 256, so every worth is a whole number
    let worth = |t: i8| rule_mobility(t) * unit;
    let names = [(2, "knight"), (3, "bishop"), (4, "rook"), (5, "queen"), (6, "king")];
    let mut lines = vec![
        "the board is a closed network of 64 nodes; the rules draw the edges (where each piece may go or strike); every move changes the network".to_string(),
        "0. a node is held by the side with more edges pointing at it: every node held counts one unit (256)".to_string(),
        "1. what a piece is worth, from the rules: its edges alone on an empty board (its moves), averaged over the 64 nodes".to_string(),
    ];
    for (t, n) in names {
        lines.push(format!("     {n:<7} {:>6.4} moves", rule_mobility(t)));
    }
    for t in 2..=5 {
        if worth(t).fract() != 0.0 {
            return Err(format!("piece {t}: worth {} is not whole at this unit", worth(t)));
        }
    }
    let (n, b, r, q) = (worth(2) as i64, worth(3) as i64, worth(4) as i64, worth(5) as i64);
    let unit = unit as i64;
    lines.push("     (the king cannot be captured, so it has no material worth: its moves count as mobility)".into());
    lines.push(format!(
        "2. a pawn is a queen in waiting: worth = queen / 2^(steps left); one step from queening {:.2} moves, at its start {:.2}",
        rule_mobility(5) / 2.0,
        rule_mobility(5) / 64.0
    ));
    // pawn power counts 2^(6 - steps left) per pawn, so the pawn's worth is (q / 64) * pawn power
    let pawn = q / 64;
    lines.push(format!("3. one legal move = {unit} (the same currency): knight {n}, bishop {b}, rook {r}, queen {q}, pawn {pawn} x 2^(6 - steps left)"));
    lines.push("4. the side to move may take now: its best capture (the victim, less the capturer on a defended square) counts for it".into());
    lines.push(format!("5. the game is won by mate: the enemy king's node cut off from the network; every free edge it keeps counts {unit} against"));
    let text = format!(
        "{unit} * (my_moves - their_moves) + {n} * (my_knights - their_knights) + {b} * (my_bishops - their_bishops) + {r} * (my_rooks - their_rooks) + {q} * (my_queens - their_queens) + {pawn} * (my_pawn_power - their_pawn_power) + my_best_capture - {unit} * their_escapes + {unit} * (my_nodes - their_nodes)"
    );
    let f = F::parse(&text)?;
    Ok((f, lines))
}

pub fn report() -> Result<String, String> {
    let (f, lines) = derive()?;
    let mut out = String::from("Nuome, the supergenius, writes the golden function from the rules of chess\n\n");
    for l in &lines {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&format!("\nTHE GOLDEN FUNCTION (value of a position for the side to move):\n  {}\n", f.show()));
    out.push_str("the move: the legal move after which this is best for the mover; a mating move wins, the draw rules give 0\n");
    let path = "out/golden/formula.txt";
    std::fs::write(path, format!("{}\n", f.show())).map_err(|e| format!("{path}: {e}"))?;
    out.push_str(&format!("written to {path}\n"));
    // a first look: a few positions everyone knows
    out.push_str("\nfirst look:\n");
    for (fen, what) in [
        ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", "the start"),
        ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", "back-rank mate in one"),
        ("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1", "a free queen"),
        ("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR b KQkq - 3 3", "Qxf7 mate is threatened"),
    ] {
        let b = Board::from_fen(fen)?;
        let a = crate::golden::golden(&b, &[], &f).ok_or("no move")?;
        out.push_str(&format!("  {what:<26} -> {}\n", a.mv.uci()));
    }
    out.push_str("\nconjecture: it plays correctly in every position, until a counterexample (python tools/golden_tablebase.py, tools/golden_games.py)\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_worths_come_out_of_the_rules() {
        assert_eq!(rule_mobility(2), 5.25);
        assert_eq!(rule_mobility(3), 8.75);
        assert_eq!(rule_mobility(4), 14.0);
        assert_eq!(rule_mobility(5), 22.75);
    }

    #[test]
    fn it_mates_and_takes_the_free_queen() {
        let (f, _) = derive().unwrap();
        let b = Board::from_fen("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1").unwrap();
        assert_eq!(crate::golden::golden(&b, &[], &f).unwrap().mv.uci(), "a1a8");
        let b = Board::from_fen("4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1").unwrap();
        assert_eq!(crate::golden::golden(&b, &[], &f).unwrap().mv.uci(), "d1d5");
    }
}
