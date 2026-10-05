// Static Exchange Evaluation: swap-list SEE with bitboard attackers and
// x-ray updates. Returns the material outcome (VAL scale) of the capture
// sequence on the target square, from the mover's point of view.

use super::bitboard::{tables, SQ120_TO_64};
use super::position::{bb_idx, m_flag, m_from, m_promo, m_to, Position, OFF, VAL, WB, WK, WN, WP, WQ, WR};

/// All pieces (both colours) attacking `sq64` given occupancy `occ`.
#[inline]
fn attackers_to(p: &Position, sq64: usize, occ: u64, bq: u64, rq: u64) -> u64 {
    let t = tables();
    // pawn_b[s] = squares diagonally below s = where a white pawn attacking s sits.
    (t.att.pawn_b[sq64] & p.bb[bb_idx(WP)])
        | (t.att.pawn_w[sq64] & p.bb[bb_idx(WP + 8)])
        | (t.att.knight[sq64] & (p.bb[bb_idx(WN)] | p.bb[bb_idx(WN + 8)]))
        | (t.att.king[sq64] & (p.bb[bb_idx(WK)] | p.bb[bb_idx(WK + 8)]))
        | (t.bishop_attacks(sq64, occ) & bq)
        | (t.rook_attacks(sq64, occ) & rq)
}

/// Least valuable attacker of colour `c` among `attackers`: (bit, piece type 1..6).
#[inline]
fn least_valuable(p: &Position, attackers: u64, c: usize) -> Option<(u64, usize)> {
    for ty in 1..=6usize {
        let bb = attackers & p.bb[ty - 1 + 6 * c];
        if bb != 0 {
            return Some((bb & bb.wrapping_neg(), ty));
        }
    }
    None
}

pub fn see(p: &Position, m: u32) -> i32 {
    see_side(p, m, p.turn as usize)
}

/// Threats against side `c` (the side to move): for every piece of `c`
/// except the king, the SEE gain of the opponent capturing it with its least
/// valuable attacker. Returns the largest and second largest positive gains
/// (VAL scale), one per target square.
pub fn threats_against(p: &Position, c: usize) -> (i32, i32) {
    let opp = 1 - c;
    let t = tables();
    let bq = p.bb[bb_idx(WB)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WB + 8)] | p.bb[bb_idx(WQ + 8)];
    let rq = p.bb[bb_idx(WR)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WR + 8)] | p.bb[bb_idx(WQ + 8)];
    let _ = t;
    let (mut g1, mut g2) = (0i32, 0i32);
    let mut own = p.by_color[c] & !p.bb[5 + 6 * c];
    while own != 0 {
        let sq = own.trailing_zeros() as usize;
        own &= own - 1;
        let att = attackers_to(p, sq, p.occ, bq, rq) & p.by_color[opp];
        let Some((bit, _)) = least_valuable(p, att, opp) else { continue };
        let from = bit.trailing_zeros() as usize;
        let m = super::position::mk_move(super::bitboard::SQ64_TO_120[from], super::bitboard::SQ64_TO_120[sq], 0, 1);
        let g = see_side(p, m, opp);
        if g > g1 {
            g2 = g1;
            g1 = g;
        } else if g > g2 {
            g2 = g;
        }
    }
    (g1, g2)
}

/// SEE of capture `m` made by side `us` (0 white, 1 black).
pub fn see_side(p: &Position, m: u32, us: usize) -> i32 {
    see_side_ex(p, m, us, 0)
}

/// Pieces of side `c` pinned to their own king by an enemy slider.
pub fn pinned(p: &Position, c: usize) -> u64 {
    let t = tables();
    let kb = p.bb[5 + 6 * c];
    if kb == 0 {
        return 0;
    }
    let k = kb.trailing_zeros() as usize;
    let o = 1 - c;
    let rq = p.bb[3 + 6 * o] | p.bb[4 + 6 * o];
    let bq = p.bb[2 + 6 * o] | p.bb[4 + 6 * o];
    let own = p.by_color[c];
    let mut out = 0u64;
    let mut snipers = t.rook_attacks(k, 0) & rq;
    while snipers != 0 {
        let sq = snipers.trailing_zeros() as usize;
        snipers &= snipers - 1;
        let between = t.rook_attacks(k, 1u64 << sq) & t.rook_attacks(sq, 1u64 << k) & p.occ;
        if between.count_ones() == 1 && between & own != 0 {
            out |= between;
        }
    }
    let mut snipers = t.bishop_attacks(k, 0) & bq;
    while snipers != 0 {
        let sq = snipers.trailing_zeros() as usize;
        snipers &= snipers - 1;
        let between = t.bishop_attacks(k, 1u64 << sq) & t.bishop_attacks(sq, 1u64 << k) & p.occ;
        if between.count_ones() == 1 && between & own != 0 {
            out |= between;
        }
    }
    out
}

/// StarSEE: a NEW threat against the side to move - the piece the opponent
/// just moved (last move's to-square) can take one of our pieces (not the
/// king) with a pin-aware SEE gain. Returns the largest such gain (VAL scale).
pub fn new_threat(p: &Position) -> i32 {
    let c = p.turn as usize;
    let opp = 1 - c;
    let Some(u) = p.undo.last() else { return 0 };
    if u.null {
        return 0;
    }
    let to120 = m_to(u.mv) as usize;
    let to = SQ120_TO_64[to120];
    if to < 0 {
        return 0;
    }
    let from = to as usize;
    let mover = p.board[to120];
    if mover == 0 || mover == OFF {
        return 0;
    }
    let pins = pinned(p, c) | pinned(p, opp);
    if pins & (1u64 << from) != 0 {
        return 0;
    }
    let t = tables();
    let ty = (mover & 7) as usize;
    let atk = match ty {
        1 => if opp == 0 { t.att.pawn_w[from] } else { t.att.pawn_b[from] },
        2 => t.att.knight[from],
        3 => t.bishop_attacks(from, p.occ),
        4 => t.rook_attacks(from, p.occ),
        5 => t.queen_attacks(from, p.occ),
        _ => t.att.king[from],
    };
    let mut targets = atk & p.by_color[c] & !p.bb[5 + 6 * c];
    let mut best = 0i32;
    while targets != 0 {
        let sq = targets.trailing_zeros() as usize;
        targets &= targets - 1;
        let m = super::position::mk_move(super::bitboard::SQ64_TO_120[from], super::bitboard::SQ64_TO_120[sq], 0, 1);
        best = best.max(see_side_ex(p, m, opp, pins));
    }
    best
}

/// Pin-aware SEE gain for side `by` capturing whatever stands on `sq64`
/// with its least valuable attacker (0 if it cannot or would lose).
pub fn capture_gain(p: &Position, sq64: usize, by: usize) -> i32 {
    let bq = p.bb[bb_idx(WB)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WB + 8)] | p.bb[bb_idx(WQ + 8)];
    let rq = p.bb[bb_idx(WR)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WR + 8)] | p.bb[bb_idx(WQ + 8)];
    let pins = pinned(p, 0) | pinned(p, 1);
    let att = attackers_to(p, sq64, p.occ, bq, rq) & p.by_color[by] & !pins;
    let Some((bit, _)) = least_valuable(p, att, by) else { return 0 };
    let from = bit.trailing_zeros() as usize;
    let m = super::position::mk_move(super::bitboard::SQ64_TO_120[from], super::bitboard::SQ64_TO_120[sq64], 0, 1);
    see_side_ex(p, m, by, pins).max(0)
}

/// SEE of capture `m` by side `us`; pieces in `excl` (pinned) never recapture.
pub fn see_side_ex(p: &Position, m: u32, us: usize, excl: u64) -> i32 {
    let from120 = m_from(m) as usize;
    let to120 = m_to(m) as usize;
    if from120 >= 120 || to120 >= 120 {
        return 0;
    }
    let (from, to) = (SQ120_TO_64[from120], SQ120_TO_64[to120]);
    if from < 0 || to < 0 {
        return 0;
    }
    let (from, to) = (from as usize, to as usize);
    let flag = m_flag(m);
    if flag == 3 {
        return 0;
    }
    let mover = p.board[from120];
    if mover == 0 || mover == OFF {
        return 0;
    }
    let promo = m_promo(m) as usize;

    let mut occ = p.occ & !(1u64 << from);
    let mut gain = [0i32; 32];
    let victim = p.board[to120] as usize;
    gain[0] = if flag == 2 { VAL[WP as usize] } else { VAL[victim & 15] };
    if flag == 2 {
        // Remove the captured pawn so x-rays through it are seen.
        let cap120 = if us == 0 { to120.wrapping_sub(10) } else { to120 + 10 };
        if cap120 < 120 {
            let c64 = SQ120_TO_64[cap120];
            if c64 >= 0 {
                occ &= !(1u64 << c64);
            }
        }
    }
    // Value of the piece now standing on `to` (what the next capturer wins).
    let mut on_sq = if promo != 0 && promo < VAL.len() {
        gain[0] += VAL[promo] - VAL[WP as usize];
        VAL[promo]
    } else {
        VAL[(mover & 7) as usize]
    };

    let t = tables();
    let bq = p.bb[bb_idx(WB)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WB + 8)] | p.bb[bb_idx(WQ + 8)];
    let rq = p.bb[bb_idx(WR)] | p.bb[bb_idx(WQ)] | p.bb[bb_idx(WR + 8)] | p.bb[bb_idx(WQ + 8)];

    let mut attackers = attackers_to(p, to, occ, bq, rq) & occ & !excl;
    let mut side = 1 - us;
    let mut d = 0usize;
    loop {
        let Some((bit, ty)) = least_valuable(p, attackers & p.by_color[side], side) else { break };
        if d + 1 >= gain.len() {
            break;
        }
        // A king may only recapture if the square is not attacked afterwards.
        if ty == 6 && attackers & p.by_color[1 - side] & occ != 0 {
            break;
        }
        d += 1;
        gain[d] = on_sq - gain[d - 1];
        on_sq = VAL[ty];
        occ &= !bit;
        if ty == 1 || ty == 3 || ty == 5 {
            attackers |= t.bishop_attacks(to, occ) & bq;
        }
        if ty == 4 || ty == 5 {
            attackers |= t.rook_attacks(to, occ) & rq;
        }
        attackers &= occ & !excl;
        side = 1 - side;
    }
    while d > 0 {
        d -= 1;
        gain[d] = -((-gain[d]).max(gain[d + 1]));
    }
    gain[0]
}

