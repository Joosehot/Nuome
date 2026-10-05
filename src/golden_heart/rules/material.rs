//! Material: how many of each piece I have more than they do, and the bishop pair.

use super::{mine, Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "material", start: 0, len: 6, read, label };

fn read(b: &Board, me: bool, _: &Shared, out: &mut Vec<(u16, f32)>) {
    let mut d = [0f32; 6];
    let mut bishops = [0; 2];
    for &p in &b.sq {
        if p == 0 || p.abs() == 6 {
            continue;
        }
        let s = if mine(p, me) { 1.0 } else { -1.0 };
        d[p.unsigned_abs() as usize - 1] += s;
        if p.abs() == 3 {
            bishops[mine(p, me) as usize] += 1;
        }
    }
    d[5] = (bishops[1] >= 2) as i32 as f32 - (bishops[0] >= 2) as i32 as f32;
    for (i, &v) in d.iter().enumerate() {
        if v != 0.0 {
            out.push(((RULE.start + i) as u16, v));
        }
    }
}

fn label(i: usize) -> String {
    ["pawns", "knights", "bishops", "rooks", "queens", "bishop pair"][i].to_string()
}
