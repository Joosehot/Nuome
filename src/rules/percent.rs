//! Percentages in words: what percent one number is of another, the
//! percentage change from one to another, raising or lowering by a percent.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::Named;
use crate::expr::{self, Expr, Math};

pub struct Percent;

impl Rule for Percent {
    fn name(&self) -> &'static str {
        "percent"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["part_of_whole", "change", "multiplier"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("percent", m, |e, _| {
            let Expr::Call(f, args) = e else { return vec![] };
            let hundred = || expr::num(100);
            match (f, &args[..]) {
                (Named::WhatPercent, [part, whole]) => {
                    let new = Expr::Mul(vec![expr::div(part.clone(), whole.clone()), hundred()]);
                    vec![Rewrite { variant: "part_of_whole", new, says: Line::new().t("Divide the part by the whole, then multiply by 100."), work: vec![] }]
                }
                (Named::PercentChange, [a, b]) => {
                    let new = Expr::Mul(vec![expr::div(Expr::Add(vec![b.clone(), expr::neg(a.clone())]), a.clone()), hundred()]);
                    vec![Rewrite { variant: "change", new, says: Line::new().t("The change divided by the starting value, times 100."), work: vec![] }]
                }
                (Named::Raise, [a, p]) => {
                    let Some(pq) = p.as_num() else { return vec![] };
                    let factor = Expr::Add(vec![expr::num(1), expr::tidy(expr::div(Expr::Num(pq), hundred()))]);
                    let (up, abs) = (!pq.is_neg(), pq.abs());
                    let says = Line::new().t(format!("{} {abs}% means multiplying by 1 {} {abs}/100.", if up { "Adding" } else { "Taking off" }, if up { "+" } else { "-" }));
                    vec![Rewrite { variant: "multiplier", new: Expr::Mul(vec![a.clone(), factor]), says, work: vec![] }]
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn percentages() {
        assert_eq!(test_moves(&Percent, "what percent of 80 is 12"), vec!["(12/80) * 100"]);
        assert_eq!(test_moves(&Percent, "12 is what percent of 80"), vec!["(12/80) * 100"]);
        assert_eq!(test_moves(&Percent, "percentage change from 80 to 100"), vec!["((100 - 80)/80) * 100"]);
        assert_eq!(test_moves(&Percent, "increase 80 by 15%"), vec!["80(1 + 15/100)"]);
        assert!(test_moves(&Percent, "what is 15% of 80").is_empty());
    }
}
