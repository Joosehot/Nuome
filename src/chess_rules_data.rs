//! Teaching material for a network that learns the rules of chess: random
//! legal positions (from random games played by the rules in golden.rs) and,
//! for each, the exact answers the rules give - nothing else, no
//! evaluation, no results.
//!
//! The board is always seen from the side to move (black's boards are
//! turned around), as 17 planes of 64 nodes: own pawn, knight, bishop,
//! rook, queen, king, the same for the opponent, the four castling rights
//! (whole planes), and the en passant square. The answers: every legal move
//! as (from, to) - 4096 bits - and the nodes each side strikes (2 x 64
//! bits), plus whether the side to move is in check, mated or stalemated.
//!
//! File: "NUOMERULES1", the count (u32), then per position, bit-packed:
//! input 17 x 64 bits, moves 64 x 64 bits (index to * 64 + from), strikes
//! 2 x 64 bits (own, then the opponent's), one flag byte (1 check, 2 mate,
//! 4 stalemate).

use crate::evolve::Rng;
use crate::golden::{attack_edges, Board};
use std::io::Write;

pub const PLANES: usize = 17;
pub const RECORD: usize = (PLANES * 64 + 64 * 64 + 2 * 64) / 8 + 1;

/// The node a square becomes when the board is seen from the side to move.
fn view(sq: usize, white: bool) -> usize {
    if white { sq } else { sq ^ 56 }
}

fn set(bits: &mut [u8], i: usize) {
    bits[i / 8] |= 1 << (i % 8);
}

/// One position as a record (the bytes of the file format).
pub fn encode(b: &Board) -> Vec<u8> {
    let mut r = vec![0u8; RECORD];
    let w = b.white;
    let mine = |p: i8| if w { p > 0 } else { p < 0 };
    // planes
    for s in 0..64 {
        let p = b.sq[s];
        if p != 0 {
            let plane = (p.unsigned_abs() as usize - 1) + if mine(p) { 0 } else { 6 };
            set(&mut r, plane * 64 + view(s, w));
        }
    }
    let rights = if w { [1u8, 2, 4, 8] } else { [4, 8, 1, 2] };
    for (k, right) in rights.iter().enumerate() {
        if b.castle & right != 0 {
            for n in 0..64 {
                set(&mut r, (12 + k) * 64 + n);
            }
        }
    }
    if let Some(e) = b.ep {
        set(&mut r, 16 * 64 + view(e as usize, w));
    }
    // the legal moves
    let base = PLANES * 64;
    let moves = b.moves();
    for m in &moves {
        let (f, t) = (view(m.from as usize, w), view(m.to as usize, w));
        set(&mut r, base + t * 64 + f);
    }
    // strikes
    let e = attack_edges(b);
    let (me, them) = if w { (0, 1) } else { (1, 0) };
    let sbase = base + 64 * 64;
    for s in 0..64 {
        if e[me][s] > 0 {
            set(&mut r, sbase + view(s, w));
        }
        if e[them][s] > 0 {
            set(&mut r, sbase + 64 + view(s, w));
        }
    }
    let check = b.in_check();
    r[RECORD - 1] = check as u8 | ((check && moves.is_empty()) as u8) << 1 | ((!check && moves.is_empty()) as u8) << 2;
    r
}

/// `count` positions from random games (seeded): each game is played by
/// random legal moves until it ends or reaches `max_plies`; every position
/// on the way is kept with probability `keep`.
pub fn generate(count: usize, seed: u64, max_plies: usize) -> Vec<Vec<u8>> {
    let mut r = Rng(seed);
    let mut out = Vec::with_capacity(count);
    while out.len() < count {
        let mut b = Board::start();
        for _ in 0..max_plies {
            if r.unit() < 0.15 {
                out.push(encode(&b));
                if out.len() == count {
                    break;
                }
            }
            let ms = b.moves();
            if ms.is_empty() || b.half >= 100 || b.insufficient() {
                out.push(encode(&b)); // the end: mate, stalemate or a draw
                break;
            }
            b = b.play(ms[r.below(ms.len())]);
        }
    }
    out.truncate(count);
    out
}

/// Write `count` positions to `path` on every core (deterministic: shard k uses seed + k).
pub fn write(path: &str, count: usize, seed: u64) -> Result<String, String> {
    let shards = 12usize;
    let per = count.div_ceil(shards);
    let parts: Vec<Vec<Vec<u8>>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..shards).map(|k| sc.spawn(move || generate(per.min(count.saturating_sub(k * per)), seed + k as u64 * 7919, 200))).collect();
        hs.into_iter().map(|h| h.join().expect("shard")).collect()
    });
    let all: Vec<&Vec<u8>> = parts.iter().flatten().collect();
    let mut f = std::io::BufWriter::new(std::fs::File::create(path).map_err(|e| format!("{path}: {e}"))?);
    f.write_all(b"NUOMERULES1").map_err(|e| e.to_string())?;
    f.write_all(&(all.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
    let (mut checks, mut mates, mut stale, mut castles, mut eps) = (0, 0, 0, 0, 0);
    for rec in &all {
        f.write_all(rec).map_err(|e| e.to_string())?;
        let fl = rec[RECORD - 1];
        checks += (fl & 1) as usize;
        mates += (fl >> 1 & 1) as usize;
        stale += (fl >> 2 & 1) as usize;
        let ep_plane = &rec[16 * 64 / 8..17 * 64 / 8];
        eps += ep_plane.iter().any(|&x| x != 0) as usize;
        let castle_planes = &rec[12 * 64 / 8..16 * 64 / 8];
        castles += castle_planes.iter().any(|&x| x != 0) as usize;
    }
    Ok(format!(
        "{} positions to {path} ({} bytes each): {checks} in check, {mates} mated, {stale} stalemated, {castles} with a castling right, {eps} with an en passant square",
        all.len(),
        RECORD
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bit(r: &[u8], i: usize) -> bool {
        r[i / 8] >> (i % 8) & 1 == 1
    }

    #[test]
    fn the_start_has_twenty_moves_and_is_seen_the_same_from_both_sides() {
        let w = encode(&Board::start());
        let n = (0..4096).filter(|&i| bit(&w, PLANES * 64 + i)).count();
        assert_eq!(n, 20);
        // after 1. e4 e5 2. Nf3 Nc6 the position is symmetric up to e4/e5 ... check a mirrored start instead
        let b = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").unwrap();
        assert_eq!(encode(&b), w);
    }

    #[test]
    fn mate_is_flagged() {
        let b = Board::from_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        let r = encode(&b);
        assert_eq!(r[RECORD - 1], 3);
    }
}
