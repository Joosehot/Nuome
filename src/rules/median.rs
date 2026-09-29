//! The median: sort, take the middle value, or the mean of the two middle
//! values when the count is even.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, Named};
use crate::expr::{self, Expr, Math};

pub struct Median;

/// "Sorted: 1, 3, 5, 9."
pub fn sorted_line(xs: &[crate::q::Q]) -> Line {
    let text: Vec<String> = xs.iter().map(|q| q.to_string()).collect();
    Line::new().t(format!("Sorted: {}.", text.join(", ")))
}

impl Rule for Median {
    fn name(&self) -> &'static str {
        "median"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["middle", "two_middles"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("median", m, |e, _| {
            let Expr::Call(Named::Median, args) = e else { return vec![] };
            let Some(mut xs) = nums(args) else { return vec![] };
            if xs.is_empty() {
                return vec![];
            }
            xs.sort();
            let n = xs.len();
            let work = vec![sorted_line(&xs)];
            if n % 2 == 1 {
                let says = Line::new().t(format!("Sort the {n} values; the middle one ({} of {n}) is the median.", n / 2 + 1));
                vec![Rewrite { variant: "middle", new: Expr::Num(xs[n / 2]), says, work }]
            } else {
                let new = expr::div(Expr::Add(vec![Expr::Num(xs[n / 2 - 1]), Expr::Num(xs[n / 2])]), expr::num(2));
                let says = Line::new().t(format!("Sort the {n} values; with an even count the median is halfway between the two middle ones."));
                vec![Rewrite { variant: "two_middles", new, says, work }]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn middle_value() {
        assert_eq!(test_moves(&Median, "median of 9, 1, 5"), vec!["5"]);
        assert_eq!(test_moves(&Median, "median of 9, 1, 5, 3"), vec!["(3 + 5)/2"]);
        assert!(test_moves(&Median, "mean of 9, 1, 5").is_empty());
    }
}
