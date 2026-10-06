//! StarNet's evaluation, StarNNUE (from Steldens, steldens-rs/src/nnue2.rs):
//! a king-bucketed 768 -> H x2 -> 1 net (STNN0002). Only the evaluation from
//! scratch is copied; the net itself (about 100 MB) stays in ChessEngine and
//! is read from there (STARNNUE overrides the path).
//!
//! Perspective c: own king square k (ranks flipped for black), files
//! mirrored when k is on files e-h; bucket = rank*4 + file of the mirrored
//! king (32 buckets); feature = bucket*768 + (colour_rel*6 + type)*64 +
//! square_rel; out = w . [SCReLU(acc_stm), SCReLU(acc_nstm)] + b,
//! SCReLU = clamp(0,1)^2; centipawns for the side to move = out * scale.

use super::bitboard::SQ120_TO_64;
use super::position::{color_of, type_of, Position, OFF};
use std::sync::OnceLock;

pub const DEFAULT_PATH: &str = "C:/Users/joose/Desktop/ChessEngine/nnue/nnue1024_sib.bin";
const NF: usize = 32 * 768;
const MAX_H: usize = 2048;

pub struct Net {
    pub h: usize,
    pub scale: f32,
    pub ft_w: Vec<f32>,
    pub ft_b: Vec<f32>,
    pub out_w: Vec<f32>,
    pub out_b: f32,
}

static NET: OnceLock<Result<Net, String>> = OnceLock::new();

/// The net, read once (None with the reason when it cannot be read).
pub fn net() -> Result<&'static Net, String> {
    NET.get_or_init(|| {
        let path = std::env::var("STARNNUE").unwrap_or_else(|_| DEFAULT_PATH.to_string());
        let b = std::fs::read(&path).map_err(|e| format!("StarNNUE {path}: {e}"))?;
        parse(&b)
    })
    .as_ref()
    .map_err(|e| e.clone())
}

pub fn parse(b: &[u8]) -> Result<Net, String> {
    if b.len() < 20 || (&b[..8] != b"STNN0002" && &b[..8] != b"STNN0003") {
        return Err("bad magic (expected STNN0002 or STNN0003)".into());
    }
    let hdr = if &b[..8] == b"STNN0003" { 24 } else { 20 };
    let u32_at = |o: usize| u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize;
    let h = u32_at(8);
    let nf = u32_at(12);
    let scale = f32::from_le_bytes(b[16..20].try_into().unwrap());
    if h == 0 || h > MAX_H {
        return Err(format!("hidden {h} out of range 1..{MAX_H}"));
    }
    if nf != NF {
        return Err(format!("feature count {nf} != {NF}"));
    }
    let need = hdr + 4 * (NF * h + h + 2 * h + 1);
    if b.len() < need {
        return Err(format!("file too short: {} < {need}", b.len()));
    }
    let floats = |o: usize, n: usize| -> Vec<f32> { b[o..o + 4 * n].chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect() };
    let mut o = hdr;
    let ft_w = floats(o, NF * h);
    o += 4 * NF * h;
    let ft_b = floats(o, h);
    o += 4 * h;
    let out_w = floats(o, 2 * h);
    o += 4 * 2 * h;
    let out_b = floats(o, 1)[0];
    Ok(Net { h, scale, ft_w, ft_b, out_w, out_b })
}

impl Net {
    fn acc(&self, p: &Position, c: usize, ksq: [usize; 2], acc: &mut [f32]) {
        acc.copy_from_slice(&self.ft_b);
        let flip = if c == 1 { 56 } else { 0 };
        let k_rel = ksq[c] ^ flip;
        let mirror = if (k_rel & 7) >= 4 { 7 } else { 0 };
        let kr = k_rel ^ mirror;
        let bucket = (kr >> 3) * 4 + (kr & 7);
        for m in 21..99 {
            let pc = p.board[m];
            if pc == 0 || pc == OFF {
                continue;
            }
            let s = SQ120_TO_64[m] as usize;
            let col_rel = (color_of(pc) as usize) ^ c;
            let ty = type_of(pc) as usize - 1;
            let f = bucket * 768 + (col_rel * 6 + ty) * 64 + (s ^ flip ^ mirror);
            let row = &self.ft_w[f * self.h..(f + 1) * self.h];
            for (a, w) in acc.iter_mut().zip(row) {
                *a += *w;
            }
        }
    }

    /// One perspective's accumulator from scratch (own king squares `ksq`, 64-based).
    pub fn refresh(&self, p: &Position, c: usize, ksq: [usize; 2], out: &mut [f32]) {
        self.acc(p, c, ksq, out);
    }

    /// Add (+1) or remove (-1) piece `pc` on square `s64` in perspective `c`'s accumulator.
    pub fn update(&self, acc: &mut [f32], c: usize, ksq: [usize; 2], pc: u8, s64: usize, sign: f32) {
        let flip = if c == 1 { 56 } else { 0 };
        let k_rel = ksq[c] ^ flip;
        let mirror = if (k_rel & 7) >= 4 { 7 } else { 0 };
        let kr = k_rel ^ mirror;
        let bucket = (kr >> 3) * 4 + (kr & 7);
        let col_rel = (color_of(pc) as usize) ^ c;
        let ty = type_of(pc) as usize - 1;
        let f = bucket * 768 + (col_rel * 6 + ty) * 64 + (s64 ^ flip ^ mirror);
        let row = &self.ft_w[f * self.h..(f + 1) * self.h];
        for (a, w) in acc.iter_mut().zip(row) {
            *a += sign * *w;
        }
    }

    /// The output from the two accumulators, the side to move's first: centipawns.
    pub fn forward(&self, us: &[f32], them: &[f32]) -> f32 {
        let mut out = self.out_b;
        for (x, w) in us.iter().chain(them.iter()).zip(&self.out_w) {
            let c = x.clamp(0.0, 1.0);
            out += c * c * w;
        }
        out * self.scale
    }

    /// Side-to-move evaluation in centipawns.
    pub fn eval(&self, p: &Position) -> i32 {
        let mut ksq = [0usize; 2];
        for m in 21..99 {
            let pc = p.board[m];
            if pc != 0 && pc != OFF && type_of(pc) == 6 {
                ksq[color_of(pc) as usize] = SQ120_TO_64[m] as usize;
            }
        }
        let stm = p.turn as usize;
        let mut store = [0f32; 2 * MAX_H];
        let buf = &mut store[..2 * self.h];
        let (a, b) = buf.split_at_mut(self.h);
        self.acc(p, stm, ksq, a);
        self.acc(p, 1 - stm, ksq, b);
        let mut out = self.out_b;
        for (x, w) in buf.iter().zip(&self.out_w) {
            let c = x.clamp(0.0, 1.0);
            out += c * c * w;
        }
        (out * self.scale).round() as i32
    }
}
