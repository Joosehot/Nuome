//! Steldens' classic (traditional) evaluation, copied from steldens-rs/src/eval
//! (the eval of ChessEngine's StarNet bench100, 76/100); NNUE switches removed.

// Evaluation: line-by-line port of fast.js L159-360. All terms white-centric,
// computed per side and summed with sign; f64 throughout, rounded at the end.
pub mod weights;

use weights::{CENTER4, EXT_CENTER, PHASE, PST_EG, PST_MG, W};
use crate::fast::bitboard::{tables, SQ120_TO_64, SQ64_TO_120};
use crate::fast::position::*;

const K_DIR: [i32; 8] = [-11, -10, -9, -1, 1, 9, 10, 11];

// Pawn span masks (sq64-indexed, [side]): same file strictly ahead; adjacent
// files strictly ahead; adjacent files on the same rank or behind. These are
// exactly the square sets the old mailbox loops visited.
const FILE_A: u64 = 0x0101_0101_0101_0101;
const RANK_1: u64 = 0xFF;
struct PawnMasks {
    file_fwd: [[u64; 64]; 2],
    adj_fwd: [[u64; 64]; 2],
    adj_back: [[u64; 64]; 2],
}
const fn build_pawn_masks() -> PawnMasks {
    let mut pm = PawnMasks { file_fwd: [[0; 64]; 2], adj_fwd: [[0; 64]; 2], adj_back: [[0; 64]; 2] };
    let mut s = 0;
    while s < 64 {
        let f = (s & 7) as i32;
        let r = (s >> 3) as i32;
        let mut adj = 0u64;
        if f > 0 { adj |= FILE_A << (f - 1); }
        if f < 7 { adj |= FILE_A << (f + 1); }
        let file = FILE_A << f;
        // ranks strictly above r / strictly below r / at or below r
        let above: u64 = if r < 7 { !0u64 << (8 * (r + 1)) } else { 0 };
        let at_or_below: u64 = !above;
        let below: u64 = if r > 0 { (1u64 << (8 * r)) - 1 } else { 0 };
        let at_or_above: u64 = !below;
        pm.file_fwd[0][s] = file & above;
        pm.adj_fwd[0][s] = adj & above;
        pm.adj_back[0][s] = adj & at_or_below;
        pm.file_fwd[1][s] = file & below;
        pm.adj_fwd[1][s] = adj & below;
        pm.adj_back[1][s] = adj & at_or_above;
        s += 1;
    }
    pm
}
const PM: PawnMasks = build_pawn_masks();
// Squares the old center_space loop scanned: ranks 5-8 for white, 1-4 for black.
const HALF: [u64; 2] = [0xFFFF_FFFF_0000_0000, 0x0000_0000_FFFF_FFFF];

#[inline]
fn bit_iter(mut bb: u64, mut f: impl FnMut(usize)) {
    while bb != 0 {
        f(bb.trailing_zeros() as usize);
        bb &= bb - 1;
    }
}

// JS `x >> n` masks n to 5 bits (replicates fast.js pawnFiles shifts at f-1).
#[inline]
fn js_shift(x: u32, n: i32) -> u32 {
    x >> ((n as u32) & 31)
}

pub struct Eval<'a> {
    b: &'a [u8; 120],
    turn: u8,
    castle: u8,
    piece_list: [[usize; 32]; 2],
    piece_len: [usize; 2],
    cnt: [[u8; 120]; 2],
    lva: [[i16; 120]; 2],
    atk_types: [[u8; 120]; 2],
    pawn_files: [u32; 2],
    // Per-side derived data cached once in `new` (identical to recomputing).
    king_sq: [usize; 2],
    by_color: [u64; 2],
    occ: u64,
    bb: &'a [u64; 12],
    att_bb: [u64; 2],   // squares attacked at least once (cnt > 0)
    pawns: [u64; 2],
    weak: [u64; 2],     // pawns for which is_weak_pawn() holds
    material: f64,
    pub ph: i32,
    pub mg_p: i32,
    pub opening: bool,
    pub endgame: bool,
}

#[inline]
fn is_pawn(pc: u8, side: u8) -> bool {
    pc == if side != 0 { BP } else { WP }
}

impl<'a> Eval<'a> {
    pub fn new(p: &'a Position) -> Eval<'a> {
        let mut e = Eval {
            b: &p.board,
            turn: p.turn,
            castle: p.castle,
            piece_list: [[0; 32]; 2],
            piece_len: [0, 0],
            cnt: [[0; 120]; 2],
            lva: [[9999; 120]; 2],
            atk_types: [[0; 120]; 2],
            pawn_files: [0, 0],
            king_sq: [0, 0],
            by_color: p.by_color,
            occ: p.occ,
            bb: &p.bb,
            att_bb: [0, 0],
            pawns: [p.bb[bb_idx(WP)], p.bb[bb_idx(BP)]],
            weak: [0, 0],
            material: 0.0,
            ph: 0,
            mg_p: 24,
            opening: false,
            endgame: false,
        };
        e.build_attack_map();
        for side in 0..2 {
            let pf = &mut e.pawn_files[side];
            for i in 0..e.piece_len[side] {
                let m = e.piece_list[side][i];
                let pc = e.b[m];
                if pc == if side != 0 { BP } else { WP } {
                    *pf |= 1 << file_of120(m);
                }
            }
            e.king_sq[side] = e.find_king(side);
        }
        for side in 0..2 {
            let my = e.pawns[side];
            let files = e.pawn_files[side];
            let mut weak = 0u64;
            bit_iter(my, |s| {
                let f = (s & 7) as i32;
                let isolated =
                    js_shift(files, f - 1) & 1 == 0 && js_shift(files, f + 1) & 1 == 0;
                if isolated || PM.file_fwd[side][s] & my != 0 || PM.adj_back[side][s] & my == 0 {
                    weak |= 1u64 << s;
                }
            });
            e.weak[side] = weak;
        }
        e.material = e.material_pst_compute(); // sets ph/mg_p/opening/endgame
        e
    }

    // Attack map from bitboards. Pieces are visited in ascending sq64 order,
    // which is the same as ascending mailbox order (21..99), so piece_list
    // is identical to the old board scan. cnt/lva/atk_types are
    // order-independent (count / min / or). A slider's magic attack set is
    // exactly the ray-walk set (all squares up to and including the first
    // blocker of either colour).
    fn build_attack_map(&mut self) {
        let tb = tables();
        let occ = self.occ;
        let mut rem = occ;
        while rem != 0 {
            let s = rem.trailing_zeros() as usize;
            rem &= rem - 1;
            let m = SQ64_TO_120[s];
            let pc = self.b[m];
            let side = color_of(pc) as usize;
            let t = type_of(pc) as usize;
            let v = VAL[pc as usize];
            let type_bit = 1u8 << t;
            self.piece_list[side][self.piece_len[side]] = m;
            self.piece_len[side] += 1;
            let mut att = match t {
                1 => {
                    if side != 0 {
                        tb.att.pawn_b[s]
                    } else {
                        tb.att.pawn_w[s]
                    }
                }
                2 => tb.att.knight[s],
                3 => tb.bishop_attacks(s, occ),
                4 => tb.rook_attacks(s, occ),
                5 => tb.rook_attacks(s, occ) | tb.bishop_attacks(s, occ),
                _ => tb.att.king[s],
            };
            self.att_bb[side] |= att;
            let v = v as i16;
            let cnt = &mut self.cnt[side];
            let lva = &mut self.lva[side];
            let at = &mut self.atk_types[side];
            while att != 0 {
                let to = SQ64_TO_120[att.trailing_zeros() as usize];
                att &= att - 1;
                cnt[to] += 1;
                at[to] |= type_bit;
                if v < lva[to] {
                    lva[to] = v;
                }
            }
        }
    }

    fn loss_on(&self, side: usize, m: usize, val: i32) -> i32 {
        let o = 1 - side;
        if self.cnt[o][m] == 0 {
            return 0;
        }
        if self.cnt[side][m] == 0 {
            return val;
        }
        let l = self.lva[o][m] as i32;
        if l < val {
            val - l
        } else {
            0
        }
    }

    // Mobility from bitboards: squares reachable (not own-occupied, up to and
    // including the first enemy blocker) where the enemy's least valuable
    // attacker is not cheaper than this piece -- same set as the ray walk.
    fn mobility_of(&self, m: usize, side: usize) -> i32 {
        let pc = self.b[m];
        let t = type_of(pc) as usize;
        let v = VAL[pc as usize];
        let o = 1 - side;
        let s = SQ120_TO_64[m] as usize;
        let tb = tables();
        let mut att = match t {
            2 => tb.att.knight[s],
            3 => tb.bishop_attacks(s, self.occ),
            4 => tb.rook_attacks(s, self.occ),
            _ => tb.rook_attacks(s, self.occ) | tb.bishop_attacks(s, self.occ),
        } & !self.by_color[side];
        let lva = &self.lva[o];
        let v = v as i16;
        let mut mob = 0;
        while att != 0 {
            let to = SQ64_TO_120[att.trailing_zeros() as usize];
            att &= att - 1;
            if lva[to] >= v {
                mob += 1;
            }
        }
        mob
    }

    fn pinned_mask(&self, side: usize) -> ([usize; 8], usize) {
        self.pinned_to(side, self.king_sq_of(side))
    }

    /// Own pieces that shield `target` (king or queen) from an enemy slider.
    /// Bitboard x-ray form of the old ray walk: for each own piece that is
    /// the first blocker on a ray from `k`, the ray beyond it (up to and
    /// including the next blocker) must contain an enemy B/Q (diagonal) or
    /// R/Q (orthogonal). Buffer order is irrelevant (only `contains` is used).
    fn pinned_to(&self, side: usize, k: usize) -> ([usize; 8], usize) {
        let o = 1 - side;
        let mut buf = [0usize; 8];
        let mut n = 0usize;
        let s = SQ120_TO_64[k];
        if s < 0 {
            return (buf, n);
        }
        let s = s as usize;
        let tb = tables();
        let own = self.by_color[side];
        let occ = self.occ;
        let q = self.bb[bb_idx(if o != 0 { BQ } else { WQ })];
        let bq = self.bb[bb_idx(if o != 0 { BB } else { WB })] | q;
        let rq = self.bb[bb_idx(if o != 0 { BR } else { WR })] | q;
        let a = tb.bishop_attacks(s, occ);
        let mut blockers = a & own;
        while blockers != 0 {
            let b = blockers.trailing_zeros() as usize;
            blockers &= blockers - 1;
            let beyond = tb.bishop_attacks(s, occ & !(1u64 << b)) & !a;
            if beyond & bq != 0 && n < 8 {
                buf[n] = SQ64_TO_120[b];
                n += 1;
            }
        }
        let a = tb.rook_attacks(s, occ);
        let mut blockers = a & own;
        while blockers != 0 {
            let b = blockers.trailing_zeros() as usize;
            blockers &= blockers - 1;
            let beyond = tb.rook_attacks(s, occ & !(1u64 << b)) & !a;
            if beyond & rq != 0 && n < 8 {
                buf[n] = SQ64_TO_120[b];
                n += 1;
            }
        }
        (buf, n)
    }

    #[inline]
    fn pieces(&self, side: usize) -> impl Iterator<Item = usize> + '_ {
        self.piece_list[side][..self.piece_len[side]].iter().copied()
    }

    #[inline]
    fn king_sq_of(&self, side: usize) -> usize {
        self.king_sq[side]
    }

    fn find_king(&self, side: usize) -> usize {
        for m in self.pieces(side) {
            if type_of(self.b[m]) == 6 {
                return m;
            }
        }
        0
    }

    /// Cached result of `material_pst_compute` (computed once in `new`).
    pub fn material_pst(&mut self) -> f64 {
        self.material
    }

    fn material_pst_compute(&mut self) -> f64 {
        let mut mg = 0f64;
        let mut eg = 0f64;
        let mut ph = 0i32;
        for side in 0..2 {
            let sg: f64 = if side != 0 { -1.0 } else { 1.0 };
            for i in 0..self.piece_len[side] {
                let m = self.piece_list[side][i];
                let pc = self.b[m];
                let t = type_of(pc) as usize;
                let s = SQ120_TO_64[m] as usize;
                let idx = if side != 0 { (7 - (s >> 3)) * 8 + (s & 7) } else { s };
                mg += sg * (VAL[pc as usize] + PST_MG[t][idx]) as f64;
                eg += sg * (VAL[pc as usize] + PST_EG[t][idx]) as f64;
                ph += PHASE[t];
            }
        }
        self.ph = ph;
        self.mg_p = self.ph.min(24);
        self.opening = self.ph >= 18;
        self.endgame = self.ph <= 8;
        (mg * self.mg_p as f64 + eg * (24 - self.mg_p) as f64) / 24.0
    }

    fn mobility(&self, side: usize) -> f64 {
        let mut mob = 0f64;
        let mut trapped = 0f64;
        for m in self.pieces(side) {
            let t = type_of(self.b[m]) as usize;
            if t == 1 || t == 6 {
                continue;
            }
            let mb = self.mobility_of(m, side) as f64;
            // Quiet-escape rule (knights and bishops): a minor piece away
            // from home whose only "escapes" are captures is trapped. A
            // quiet square counts only when it is empty and either not
            // attacked at all or defended by us and attacked only by pieces
            // worth at least as much (Nxc3 with the knight stuck on c3).
            if (t == 2 || t == 3) && W.quiet_trap > 0.0 {
                let home = rank_of120(m) == if side != 0 { 7 } else { 0 };
                if !home {
                    let o = 1 - side;
                    let s = SQ120_TO_64[m] as usize;
                    let tb = tables();
                    let mut att = (if t == 2 { tb.att.knight[s] } else { tb.bishop_attacks(s, self.occ) }) & !self.occ;
                    let v = VAL[self.b[m] as usize] as i16;
                    let mut quiet_safe = 0;
                    while att != 0 {
                        let to = SQ64_TO_120[att.trailing_zeros() as usize];
                        att &= att - 1;
                        let safe = self.cnt[o][to] == 0 || (self.cnt[side][to] > 0 && self.lva[o][to] >= v);
                        if safe {
                            quiet_safe += 1;
                        }
                    }
                    if quiet_safe == 0 {
                        trapped += W.quiet_trap * VAL[self.b[m] as usize] as f64;
                    }
                }
            }
            mob += mb * (match t {
                2 => W.mob_n,
                3 => W.mob_b,
                4 => W.mob_r,
                _ => W.mob_q,
            });
            let home = rank_of120(m) == if side != 0 { 7 } else { 0 };
            if !home {
                if mb == 0.0 {
                    trapped += match t {
                        5 => 40.0,
                        4 => 30.0,
                        _ => 25.0,
                    };
                } else if mb == 1.0 {
                    trapped += if t == 5 { 15.0 } else { 10.0 };
                }
            }
        }
        // Old eval kept whole; StarCelestials adds its value-proportional
        // trapped card on top (see `celestials`).
        mob * (if self.opening { 1.2 } else { 1.5 }) * W.mob - trapped * W.trapped
    }

    /// StarCelestials: every piece is a character with strengths, weaknesses
    /// and a living value. Unscaled centipawns (added outside pos_scale).
    ///  - trapped: a piece with no safe square loses a share of its value
    ///    (a knight with nowhere to go is not worth 400);
    ///  - king: middlegame escape squares (none = mating-net risk), endgame
    ///    activity;
    ///  - "I have a good move": each piece reports its best capture gain
    ///    (one-ply exchange estimate); the side to move banks a share of the
    ///    best two reports, the other side a smaller share (a threat).
    fn celestials(&self, side: usize) -> f64 {
        if W.celestials <= 0.0 {
            return 0.0;
        }
        let o = 1 - side;
        let tb = tables();
        let mut score = 0f64;
        let mut best1 = 0i32;
        let mut best2 = 0i32;
        for m in self.pieces(side) {
            let pc = self.b[m];
            let t = type_of(pc) as usize;
            let v = VAL[pc as usize];
            let s = SQ120_TO_64[m] as usize;
            // attack set
            let att = match t {
                1 => if side == 0 { tb.att.pawn_w[s] } else { tb.att.pawn_b[s] },
                2 => tb.att.knight[s],
                3 => tb.bishop_attacks(s, self.occ),
                4 => tb.rook_attacks(s, self.occ),
                5 => tb.rook_attacks(s, self.occ) | tb.bishop_attacks(s, self.occ),
                _ => tb.att.king[s],
            };
            // good-move report: best one-ply capture gain
            let mut caps = att & self.by_color[o];
            let mut best = 0i32;
            while caps != 0 {
                let to = SQ64_TO_120[caps.trailing_zeros() as usize];
                caps &= caps - 1;
                let victim = VAL[self.b[to] as usize];
                if victim >= 20000 {
                    continue;
                }
                let defended = self.cnt[o][to] > 0;
                let gain = if !defended { victim } else { victim - v };
                if gain > best {
                    best = gain;
                }
            }
            if best > best1 {
                best2 = best1;
                best1 = best;
            } else if best > best2 {
                best2 = best;
            }
            if t == 1 {
                continue;
            }
            if t == 6 {
                // king card
                let zone = tb.att.king[s] & !self.by_color[side];
                let safe = (zone & !self.att_bb[o]).count_ones() as i32;
                if self.endgame {
                    score += W.cel_king_eg * safe as f64;
                } else {
                    score -= W.cel_king * match safe {
                        0 => 60.0,
                        1 => 30.0,
                        2 => 10.0,
                        _ => 0.0,
                    };
                }
                continue;
            }
            // trapped card: value-proportional
            let home = rank_of120(m) == if side != 0 { 7 } else { 0 };
            if !home {
                let mb = self.mobility_of(m, side);
                let full = match t {
                    2 => W.cel_trap_n,
                    3 => W.cel_trap_b,
                    4 => W.cel_trap_r,
                    _ => W.cel_trap_q,
                };
                let frac = match mb {
                    0 => full,
                    1 => full * 0.4,
                    _ => 0.0,
                };
                score -= frac * v as f64;
            }
        }
        let to_move = side == self.turn as usize;
        let best1 = best1.min(900) as f64;
        let best2 = best2.min(900) as f64;
        // The side to move can cash its best report; the other side's report
        // is only a threat the mover will usually parry (the threats term
        // covers pieces that cannot be saved), so it counts for little.
        score += if to_move { W.cel_report * best1 + W.cel_report2 * best2 } else { W.cel_threat * best1.min(400.0) };
        score
    }

    fn threats(&self, side: usize) -> f64 {
        let mut losses = [0f64; 16];
        let mut n_losses = 0usize;
        let (pins, n_pins) = self.pinned_mask(side);
        let is_pinned = |sq: usize| pins[..n_pins].contains(&sq);
        let mut pen = 0f64;
        for m in self.pieces(side) {
            let pc = self.b[m];
            let t = type_of(pc) as usize;
            if t == 6 {
                continue;
            }
            let l = self.loss_on(side, m, VAL[pc as usize]) as f64;
            if l > 0.0 {
                let v = if is_pinned(m) { l + 0.5 } else { l };
                if n_losses < 16 {
                    losses[n_losses] = v;
                    n_losses += 1;
                }
            }
            if t != 1 && is_pinned(m) {
                pen += W.pin;
            }
        }
        // Relative pins against the queen: a piece that shields the queen from
        // an enemy slider is a tactical liability (Bh4 x Nf6 - Qe7).
        for q in self.pieces(side) {
            if type_of(self.b[q]) != 5 {
                continue;
            }
            let (qp, nq) = self.pinned_to(side, q);
            for &sq in &qp[..nq] {
                if !is_pinned(sq) {
                    pen += W.pin * 5.0;
                }
            }
        }
        losses[..n_losses].sort_by(|a, b| b.partial_cmp(a).unwrap());
        if side == self.turn as usize && n_losses > 0 {
            // drop the first integer-valued loss (JS findIndex + splice)
            if let Some(i) = losses[..n_losses].iter().position(|x| x % 1.0 == 0.0) {
                losses.copy_within(i + 1..n_losses, i);
                n_losses -= 1;
            }
        } else if n_losses > 0 && W.tempo_rule > 0.0 {
            // Tempo rule: the mover cannot both rescue its own hanging piece
            // and cash in on ours. If the mover's biggest exposed piece is
            // worth at least as much as our biggest, it spends the tempo on
            // itself and we save ours next; otherwise our loss shrinks by
            // what the mover leaves behind.
            let mover = self.turn as usize;
            let mut mover_top = 0f64;
            for m in self.pieces(mover) {
                let pc = self.b[m];
                if type_of(pc) as usize == 6 {
                    continue;
                }
                let l = self.loss_on(mover, m, VAL[pc as usize]) as f64;
                if l > mover_top {
                    mover_top = l;
                }
            }
            if mover_top > 0.0 {
                let top = losses[0].floor();
                if mover_top >= top {
                    losses.copy_within(1..n_losses, 0);
                    n_losses -= 1;
                } else {
                    losses[0] = (top - mover_top) + (losses[0] - top);
                }
            }
        }
        if n_losses > 0 {
            pen += losses[0].floor()
                + if n_losses > 1 { losses[1].floor() * 0.5 } else { 0.0 };
        }
        -pen * W.threat
    }

    fn king_safety(&self, side: usize) -> f64 {
        let o = 1 - side;
        let k = self.king_sq_of(side);
        let mut danger = 0f64;
        let mut shield = 0f64;
        let mut zone = [0usize; 12];
        let mut zone_n = 0usize;
        for &d in &K_DIR {
            let to = (k as i32 + d) as usize;
            if self.b[to] != OFF {
                zone[zone_n] = to;
                zone_n += 1;
            }
        }
        zone[zone_n] = k;
        zone_n += 1;
        let dir: i32 = if side != 0 { -10 } else { 10 };
        // Extended zone: the 3x3 box plus the three squares two ranks ahead.
        for df in [-1, 0, 1] {
            let to = (k as i32 + 2 * dir + df) as usize;
            if self.b[to] != OFF && zone_n < zone.len() {
                zone[zone_n] = to;
                zone_n += 1;
            }
        }
        // Weighted attack units by attacker type, minus own defenders.
        const UNITS: [f64; 7] = [0.0, 0.0, 2.0, 2.0, 3.0, 5.0, 0.0];
        let mut units = 0f64;
        let mut defend = 0f64;
        for &z in &zone[..zone_n] {
            let a = self.atk_types[o][z];
            let d = self.atk_types[side][z];
            for t in 2..6 {
                if a & (1 << t) != 0 {
                    units += UNITS[t] * if z == k { 1.5 } else { 1.0 };
                }
                if d & (1 << t) != 0 && t != 5 {
                    defend += 1.0;
                }
            }
        }
        units = (units - defend * 0.5).max(0.0);
        danger += (units * units / 4.0).min(500.0);
        for df in [-1, 0, 1] {
            let x = self.b[(k as i32 + dir + df) as usize];
            if is_pawn(x, side as u8) {
                shield += 1.0;
            } else {
                let two = (k as i32 + 2 * dir + df) as usize;
                if self.b[two] != OFF && is_pawn(self.b[two], side as u8) {
                    shield += 0.5;
                }
            }
        }
        let mut open_f = 0f64;
        for df in [-1, 0, 1] {
            let f = file_of120(k) + df;
            if !(0..8).contains(&f) {
                continue;
            }
            let mine = if side != 0 {
                js_shift(self.pawn_files[1], f)
            } else {
                js_shift(self.pawn_files[0], f)
            } & 1;
            if mine == 0 {
                open_f += 1.0;
            }
        }
        let enemy_queen = self.bb[bb_idx(if o != 0 { BQ } else { WQ })] != 0;
        // Files kf-1..kf+1 (clipped to the board).
        let kf = file_of120(k);
        let kr = rank_of120(k);
        let mut near = FILE_A << kf;
        if kf > 0 {
            near |= FILE_A << (kf - 1);
        }
        if kf < 7 {
            near |= FILE_A << (kf + 1);
        }
        // storm / heavy_f are integer sums: order-independent, exact in f64.
        let mut storm_i = 0i32;
        bit_iter(self.pawns[o] & near, |s| {
            let dr = ((s >> 3) as i32 - kr).abs();
            if dr <= 3 {
                storm_i += 4 - dr;
            }
        });
        let storm = storm_i as f64;
        let heavy = self.bb[bb_idx(if o != 0 { BR } else { WR })]
            | self.bb[bb_idx(if o != 0 { BQ } else { WQ })];
        let mut heavy_i = 0i32;
        let my_files = self.pawn_files[side];
        bit_iter(heavy & near, |s| {
            let f = (s & 7) as i32;
            if js_shift(my_files, f) & 1 == 0 {
                heavy_i += 1;
            }
        });
        let heavy_f = heavy_i as f64;
        // King driven out of its shelter (middlegame, enemy queen present):
        // a king two or more ranks from its back rank is a mating target
        // regardless of material, so the penalty grows quadratically.
        let home_rank = if side != 0 { 7 } else { 0 };
        let out = (rank_of120(k) - home_rank).abs();
        let exposure = if out >= 2 { (out as f64) * (out as f64) * 120.0 } else { 0.0 };
        let mut score = -(danger * (W.king_danger / 8.0)
            + exposure
            + (3.0 - shield) * W.shield * {
                // a stripped king is a target as long as the enemy queen is
                // on the board, whatever the phase counter says
                let ph = self.mg_p as f64 / 24.0;
                if enemy_queen { ph.max(W.shield_floor) } else { ph }
            }
            + open_f * (if enemy_queen { W.open_file } else { W.open_file / 3.0 })
            + storm * W.storm
            + heavy_f * W.heavy_file);
        if !enemy_queen {
            score *= 0.5;
        }
        if self.endgame {
            score *= 0.3;
        }
        score - self.greek_gift(side)
    }

    /// Greek-gift / h-pawn sacrifice pattern against a short-castled king:
    /// enemy bishop on the long diagonal to the rook-pawn square (path clear),
    /// an enemy knight that can reach g5/g4 (or already there), the enemy
    /// queen on the board and the king's knight not on f6/f3. Returns a
    /// penalty in raw cp for `side` (the defender).
    fn greek_gift(&self, side: usize) -> f64 {
        let o = 1 - side;
        let k = self.king_sq_of(side);
        let kf = file_of120(k);
        let home = if side != 0 { 7 } else { 0 };
        if rank_of120(k) != home || kf < 6 {
            return 0.0; // only a short-castled king (g/h file, back rank)
        }
        let mut has_queen = false;
        for m in self.pieces(o) {
            if type_of(self.b[m]) == 5 {
                has_queen = true;
                break;
            }
        }
        if !has_queen {
            return 0.0;
        }
        // Squares (120-based) relative to the defender: h-pawn square, g5-type
        // knight outpost, f6-type defensive knight square, f3-type knight start.
        let (h_pawn, g5, f6, f3_sq) = if side != 0 { (88, 67, 76, 46) } else { (38, 57, 47, 77) };
        let my_p = if side != 0 { BP } else { WP };
        if self.b[h_pawn] != my_p {
            return 0.0;
        }
        // Enemy bishop attacking the h-pawn square along the b1-h7 (b8-h2) diagonal.
        let op_b = if side != 0 { WB } else { BB };
        let step: i32 = if side != 0 { -11 } else { 11 }; // from h7 towards b1 (h2 towards b8)
        let mut sq = h_pawn as i32 + step;
        let mut bishop = false;
        while self.b[sq as usize] != OFF {
            let pc = self.b[sq as usize];
            if pc != EMPTY {
                bishop = pc == op_b;
                break;
            }
            sq += step;
        }
        if !bishop {
            return 0.0;
        }
        // Enemy knight on g5 already, or on f3/h3/e4/e6-type squares one hop from g5.
        let op_n = if side != 0 { WN } else { BN };
        let mut knight = self.b[g5] == op_n;
        if !knight {
            for &d in &[-21, -19, -12, -8, 8, 12, 19, 21] {
                let sq = (g5 as i32 + d) as usize;
                if self.b[sq] == op_n {
                    knight = true;
                    break;
                }
            }
        }
        if !knight {
            return 0.0;
        }
        let _ = f3_sq;
        let my_n = if side != 0 { BN } else { WN };
        let defended = self.b[f6] == my_n;
        if defended { 45.0 } else { 110.0 }
    }

    fn pawn_structure(&self, side: usize) -> f64 {
        let o = 1 - side;
        let my_p = if side != 0 { BP } else { WP };
        let op_p = if side != 0 { WP } else { BP };
        let fd: i32 = if side != 0 { -10 } else { 10 };
        let mut sc = 0f64;
        let files = self.pawn_files[side];
        let my = self.pawns[side];
        let op = self.pawns[o];
        for m in self.pieces(side) {
            if self.b[m] != my_p {
                continue;
            }
            let s = SQ120_TO_64[m] as usize;
            let f = file_of120(m);
            let r = rank_of120(m);
            let doubled = PM.file_fwd[side][s] & my != 0;
            if doubled {
                sc -= W.pawn_doubled;
            }
            let isolated =
                js_shift(files, f - 1) & 1 == 0 && js_shift(files, f + 1) & 1 == 0;
            if isolated {
                sc -= W.pawn_iso;
            }
            if !isolated {
                let support = PM.adj_back[side][s] & my != 0;
                if !support
                    && (is_pawn(self.b[(m as i32 + 2 * fd - 1) as usize], o as u8)
                        || is_pawn(self.b[(m as i32 + 2 * fd + 1) as usize], o as u8))
                {
                    sc -= W.pawn_backward;
                }
            }
            let passed = (PM.file_fwd[side][s] | PM.adj_fwd[side][s]) & op == 0;
            if passed {
                let adv = if side != 0 { 7 - r } else { r };
                let mut bonus = (if self.endgame { 18.0 } else { 10.0 })
                    + (adv * adv) as f64 * (if self.endgame { 4.0 } else { 2.0 });
                let stop = (m as i32 + fd) as usize;
                if self.b[stop] != OFF {
                    if self.b[stop] != EMPTY {
                        bonus *= 0.3;
                    } else if self.cnt[o][stop] > 0 && self.cnt[side][stop] == 0 {
                        bonus *= 0.5;
                    }
                }
                if self.loss_on(side, m, VAL[WP as usize]) > 0 {
                    bonus *= 0.3;
                }
                let mut mm = m as i32 - fd;
                while self.b[mm as usize] != OFF {
                    let x = self.b[mm as usize];
                    if x == if side != 0 { BR } else { WR } {
                        bonus += 10.0;
                        break;
                    }
                    if x != EMPTY {
                        break;
                    }
                    mm -= fd;
                }
                sc += bonus * W.passed;
            }
            if self.b[(m as i32 + fd - 1) as usize] == op_p
                || self.b[(m as i32 + fd + 1) as usize] == op_p
            {
                sc += W.lever;
            }
            for df in [-1, 1] {
                let ea = (m as i32 + fd + df) as usize;
                if self.b[ea] == EMPTY && self.b[(ea as i32 + fd) as usize] == op_p {
                    sc -= W.kick_pawn;
                    break;
                }
            }
        }
        for m in self.pieces(o) {
            if self.b[m] != op_p {
                continue;
            }
            if self.cnt[side][m] > 0
                && !is_pawn(self.b[(m as i32 - fd - 1) as usize], o as u8)
                && !is_pawn(self.b[(m as i32 - fd + 1) as usize], o as u8)
            {
                sc += 6.0;
            }
        }
        sc
    }

    fn outposts(&self, side: usize) -> f64 {
        let fd: i32 = if side != 0 { -10 } else { 10 };
        let op_p = if side != 0 { WP } else { BP };
        let mut sc = 0f64;
        for m in self.pieces(side) {
            let t = type_of(self.b[m]) as usize;
            if t != 2 && t != 3 {
                continue;
            }
            let r = rank_of120(m);
            let adv = if side != 0 { 7 - r } else { r };
            if adv < 3 || adv > 5 {
                continue;
            }
            let supported = is_pawn(self.b[(m as i32 - fd - 1) as usize], side as u8)
                || is_pawn(self.b[(m as i32 - fd + 1) as usize], side as u8);
            let mut attackable = false;
            let mut mm = m as i32 + fd;
            while self.b[mm as usize] != OFF {
                if self.b[(mm - 1) as usize] == op_p || self.b[(mm + 1) as usize] == op_p {
                    attackable = true;
                    break;
                }
                mm += fd;
            }
            if !attackable {
                sc += if t == 2 {
                    if supported { W.outpost } else { W.outpost / 2.0 }
                } else if supported {
                    W.outpost_b
                } else {
                    W.outpost_b / 2.0
                };
                let f = file_of120(m);
                if (2..=5).contains(&f) {
                    sc += 6.0;
                }
            }
        }
        sc
    }

    fn center_space(&self, side: usize) -> f64 {
        let o = 1 - side;
        let mut sc = 0f64;
        for &c in &CENTER4 {
            if is_pawn(self.b[c], side as u8) && self.loss_on(side, c, VAL[WP as usize]) == 0 {
                sc += W.center_pawn;
            }
            if self.cnt[side][c] > 0 {
                sc += W.center_atk;
            }
        }
        for &c in &EXT_CENTER {
            if self.cnt[side][c] > 0 {
                sc += 1.0;
            }
        }
        let space = (self.att_bb[side] & !self.att_bb[o] & HALF[side]).count_ones() as f64;
        sc + space * (if self.opening { W.space } else { W.space / 2.0 })
    }

    fn rook_activity(&self, side: usize) -> f64 {
        let files = self.pawn_files[side];
        let ofiles = self.pawn_files[1 - side];
        let mut sc = 0f64;
        let mut rooks = [0usize; 2];
        let mut n_rooks = 0usize;
        let mut bishops = 0;
        for m in self.pieces(side) {
            let t = type_of(self.b[m]) as usize;
            if t == 4 {
                if n_rooks < 2 {
                    rooks[n_rooks] = m;
                }
                n_rooks += 1;
                let f = file_of120(m);
                if js_shift(files, f) & 1 == 0 {
                    sc += if js_shift(ofiles, f) & 1 != 0 {
                        W.rook_semi
                    } else {
                        W.rook_open
                    };
                }
                if rank_of120(m) == if side != 0 { 1 } else { 6 } {
                    sc += W.rook7;
                }
            }
            if t == 3 {
                bishops += 1;
                let s = SQ120_TO_64[m] as usize;
                let n2 = (tables().bishop_attacks(s, self.occ) & !self.occ).count_ones();
                if n2 <= 2 {
                    sc -= W.bad_bishop;
                }
            }
        }
        if n_rooks == 2 {
            let same_r = rank_of120(rooks[0]) == rank_of120(rooks[1]);
            let same_f = file_of120(rooks[0]) == file_of120(rooks[1]);
            if same_r || same_f {
                let d: i32 = if same_r {
                    if rooks[1] > rooks[0] { 1 } else { -1 }
                } else if rooks[1] > rooks[0] {
                    10
                } else {
                    -10
                };
                let mut clear = true;
                let mut m = rooks[0] as i32 + d;
                while m != rooks[1] as i32 {
                    if self.b[m as usize] != EMPTY {
                        clear = false;
                        break;
                    }
                    m += d;
                }
                if clear {
                    sc += 8.0;
                }
            }
        }
        if bishops >= 2 {
            sc += W.bishop_pair;
        }
        sc
    }

    fn opening_principles(&self, side: usize) -> f64 {
        if !self.opening {
            return 0.0;
        }
        let k = self.king_sq_of(side);
        let home = if side != 0 { 7 } else { 0 };
        let op_p = if side != 0 { WP } else { BP };
        let fd: i32 = if side != 0 { -10 } else { 10 };
        let mut sc = 0f64;
        let minors_of = |sd: usize| -> u64 {
            if sd != 0 {
                self.bb[bb_idx(BN)] | self.bb[bb_idx(BB)]
            } else {
                self.bb[bb_idx(WN)] | self.bb[bb_idx(WB)]
            }
        };
        let home_mask = |sd: usize| -> u64 { if sd != 0 { RANK_1 << 56 } else { RANK_1 } };
        let undev = (minors_of(side) & home_mask(side)).count_ones() as i32;
        sc -= undev as f64 * W.undev;
        // Development lag: being behind the opponent in developed minors and
        // castling costs quadratically (pawn grabs that cost tempi look worse).
        let dev_of = |sd: usize| -> i32 {
            let mut out = (minors_of(sd) & !home_mask(sd)).count_ones() as i32;
            let ks = self.king_sq_of(sd);
            if ks == (if sd != 0 { 97 } else { 27 }) || ks == (if sd != 0 { 93 } else { 23 }) {
                out += 1;
            }
            out
        };
        let lag = dev_of(1 - side) - dev_of(side);
        if lag > 0 {
            sc -= (lag * lag) as f64 * W.dev_lag;
        }
        let castled_k = k == if side != 0 { 97 } else { 27 };
        let castled_q = k == if side != 0 { 93 } else { 23 };
        let rights = if side != 0 { self.castle & 12 } else { self.castle & 3 };
        if castled_k {
            sc += W.castled;
        } else if castled_q {
            sc += W.castled_q;
        } else if rights == 0 {
            sc -= W.no_rights;
        } else {
            sc += 5.0;
        }
        for m in self.pieces(side) {
            let t = type_of(self.b[m]) as usize;
            if t == 5 && rank_of120(m) != home && undev >= 2 {
                sc -= W.early_q;
            }
            if t == 2 && (file_of120(m) == 0 || file_of120(m) == 7) {
                sc -= W.rim;
            }
            if (2..=5).contains(&t) {
                let mut kick = false;
                for df in [-1, 1] {
                    let ea = m as i32 + fd + df;
                    if self.b[ea as usize] != EMPTY {
                        continue;
                    }
                    if self.b[(ea + fd) as usize] == op_p {
                        kick = true;
                        break;
                    }
                    let two = ea + fd + fd;
                    if self.b[(ea + fd) as usize] == EMPTY
                        && self.b[two as usize] == op_p
                        && rank_of120(two as usize) == if side != 0 { 1 } else { 6 }
                    {
                        kick = true;
                        break;
                    }
                }
                if kick {
                    sc -= W.kick;
                }
            }
        }
        sc
    }

    fn endgame_terms(&self, side: usize) -> f64 {
        if !self.endgame {
            return 0.0;
        }
        let k = self.king_sq_of(side);
        let cf = (file_of120(k) as f64 - 3.5).abs();
        let cr = (rank_of120(k) as f64 - 3.5).abs();
        (7.0 - (cf + cr)) * W.eg_king
    }

    fn weak_pawn_targets(&self, side: usize) -> f64 {
        let o = 1 - side;
        let mut sc = 0f64;
        // Ascending sq64 order == the old piece_list order.
        bit_iter(self.weak[side], |s| {
            let m = SQ64_TO_120[s];
            if self.cnt[o][m] > 0 {
                sc -= if self.cnt[side][m] > 0 { W.weak_own / 2.0 } else { W.weak_own };
            }
        });
        bit_iter(self.weak[o], |s| {
            let m = SQ64_TO_120[s];
            if self.cnt[side][m] > 0 {
                sc += if self.cnt[o][m] > 0 { W.weak_enemy / 2.0 } else { W.weak_enemy };
            }
        });
        sc
    }

    fn pawn_breaks(&self, side: usize) -> f64 {
        let my_p = if side != 0 { BP } else { WP };
        let op_p = if side != 0 { WP } else { BP };
        let o = 1 - side;
        let fd: i32 = if side != 0 { -10 } else { 10 };
        let mut sc = 0f64;
        for m in self.pieces(side) {
            if self.b[m] != my_p {
                continue;
            }
            let to1 = (m as i32 + fd) as usize;
            if self.b[to1] != EMPTY {
                continue;
            }
            let start_rank = rank_of120(m) == if side != 0 { 6 } else { 1 };
            let targets: [usize; 2] = [to1, (to1 as i32 + fd) as usize];
            let n_targets = if start_rank && self.b[targets[1]] == EMPTY { 2 } else { 1 };
            for &to in &targets[..n_targets] {
                if (self.b[(to as i32 + fd - 1) as usize] == op_p
                    || self.b[(to as i32 + fd + 1) as usize] == op_p)
                    && !(self.cnt[o][to] > 0 && self.cnt[side][to] == 0)
                {
                    sc += W.pawn_break;
                    let mut mm = m as i32 - fd;
                    while self.b[mm as usize] != OFF {
                        let x = self.b[mm as usize];
                        if x == EMPTY {
                            mm -= fd;
                            continue;
                        }
                        if x == if side != 0 { BR } else { WR }
                            || x == if side != 0 { BQ } else { WQ }
                        {
                            sc += W.break_line;
                        }
                        break;
                    }
                }
            }
        }
        sc
    }

    fn king_attack_potential(&self, side: usize) -> f64 {
        if self.endgame {
            return 0.0;
        }
        let o = 1 - side;
        // An uncommitted king (castling rights intact) is not an attack
        // target yet; otherwise e4+d4 alone hands the opponent the bonus.
        if (if o != 0 { self.castle & 12 } else { self.castle & 3 }) != 0 {
            return 0.0;
        }
        let k = self.king_sq_of(o);
        if k < 21 || k > 98 {
            return 0.0; // no king on the board (joker probe positions)
        }
        let dir: i32 = if o != 0 { -10 } else { 10 };
        let mut shield = 0;
        for df in [-1, 0, 1] {
            if is_pawn(self.b[(k as i32 + dir + df) as usize], o as u8) {
                shield += 1;
            }
        }
        if shield > 1 {
            return 0.0;
        }
        let kf = file_of120(k);
        let mut heavy = 0;
        for m in self.pieces(side) {
            let t = type_of(self.b[m]) as usize;
            if (t == 5 || t == 4) && (file_of120(m) - kf).abs() <= 3 {
                heavy += 1;
            }
        }
        if heavy >= 2 {
            W.king_attack
        } else if heavy == 1 {
            W.king_attack / 3.0
        } else {
            0.0
        }
    }

    // Strategy layer: permanent king-placement and castling-rights value,
    // valid in every phase. Moves that strip the opponent's castling rights
    // or strand their king in the centre now earn cp directly.
    fn strategy_rights(&self, side: usize) -> f64 {
        let k = self.king_sq_of(side);
        let castled_k = k == if side != 0 { 97 } else { 27 };
        let castled_q = k == if side != 0 { 93 } else { 23 };
        let rights = if side != 0 { self.castle & 12 } else { self.castle & 3 };
        if castled_k {
            30.0
        } else if castled_q {
            // queenside castling: base 20 plus the (negative) castled_q weight
            20.0 + W.castled_q
        } else {
            match rights.count_ones() {
                2 => 15.0,
                1 => 0.0,
                _ => {
                    if self.endgame {
                        -10.0
                    } else {
                        -60.0
                    }
                }
            }
        }
    }

    pub fn evaluate(&mut self) -> f64 {
        let mut score = self.material_pst();
        let mode = GUIDE_MODE.with(|g| g.get());
        if mode == 2 {
            // Material eval: piece values + PST, threats (material about to
            // change hands) and the flat material balance. Nothing else.
            for side in 0..2 {
                let sg: f64 = if side != 0 { -1.0 } else { 1.0 };
                score += sg * W.pos_scale * self.threats(side);
            }
            let mut pure = 0i32;
            for side in 0..2 {
                for m in self.pieces(side) {
                    let v = VAL[self.b[m] as usize];
                    if v < 20000 {
                        pure += if side == 0 { v } else { -v };
                    }
                }
            }
            if pure < 0 {
                score -= W.mat_balance;
            } else if pure > 0 {
                score += W.mat_balance;
            }
            return if self.turn != 0 { -score } else { score };
        }
        if mode == 1 {
            // Stardust-net guide eval: material (+ threats = material about to
            // change hands), development, king safety, attack potential.
            for side in 0..2 {
                let sg: f64 = if side != 0 { -1.0 } else { 1.0 };
                score += sg
                    * W.pos_scale
                    * (self.threats(side)
                        + self.king_safety(side)
                        + self.opening_principles(side)
                        + self.king_attack_potential(side));
            }
            score += self.celestials(0) - self.celestials(1);
            return if self.turn != 0 { -score } else { score };
        }
        score += self.celestials(0) - self.celestials(1);
        for side in 0..2 {
            let sg: f64 = if side != 0 { -1.0 } else { 1.0 };
            score += sg
                * W.pos_scale
                * (self.mobility(side)
                    + self.threats(side)
                    + self.king_safety(side)
                    + self.pawn_structure(side)
                    + self.outposts(side)
                    + self.center_space(side)
                    + self.rook_activity(side)
                    + self.opening_principles(side)
                    + self.endgame_terms(side)
                    + self.weak_pawn_targets(side)
                    + self.pawn_breaks(side)
                    + self.king_attack_potential(side)
                    + self.strategy_rights(side));
        }
        // Material discipline (no gambits against engines): positional
        // compensation for a material deficit of a pawn or more is halved in
        // the opening and middlegame. Activity rarely cashes in against
        // accurate defence, so a pawn down must look like a pawn down.
        if !self.endgame && (W.mat_discipline < 1.0 || W.mat_balance > 0.0) {
            // pure piece values (no PST), white's view
            let mut pure = 0i32;
            for side in 0..2 {
                for m in self.pieces(side) {
                    let v = VAL[self.b[m] as usize];
                    if v < 20000 {
                        pure += if side == 0 { v } else { -v };
                    }
                }
            }
            let pure = pure as f64;
            let rest = score - pure;
            // Pawn-sized deficits only (100..250): a piece sacrifice is a
            // tactical claim the search verifies; a pawn gambit is a positional
            // claim that does not pay against engines.
            let pawn_down = (100.0..=250.0).contains(&pure.abs());
            if pawn_down && ((pure < 0.0 && rest > 0.0) || (pure > 0.0 && rest < 0.0)) {
                score = pure + rest * W.mat_discipline;
            }
            // Material balance: a flat W.mat_balance for being ahead / behind
            // in pure material (unscaled, after the discipline so it is not
            // halved). Keeps the engine from trading material for activity.
            if pure < 0.0 {
                score -= W.mat_balance;
            } else if pure > 0.0 {
                score += W.mat_balance;
            }
        }
        if self.turn != 0 {
            -score
        } else {
            score
        }
    }

    pub fn eval_terms(&mut self) -> Vec<(&'static str, [i32; 2])> {
        let material = self.material_pst().round() as i32;
        let mut out = vec![("materialPst", [material, 0])];
        out.push(("celestials", [self.celestials(0).round() as i32, self.celestials(1).round() as i32]));
        let fns = [
            ("mobility", 0usize),
            ("threats", 1),
            ("kingSafety", 2),
            ("pawnStructure", 3),
            ("outposts", 4),
            ("centerSpace", 5),
            ("rookActivity", 6),
            ("openingPrinciples", 7),
            ("endgame", 8),
            ("weakPawnTargets", 9),
            ("pawnBreaks", 10),
            ("kingAttackPotential", 11),
            ("strategyRights", 12),
        ];
        // NB: material_pst() above may re-set phase flags; terms are evaluated
        // against the SAME position so phase flags are identical.
        for (name, idx) in fns {
            let vals = [
                match idx {
                    0 => self.mobility(0),
                    1 => self.threats(0),
                    2 => self.king_safety(0),
                    3 => self.pawn_structure(0),
                    4 => self.outposts(0),
                    5 => self.center_space(0),
                    6 => self.rook_activity(0),
                    7 => self.opening_principles(0),
                    8 => self.endgame_terms(0),
                    9 => self.weak_pawn_targets(0),
                    10 => self.pawn_breaks(0),
                    11 => self.king_attack_potential(0),
                    _ => self.strategy_rights(0),
                }
                .round() as i32,
                match idx {
                    0 => self.mobility(1),
                    1 => self.threats(1),
                    2 => self.king_safety(1),
                    3 => self.pawn_structure(1),
                    4 => self.outposts(1),
                    5 => self.center_space(1),
                    6 => self.rook_activity(1),
                    7 => self.opening_principles(1),
                    8 => self.endgame_terms(1),
                    9 => self.weak_pawn_targets(1),
                    10 => self.pawn_breaks(1),
                    11 => self.king_attack_potential(1),
                    _ => self.strategy_rights(1),
                }
                .round() as i32,
            ];
            out.push((name, vals));
        }
        out
    }
}

/// Side-to-move-perspective eval, rounded (testing entry point).
// Lockless shared eval cache: slot = (key ^ value, value). 4M slots * 16 B.
const EC_BITS: u32 = 22;
struct EcSlot {
    k: std::sync::atomic::AtomicU64,
    v: std::sync::atomic::AtomicU64,
}
fn eval_cache() -> &'static Vec<EcSlot> {
    static EC: std::sync::OnceLock<Vec<EcSlot>> = std::sync::OnceLock::new();
    EC.get_or_init(|| {
        (0..1usize << EC_BITS)
            .map(|_| EcSlot {
                k: std::sync::atomic::AtomicU64::new(0),
                v: std::sync::atomic::AtomicU64::new(0),
            })
            .collect()
    })
}

/// EvalMix: percentage of the NNUE value in the blend with the classic eval
/// when Eval=nnue (100 = pure NNUE).
pub static EVAL_MIX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(100);

/// StarMoss: the three evaluations as layers of one whole. When `MOSS_ON`,
/// every evaluation is
///   (w_mat * material_eval + w_cls * classic_eval + w_nn * nnue_eval) / sum
/// where material_eval = piece values + PST + threats + balance (mode 2),
/// classic = the full hand-written eval, nnue = the value network. Weights
/// in percent (UCI StarMoss / MossMat / MossClassic / MossNnue).
pub static MOSS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
pub static MOSS_MAT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
pub static MOSS_CLS: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(50);
pub static MOSS_NN: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(100);

thread_local! {
    /// Per-thread guide-eval switch (set by Stardust-net graph workers when
    /// `StarEval` = 1); alpha-beta threads keep the full evaluation.
    pub static GUIDE_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    /// Per-thread eval override (StarGlance sources): 0 = normal, 1 = MOSS
    /// (classic + old NNUE blend, no StarNNUE), 2 = classic only.
    pub static EVAL_OVERRIDE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}
pub fn set_eval_override(v: u8) {
    EVAL_OVERRIDE.with(|o| o.set(v));
}
/// 0 = full eval, 1 = guide eval, 2 = material eval (material, threats, balance).
pub fn guide_mode() -> u8 {
    GUIDE_MODE.with(|g| g.get())
}

pub fn set_guide_mode(mode: u8) {
    GUIDE_MODE.with(|g| g.set(mode));
}

/// StarOthello weight (UCI `StarOthello`, cp per square, 0 = off): the eval
/// gets weight * (squares our pieces cover - squares theirs cover), each
/// square counted once however many pieces attack it (Othello-style board
/// control, not move counts).
pub static OTHELLO_CP: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(4);

/// StarOthello king-zone weight (UCI `StarOthelloKing`): a covered square in
/// either king's zone (king ring + the rank in front of it) counts 1 + this.
pub static OTHELLO_KING: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(3);

/// King zone of side `c`: the king's square, its ring and the rank in front.
pub fn king_zone(p: &Position, c: usize) -> u64 {
    let t = crate::fast::bitboard::tables();
    let kb = p.bb[c * 6 + 5];
    if kb == 0 {
        return 0;
    }
    let ks = kb.trailing_zeros() as usize;
    let ring = t.att.king[ks] | (1u64 << ks);
    ring | if c == 0 { ring << 8 } else { ring >> 8 }
}

/// StarOthello score of side `c`: covered squares, plus OTHELLO_KING extra
/// for each covered square in its own king zone (defence) and in the enemy
/// king zone (attack).
pub fn othello_score(p: &Position, c: usize) -> i32 {
    let a = coverage_bb(p, c);
    let k = OTHELLO_KING.load(std::sync::atomic::Ordering::Relaxed);
    let st = OTHELLO_PAWN.load(std::sync::atomic::Ordering::Relaxed);
    let own = king_zone(p, c);
    a.count_ones() as i32 + k * ((a & own).count_ones() as i32 + (a & king_zone(p, 1 - c)).count_ones() as i32)
        + st * (pawn_cover(p, c) & own).count_ones() as i32
}

/// StarOthello stable squares weight (UCI `StarOthelloPawn`): own king-zone
/// squares covered by own pawns count this much extra. Like Othello's stable
/// discs they are permanent - a pawn never moves back, so a pawn push next
/// to the king gives those squares up for good, whatever pieces cover them now.
pub static OTHELLO_PAWN: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(4);

/// Squares attacked by the pawns of side `c`.
pub fn pawn_cover(p: &Position, c: usize) -> u64 {
    let pw = p.bb[c * 6];
    const NOT_A: u64 = 0xfefe_fefe_fefe_fefe;
    const NOT_H: u64 = 0x7f7f_7f7f_7f7f_7f7f;
    if c == 0 {
        ((pw << 7) & NOT_H) | ((pw << 9) & NOT_A)
    } else {
        ((pw >> 9) & NOT_H) | ((pw >> 7) & NOT_A)
    }
}

/// Number of distinct squares attacked by side `c` (0 white, 1 black).
pub fn coverage(p: &Position, c: usize) -> u32 {
    coverage_bb(p, c).count_ones()
}

/// Squares attacked by side `c`.
pub fn coverage_bb(p: &Position, c: usize) -> u64 {
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    let mut a = 0u64;
    let pawns = if c == 0 { &t.att.pawn_w } else { &t.att.pawn_b };
    let base = c * 6;
    let mut each = |mut bb: u64, f: &dyn Fn(usize) -> u64| {
        while bb != 0 {
            let s = bb.trailing_zeros() as usize;
            a |= f(s);
            bb &= bb - 1;
        }
    };
    each(p.bb[base], &|s| pawns[s]);
    each(p.bb[base + 1], &|s| t.att.knight[s]);
    each(p.bb[base + 2], &|s| t.bishop_attacks(s, occ));
    each(p.bb[base + 3], &|s| t.rook_attacks(s, occ));
    each(p.bb[base + 4], &|s| t.queen_attacks(s, occ));
    each(p.bb[base + 5], &|s| t.att.king[s]);
    a
}

/// Classic-eval share in percent blended into StarNNUE (UCI `StarNNUEMix`):
/// the net misjudges plain material losses (16.Na4 Bxe1 vs dala-2000: +70
/// against Stockfish's -102), the hand-written eval does not.
pub static NNUE_MIX: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
/// StarDiscipline (percent, 100 = off): material discipline on top of the
/// net. When a side is a pawn or more down in pure material, the net's
/// compensation beyond that material is scaled to this percentage (the net
/// rates sacrifices like 16.Bxh6 gxh6 Qxh6 at +130 where Stockfish says -230).
pub static STAR_DISCIPLINE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(100);

/// Pure material (piece values, no king) from the side to move's view.
fn pure_material_stm(p: &Position) -> i32 {
    use crate::fast::position::{bb_idx, VAL};
    let mut m = 0;
    for t in 1..=5u8 {
        m += (p.bb[bb_idx(t)].count_ones() as i32 - p.bb[bb_idx(t + 8)].count_ones() as i32) * VAL[t as usize];
    }
    if p.turn == 0 { m } else { -m }
}

/// Apply StarDiscipline to a side-to-move eval `v`.
fn discipline(p: &Position, v: i32) -> i32 {
    let d = STAR_DISCIPLINE.load(std::sync::atomic::Ordering::Relaxed);
    if d >= 100 {
        return v;
    }
    let mat = pure_material_stm(p);
    let comp = v - mat;
    // down material with positive compensation, or up material facing the
    // opponent's compensation: scale the compensation
    if (mat <= -100 && comp > 0) || (mat >= 100 && comp < 0) {
        mat + comp * d.max(0) / 100
    } else {
        v
    }
}

/// Side-to-move-perspective eval, exact f64 (search entry point — JS parity).
pub fn evaluate_f64(p: &Position) -> f64 {
    Eval::new(p).evaluate()
}

/// White-centric eval, rounded (parity/testing entry point).
pub fn evaluate_white(p: &Position) -> i32 {
    let mut e = Eval::new(p);
    let v = e.evaluate();
    let v = if p.turn != 0 { -v } else { v };
    v.round() as i32
}

/// King-safety term of `side` (raw, unscaled) - used by the king joker.
pub fn king_safety_of(p: &Position, side: usize) -> i32 {
    let mut e = Eval::new(p);
    let _ = e.material_pst(); // sets the phase flags
    e.king_safety(side).round() as i32
}

pub fn eval_terms(p: &Position) -> Vec<(&'static str, [i32; 2])> {
    Eval::new(p).eval_terms()
}
