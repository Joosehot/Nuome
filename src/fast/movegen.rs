// Move generation. Sliders/knights/kings via bitboards; pawns and castling
// ported line-by-line from fast.js genMoves (L86-119) including its quirks:
// - capsOnly still generates quiet promotions (pawn push to promo rank)
// - pawn captures never target the king (TYPE(v)!==6)
// - castling requires the rook on its corner square (fast.js safety check)
use super::bitboard::{tables, SQ64_TO_120, SQ120_TO_64};
use super::position::*;

pub const MAX_MOVES: usize = 256;

pub fn gen_moves(p: &Position, list: &mut [u32; MAX_MOVES], caps_only: bool) -> usize {
    // Square-ascending piece iteration = identical move order to fast.js genMoves.
    let mut n = 0;
    let us = p.turn as usize;
    let t = tables();
    let dir: i32 = if us != 0 { -10 } else { 10 };
    let start_rank: i32 = if us != 0 { 6 } else { 1 };
    let promo_rank: i32 = if us != 0 { 0 } else { 7 };

    let mut occ_us = p.by_color[us];
    while occ_us != 0 {
        let s64 = occ_us.trailing_zeros() as usize;
        occ_us &= occ_us - 1;
        let f = SQ64_TO_120[s64];
        let pc = p.board[f];
        if pc == 0 || pc == OFF {
            continue;
        }
        let ty = type_of(pc) as usize;
        match ty {
            1 => {
                // pawn block: fast.js L93-102
                let to = (f as i32 + dir) as usize;
                if p.board[to] == EMPTY {
                    if rank_of120(to) == promo_rank {
                        for &pr in &[WQ, WN, WR, WB] {
                            list[n] = mk_move(f, to, pr, 0);
                            n += 1;
                        }
                    } else if !caps_only {
                        list[n] = mk_move(f, to, 0, 0);
                        n += 1;
                        if rank_of120(f) == start_rank
                            && p.board[(to as i32 + dir) as usize] == EMPTY
                        {
                            list[n] = mk_move(f, (to as i32 + dir) as usize, 0, 4);
                            n += 1;
                        }
                    }
                }
                for d in [dir - 1, dir + 1] {
                    let c = (f as i32 + d) as usize;
                    let v = p.board[c];
                    if v != EMPTY && v != OFF && color_of(v) as usize != us && type_of(v) != 6 {
                        if rank_of120(c) == promo_rank {
                            for &pr in &[WQ, WN, WR, WB] {
                                list[n] = mk_move(f, c, pr, 1);
                                n += 1;
                            }
                        } else {
                            list[n] = mk_move(f, c, 0, 1);
                            n += 1;
                        }
                    } else if c as i8 == p.ep {
                        list[n] = mk_move(f, c, 0, 2);
                        n += 1;
                    }
                }
            }
            2 | 6 => {
                // knight / king block: fast.js L103-111
                let dirs: &[i32] = if ty == 2 { &[-21, -19, -12, -8, 8, 12, 19, 21] } else { &[-11, -10, -9, -1, 1, 9, 10, 11] };
                for &d in dirs {
                    let to = (f as i32 + d) as usize;
                    let v = p.board[to];
                    if v == OFF {
                        continue;
                    }
                    if v == EMPTY {
                        if !caps_only {
                            list[n] = mk_move(f, to, 0, 0);
                            n += 1;
                        }
                    } else if color_of(v) as usize != us {
                        list[n] = mk_move(f, to, 0, 1);
                        n += 1;
                    }
                }
                if ty == 6 && !caps_only && f == if us != 0 { 95 } else { 25 } {
                    let them_c = 1 - us as u8;
                    if us == 0 {
                        if p.castle & 1 != 0
                            && p.board[26] == EMPTY
                            && p.board[27] == EMPTY
                            && p.board[28] == WR
                            && !p.attacked(25, 1)
                            && !p.attacked(26, 1)
                            && !p.attacked(27, 1)
                        {
                            list[n] = mk_move(25, 27, 0, 3);
                            n += 1;
                        }
                        if p.castle & 2 != 0
                            && p.board[24] == EMPTY
                            && p.board[23] == EMPTY
                            && p.board[22] == EMPTY
                            && p.board[21] == WR
                            && !p.attacked(25, 1)
                            && !p.attacked(24, 1)
                            && !p.attacked(23, 1)
                        {
                            list[n] = mk_move(25, 23, 0, 3);
                            n += 1;
                        }
                    } else {
                        if p.castle & 4 != 0
                            && p.board[96] == EMPTY
                            && p.board[97] == EMPTY
                            && p.board[98] == BR
                            && !p.attacked(95, 0)
                            && !p.attacked(96, 0)
                            && !p.attacked(97, 0)
                        {
                            list[n] = mk_move(95, 97, 0, 3);
                            n += 1;
                        }
                        if p.castle & 8 != 0
                            && p.board[94] == EMPTY
                            && p.board[93] == EMPTY
                            && p.board[92] == EMPTY
                            && p.board[91] == BR
                            && !p.attacked(95, 0)
                            && !p.attacked(94, 0)
                            && !p.attacked(93, 0)
                        {
                            list[n] = mk_move(95, 93, 0, 3);
                            n += 1;
                        }
                    }
                    let _ = them_c;
                }
            }
            _ => {
                // sliders: fast.js L112-116
                let dirs: &[i32] = match ty {
                    3 => &[-11, -9, 9, 11],
                    4 => &[-10, -1, 1, 10],
                    _ => &[-11, -10, -9, -1, 1, 9, 10, 11],
                };
                let nd = if ty == 5 { 8 } else { 4 };
                for i in 0..nd {
                    let d = dirs[i];
                    let mut to = f as i32 + d;
                    loop {
                        let v = p.board[to as usize];
                        if v == OFF {
                            break;
                        }
                        if v == EMPTY {
                            if !caps_only {
                                list[n] = mk_move(f, to as usize, 0, 0);
                                n += 1;
                            }
                        } else {
                            if color_of(v) as usize != us {
                                list[n] = mk_move(f, to as usize, 0, 1);
                                n += 1;
                            }
                            break;
                        }
                        to += d;
                    }
                }
            }
        }
    }
    let _ = t;
    n
}

/// Legal moves: pseudo-legal + make/in-check/unmake filter (fast.js legalPacked).
pub fn legal_moves(p: &mut Position) -> Vec<u32> {
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, false);
    let mut out = Vec::new();
    for i in 0..n {
        let m = list[i];
        p.make(m);
        let ok = !p.in_check_c(1 - p.turn);
        p.unmake();
        if ok {
            out.push(m);
        }
    }
    out
}

/// Parse a UCI move string against this position's legal moves.
pub fn uci_to_move(p: &mut Position, uci: &str) -> Option<u32> {
    let b = uci.as_bytes();
    if b.len() < 4 {
        return None;
    }
    let ff = (b[0] - b'a') as usize;
    let fr = (b[1] - b'1') as usize;
    let tf = (b[2] - b'a') as usize;
    let tr = (b[3] - b'1') as usize;
    if ff > 7 || fr > 7 || tf > 7 || tr > 7 {
        return None;
    }
    let promo = match b.get(4) {
        Some(b'n') => 2, Some(b'b') => 3, Some(b'r') => 4, Some(b'q') => 5,
        None => 0, _ => return None,
    };
    let from = SQ64_TO_120[fr * 8 + ff];
    let to = SQ64_TO_120[tr * 8 + tf];
    for m in legal_moves(p) {
        if m_from(m) as usize == from && m_to(m) as usize == to && m_promo(m) == promo {
            return Some(m);
        }
    }
    None
}
