//! The factor theorem: if P(r) = 0 then (x - r) divides P. Try the
//! candidates of the rational root theorem (divisors of the constant over
//! divisors of the leading coefficient) until one gives 0, then divide the
//! factor out by synthetic division: x^3 - 6x^2 + 11x - 6 ->
//! (x - 1)(x^2 - 5x + 6). Variants: a whole-number root, or a fraction
//! root p/q, whose factor is (qx - p).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Math};
use crate::model::Task;
use crate::poly::{self, Poly};
use crate::q::Q;

pub struct FactorTheorem;

/// The order a person tries candidates in: 1, -1, 2, -2, ..., then fractions.
fn try_order(r: &Q) -> (i128, i128, bool) {
    (r.den(), r.num().abs(), r.is_neg())
}

/// P(r) written out term by term: 1 - 6 + 11 - 6.
fn value_terms(p: &Poly, r: &Q) -> Option<Expr> {
    let mut ts = Vec::new();
    for i in (0..p.0.len()).rev() {
        if p.0[i].is_zero() {
            continue;
        }
        ts.push(Expr::Num(p.0[i].mul(&r.pow(i as i64)?)?));
    }
    Some(Expr::Add(ts))
}

/// Coefficients as a row: "1  -6  11  -6".
pub fn row(p: &Poly) -> String {
    p.0.iter().rev().map(|c| c.to_string()).collect::<Vec<_>>().join("  ")
}

/// The positive candidates p/q of the rational root theorem, in the order they are tried.
fn candidates(p: &Poly) -> Option<Vec<Q>> {
    let (c0, an) = (p.coef(0).num().abs(), p.lead().num().abs());
    if an > 10_000 || c0 > 10_000 {
        return None;
    }
    let mut out: Vec<Q> = Vec::new();
    for q in (1..=an).filter(|q| an % q == 0) {
        for n in (1..=c0).filter(|n| c0 % n == 0) {
            let c = Q::new(n, q)?;
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out.sort_by_key(try_order);
    Some(out)
}

/// Divide out one rational root: (the factor, the quotient, variant, working).
pub fn divide_out(p: &Poly, v: &str) -> Option<(Expr, Expr, &'static str, Vec<Line>)> {
    let mut roots = p.rational_roots();
    roots.sort_by_key(try_order);
    let r = *roots.first()?;
    let cands = candidates(p)?;
    let (c0, an) = (p.coef(0).num().abs(), p.lead().num().abs());
    let mut line = Line::new().t("Candidates: ");
    for (i, c) in cands.iter().take(8).enumerate() {
        line = if i == 0 { line.pm() } else { line.t(", ").pm() }.e(&Expr::Num(*c));
    }
    let more = if cands.len() > 8 { ", ..." } else { "" };
    let of = if an == 1 { format!("{more} (the divisors of {c0}).") } else { format!("{more} (divisors of {c0} over divisors of {an}).") };
    let mut work = vec![line.t(of)];
    // the candidates tried before the root: the last few
    let mut tried: Vec<Q> = Vec::new();
    'cands: for c in &cands {
        for s in [Q::ONE, Q::int(-1)] {
            let c = c.mul(&s)?;
            if try_order(&c) >= try_order(&r) {
                break 'cands;
            }
            tried.push(c);
        }
    }
    let x = expr::var(v);
    let skip = tried.len().saturating_sub(3);
    for c in &tried[skip..] {
        let val = p.eval(c)?;
        work.push(Line::new().e(&x).t(" = ").e(&Expr::Num(*c)).t(format!(" gives {val}, not 0.")));
    }
    let f = Poly::linear_factor(&r);
    let fx = f.to_expr(v);
    work.push(Line::new().e(&x).t(" = ").e(&Expr::Num(r)).t(" gives ").e(&value_terms(p, &r)?).t(" = 0."));
    let (quot, rem) = p.divmod(&f)?;
    if !rem.is_zero() {
        return None;
    }
    let variant = if r.is_int() {
        let (syn, _) = p.divmod(&Poly(vec![r.neg(), Q::ONE]))?;
        work.push(Line::new().t("Synthetic division by ").e(&fx).t(format!(": {}  ->  {}, remainder 0.", row(p), row(&syn))));
        "integer_root"
    } else {
        work.push(Line::new().t("Divide by ").e(&fx).t(": the quotient is ").e(&quot.to_expr(v)).t(", remainder 0."));
        "fraction_root"
    };
    Some((fx, quot.to_expr(v), variant, work))
}

impl Rule for FactorTheorem {
    fn name(&self) -> &'static str {
        "factor_theorem"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["integer_root", "fraction_root"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        local("factor_theorem", m, |e, at| {
            // when solving, only "... = 0" wants factors
            if cx.task() == Task::Solve && !at.is_side_against_zero() {
                return vec![];
            }
            let Expr::Add(_) = e else { return vec![] };
            let vars = e.vars();
            if vars.len() != 1 {
                return vec![];
            }
            let v = vars.iter().next().expect("one letter");
            let Some(p) = poly::from_expr(e, v) else { return vec![] };
            // cubics and up with integer coefficients, no common factor and no factor x (factor_common goes first)
            if p.deg().is_none_or(|d| d < 3) || p.coef(0).is_zero() || p.primitive().is_none_or(|ints| ints.iter().map(|&c| Q::int(c)).collect::<Vec<_>>() != p.0) {
                return vec![];
            }
            let Some((f, quot, variant, work)) = divide_out(&p, v) else { return vec![] };
            let root = p.rational_roots().into_iter().min_by_key(try_order).expect("divide_out found one");
            let says = Line::new().e(&expr::var(v)).t(" = ").e(&Expr::Num(root)).t(" makes it 0, so ").e(&f).t(" is a factor (the factor theorem).");
            vec![Rewrite { variant, new: Expr::Mul(vec![f, quot]), says, work }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn divides_out_a_root() {
        assert_eq!(test_moves(&FactorTheorem, "factor x^3 - 6x^2 + 11x - 6"), vec!["(x - 1)(x^2 - 5x + 6)"]);
        assert_eq!(test_moves(&FactorTheorem, "solve 2x^3 - 3x^2 - 3x + 2 = 0"), vec!["(x + 1)(2x^2 - 5x + 2) = 0"]);
        assert_eq!(test_moves(&FactorTheorem, "factor 2x^3 + x^2 + x - 1"), vec!["(2x - 1)(x^2 + x + 1)"]);
        assert!(test_moves(&FactorTheorem, "factor x^3 + x + 1").is_empty());
        assert!(test_moves(&FactorTheorem, "factor x^2 - 5x + 6").is_empty());
    }
}
