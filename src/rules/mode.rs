//! The mode: the value that appears most often. When several tie, or every
//! value appears once, there is no single mode and nothing is claimed.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, Named};
use crate::expr::{Expr, Math};
use crate::q::Q;

pub struct Mode;

impl Rule for Mode {
    fn name(&self) -> &'static str {
        "mode"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["count"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("mode", m, |e, _| {
            let Expr::Call(Named::Mode, args) = e else { return vec![] };
            let Some(mut xs) = nums(args) else { return vec![] };
            xs.sort();
            let mut counts: Vec<(Q, usize)> = Vec::new();
            for x in xs {
                match counts.iter_mut().find(|(v, _)| *v == x) {
                    Some((_, c)) => *c += 1,
                    None => counts.push((x, 1)),
                }
            }
            let Some(top) = counts.iter().map(|(_, c)| *c).max() else { return vec![] };
            let winners: Vec<Q> = counts.iter().filter(|(_, c)| *c == top).map(|(v, _)| *v).collect();
            if winners.len() != 1 || top < 2 {
                return vec![];
            }
            let tally: Vec<String> = counts.iter().map(|(v, c)| format!("{v}: {c}")).collect();
            let says = Line::new().t(format!("{} appears most often ({top} times).", winners[0]));
            vec![Rewrite { variant: "count", new: Expr::Num(winners[0]), says, work: vec![Line::new().t(format!("Counts: {}.", tally.join(", ")))] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn most_common_value() {
        assert_eq!(test_moves(&Mode, "mode of 2, 3, 3, 5"), vec!["3"]);
        assert!(test_moves(&Mode, "mode of 2, 3, 5").is_empty());
    }
}
