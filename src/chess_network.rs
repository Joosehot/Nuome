//! How chess works, told as a network - and famous games laid into it.
//!
//! The board is a closed network of 64 nodes (the squares). The rules of
//! chess draw its edges: from every piece, an edge to each node it may move
//! to or strike. A move rewires the network: the moved piece's edges leave
//! with it, pieces it unblocked gain edges, pieces it blocks lose them, and
//! a capture deletes the captured piece's edges. `explain()` says how the
//! game works in these terms; `walk()` replays a game move by move and
//! shows what each move did to the network, ending in the mate: the enemy
//! king's node struck by an edge with no free edge left.

use crate::golden::{atoms, attack_edges, Board, ATOMS};

/// The famous games (UCI moves), each ending in mate.
pub const GAMES: [(&str, &str, &str); 4] = [
    ("fool", "Fool's mate (the shortest mate)", "f2f3 e7e5 g2g4 d8h4"),
    ("scholar", "Scholar's mate (queen and bishop on f7)", "e2e4 e7e5 f1c4 b8c6 d1h5 g8f6 h5f7"),
    ("legal", "Legal's mate (de Legal - Saint Brie, Paris 1750)", "e2e4 e7e5 g1f3 d7d6 f1c4 c8g4 b1c3 g7g6 f3e5 g4d1 c4f7 e8e7 c3d5"),
    (
        "opera",
        "the Opera Game (Morphy - Duke of Brunswick and Count Isouard, Paris 1858)",
        "e2e4 e7e5 g1f3 d7d6 d2d4 c8g4 d4e5 g4f3 d1f3 d6e5 f1c4 g8f6 f3b3 d8e7 b1c3 c7c6 c1g5 b7b5 c3b5 c6b5 c4b5 b8d7 e1c1 a8d8 d1d7 d8d7 h1d1 e7e6 b5d7 f6d7 b3b8 d7b8 d1d8",
    ),
];

fn sq_name(s: usize) -> String {
    format!("{}{}", (b'a' + (s % 8) as u8) as char, s / 8 + 1)
}

fn piece_name(p: i8) -> &'static str {
    ["", "pawn", "knight", "bishop", "rook", "queen", "king"][p.unsigned_abs() as usize]
}

/// How many edges a lone piece of type `t` has from node `s` (empty board).
fn lone_edges(t: i8, s: usize) -> usize {
    let mut b = Board::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").expect("empty board");
    b.sq[s] = t;
    attack_edges(&b)[0].iter().map(|&x| x as usize).sum()
}

/// The rules and the ideas of chess, in network terms.
pub fn explain() -> String {
    let mut out = String::from("HOW CHESS WORKS, AS A NETWORK\n\n");
    out.push_str("The network. 64 nodes (squares a1..h8). Each piece sits on a node; the rules give it edges:\n");
    out.push_str("  - a rook: edges along its row and column until the first piece (that piece's node is the last edge)\n");
    out.push_str("  - a bishop: the same along the diagonals; a queen: both; a knight: 8 jumps, never blocked;\n");
    out.push_str("  - a king: one step each way; a pawn: steps forward (one, or two from its start) but strikes diagonally forward.\n");
    out.push_str("  An edge to an empty node is a move; an edge to an enemy piece's node is a capture; an edge to a friend's node defends it.\n\n");
    out.push_str("Where a piece stands changes how many edges it has (its reach):\n");
    for (t, n) in [(2, "knight"), (3, "bishop"), (4, "rook"), (5, "queen")] {
        out.push_str(&format!("  - {n:<6}: corner a1 {} edges, edge a4 {}, centre d4 {}\n", lone_edges(t, 0), lone_edges(t, 24), lone_edges(t, 27)));
    }
    out.push_str("  So pieces in the centre reach more of the network: that is why players fight for the centre.\n\n");
    out.push_str("A move rewires the network: the piece takes its edges to the new node, pieces behind it get theirs back,\n");
    out.push_str("pieces now blocked lose theirs, a captured piece and all its edges disappear.\n\n");
    out.push_str("The goal. Check: an enemy edge points at your king's node - you must answer it at once (move the king,\n");
    out.push_str("block the edge with a piece, or capture the piece that sends it). Mate: you are in check and no answer exists -\n");
    out.push_str("the king's node is struck and every node it could step to is struck or full. Stalemate: not in check but no\n");
    out.push_str("legal move at all - a draw. So the whole game is about cutting the enemy king's node off from the network.\n\n");
    out.push_str("Special rules: castling (king two nodes sideways, the rook jumps over it; only if neither has moved, the nodes\n");
    out.push_str("between are empty and the king's nodes are not struck), en passant (a pawn that steps two may be taken as if it\n");
    out.push_str("stepped one), promotion (a pawn reaching the last row becomes a queen, rook, bishop or knight: its few edges\n");
    out.push_str("become many). Draws: stalemate, the same network three times, 50 moves without a pawn move or capture,\n");
    out.push_str("or too little material left to mate.\n\n");
    out.push_str("How strong players use the network:\n");
    out.push_str("  - development: bring the knights and bishops off the back row early - each one adds edges into the centre;\n");
    out.push_str("  - king safety: castle; keep pawns in front of the king, so few enemy edges reach the nodes around it;\n");
    out.push_str("  - material: a piece is worth roughly its reach (knight ~3 pawns, bishop ~3, rook ~5, queen ~9);\n");
    out.push_str("  - tactics are network patterns: a fork (one piece's edges hit two enemy pieces), a pin (an edge passes through\n");
    out.push_str("    a piece to a more valuable one behind it), a discovered attack (moving one piece opens another's edges),\n");
    out.push_str("    an overloaded defender (one piece's edges must guard two nodes at once);\n");
    out.push_str("  - endgames: few pieces left, the king becomes a fighting piece (its edges count), a passed pawn's path to the\n");
    out.push_str("    last row is the shortest path in the network, and the defending king must reach that path in time\n");
    out.push_str("    (the 'square of the pawn'); two kings facing each other with one node between them fight for 'the opposition'.\n");
    out
}

/// The network's numbers in a position: (edges of white, edges of black,
/// enemy edges on the side-to-move's king node, its free king edges, its legal moves).
fn numbers(b: &Board) -> (usize, usize, usize, i64, usize) {
    let e = attack_edges(b);
    let we: usize = e[0].iter().map(|&x| x as usize).sum();
    let be: usize = e[1].iter().map(|&x| x as usize).sum();
    let k = b.king(b.white);
    let them = if b.white { 1 } else { 0 };
    let on_king = if k >= 0 { e[them][k as usize] as usize } else { 0 };
    let ms = b.moves();
    let escapes_i = ATOMS.iter().position(|a| *a == "my_escapes").expect("atom");
    let a = atoms(b, &ms, 1 << escapes_i);
    (we, be, on_king, a[escapes_i], ms.len())
}

/// Replay `moves` (UCI) and tell what every move did to the network.
pub fn walk(title: &str, moves: &str) -> Result<String, String> {
    let mut b = Board::start();
    let mut out = format!("{title}\n");
    let (w0, b0, _, _, _) = numbers(&b);
    out.push_str(&format!("  start: white has {w0} edges, black {b0}\n"));
    for (i, u) in moves.split_whitespace().enumerate() {
        let m = b.parse(u).ok_or_else(|| format!("move {} ({u}) is not legal in {}", i + 1, b.fen()))?;
        let mover = b.sq[m.from as usize];
        let victim = b.sq[m.to as usize];
        let before = attack_edges(&b);
        let after_b = b.play(m);
        let after = attack_edges(&after_b);
        let changed: usize = (0..2).map(|s| (0..64).map(|n| (before[s][n] as i32 - after[s][n] as i32).unsigned_abs() as usize).sum::<usize>()).sum();
        let (we, be, on_king, free, legal) = numbers(&after_b);
        let num = if i % 2 == 0 { format!("{}.", i / 2 + 1) } else { format!("{}...", i / 2 + 1) };
        let mut note = format!(
            "  {num:<5} {u:<6} {} {} -> {}{}: {changed} edge ends rewired; white {we} edges, black {be}",
            if mover > 0 { "white" } else { "black" },
            piece_name(mover),
            sq_name(m.to as usize),
            if victim != 0 { format!(" (captures the {}, its edges vanish)", piece_name(victim)) } else { String::new() }
        );
        let defender = if after_b.white { "white" } else { "black" };
        if on_king > 0 && legal == 0 {
            note.push_str(&format!("\n          MATE: {on_king} edge(s) strike the {defender} king's node, it has {free} free edge(s) and no answer exists"));
        } else if on_king > 0 {
            note.push_str(&format!("\n          check: {on_king} edge(s) strike the {defender} king's node; it keeps {free} free edge(s), {legal} answers"));
        } else if legal == 0 {
            note.push_str("\n          stalemate: no edge strikes the king, but no legal move exists");
        }
        out.push_str(&note);
        out.push('\n');
        b = after_b;
    }
    Ok(out)
}

pub fn report(which: &str) -> Result<String, String> {
    let mut out = explain();
    out.push_str("\nTHE GAMES, LAID INTO THE NETWORK\n\n");
    for (key, title, moves) in GAMES {
        if which.is_empty() || which == "all" || which == key {
            out.push_str(&walk(title, moves)?);
            out.push('\n');
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_game_is_legal_and_ends_in_mate() {
        for (_, title, moves) in GAMES {
            let text = walk(title, moves).unwrap();
            assert!(text.contains("MATE:"), "{title}");
        }
    }

    #[test]
    fn a_knight_reaches_more_from_the_centre() {
        assert_eq!(lone_edges(2, 0), 2);
        assert_eq!(lone_edges(2, 27), 8);
        assert_eq!(lone_edges(4, 27), 14);
    }
}
