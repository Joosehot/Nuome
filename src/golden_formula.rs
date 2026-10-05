//! The Golden formula for chess, made like the Goldbach bound: the problem
//! defined first, the formula given as Golden Answer's answer to it, then
//! checked exactly where the truth is known. No data chooses anything.
//!
//! The problem.
//!   object:  a legal position P, s the side to move
//!   truth:   V(P), win / draw / loss for s with perfect play, by the rules alone
//!   pair:    every move m of s and every reply r to it
//!   score:   p(m, r), the five checks — legality, safety, force, material,
//!            tempo — made at once on every pair; Golden Answer says how each
//!            is scored (the questions and answers are in `PARTS`)
//!   formula: f(m) from the pair scores
//!   demand:  f orders the moves as V does
//!   check:   exactly where V is known (Syzygy tables, up to five pieces):
//!            either it holds so far, or the first counterexample
//!
//! The answers (kind, and why):
//!   legality (logical, the rules): an illegal move is no move; no reply
//!     after m: mate wins, stalemate draws; no move for s after r: mate
//!     loses, stalemate draws; a draw by the rules (insufficient material,
//!     fifty moves, repetition) is a draw. A draw is worth 0.
//!   material (theoretical, Kaufman 1999): pawn 100, knight 325, bishop 325,
//!     rook 500, queen 975; after the pair, mine minus theirs.
//!   safety and force (logical): a capture surely wins the piece's worth if
//!     nothing can take back, and at least the worth minus the capturer's if
//!     something can (whatever follows, the capturer can stop there). After
//!     the pair s is to move and makes one move: it takes its largest sure
//!     gain and suffers the largest sure loss, or it saves its most valuable
//!     threatened piece and suffers the second: max(gain1 - loss1, -loss2).
//!   tempo (theoretical, the opening rule of thumb: three tempi are about a
//!     pawn): a check spends the other side's move; +1 if m checks, -1 if r
//!     checks, 33 each.
//!   pair (logical): every part is in centipawns, so the pair's score is
//!     their sum.
//!   move (logical): the other side chooses its reply, so a move is worth its
//!     worst reply: f(m) = min over r of p(m, r).

use crate::fast::movegen::{gen_moves, legal_moves, MAX_MOVES};
use crate::fast::position::{m_from, m_promo, m_to, move_to_uci, type_of, Position};

const WORTH: [i64; 7] = [0, 100, 325, 325, 500, 975, 0];
pub const WIN: i64 = 1_000_000;
const TEMPO: i64 = 33;

/// Golden Answer's questions about the pair score and its answers: (question, kind, answer, source).
pub const PARTS: [(&str, &str, &str, &str); 7] = [
    ("is the pair (m, r) a game at all, and does it end the game?", "logical", "an illegal move is no move; mate after m wins, mate after r loses; stalemate and the draws by rule are worth 0", "FIDE Laws of Chess, articles 3, 5, 9"),
    ("what is the material after the pair?", "theoretical", "mine minus theirs at 100 / 325 / 325 / 500 / 975", "Kaufman, The Evaluation of Material Imbalances (1999)"),
    ("what does s surely lose next (safety)?", "logical", "a capture by them surely wins the piece if nothing takes back, else at least its worth minus the capturer's; s saves one piece with its move, so the second loss stands", "the rules: the capturer may stop after any capture"),
    ("what does s surely win next (force)?", "logical", "its largest sure capture, paid for by the largest sure loss: max(gain1 - loss1, -loss2)", "the rules: one move per turn"),
    ("what is the tempo of the pair?", "theoretical", "a check spends the other side's move: +1 if m checks, -1 if r checks, a third of a pawn each", "the opening rule of thumb: three tempi are about a pawn"),
    ("how do the five combine on a pair?", "logical", "all are centipawns: p(m, r) = material + max(gain1 - loss1, -loss2) + 33 * tempo, unless legality ended the game", "one currency"),
    ("how do the pairs make f?", "logical", "the other side chooses the reply: f(m) = min over r of p(m, r)", "the rules: the other side moves freely"),
];

fn material(p: &Position, side: usize) -> i64 {
    let mut v = [0i64; 2];
    for c in 0..2 {
        for t in 0..6 {
            v[c] += WORTH[t + 1] * p.bb[c * 6 + t].count_ones() as i64;
        }
    }
    v[side] - v[1 - side]
}

/// The sure gains of the side to move's captures and promotions, largest first.
fn sure_gains(p: &mut Position) -> Vec<i64> {
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, true);
    let mut out = Vec::new();
    for &m in &list[..n] {
        let target = p.board[m_to(m) as usize];
        if target != 0 && type_of(target) == 6 {
            continue;
        }
        let taken = if target != 0 { WORTH[type_of(target) as usize] } else if m_promo(m) == 0 { WORTH[1] } else { 0 }; // en passant takes a pawn
        let mover = type_of(p.board[m_from(m) as usize]);
        let promo = if m_promo(m) != 0 { WORTH[type_of(m_promo(m) as u8) as usize] - WORTH[1] } else { 0 };
        let lands = if m_promo(m) != 0 { WORTH[type_of(m_promo(m) as u8) as usize] } else { WORTH[mover as usize] };
        p.make(m);
        if p.in_check_c(me) {
            p.unmake();
            continue;
        }
        let to = m_to(m) as usize;
        let retaken = p.attacked(to, 1 - me);
        p.unmake();
        let g = if retaken { (taken + promo - lands).max(0) } else { taken + promo };
        if g > 0 {
            out.push(g);
        }
    }
    out.sort_unstable_by(|a, b| b.cmp(a));
    out
}

/// What the side to move can expect from its next move: max(gain1 - loss1, -loss2).
fn next_move(p: &mut Position) -> i64 {
    let gains = sure_gains(p);
    p.make_null();
    let losses = sure_gains(p);
    p.unmake_null();
    let (g1, l1, l2) = (gains.first().copied().unwrap_or(0), losses.first().copied().unwrap_or(0), losses.get(1).copied().unwrap_or(0));
    (g1 - l1).max(-l2)
}

/// p(m, r) with m and r already played, s = `me` to move again; `m_check` / `r_check`: whether they gave check.
fn pair_score(p: &mut Position, me: usize, m_check: bool, r_check: bool) -> i64 {
    if legal_moves(p).is_empty() {
        return if p.in_check() { -WIN } else { 0 };
    }
    if p.drawn() {
        return 0;
    }
    material(p, me) + next_move(p) + TEMPO * (m_check as i64 - r_check as i64)
}

/// f(m): the worst p(m, r) over the replies.
pub fn f(p: &mut Position, m: u32) -> i64 {
    let me = p.turn as usize;
    p.make(m);
    let m_check = p.in_check();
    let replies = legal_moves(p);
    let v = if replies.is_empty() {
        if m_check {
            WIN
        } else {
            0
        }
    } else if p.drawn() {
        0
    } else {
        let mut worst = i64::MAX;
        for r in replies {
            p.make(r);
            let r_check = p.in_check();
            let v = pair_score(p, me, m_check, r_check);
            p.unmake();
            worst = worst.min(v);
        }
        worst
    };
    p.unmake();
    v
}

/// Every legal move with f, best first (ties in the rules' order).
pub fn rank(fen: &str) -> Option<Vec<(String, i64)>> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut rows: Vec<(String, i64)> = legal_moves(&mut p).into_iter().map(|m| (move_to_uci(m), f(&mut p, m))).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    Some(rows)
}

pub fn written() -> &'static str {
    "f(m) = min over replies r of p(m, r);  p(m, r) = ±WIN for mate, 0 for a draw, else material + max(gain1 - loss1, -loss2) + 33 * (m checks - r checks)"
}

/// Golden Answer's answer to the question, in its kind.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let lines: Vec<String> = PARTS.iter().map(|(q, k, a, _)| format!("{q} ({k}) {a}")).collect();
    if kind_logical {
        let theo: Vec<&str> = PARTS.iter().filter(|x| x.1 == "theoretical").map(|x| x.0).collect();
        return Err(format!("not wholly logical: the rules decide legality, safety, force and how the parts combine, but the worths in \"{}\" are known results, not consequences of the rules; ask theoretical", theo.join("\" and \"")));
    }
    if kind_abstract {
        let mut l = lines;
        l.push("this is an answer that claims nothing".into());
        return Ok((written().into(), l, vec![]));
    }
    Ok((written().into(), lines, PARTS.iter().map(|x| x.3.to_string()).collect::<std::collections::BTreeSet<_>>().into_iter().collect()))
}

/// The question, read whole.
pub fn read(q: &str) -> bool {
    let t: String = q.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_end_matches(['.', '?', '!']);
    [
        "give one function of a chess position that checks legality, safety, force, material and tempo at the same time",
        "the golden formula for chess",
        "what is the golden formula for chess",
    ]
    .contains(&t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn best(fen: &str) -> String {
        rank(fen).unwrap()[0].0.clone()
    }

    #[test]
    fn mate_in_one_and_a_free_queen() {
        assert_eq!(best("6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1"), "d1d8");
        assert_eq!(best("6k1/5ppp/8/3q4/8/8/5PPP/3R2K1 w - - 0 1"), "d1d5");
    }

    #[test]
    fn does_not_hang_the_queen() {
        // Qxb7?? is met by Rxb7; the formula must not take a guarded pawn with the queen
        let rows = rank("1r4k1/1p3ppp/8/8/8/8/5PPP/1Q4K1 w - - 0 1").unwrap();
        assert_ne!(rows[0].0, "b1b7");
    }
}
