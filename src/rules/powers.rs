//! Laws of exponents: x * x^2 -> x^3, (x^2)^3 -> x^6, (2x)^2 -> 2^2 x^2.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::q::Q;

pub struct Powers;

/// base and numeric exponent of a factor: x -> (x, 1), x^3 -> (x, 3).
pub fn base_exp(f: &Expr) -> Option<(Expr, Q)> {
    match f {
        Expr::Pow(b, e) => Some(((**b).clone(), e.as_num()?)),
        Expr::Num(_) => None,
        x => Some((x.clone(), Q::ONE)),
    }
}

pub fn power_of(base: Expr, e: Q) -> Expr {
    if e.is_one() {
        base
    } else {
        expr::pow(base, Expr::Num(e))
    }
}

impl Rule for Powers {
    fn name(&self) -> &'static str {
        "powers"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["same_base", "power_of_power", "power_of_product", "split_exponent", "number_base"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        local("powers", m, |e, _| {
            let mut out = Vec::new();
            match e {
                Expr::Mul(v) => {
                    // the first pair of factors with the same base
                    'find: for i in 0..v.len() {
                        let Some((bi, ei)) = base_exp(&v[i]) else { continue };
                        for j in i + 1..v.len() {
                            let Some((bj, ej)) = base_exp(&v[j]) else { continue };
                            if bi != bj {
                                continue;
                            }
                            let Some(sum) = ei.add(&ej) else { break 'find };
                            let mut nv = v.clone();
                            nv[i] = power_of(bi.clone(), sum);
                            nv.remove(j);
                            let says = Line::new().t("Same base: add the exponents, ").e(&Expr::Mul(vec![v[i].clone(), v[j].clone()])).t(" = ").e(&nv[i]).t(".");
                            out.push(Rewrite { variant: "same_base", new: expr::mul(nv), says, work: vec![] });
                            break 'find;
                        }
                    }
                }
                // 2^(n + 1) = 2 * 2^n: a number base with a whole number added to the exponent
                Expr::Pow(b, x) if b.as_num().is_some() && matches!(**x, Expr::Add(_)) => {
                    let Expr::Add(ts) = &**x else { return out };
                    let Some(k) = ts.iter().find_map(|t| t.as_num().filter(|q| q.is_int() && !q.is_zero())) else { return out };
                    let rest = expr::add(ts.iter().filter(|t| t.as_num() != Some(k)).cloned().collect());
                    let Some(bk) = b.as_num().and_then(|q| q.pow(k.num() as i64)) else { return out };
                    let new = expr::mul(vec![Expr::Num(bk), expr::pow((**b).clone(), rest)]);
                    out.push(Rewrite { variant: "split_exponent", new: new.clone(), says: Line::new().t("Split off the whole part of the exponent: ").e(e).t(" = ").e(&new).t("."), work: vec![] });
                }
                Expr::Pow(b, n) => {
                    let Some(n) = n.as_num() else { return out };
                    match &**b {
                        Expr::Pow(bb, m) => {
                            if let Some(p) = m.as_num().and_then(|m| m.mul(&n)) {
                                let new = power_of((**bb).clone(), p);
                                out.push(Rewrite { variant: "power_of_power", new: new.clone(), says: Line::new().t("Power of a power: multiply the exponents, ").e(e).t(" = ").e(&new).t("."), work: vec![] });
                            }
                        }
                        Expr::Mul(f) if n.is_int() => {
                            let new = Expr::Mul(f.iter().map(|x| match base_exp(x) {
                                Some((bx, ex)) => ex.mul(&n).map_or_else(|| expr::pow(x.clone(), Expr::Num(n)), |p| power_of(bx, p)),
                                None => expr::pow(x.clone(), Expr::Num(n)),
                            }).collect());
                            out.push(Rewrite { variant: "power_of_product", new: new.clone(), says: Line::new().t("Raise each factor to the power: ").e(e).t(" = ").e(&new).t("."), work: vec![] });
                        }
                        _ => {}
                    }
                }
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
    fn combines_powers() {
        assert_eq!(test_moves(&Powers, "simplify x * x^2"), vec!["x^3"]);
        assert_eq!(test_moves(&Powers, "simplify (x^2)^3"), vec!["x^6"]);
        assert_eq!(test_moves(&Powers, "simplify (2x)^2"), vec!["2^2 * x^2"]);
        assert!(test_moves(&Powers, "simplify x * y").is_empty());
    }
}
