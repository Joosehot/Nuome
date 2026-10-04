//! The supergenius solves a whole class of puzzles at once, from the rules
//! alone: every position with the same pieces on the board, worked
//! backwards from the mates (retrograde analysis). A position is won when
//! some move reaches a position lost for the other side, lost when every
//! move reaches a won one, a draw when neither ever follows. Captures and
//! promotions lead into smaller classes, solved first and remembered. The
//! result is exact: a move is a proof, and the mate is counted in plies.
//!
//! Classes with up to 3 pieces are solved in seconds here (half a million
//! positions each); the forward calculation (neuro.rs) probes these tables
//! as the rules' verdict whenever a capture or promotion reaches them.

use crate::golden::{Board, Mv};
use std::collections::HashMap;
use std::sync::Mutex;

/// Value for the side to move: a win is the plies until it mates (v > 0); a
/// loss is -(plies until it is mated + 1), so a mated position is -1; 0 a
/// draw; UNSET while being solved; NONE an impossible position.
pub type Val = i16;
pub const UNSET: Val = i16::MIN;
pub const NONE: Val = i16::MIN + 1;

pub struct Table {
    /// the pieces in index order: white king, black king, the rest (signed types)
    pub pieces: Vec<i8>,
    pub val: Vec<Val>,
}

/// The class of a board: its pieces, kings first, then white pieces by type
/// descending, then black; the key names it ("KRvK").
pub fn pieces_of(b: &Board) -> Vec<i8> {
    let mut rest: Vec<i8> = b.sq.iter().copied().filter(|&p| p != 0 && p.abs() != 6).collect();
    rest.sort_by_key(|&p| (p < 0, -(p.abs() as i32)));
    let mut out = vec![6, -6];
    out.extend(rest);
    out
}

pub fn key_of(pieces: &[i8]) -> String {
    let name = |p: i8| ["", "P", "N", "B", "R", "Q", "K"][p.unsigned_abs() as usize];
    let w: String = pieces.iter().filter(|&&p| p > 0).map(|&p| name(p)).collect();
    let bl: String = pieces.iter().filter(|&&p| p < 0).map(|&p| name(p)).collect();
    format!("{w}v{bl}")
}

impl Table {
    fn count(&self) -> usize {
        2 * 64usize.pow(self.pieces.len() as u32)
    }

    /// The index of `b` in this table (its pieces must be this class).
    pub fn index(&self, b: &Board) -> Option<usize> {
        let mut used = [false; 64];
        let mut idx = 0usize;
        let mut mul = 1usize;
        for &p in &self.pieces {
            let s = (0..64).find(|&s| b.sq[s] == p && !used[s])?;
            used[s] = true;
            idx += s * mul;
            mul *= 64;
        }
        Some(idx * 2 + (!b.white) as usize)
    }

    fn board(&self, idx: usize) -> Option<Board> {
        let white = idx % 2 == 0;
        let mut rest = idx / 2;
        let mut sq = [0i8; 64];
        for &p in &self.pieces {
            let s = rest % 64;
            rest /= 64;
            if sq[s] != 0 {
                return None;
            }
            if p.abs() == 1 && (s < 8 || s >= 56) {
                return None;
            }
            sq[s] = p;
        }
        let b = Board { sq, white, castle: 0, ep: None, half: 0, full: 1 };
        // the side not to move may not stand in check
        let k = b.king(!b.white);
        if k < 0 || b.attacked(k, b.white) {
            return None;
        }
        Some(b)
    }
}

static TABLES: Mutex<Option<HashMap<String, &'static Table>>> = Mutex::new(None);

/// The solved table of this class (built now if needed), or None when the
/// class is too large to solve here (more than 3 pieces).
pub fn table(pieces: &[i8]) -> Option<&'static Table> {
    if pieces.len() > 3 {
        return None;
    }
    let key = key_of(pieces);
    {
        let g = TABLES.lock().unwrap();
        if let Some(m) = g.as_ref() {
            if let Some(t) = m.get(&key) {
                return Some(t);
            }
        }
    }
    let path = format!("out/golden/retro/{key}.bin");
    let t: &'static Table = Box::leak(Box::new(match std::fs::read(&path) {
        Ok(bytes) if bytes.len() == 2 * 2 * 64usize.pow(pieces.len() as u32) => Table {
            pieces: pieces.to_vec(),
            val: bytes.chunks(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect(),
        },
        _ => {
            let t = solve(pieces);
            let _ = std::fs::create_dir_all("out/golden/retro");
            let _ = std::fs::write(&path, t.val.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>());
            t
        }
    }));
    let mut g = TABLES.lock().unwrap();
    g.get_or_insert_with(HashMap::new).insert(key, t);
    Some(t)
}

/// The value, for the side to move, of the position after `m` in `b`, when
/// the tables know it: the rules' verdict (mate, stalemate), the smaller
/// class after a capture or promotion, or this class's own table.
pub fn child_value(b: &Board, m: Mv, own: Option<&Table>) -> Option<Val> {
    let a = b.play(m);
    let ms = a.moves();
    if ms.is_empty() {
        return Some(if a.in_check() { -1 } else { 0 });
    }
    if a.insufficient() {
        return Some(0);
    }
    let same = b.sq[m.to as usize] == 0 && m.promo == 0;
    let t = if same { own? } else { table(&pieces_of(&a))? };
    let v = t.val[t.index(&a)?];
    if v == UNSET || v == NONE {
        None
    } else {
        Some(v)
    }
}

/// The mover's view of a child's value: a mated child is a win one ply later.
pub fn flip(v: Val) -> Val {
    if v > 0 {
        -(v + 2)
    } else if v < 0 {
        -v
    } else {
        0
    }
}

/// Solve the class by passes until nothing changes; the rest are draws.
pub fn solve(pieces: &[i8]) -> Table {
    let mut t = Table { pieces: pieces.to_vec(), val: Vec::new() };
    let n = t.count();
    t.val = vec![UNSET; n];
    let boards: Vec<Option<Board>> = (0..n).map(|i| t.board(i)).collect();
    for i in 0..n {
        if boards[i].is_none() {
            t.val[i] = NONE;
        }
    }
    // the moves of every position, once: (same-class child index) or (value from a smaller class)
    enum Kid {
        Same(usize),
        Known(Val),
    }
    let kids: Vec<Vec<Kid>> = boards
        .iter()
        .map(|b| {
            let Some(b) = b else { return Vec::new() };
            b.moves()
                .into_iter()
                .map(|m| {
                    let a = b.play(m);
                    if a.moves().is_empty() {
                        Kid::Known(if a.in_check() { -1 } else { 0 })
                    } else if a.insufficient() {
                        Kid::Known(0)
                    } else if b.sq[m.to as usize] == 0 && m.promo == 0 {
                        Kid::Same(t.index(&a).expect("same class"))
                    } else {
                        let sub = table(&pieces_of(&a)).expect("a smaller class");
                        Kid::Known(sub.val[sub.index(&a).expect("in the smaller class")])
                    }
                })
                .collect()
        })
        .collect();
    // every position recomputed each pass from its children (wins: the fastest;
    // losses: the slowest, once every move is known to lose) until nothing moves
    loop {
        let mut changed = false;
        for i in 0..n {
            if t.val[i] == NONE {
                continue;
            }
            let ks = &kids[i];
            let cand: Val = if ks.is_empty() {
                if boards[i].as_ref().unwrap().in_check() { -1 } else { 0 }
            } else {
                let mut best_win: Option<Val> = None;
                let mut all_lost = true;
                let mut worst_loss: Val = 0;
                for k in ks {
                    let v = match k {
                        Kid::Same(j) => t.val[*j],
                        Kid::Known(v) => *v,
                    };
                    if v == UNSET {
                        all_lost = false;
                        continue;
                    }
                    let mine = flip(v);
                    if mine > 0 {
                        best_win = Some(best_win.map_or(mine, |b| b.min(mine)));
                    } else if mine == 0 {
                        all_lost = false;
                    } else {
                        worst_loss = worst_loss.min(mine);
                    }
                }
                match best_win {
                    Some(w) => w,
                    None if all_lost => worst_loss,
                    None => UNSET,
                }
            };
            if cand != t.val[i] {
                t.val[i] = cand;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for v in t.val.iter_mut() {
        if *v == UNSET {
            *v = 0;
        }
    }
    t
}

/// The exact move in `b` from the tables: (move, value for the mover) or None when unknown.
pub fn exact_move(b: &Board) -> Option<(Mv, Val, Vec<(Mv, Val)>)> {
    let own = table(&pieces_of(b));
    let mut rows = Vec::new();
    for m in b.moves() {
        let v = child_value(b, m, own)?;
        rows.push((m, flip(v)));
    }
    if rows.is_empty() {
        return None;
    }
    // a win: the fastest; a draw over a loss; a loss: the slowest
    rows.sort_by_key(|&(_, v)| if v > 0 { (0, v as i32) } else if v == 0 { (1, 0) } else { (2, v as i32) });
    Some((rows[0].0, rows[0].1, rows))
}

pub fn show(v: Val) -> String {
    if v > 0 {
        format!("mate in {} ({} plies)", (v + 1) / 2, v)
    } else if v < 0 {
        format!("mated in {} ({} plies)", (-v) / 2, -v - 1)
    } else {
        "draw".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn krvk_is_solved_and_the_longest_mate_is_sixteen_moves() {
        let t = table(&[6, -6, 4]).unwrap();
        let longest = t.val.iter().filter(|&&v| v > 0).max().copied().unwrap();
        assert_eq!((longest + 1) / 2, 16);
        // a known mate in 1
        let b = Board::from_fen("6k1/8/6K1/8/8/8/8/R7 w - - 0 1").unwrap();
        let (m, v, _) = exact_move(&b).unwrap();
        assert_eq!((m.uci(), v), ("a1a8".to_string(), 1));
    }

    #[test]
    fn kpvk_knows_the_opposition() {
        // white to move wins; black to move draws (the known opposition result)
        let w = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        let bl = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 b - - 0 1").unwrap();
        assert!(exact_move(&w).unwrap().1 > 0);
        assert_eq!(exact_move(&bl).unwrap().1, 0);
    }
}
