//! Safety: pieces struck and not guarded (hanging), mine and theirs, counted and weighed;
//! and pieces struck by something cheaper.

use super::{mine, worth, Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "safety", start: 778, len: 6, read, label };

fn read(b: &Board, me: bool, sh: &Shared, out: &mut Vec<(u16, f32)>) {
    let (m, t) = if me { (0, 1) } else { (1, 0) };
    let mut v = [0f32; 6];
    for n in 0..64 {
        let p = b.sq[n];
        if p == 0 || p.abs() == 6 {
            continue;
        }
        let (own, opp) = if mine(p, me) { (m, t) } else { (t, m) };
        let side = if mine(p, me) { 0 } else { 1 };
        if sh.edges[opp][n] > 0 && sh.edges[own][n] == 0 {
            v[side * 2] += 1.0;
            v[side * 2 + 1] += worth(p);
        }
        // struck by a cheaper piece: a pawn's strike is enough to tell for minor and major pieces
        if p.abs() > 1 && sh.edges[opp][n] > 0 {
            let pawn_strikes = {
                let (f, r) = ((n % 8) as i32, (n / 8) as i32);
                let dir = if p > 0 { 1 } else { -1 }; // an enemy pawn strikes from the side the piece faces
                [-1, 1].iter().any(|df| {
                    let (ff, rr) = (f + df, r + dir);
                    (0..8).contains(&ff) && (0..8).contains(&rr) && b.sq[(rr * 8 + ff) as usize] == if p > 0 { -1 } else { 1 }
                })
            };
            if pawn_strikes {
                v[4 + side] += 1.0;
            }
        }
    }
    for (i, &x) in v.iter().enumerate() {
        if x != 0.0 {
            out.push(((RULE.start + i) as u16, x));
        }
    }
}

fn label(i: usize) -> String {
    ["my hanging pieces", "my hanging worth", "their hanging pieces", "their hanging worth", "my pieces a pawn strikes", "their pieces a pawn strikes"][i].to_string()
}
