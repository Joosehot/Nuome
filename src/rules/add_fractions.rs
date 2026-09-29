//! Add or subtract fractions with letters in the denominators: rewrite each
//! over the least common denominator, then add the numerators:
//! 1/x + 1/(x + 1) = (x + 1)/(x(x + 1)) + x/(x(x + 1)) = (x + 1 + x)/(x(x + 1)).
//! Denominators are factored first, so x^2 - 1 and x + 1 share x + 1.

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, with_coeff, Expr, Math};
use crate::poly::{self, Poly};
use crate::q::{lcm, Q};

pub struct AddFractions;

/// A term as (numerator, denominator): -3/(x + 1) -> (-3, x + 1), x -> (x, 1).
fn split(t: &Expr) -> (Expr, Expr) {
    match t {
        Expr::Div(n, d) => ((**n).clone(), (**d).clone()),
        Expr::Neg(a) => {
            let (n, d) = split(a);
            (expr::neg(n), d)
        }
        Expr::Mul(fs) if fs.iter().filter(|f| matches!(f, Expr::Div(..))).count() == 1 => {
            let mut top = Vec::new();
            let mut bottom = expr::num(1);
            for f in fs {
                match f {
                    Expr::Div(n, d) => {
                        top.push((**n).clone());
                        bottom = (**d).clone();
                    }
                    f => top.push(f.clone()),
                }
            }
            (expr::mul(top), bottom)
        }
        t => (t.clone(), expr::num(1)),
    }
}

/// A factor of a denominator: its polynomial (integer, positive leading
/// coefficient) and how it is written.
#[derive(Clone)]
struct Factor {
    p: Poly,
    e: Expr,
}

/// A denominator as a whole number times factors with powers: x^2 - 1 -> 1 * (x - 1)(x + 1).
fn factors(d: &Expr, v: &str) -> Option<(Q, Vec<(Factor, usize)>)> {
    let written: Vec<Expr> = match d {
        Expr::Mul(fs) => fs.clone(),
        d => vec![d.clone()],
    };
    let mut c = Q::ONE;
    let mut out: Vec<(Factor, usize)> = Vec::new();
    for w in written {
        let (base, k) = match &w {
            Expr::Pow(b, n) => ((**b).clone(), usize::try_from(n.as_num().filter(|q| q.is_int())?.num()).ok()?),
            w => (w.clone(), 1),
        };
        let f = poly::from_expr(&base, v)?.factor()?;
        c = c.mul(&f.c.pow(k as i64)?)?;
        let mut parts: Vec<(Poly, usize)> = f.linear.iter().map(|(r, m)| (Poly::linear_factor(r), *m)).collect();
        if f.rest.deg().is_some_and(|d| d > 0) {
            parts.push((f.rest.clone(), 1));
        }
        for (p, m) in parts {
            let e = p.to_expr(v);
            match out.iter_mut().find(|(g, _)| g.p == p) {
                Some((_, n)) => *n += m * k,
                None => out.push((Factor { p, e }, m * k)),
            }
        }
    }
    c.is_int().then_some((c, out))
}

fn product(c: Q, fs: &[(Factor, usize)]) -> Expr {
    let parts: Vec<Expr> = fs.iter().filter(|(_, k)| *k > 0).map(|(f, k)| if *k == 1 { f.e.clone() } else { expr::pow(f.e.clone(), expr::num(*k as i128)) }).collect();
    with_coeff(c, expr::mul(parts))
}

impl Rule for AddFractions {
    fn name(&self) -> &'static str {
        "add_fractions"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["common_denominator"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        // an equation clears its denominators instead; an inequality can't
        if cx.task() == crate::model::Task::Solve && !matches!(m, Math::Ineq(..)) {
            return vec![];
        }
        local("add_fractions", m, |e, _| {
            let Expr::Add(ts) = e else { return vec![] };
            let vars = e.vars();
            if vars.len() != 1 {
                return vec![];
            }
            let v = vars.iter().next().expect("one letter");
            let parts: Vec<(Expr, Expr)> = ts.iter().map(split).collect();
            if !parts.iter().any(|(_, d)| d.has_var(v)) {
                return vec![];
            }
            let Some(dens) = parts.iter().map(|(_, d)| factors(d, v)).collect::<Option<Vec<_>>>() else { return vec![] };
            // least common denominator: lcm of the numbers, every factor to its highest power
            let Some(big) = dens.iter().try_fold(1i128, |a, (c, _)| lcm(a, c.num())) else { return vec![] };
            let mut lcd: Vec<(Factor, usize)> = Vec::new();
            for (_, fs) in &dens {
                for (f, k) in fs {
                    match lcd.iter_mut().find(|(g, _)| g.p == f.p) {
                        Some((_, n)) => *n = (*n).max(*k),
                        None => lcd.push((f.clone(), *k)),
                    }
                }
            }
            let bottom = product(Q::int(big), &lcd);
            let mut tops = Vec::new();
            let mut work = Vec::new();
            for ((n, d), (c, fs)) in parts.iter().zip(&dens) {
                let Some(scale) = Q::int(big).div(c) else { return vec![] };
                let missing: Vec<(Factor, usize)> = lcd.iter().map(|(f, k)| (f.clone(), k - fs.iter().find(|(g, _)| g.p == f.p).map_or(0, |(_, j)| *j))).collect();
                let times = product(Q::ONE, &missing);
                let top = match n.as_num() {
                    Some(k) => match k.mul(&scale) {
                        Some(k) => with_coeff(k, times),
                        None => return vec![],
                    },
                    None => {
                        let (neg, body) = crate::print::split_sign(n);
                        let mut fs = vec![body];
                        if !times.is_num(1) {
                            fs.push(times);
                        }
                        let s = if neg { scale.neg() } else { scale };
                        with_coeff(s, expr::mul(fs))
                    }
                };
                if *d != bottom {
                    let was = if d.is_num(1) { n.clone() } else { expr::div(n.clone(), d.clone()) };
                    work.push(Line::new().e(&was).t(" = ").e(&expr::div(top.clone(), bottom.clone())));
                }
                tops.push(top);
            }
            let new = expr::div(expr::add(tops), bottom.clone());
            let says = Line::new().t("Write each fraction over the common denominator ").e(&bottom).t(" and combine the numerators.");
            vec![Rewrite { variant: "common_denominator", new, says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn adds_over_a_common_denominator() {
        assert_eq!(test_moves(&AddFractions, "simplify 1/x + 1/(x + 1)"), vec!["(x + 1 + x)/(x(x + 1))"]);
        assert_eq!(test_moves(&AddFractions, "simplify x/(x^2 - 1) - 1/(x + 1)"), vec!["(x - (x - 1))/((x - 1)(x + 1))"]);
        assert!(test_moves(&AddFractions, "simplify x/2 + x/3").is_empty());
    }
}
