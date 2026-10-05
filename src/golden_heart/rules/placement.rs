//! Placement: which piece stands on which square (mine and theirs), seen from me.

use super::{mine, seen, Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "placement", start: 6, len: 768, read, label };

fn read(b: &Board, me: bool, _: &Shared, out: &mut Vec<(u16, f32)>) {
    for n in 0..64 {
        let p = b.sq[n];
        if p == 0 {
            continue;
        }
        let theirs = !mine(p, me) as usize;
        out.push(((RULE.start + (theirs * 6 + p.unsigned_abs() as usize - 1) * 64 + seen(n, me)) as u16, 1.0));
    }
}

fn label(i: usize) -> String {
    let (whose, rest) = (i / 384, i % 384);
    let (t, sq) = (rest / 64, rest % 64);
    format!("{} {} on {}{}", ["my", "their"][whose], ["pawn", "knight", "bishop", "rook", "queen", "king"][t], (b'a' + (sq % 8) as u8) as char, sq / 8 + 1)
}
