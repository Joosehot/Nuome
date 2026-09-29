//! The laws of logarithms. Combining (simplify, solve): log A + log B =
//! log(AB), log A - log B = log(A/B), k log A = log(A^k). Splitting
//! (expand): the same laws read right to left. Change of base,
//! log_b(a) = ln(a)/ln(b), when a decimal answer is asked for.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Konst, Math};
use crate::model::Task;
use crate::q::Q;

pub struct LogLaws;

/// log to a base, written as ln for base e.
pub fn log(base: Expr, arg: Expr) -> Expr {
    if base == Expr::Const(Konst::E) {
        expr::func(Func::Ln, arg)
    } else {
        Expr::Log(Box::new(base), Box::new(arg))
    }
}

/// (base, argument) of a logarithm; ln has base e.
pub fn as_log(e: &Expr) -> Option<(Expr, Expr)> {
    match e {
        Expr::Log(b, a) => Some(((**b).clone(), (**a).clone())),
        Expr::Func(Func::Ln, a) => Some((Expr::Const(Konst::E), (**a).clone())),
        _ => None,
    }
}

/// A term as (coefficient, base, argument): -2log(x) -> (-2, 10, x).
fn log_term(t: &Expr) -> Option<(Q, Expr, Expr)> {
    let (c, rest) = expr::coeff(t);
    let (b, a) = as_log(&rest)?;
    Some((c, b, a))
}

impl Rule for LogLaws {
    fn name(&self) -> &'static str {
        "log_laws"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["product", "quotient", "power", "split_product", "split_quotient", "power_down", "change_of_base"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let expand = cx.task() == Task::Expand;
        // every base used anywhere: a power law only helps when another log of its base is there
        let logs: Vec<Expr> = m.slots().iter().flat_map(|e| e.walk().into_iter().filter_map(|(_, n)| as_log(n).map(|(b, _)| b)).collect::<Vec<_>>()).collect();
        let decimals = cx.req.decimals.is_some();
        local("log_laws", m, |e, _| {
            let mut out = Vec::new();
            if expand {
                let Some((b, a)) = as_log(e) else { return out };
                let rw = |variant, new: Expr, law: &str| Rewrite { variant, new: new.clone(), says: Line::new().t(law).t(": ").e(e).t(" = ").e(&new).t("."), work: vec![] };
                match &a {
                    Expr::Mul(fs) => out.push(rw("split_product", expr::add(fs.iter().map(|f| log(b.clone(), f.clone())).collect()), "The log of a product is the sum of the logs")),
                    Expr::Div(n, d) => out.push(rw("split_quotient", expr::add(vec![log(b.clone(), (**n).clone()), expr::neg(log(b.clone(), (**d).clone()))]), "The log of a quotient is the difference of the logs")),
                    Expr::Pow(x, k) if k.as_num().is_some() => out.push(rw("power_down", expr::mul(vec![(**k).clone(), log(b.clone(), (**x).clone())]), "A power inside a log comes out in front")),
                    Expr::Func(Func::Sqrt, x) => out.push(rw("power_down", Expr::Mul(vec![Expr::Num(Q::new(1, 2).expect("nonzero")), log(b.clone(), (**x).clone())]), "A square root is the power 1/2, which comes out in front")),
                    _ => {}
                }
                return out;
            }
            // change of base, for a decimal answer
            if let Expr::Log(b, a) = e {
                if decimals && b.as_num().is_some() && a.as_num().is_some() && e.eval_q(&|_| None).is_none() {
                    let new = expr::div(expr::func(Func::Ln, (**a).clone()), expr::func(Func::Ln, (**b).clone()));
                    out.push(Rewrite { variant: "change_of_base", new: new.clone(), says: Line::new().t("Change to base e: ").e(e).t(" = ").e(&new).t("."), work: vec![] });
                }
            }
            // k log A = log(A^k), when another log of the same base is there to combine with
            if let Some((k, b, a)) = log_term(e) {
                if k.is_int() && k.num() > 1 && logs.iter().filter(|x| **x == b).count() >= 2 {
                    let new = log(b, expr::pow(a, Expr::Num(k)));
                    out.push(Rewrite { variant: "power", new: new.clone(), says: Line::new().t("A number in front of a log becomes a power inside: ").e(e).t(" = ").e(&new).t("."), work: vec![] });
                }
            }
            let Expr::Add(ts) = e else { return out };
            // the first pair of plain logs with the same base
            for i in 0..ts.len() {
                let Some((ci, bi, ai)) = log_term(&ts[i]) else { continue };
                if !(ci.is_one() || ci == Q::int(-1)) {
                    continue;
                }
                for j in i + 1..ts.len() {
                    let Some((cj, bj, aj)) = log_term(&ts[j]) else { continue };
                    if bj != bi || !(cj.is_one() || cj == Q::int(-1)) || (ci.is_neg() && cj.is_neg()) {
                        continue;
                    }
                    let (variant, arg, law) = match (ci.is_neg(), cj.is_neg()) {
                        (false, false) => ("product", expr::mul(vec![ai.clone(), aj.clone()]), "The sum of logs is the log of the product"),
                        (false, true) => ("quotient", expr::div(ai.clone(), aj.clone()), "The difference of logs is the log of the quotient"),
                        _ => ("quotient", expr::div(aj.clone(), ai.clone()), "The difference of logs is the log of the quotient"),
                    };
                    let combined = log(bi.clone(), arg);
                    let mut nts = ts.clone();
                    nts[i] = combined.clone();
                    nts.remove(j);
                    let pair = expr::add(vec![ts[i].clone(), ts[j].clone()]);
                    let says = Line::new().t(law).t(": ").e(&pair).t(" = ").e(&combined).t(".");
                    out.push(Rewrite { variant, new: expr::add(nts), says, work: vec![] });
                    return out;
                }
            }
            out
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn combines_and_splits_logs() {
        assert_eq!(test_moves(&LogLaws, "simplify log(2) + log(5)"), vec!["log(2 * 5)"]);
        assert_eq!(test_moves(&LogLaws, "solve log_2(x + 6) - log_2(x) = 2"), vec!["log_2((x + 6)/x) = 2"]);
        assert_eq!(test_moves(&LogLaws, "expand log(x^2)"), vec!["2log(x)"]);
        assert!(test_moves(&LogLaws, "simplify log(2) + ln(5)").is_empty());
    }
}
