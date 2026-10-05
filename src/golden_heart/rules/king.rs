//! King: strikes on the squares around each king, and the pawns standing in front of it.

use super::{Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "king", start: 784, len: 4, read, label };

fn read(b: &Board, me: bool, sh: &Shared, out: &mut Vec<(u16, f32)>) {
    let (m, t) = if me { (0, 1) } else { (1, 0) };
    let ring = |k: i32| -> Vec<usize> {
        if k < 0 {
            return vec![];
        }
        let (f, r) = (k % 8, k / 8);
        let mut v = Vec::new();
        for df in -1..=1 {
            for dr in -1..=1 {
                let (ff, rr) = (f + df, r + dr);
                if (0..8).contains(&ff) && (0..8).contains(&rr) {
                    v.push((rr * 8 + ff) as usize);
                }
            }
        }
        v
    };
    let shield = |k: i32, white: bool| -> f32 {
        if k < 0 {
            return 0.0;
        }
        let (f, r) = (k % 8, k / 8);
        let dir = if white { 1 } else { -1 };
        let pawn = if white { 1 } else { -1 };
        (-1..=1).filter(|df| {
            let ff = f + df;
            (1..=2).any(|s| {
                let rr = r + dir * s;
                (0..8).contains(&ff) && (0..8).contains(&rr) && b.sq[(rr * 8 + ff) as usize] == pawn
            })
        }).count() as f32
    };
    let (mk, tk) = (b.king(me), b.king(!me));
    let v = [
        ring(tk).iter().map(|&n| sh.edges[m][n] as f32).sum::<f32>(),
        ring(mk).iter().map(|&n| sh.edges[t][n] as f32).sum::<f32>(),
        shield(mk, me),
        shield(tk, !me),
    ];
    for (i, &x) in v.iter().enumerate() {
        if x != 0.0 {
            out.push(((RULE.start + i) as u16, x));
        }
    }
}

fn label(i: usize) -> String {
    ["my strikes around their king", "their strikes around my king", "pawns sheltering my king", "pawns sheltering their king"][i].to_string()
}
