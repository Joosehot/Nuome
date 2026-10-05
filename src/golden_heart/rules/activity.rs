//! Activity: how many squares my pieces strike, and how many theirs do; the centre's share.

use super::{Rule, Shared};
use crate::golden::Board;

pub const RULE: Rule = Rule { name: "activity", start: 774, len: 4, read, label };

fn read(_: &Board, me: bool, sh: &Shared, out: &mut Vec<(u16, f32)>) {
    let (m, t) = if me { (0, 1) } else { (1, 0) };
    let count = |s: usize| sh.edges[s].iter().filter(|&&e| e > 0).count() as f32;
    let centre = |s: usize| [27, 28, 35, 36].iter().filter(|&&n| sh.edges[s][n] > 0).count() as f32;
    let v = [count(m) / 10.0, count(t) / 10.0, centre(m), centre(t)];
    for (i, &x) in v.iter().enumerate() {
        if x != 0.0 {
            out.push(((RULE.start + i) as u16, x));
        }
    }
}

fn label(i: usize) -> String {
    ["squares I strike (tens)", "squares they strike (tens)", "centre squares I strike", "centre squares they strike"][i].to_string()
}
