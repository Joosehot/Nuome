// Position: mailbox 120 (eval port fidelity) + bitboards (speed), incremental u64 Zobrist.
// Piece codes match fast.js: EMPTY=0 WP=1..WK=6 OFF=7 BP=9..BK=14.
use super::bitboard::{tables, SQ64_TO_120, SQ120_TO_64};
use super::zobrist::zob;

pub const EMPTY: u8 = 0;
#[cfg(test)]
mod regression_tests {
    use super::*;
    #[test]
    fn null_move_restores_both_keys_and_position() {
        for fen in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        ] {
            let mut p = Position::from_fen(fen).unwrap();
            let before = p.clone();
            p.make_null();
            let first_key = p.key64();
            p.make_null();
            p.unmake_null();
            assert_eq!(p.key64(), first_key);
            p.unmake_null();
            assert_eq!(p.key64(), before.key64());
            assert_eq!(p.to_fen(), before.to_fen());
            assert_eq!(p.board, before.board);
            assert_eq!(p.bb, before.bb);
            assert_eq!(p.ply, before.ply);
            assert_eq!(p.tempo, before.tempo);
            assert_eq!(p.keys, before.keys);
            assert_eq!(p.undo.len(), before.undo.len());
        }
    }
}
pub const WP: u8 = 1;
pub const WN: u8 = 2;
pub const WB: u8 = 3;
pub const WR: u8 = 4;
pub const WQ: u8 = 5;
pub const WK: u8 = 6;
pub const OFF: u8 = 7;
pub const BP: u8 = 9;
pub const BN: u8 = 10;
pub const BB: u8 = 11;
pub const BR: u8 = 12;
pub const BQ: u8 = 13;
pub const BK: u8 = 14;

pub const VAL: [i32; 15] = [0, 120, 400, 410, 620, 1300, 20000, 0, 0, 120, 400, 410, 620, 1300, 20000];

#[inline(always)]
pub fn type_of(pc: u8) -> u8 {
    pc & 7
}
#[inline(always)]
pub fn color_of(pc: u8) -> u8 {
    pc >> 3
}
#[inline(always)]
pub fn file_of120(m: usize) -> i32 {
    (m % 10) as i32 - 1
}
#[inline(always)]
pub fn rank_of120(m: usize) -> i32 {
    (m / 10) as i32 - 2
}

// bitboard index for a piece code (0..11)
#[inline(always)]
pub fn bb_idx(pc: u8) -> usize {
    if pc < 8 { pc as usize - 1 } else { pc as usize - 9 + 6 }
}

#[derive(Clone, Copy)]
pub struct Undo {
    pub tempo: [u32; 2],
    pub cap: u8,
    pub castle: u8,
    pub ep: i8,
    pub halfmove: u32,
    pub hash: u32,
    pub hash2: u32,
    pub mv: u32,
    pub null: bool,
    /// make() pushed a StarNNUE accumulator copy (unmake pops it).
    pub acc2: bool,
}

#[derive(Clone)]
pub struct Position {
    pub tempo: [u32; 2],
    pub board: [u8; 120],
    pub bb: [u64; 12],
    pub by_color: [u64; 2],
    pub occ: u64,
    pub turn: u8, // 0 white, 1 black
    pub castle: u8, // 1 wK, 2 wQ, 4 bK, 8 bQ (same bits as fast.js)
    pub ep: i8,    // 120-square or -1
    pub halfmove: u32,
    pub king_sq: [usize; 2], // 120-based
    pub hash: u32,           // dual 32-bit keys, identical to fast.js
    pub hash2: u32,
    pub ply: usize,
    pub undo: Vec<Undo>,
    pub keys: Vec<u64>, // combined hash before each move, for repetition
    /// NNUE accumulators [white perspective, black perspective]; only
    /// maintained while super::nnue::active() (Eval=nnue with a loaded net).
    pub acc: super::nnue::Acc,
    /// Copies of `acc` pushed by make() (non-null moves) and restored by unmake().
    pub acc_stack: Vec<super::nnue::Acc>,
    /// StarNNUE lazy accumulator stack (see super::nnue2::Nn2).
    pub nn2: super::nnue2::Nn2Cell,
}

impl Position {
    pub fn new() -> Position {
        let mut p = Position {
            tempo: [0; 2],
            board: [OFF; 120],
            bb: [0; 12],
            by_color: [0; 2],
            occ: 0,
            turn: 0,
            castle: 0,
            ep: -1,
            halfmove: 0,
            king_sq: [0, 0],
            hash: 0,
            hash2: 0,
            ply: 0,
            undo: Vec::with_capacity(512),
            keys: Vec::with_capacity(512),
            acc: [[0.0; super::nnue::H1]; 2],
            acc_stack: Vec::new(),
            nn2: super::nnue2::Nn2Cell::default(),
        };
        for s in 0..64 {
            p.board[SQ64_TO_120[s]] = EMPTY;
        }
        p
    }

    #[inline]
    pub fn key64(&self) -> u64 {
        ((self.hash as u64) << 32) | self.hash2 as u64
    }

    #[inline]
    pub fn set_pc(&mut self, pc: u8, s120: usize) {
        let s64 = SQ120_TO_64[s120] as usize;
        let i = bb_idx(pc);
        let bit = 1u64 << s64;
        self.bb[i] |= bit;
        self.by_color[color_of(pc) as usize] |= bit;
        self.occ |= bit;
        self.board[s120] = pc;
    }

    #[inline]
    pub fn clear_pc(&mut self, pc: u8, s120: usize) {
        let s64 = SQ120_TO_64[s120] as usize;
        let i = bb_idx(pc);
        let bit = !(1u64 << s64);
        self.bb[i] &= bit;
        self.by_color[color_of(pc) as usize] &= bit;
        self.occ &= bit;
        self.board[s120] = EMPTY;
    }

    /// Port of fast.js computeHashA/computeHashB.
    pub fn compute_hashes(&self) -> (u32, u32) {
        let z = zob();
        let mut h = 0u32;
        let mut h2 = 0u32;
        for m in 21..99 {
            let pc = self.board[m];
            if pc > 0 && pc != OFF {
                h ^= z.piece[pc as usize * 120 + m];
                h2 ^= z.piece2[pc as usize * 120 + m];
            }
        }
        if self.turn != 0 {
            h ^= z.side;
            h2 ^= z.side2;
        }
        h ^= z.castle[self.castle as usize];
        h2 ^= z.castle2[self.castle as usize];
        if self.ep >= 0 {
            h ^= z.ep[self.ep as usize];
            h2 ^= z.ep2[self.ep as usize];
        }
        (h, h2)
    }

    pub fn compute_hash(&self) -> u64 {
        let (a, b) = self.compute_hashes();
        ((a as u64) << 32) | b as u64
    }

    pub fn from_fen(fen: &str) -> Result<Position, String> {
        let mut p = Position::new();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("bad FEN".into());
        }
        let rows: Vec<&str> = parts[0].split('/').collect();
        if rows.len() != 8 {
            return Err("bad FEN board".into());
        }
        let piece_of = |c: char| -> Option<u8> {
            Some(match c {
                'P' => WP, 'N' => WN, 'B' => WB, 'R' => WR, 'Q' => WQ, 'K' => WK,
                'p' => BP, 'n' => BN, 'b' => BB, 'r' => BR, 'q' => BQ, 'k' => BK,
                _ => return None,
            })
        };
        for (ri, row) in rows.iter().enumerate() {
            let rank = 7 - ri;
            let mut file = 0i32;
            for c in row.chars() {
                if let Some(d) = c.to_digit(10) {
                    file += d as i32;
                } else {
                    let pc = piece_of(c).ok_or("bad FEN piece")?;
                    let s120 = SQ64_TO_120[rank * 8 + file as usize];
                    if type_of(pc) == 6 {
                        p.king_sq[color_of(pc) as usize] = s120;
                    }
                    p.set_pc(pc, s120);
                    file += 1;
                }
            }
            if file != 8 {
                return Err("bad FEN row".into());
            }
        }
        p.turn = match parts[1] {
            "w" => 0,
            "b" => 1,
            _ => return Err("bad FEN turn".into()),
        };
        p.castle = 0;
        if parts.len() > 2 && parts[2] != "-" {
            for c in parts[2].chars() {
                p.castle |= match c {
                    'K' => 1, 'Q' => 2, 'k' => 4, 'q' => 8, _ => 0,
                };
            }
        }
        p.ep = -1;
        if parts.len() > 3 && parts[3] != "-" {
            let b = parts[3].as_bytes();
            if b.len() == 2 {
                let f = (b[0] - b'a') as usize;
                let r = (b[1] - b'1') as usize;
                p.ep = SQ64_TO_120[r * 8 + f] as i8;
            }
        }
        p.halfmove = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let (ha, hb) = p.compute_hashes();
        p.hash = ha;
        p.hash2 = hb;
        if super::nnue::active() {
            super::nnue::refresh(&mut p);
        }
        Ok(p)
    }

    pub fn to_fen(&self) -> String {
        const CHARS: [char; 15] = [
            '.', 'P', 'N', 'B', 'R', 'Q', 'K', '.', '.', 'p', 'n', 'b', 'r', 'q', 'k',
        ];
        let mut s = String::new();
        for r in (0..8).rev() {
            let mut empty = 0;
            for f in 0..8 {
                let pc = self.board[SQ64_TO_120[r * 8 + f]];
                if pc == EMPTY {
                    empty += 1;
                } else {
                    if empty > 0 {
                        s.push_str(&empty.to_string());
                        empty = 0;
                    }
                    s.push(CHARS[pc as usize]);
                }
            }
            if empty > 0 {
                s.push_str(&empty.to_string());
            }
            if r > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.turn == 0 { 'w' } else { 'b' });
        s.push(' ');
        let mut cr = String::new();
        if self.castle & 1 != 0 { cr.push('K'); }
        if self.castle & 2 != 0 { cr.push('Q'); }
        if self.castle & 4 != 0 { cr.push('k'); }
        if self.castle & 8 != 0 { cr.push('q'); }
        if cr.is_empty() { cr.push('-'); }
        s.push_str(&cr);
        s.push(' ');
        if self.ep >= 0 {
            let e = SQ120_TO_64[self.ep as usize] as usize;
            s.push((b'a' + (e & 7) as u8) as char);
            s.push((b'1' + (e >> 3) as u8) as char);
        } else {
            s.push('-');
        }
        s.push(' ');
        s.push_str(&self.halfmove.to_string());
        s.push_str(" 1");
        s
    }

    // Is square s120 attacked by color `by`? (bitboard implementation)
    pub fn attacked(&self, s120: usize, by: u8) -> bool {
        let t = tables();
        let s = SQ120_TO_64[s120] as usize;
        let them = self.by_color[by as usize];
        let bit = 1u64 << s;
        let pawn_att = if by == 0 { t.att.pawn_b[s] } else { t.att.pawn_w[s] };
        if pawn_att & self.bb[bb_idx(if by == 0 { WP } else { BP })] != 0 {
            return true;
        }
        if t.att.knight[s] & self.bb[bb_idx(if by == 0 { WN } else { BN })] & them != 0 {
            return true;
        }
        if t.att.king[s] & self.bb[bb_idx(if by == 0 { WK } else { BK })] != 0 {
            return true;
        }
        let bq = self.bb[bb_idx(if by == 0 { WB } else { BB })] | self.bb[bb_idx(if by == 0 { WQ } else { BQ })];
        if t.bishop_attacks(s, self.occ) & bq & them != 0 {
            return true;
        }
        let rq = self.bb[bb_idx(if by == 0 { WR } else { BR })] | self.bb[bb_idx(if by == 0 { WQ } else { BQ })];
        if t.rook_attacks(s, self.occ) & rq & them != 0 {
            return true;
        }
        let _ = bit;
        false
    }

    #[inline]
    pub fn in_check_c(&self, c: u8) -> bool {
        self.attacked(self.king_sq[c as usize], 1 - c)
    }

    /// Piece type (1..=6) on a 64-square, if occupied.
    #[inline]
    pub fn type_at64(&self, s: usize) -> Option<u8> {
        let bit = 1u64 << s;
        for (i, &b) in self.bb.iter().enumerate() {
            if b & bit != 0 {
                return Some(if i < 6 { i as u8 + 1 } else { i as u8 - 5 });
            }
        }
        None
    }

    /// King square (64-based) for a color, or None.
    #[inline]
    pub fn king_sq64(&self, c: u8) -> Option<usize> {
        let b = self.bb[bb_idx(if c == 0 { WK } else { BK })];
        if b == 0 { None } else { Some(b.trailing_zeros() as usize) }
    }
    #[inline]
    pub fn in_check(&self) -> bool {
        self.in_check_c(self.turn)
    }

    pub fn make(&mut self, m: u32) {
        let f = m_from(m) as usize;
        let t = m_to(m) as usize;
        let promo = m_promo(m);
        let flag = m_flag(m);
        let pc = self.board[f];
        let cap = self.board[t];
        if pc == EMPTY || pc == OFF {
            panic!("make() on empty from-square f={f} mv={m:#x} fen={} undo={} last={:?}", self.to_fen(), self.undo.len(), self.undo.last().map(|u| (u.mv, u.cap, u.null)));
        }
        self.undo.push(Undo {
            cap, castle: self.castle, ep: self.ep, halfmove: self.halfmove,
            tempo: self.tempo,
            hash: self.hash, hash2: self.hash2, mv: m, null: false, acc2: false,
        });
        self.keys.push(self.key64());
        let n2 = super::nnue2::live().is_some();
        if n2 {
            self.undo.last_mut().unwrap().acc2 = true;
        }
        let nn = if super::nnue::active() { super::nnue::net() } else { None };
        if let Some(n) = &nn {
            self.acc_stack.push(self.acc);
            n.remove_piece(&mut self.acc, pc, f);
            if cap != EMPTY {
                n.remove_piece(&mut self.acc, cap, t);
            }
        }

        let ty = type_of(pc);
        let s = SQ120_TO_64[f] as usize;
        let home = if self.turn == 0 { 0 } else { 56 };
        let on_home = match ty {
            2 => s == home + 1 || s == home + 6,
            3 => s == home + 2 || s == home + 5,
            4 => s == home || s == home + 7,
            5 => s == home + 3,
            _ => true,
        };
        if !on_home && flag != 1 && flag != 2 && self.occ.count_ones() > 20 {
            self.tempo[self.turn as usize] += 1;
        }

        let z = zob();
        if self.ep >= 0 {
            self.hash ^= z.ep[self.ep as usize];
            self.hash2 ^= z.ep2[self.ep as usize];
        }
        self.ep = -1;

        self.hash ^= z.piece[pc as usize * 120 + f];
        self.hash2 ^= z.piece2[pc as usize * 120 + f];
        self.clear_pc(pc, f);
        if cap != EMPTY {
            self.hash ^= z.piece[cap as usize * 120 + t];
            self.hash2 ^= z.piece2[cap as usize * 120 + t];
            // fast.js overwrote the mailbox slot; bitboards need an explicit clear
            self.clear_pc(cap, t);
        }
        let np = if promo != 0 {
            if self.turn != 0 { promo as u8 + 8 } else { promo as u8 }
        } else {
            pc
        };
        self.set_pc(np, t);
        self.hash ^= z.piece[np as usize * 120 + t];
        self.hash2 ^= z.piece2[np as usize * 120 + t];
        if let Some(n) = &nn {
            n.add_piece(&mut self.acc, np, t);
        }

        if flag == 2 {
            // en passant capture
            let cs = (t as i32 + if self.turn != 0 { 10 } else { -10 }) as usize;
            let cpp = self.board[cs];
            self.hash ^= z.piece[cpp as usize * 120 + cs];
            self.hash2 ^= z.piece2[cpp as usize * 120 + cs];
            self.clear_pc(cpp, cs);
            if let Some(n) = &nn {
                n.remove_piece(&mut self.acc, cpp, cs);
            }
        } else if flag == 3 {
            // castle: move the rook
            let rf = if t > f { t + 1 } else { t - 2 };
            let rt = if t > f { t - 1 } else { t + 1 };
            let r = self.board[rf];
            self.hash ^= z.piece[r as usize * 120 + rf] ^ z.piece[r as usize * 120 + rt];
            self.hash2 ^= z.piece2[r as usize * 120 + rf] ^ z.piece2[r as usize * 120 + rt];
            self.clear_pc(r, rf);
            self.set_pc(r, rt);
            if let Some(n) = &nn {
                n.remove_piece(&mut self.acc, r, rf);
                n.add_piece(&mut self.acc, r, rt);
            }
        } else if flag == 4 {
            // double push
            self.ep = (f as i32 + if self.turn != 0 { -10 } else { 10 }) as i8;
            self.hash ^= z.ep[self.ep as usize];
            self.hash2 ^= z.ep2[self.ep as usize];
        }

        if type_of(pc) == 6 {
            self.king_sq[self.turn as usize] = t;
        }

        self.hash ^= z.castle[self.castle as usize];
        self.hash2 ^= z.castle2[self.castle as usize];
        if f == 25 || t == 25 { self.castle &= !3; }
        if f == 95 || t == 95 { self.castle &= !12; }
        if f == 28 || t == 28 { self.castle &= !1; }
        if f == 21 || t == 21 { self.castle &= !2; }
        if f == 98 || t == 98 { self.castle &= !4; }
        if f == 91 || t == 91 { self.castle &= !8; }
        self.hash ^= z.castle[self.castle as usize];
        self.hash2 ^= z.castle2[self.castle as usize];

        // halfmove clock (fast.js lacks it; needed for repetition/50-move)
        if type_of(pc) == 1 || cap != EMPTY {
            self.halfmove = 0;
        } else {
            self.halfmove += 1;
        }

        if n2 {
            let mut d = super::nnue2::Dirty { king: type_of(pc) == 6, n_rem: 1, rem: [(pc, f), (0, 0), (0, 0)], add: (np, t), ks: self.king_sq };
            if cap != EMPTY {
                d.rem[d.n_rem as usize] = (cap, t);
                d.n_rem += 1;
            }
            if flag == 2 {
                let cs = (t as i32 + if self.turn != 0 { 10 } else { -10 }) as usize;
                d.rem[d.n_rem as usize] = (if self.turn != 0 { WP } else { BP }, cs);
                d.n_rem += 1;
            }
            self.nn2.get_mut().push(d);
        }

        self.turn = 1 - self.turn;
        self.hash ^= z.side;
        self.hash2 ^= z.side2;
        self.ply += 1;
    }

    pub fn unmake(&mut self) {
        let u = self.undo.pop().unwrap();
        if u.acc2 {
            self.nn2.get_mut().pop();
        }
        self.tempo = u.tempo;
        self.keys.pop();
        self.turn = 1 - self.turn;
        self.ply -= 1;
        if super::nnue::active() {
            if let Some(a) = self.acc_stack.pop() {
                self.acc = a;
            }
        }
        let m = u.mv;
        let f = m_from(m) as usize;
        let t = m_to(m) as usize;
        let promo = m_promo(m);
        let flag = m_flag(m);
        let mut pc = self.board[t];
        if promo != 0 {
            pc = if self.turn != 0 { BP } else { WP };
        }
        self.clear_pc(self.board[t], t);
        self.set_pc(pc, f);
        if u.cap != EMPTY {
            self.set_pc(u.cap, t);
        }
        self.board[t] = u.cap;
        if flag == 2 {
            let cs = (t as i32 + if self.turn != 0 { 10 } else { -10 }) as usize;
            self.set_pc(if self.turn != 0 { WP } else { BP }, cs);
        } else if flag == 3 {
            let rf = if t > f { t + 1 } else { t - 2 };
            let rt = if t > f { t - 1 } else { t + 1 };
            let r = self.board[rt];
            self.clear_pc(r, rt);
            self.set_pc(r, rf);
        }
        if type_of(pc) == 6 {
            self.king_sq[self.turn as usize] = f;
        }
        self.castle = u.castle;
        self.ep = u.ep;
        self.halfmove = u.halfmove;
        self.hash = u.hash;
        self.hash2 = u.hash2;
    }

    pub fn make_null(&mut self) {
        self.undo.push(Undo {
            cap: 0, castle: self.castle, ep: self.ep, halfmove: self.halfmove,
            tempo: self.tempo,
            hash: self.hash, hash2: self.hash2, mv: 0, null: true, acc2: false,
        });
        self.keys.push(self.key64());
        let z = zob();
        if self.ep >= 0 {
            self.hash ^= z.ep[self.ep as usize];
            self.hash2 ^= z.ep2[self.ep as usize];
        }
        self.ep = -1;
        self.turn = 1 - self.turn;
        self.hash ^= z.side;
        self.hash2 ^= z.side2;
        self.ply += 1;
        self.halfmove += 1;
    }

    pub fn unmake_null(&mut self) {
        let u = self.undo.pop().unwrap();
        self.tempo = u.tempo;
        self.keys.pop();
        self.turn = 1 - self.turn;
        self.ply -= 1;
        self.ep = u.ep;
        self.hash = u.hash;
        self.hash2 = u.hash2;
        self.halfmove = u.halfmove;
        self.castle = u.castle;
    }

    /// Threefold repetition (roaring_engine.js rule, null-safe step-1 scan).
    pub fn repetition(&self) -> bool {
        if self.halfmove < 8 {
            return false;
        }
        let mut count = 1;
        let n = self.keys.len();
        let window = (self.halfmove as usize).min(n);
        for i in (n - window)..n {
            if self.keys[i] == self.key64() {
                count += 1;
                if count >= 3 {
                    return true;
                }
            }
        }
        false
    }

    /// Insufficient material (roaring_engine.js rule).
    pub fn insufficient(&self) -> bool {
        let cnt = |pc: u8| self.bb[bb_idx(pc)].count_ones() as i32;
        if cnt(WP) > 0 || cnt(BP) > 0 || cnt(WR) > 0 || cnt(BR) > 0 || cnt(WQ) > 0 || cnt(BQ) > 0 {
            return false;
        }
        let knights = cnt(WN) + cnt(BN);
        let bishops = cnt(WB) + cnt(BB);
        if knights + bishops <= 1 {
            return true;
        }
        if knights > 0 {
            return false;
        }
        let mut colors = 0u32;
        for pc in [WB, BB] {
            let mut b = self.bb[bb_idx(pc)];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                colors |= 1 << (((s >> 3) + (s & 7)) & 1);
                b &= b - 1;
            }
        }
        colors != 3
    }

    pub fn drawn(&self) -> bool {
        self.halfmove >= 100 || self.repetition() || self.insufficient()
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

// ── Move packing: identical layout to fast.js ──
// from | to<<7 | promo<<14 | flag<<18  (flag: 1 capture, 2 ep, 3 castle, 4 double push)
#[inline(always)]
pub fn mk_move(f: usize, t: usize, promo: u8, flag: u8) -> u32 {
    (f as u32) | ((t as u32) << 7) | ((promo as u32) << 14) | ((flag as u32) << 18)
}
#[inline(always)]
pub fn m_from(m: u32) -> u32 {
    m & 127
}
#[inline(always)]
pub fn m_to(m: u32) -> u32 {
    (m >> 7) & 127
}
#[inline(always)]
pub fn m_promo(m: u32) -> u32 {
    (m >> 14) & 15
}
#[inline(always)]
pub fn m_flag(m: u32) -> u32 {
    (m >> 18) & 7
}

/// UCI string for a move (e.g. e2e4, e7e8q). Castling is the king move.
pub fn move_to_uci(m: u32) -> String {
    let f = SQ120_TO_64[m_from(m) as usize] as usize;
    let t = SQ120_TO_64[m_to(m) as usize] as usize;
    let mut s = String::with_capacity(5);
    s.push((b'a' + (f & 7) as u8) as char);
    s.push((b'1' + (f >> 3) as u8) as char);
    s.push((b'a' + (t & 7) as u8) as char);
    s.push((b'1' + (t >> 3) as u8) as char);
    let p = m_promo(m);
    if p != 0 {
        s.push(match p {
            2 => 'n', 3 => 'b', 4 => 'r', 5 => 'q', _ => '?',
        });
    }
    s
}
