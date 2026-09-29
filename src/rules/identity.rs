//! The bookkeeping identities: x + 0, 1x, 0x, x^1, x^0, x/1, 0/x; a number
//! factor goes first (x * 3 -> 3x); a minus sign inside a product comes out.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};

pub struct Identity;

/// The usual order of factors: numbers, letters, constants, functions,
/// brackets, anything else. Stable, so equal kinds keep their order.
fn rank(e: &Expr) -> u8 {
    match e {
        Expr::Num(_) => 0,
        Expr::Var(_) => 1,
        Expr::Pow(b, _) if matches!(**b, Expr::Var(_)) => 1,
        Expr::Const(_) => 2,
        Expr::Func(..) => 3,
        Expr::Pow(b, _) if matches!(**b, Expr::Func(..)) => 3,
        Expr::Add(_) => 4,
        Expr::Pow(b, _) if matches!(**b, Expr::Add(_)) => 4,
        _ => 5,
    }
}

fn ordered(v: &[Expr]) -> Vec<Expr> {
    let mut o = v.to_vec();
    o.sort_by_key(rank);
    o
}

fn rw(new: Expr, says: Line) -> Rewrite {
    Rewrite { variant: "tidy", new, says, work: vec![] }
}

impl Rule for Identity {
    fn name(&self) -> &'static str {
        "identity"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["tidy"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("identity", m, |e, _| {
            let mut out = Vec::new();
            match e {
                Expr::Add(v) if v.iter().any(|t| t.is_num(0)) => {
                    out.push(rw(expr::add(v.iter().filter(|t| !t.is_num(0)).cloned().collect()), Line::new().t("Adding 0 changes nothing.")));
                }
                // 2 + sqrt(3), not sqrt(3) + 2: a number leads a sum with no letters
                Expr::Add(v) if e.vars().is_empty() && v.len() >= 2 && v.last().is_some_and(|t| t.as_num().is_some_and(|q| !q.is_neg())) && v[..v.len() - 1].iter().all(|t| t.as_num().is_none()) => {
                    let mut o = vec![v[v.len() - 1].clone()];
                    o.extend(v[..v.len() - 1].iter().cloned());
                    out.push(rw(Expr::Add(o), Line::new().t("Write the number first.")));
                }
                Expr::Mul(v) if v.iter().any(|t| t.is_num(0)) => {
                    out.push(rw(expr::num(0), Line::new().t("Anything times 0 is 0.")));
                }
                Expr::Mul(v) if v.iter().any(|t| t.is_num(1)) => {
                    out.push(rw(expr::mul(v.iter().filter(|t| !t.is_num(1)).cloned().collect()), Line::new().t("Multiplying by 1 changes nothing.")));
                }
                // 2 * (1/d) = 2/d, unless d is there to cancel (calculus and trig, agent B)
                Expr::Mul(v) if v.iter().any(|t| matches!(t, Expr::Div(n, d) if n.is_num(1) && !v.contains(d))) => {
                    let k = v.iter().position(|t| matches!(t, Expr::Div(n, d) if n.is_num(1) && !v.contains(d))).expect("found above");
                    let Expr::Div(_, d) = &v[k] else { unreachable!("a quotient") };
                    let rest: Vec<Expr> = v.iter().enumerate().filter(|(i, _)| *i != k).map(|(_, t)| t.clone()).collect();
                    out.push(rw(expr::div(expr::mul(rest), (**d).clone()), Line::new().t("Multiplying by 1/").e(d).t(" is dividing by ").e(d).t(".")));
                }
                Expr::Mul(v) if !v.iter().any(|t| matches!(t, Expr::Neg(_))) && ordered(v) != *v => {
                    let o = ordered(v);
                    let says = if v.iter().skip(1).any(|t| t.as_num().is_some()) { "Write the number first." } else { "Write the factors in the usual order." };
                    out.push(rw(Expr::Mul(o), Line::new().t(says)));
                }
                Expr::Mul(v) if v.iter().any(|t| matches!(t, Expr::Neg(_))) => {
                    let inner: Vec<Expr> = v.iter().map(|t| if let Expr::Neg(a) = t { (**a).clone() } else { t.clone() }).collect();
                    let flips = v.iter().filter(|t| matches!(t, Expr::Neg(_))).count();
                    let prod = expr::mul(inner);
                    out.push(rw(if flips % 2 == 1 { expr::neg(prod) } else { prod }, Line::new().t("Bring the minus sign to the front.")));
                }
                Expr::Pow(a, b) if b.is_num(1) => out.push(rw((**a).clone(), Line::new().t("A power of 1 changes nothing."))),
                Expr::Pow(a, b) if b.is_num(0) && !a.is_num(0) => out.push(rw(expr::num(1), Line::new().t("Anything to the power 0 is 1."))),
                Expr::Div(a, b) if b.is_num(1) => out.push(rw((**a).clone(), Line::new().t("Dividing by 1 changes nothing."))),
                Expr::Div(a, b) if a.is_num(0) && !b.is_num(0) => out.push(rw(expr::num(0), Line::new().t("0 divided by anything is 0."))),
                _ => {}
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
    fn removes_neutral_elements() {
        assert_eq!(test_moves(&Identity, "simplify x + 0"), vec!["x"]);
        assert_eq!(test_moves(&Identity, "simplify x * 3"), vec!["3x"]);
        assert_eq!(test_moves(&Identity, "simplify (x + 1)^4 * x"), vec!["x(x + 1)^4"]);
        assert_eq!(test_moves(&Identity, "what is sqrt(3) + 2"), vec!["2 + sqrt(3)"]);
        assert!(test_moves(&Identity, "simplify 2x + 1").is_empty());
    }
}
