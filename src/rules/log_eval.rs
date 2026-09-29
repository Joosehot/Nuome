//! Logarithms that come out exactly: log_2(8) = 3 because 2^3 = 8, and
//! logs and powers undoing each other: ln(e^u) = u, e^(ln u) = u,
//! log_b(b^u) = u. Variants: an exact value, or an inverse pair.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, exact_log, Expr, Func, Konst, Math};

pub struct LogEval;

impl Rule for LogEval {
    fn name(&self) -> &'static str {
        "log_eval"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["exact", "inverse"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("log_eval", m, |e, _| {
            let inverse = |new: Expr, why: &str| vec![Rewrite { variant: "inverse", new: new.clone(), says: Line::new().t(why).t(": ").e(e).t(" = ").e(&new).t("."), work: vec![] }];
            match e {
                Expr::Log(b, a) => {
                    if let (Some(bq), Some(aq)) = (b.as_num(), a.as_num()) {
                        let Some(k) = exact_log(&bq, &aq) else { return vec![] };
                        let says = Line::new().e(e).t(" = ").e(&Expr::Num(k)).t(", because ").e(&expr::pow((**b).clone(), Expr::Num(k))).t(format!(" = {aq}."));
                        return vec![Rewrite { variant: "exact", new: Expr::Num(k), says, work: vec![] }];
                    }
                    match &**a {
                        Expr::Pow(bb, u) if bb == b => inverse((**u).clone(), "A log undoes the power of its base"),
                        _ if a == b => inverse(expr::num(1), "The log of the base itself is 1"),
                        _ => vec![],
                    }
                }
                Expr::Func(Func::Ln, a) => match &**a {
                    Expr::Const(Konst::E) => inverse(expr::num(1), "ln(e) is 1"),
                    Expr::Num(q) if q.is_one() => vec![Rewrite { variant: "exact", new: expr::num(0), says: Line::new().e(e).t(" = 0, because e^0 = 1."), work: vec![] }],
                    Expr::Func(Func::Exp, u) => inverse((**u).clone(), "ln undoes e^"),
                    _ => vec![],
                },
                Expr::Func(Func::Exp, a) => match &**a {
                    Expr::Func(Func::Ln, u) => inverse((**u).clone(), "e^ undoes ln"),
                    _ => vec![],
                },
                Expr::Pow(b, a) => match &**a {
                    Expr::Log(bb, u) if bb == b => inverse((**u).clone(), "A power undoes the log of its base"),
                    _ => vec![],
                },
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
    fn evaluates_exact_logs() {
        assert_eq!(test_moves(&LogEval, "what is log_2 8"), vec!["3"]);
        assert_eq!(test_moves(&LogEval, "what is log(1/100)"), vec!["-2"]);
        assert_eq!(test_moves(&LogEval, "simplify ln(e^(2x))"), vec!["2x"]);
        assert!(test_moves(&LogEval, "what is log_2 5").is_empty());
    }
}
