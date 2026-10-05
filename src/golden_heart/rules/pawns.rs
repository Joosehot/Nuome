//! Pawns: passed pawns by how far they have come, mine and theirs; doubled and isolated pawns.

use super::{Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "pawns", start: 788, len: 16, read, label };

fn read(b: &Board, me: bool, _: &Shared, out: &mut Vec<(u16, f32)>) {
    let mut v = [0f32; 16];
    for (side, white) in [(0usize, me), (1usize, !me)] {
        let pawn = if white { 1 } else { -1 };
        let mut files = [0u8; 8];
        for n in 0..64 {
            if b.sq[n] == pawn {
                files[n % 8] += 1;
            }
        }
        for n in 0..64 {
            if b.sq[n] != pawn {
                continue;
            }
            let (f, r) = ((n % 8) as i32, (n / 8) as i32);
            let dir = if white { 1 } else { -1 };
            let passed = !(0..64).any(|m| b.sq[m] == -pawn && ((m % 8) as i32 - f).abs() <= 1 && ((m / 8) as i32 - r) * dir > 0);
            if passed {
                let advanced = if white { r } else { 7 - r }; // 1..6
                if (1..=6).contains(&advanced) {
                    v[side * 6 + advanced as usize - 1] += 1.0;
                }
            }
            let isolated = (f == 0 || files[(f - 1) as usize] == 0) && (f == 7 || files[(f + 1) as usize] == 0);
            if isolated {
                v[12 + side] += 1.0;
            }
        }
        v[14 + side] = files.iter().map(|&c| c.saturating_sub(1) as f32).sum();
    }
    for (i, &x) in v.iter().enumerate() {
        if x != 0.0 {
            out.push(((RULE.start + i) as u16, x));
        }
    }
}

fn label(i: usize) -> String {
    match i {
        0..=5 => format!("my passed pawns on rank {} (seen from me)", i + 2),
        6..=11 => format!("their passed pawns on rank {} (seen from them)", i - 4),
        12 => "my isolated pawns".into(),
        13 => "their isolated pawns".into(),
        14 => "my doubled pawns".into(),
        _ => "their doubled pawns".into(),
    }
}
