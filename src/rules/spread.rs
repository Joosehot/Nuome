//! Variance and standard deviation. The variance is the mean of the
//! squared distances from the mean (divide by n - 1 instead of n for a
//! sample); the standard deviation is its square root.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::calls::{self, nums, Named};
use crate::expr::{self, Expr, Math};

pub struct Spread;

impl Rule for Spread {
    fn name(&self) -> &'static str {
        "spread"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["squared_deviations", "root_of_variance"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("spread", m, |e, _| {
            let Expr::Call(f, args) = e else { return vec![] };
            let Some(xs) = nums(args) else { return vec![] };
            match f {
                Named::StdDev | Named::SampleStdDev => {
                    let v = if *f == Named::StdDev { Named::Variance } else { Named::SampleVariance };
                    let new = expr::sqrt(Expr::Call(v, args.clone()));
                    vec![Rewrite { variant: "root_of_variance", new, says: Line::new().t("The standard deviation is the square root of the variance."), work: vec![] }]
                }
                Named::Variance | Named::SampleVariance => {
                    let sample = *f == Named::SampleVariance;
                    let n = xs.len() as i128;
                    if n < 2 {
                        return vec![];
                    }
                    let Some(mean) = calls::eval_q(Named::Mean, &xs) else { return vec![] };
                    let squares: Vec<Expr> = xs.iter().map(|x| expr::pow(Expr::Add(vec![Expr::Num(*x), Expr::Num(mean.neg())]), expr::num(2))).collect();
                    let d = if sample { n - 1 } else { n };
                    let new = expr::div(expr::tidy(Expr::Add(squares)), expr::num(d));
                    let says = if sample {
                        Line::new().t(format!("Square each distance from the mean {mean}, add them, and divide by n - 1 = {d} (a sample)."))
                    } else {
                        Line::new().t(format!("Square each distance from the mean {mean}, add them, and divide by n = {d}."))
                    };
                    let work = vec![Line::new().t("mean = ").e(&expr::div(Expr::Add(args.clone()), expr::num(n))).t(format!(" = {mean}"))];
                    vec![Rewrite { variant: "squared_deviations", new, says, work }]
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
    fn variance_and_sd() {
        assert_eq!(test_moves(&Spread, "variance of 1, 3, 5"), vec!["((1 - 3)^2 + (3 - 3)^2 + (5 - 3)^2)/3"]);
        assert_eq!(test_moves(&Spread, "standard deviation of 1, 3, 5"), vec!["sqrt(variance(1, 3, 5))"]);
        assert!(test_moves(&Spread, "mean of 1, 3, 5").is_empty());
    }
}
