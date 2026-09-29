//! Compound growth: every period multiplies the amount by (1 + rate), so
//! after n periods it is P(1 + r)^n.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{nums, Named};
use crate::expr::{self, Expr, Math};

pub struct Compound;

impl Rule for Compound {
    fn name(&self) -> &'static str {
        "compound"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["growth_factor"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("compound", m, |e, _| {
            let Expr::Call(Named::Compound, args) = e else { return vec![] };
            let Some(v) = nums(args) else { return vec![] };
            let [p, r, n] = v[..] else { return vec![] };
            let growth = expr::pow(Expr::Add(vec![expr::num(1), Expr::Num(r)]), Expr::Num(n));
            let new = if p.is_one() { growth } else { Expr::Mul(vec![Expr::Num(p), growth]) };
            let pct = r.mul(&crate::q::Q::int(100)).map_or(r.to_string(), |x| format!("{}%", crate::q::decimal(x.to_f64(), 4).trim_end_matches('0').trim_end_matches('.')));
            let says = Line::new().t(format!("Each period multiplies the amount by 1 + {pct}; {n} periods multiply it {n} times."));
            vec![Rewrite { variant: "growth_factor", new, says, work: vec![Line::new().t("amount = start * (1 + rate)^periods")] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn growth() {
        assert_eq!(test_moves(&Compound, "1000 invested at 5% per year for 10 years"), vec!["1000(1 + 1/20)^10"]);
        assert!(test_moves(&Compound, "what is 1000 * 1.05").is_empty());
    }
}
