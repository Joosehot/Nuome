//! A fractional power is a root: 8^(2/3) is the cube root of 8, squared,
//! = 2^2. Taken root first, so the numbers stay small.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct FracExponent;

pub fn root_name(k: i128) -> String {
    match k {
        2 => "square root".into(),
        3 => "cube root".into(),
        k => format!("{k}th root"),
    }
}

impl Rule for FracExponent {
    fn name(&self) -> &'static str {
        "frac_exponent"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["root_first"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("frac_exponent", m, |e, _| {
            let Expr::Pow(b, n) = e else { return vec![] };
            let (Some(base), Some(p)) = (b.as_num(), n.as_num()) else { return vec![] };
            if p.is_int() || p.is_neg() || base.is_neg() || p.den() > 12 {
                return vec![];
            }
            let k = p.den();
            let Some(r) = base.root(k as i64) else { return vec![] };
            let name = root_name(k);
            let new = if p.num() == 1 { Expr::Num(r) } else { expr::pow(Expr::Num(r), expr::num(p.num())) };
            // x^(1/2) prints as sqrt(x): just take the root
            if k == 2 && p.num() == 1 {
                let says = Line::new().t("Take the square root: ").e(e).t(" = ").e(&new).t(format!(", because {r} squared is {base}."));
                return vec![Rewrite { variant: "root_first", new, says, work: vec![] }];
            }
            let says = if p.num() == 1 {
                Line::new().e(e).t(format!(" is the {name} of ")).e(b).t(".")
            } else {
                Line::new().e(e).t(format!(" is the {name} of ")).e(b).t(format!(", to the power {}.", p.num()))
            };
            let work = vec![Line::new().t(format!("The {name} of {base} is {r}, because ")).e(&expr::pow(Expr::Num(r), expr::num(k))).t(format!(" = {base}."))];
            vec![Rewrite { variant: "root_first", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn takes_the_root_first() {
        assert_eq!(test_moves(&FracExponent, "what is 8^(2/3)"), vec!["2^2"]);
        assert_eq!(test_moves(&FracExponent, "what is 16^(1/4)"), vec!["2"]);
        assert!(test_moves(&FracExponent, "what is 2^(1/2)").is_empty());
    }
}
