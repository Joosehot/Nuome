//! Solve "P rel 0" by a sign chart: find where P is 0 (and, for a fraction,
//! where it is undefined), then test one point in each interval between.
//! x^2 - 5x + 6 < 0: zeros 2 and 3; at 0 it is 6 (+), at 5/2 it is -1/4
//! (-), at 4 it is 2 (+); so 2 < x < 3. Variants: a polynomial, or a
//! fraction (whose undefined points are never included).

use super::{Cx, Line, Move, Rule};
use crate::expr::{self, with_coeff, Bound, Expr, Interval, Math};
use crate::poly::{self, Poly};
use crate::print::{self, Style};
use crate::q::{self, Q};

pub struct SignChart;

/// A point where the sign can change.
struct Crit {
    at: Expr,
    x: f64,
    /// P is 0 there (else undefined).
    zero: bool,
}

/// The real roots of a polynomial, exactly: rational ones, and a leftover
/// quadratic's by the formula, simplified. None if a bigger piece is left.
pub fn exact_roots(p: &Poly) -> Option<Vec<Expr>> {
    let f = p.factor()?;
    let mut out: Vec<Expr> = f.linear.iter().map(|(r, _)| Expr::Num(*r)).collect();
    match f.rest.deg()? {
        0 => {}
        2 => {
            let (a, b, c) = (f.rest.coef(2).num(), f.rest.coef(1).num(), f.rest.coef(0).num());
            let d = b.checked_mul(b)?.checked_sub(a.checked_mul(c)?.checked_mul(4)?)?;
            if d > 0 {
                let (k, m) = q::split_square(d);
                let (mut two_a, mut minus_b, mut k) = (2 * a, -b, k);
                let g = q::gcd(q::gcd(minus_b, k), two_a).max(1);
                (two_a, minus_b, k) = (two_a / g, minus_b / g, k / g);
                if two_a < 0 {
                    (two_a, minus_b) = (-two_a, -minus_b);
                    k = -k;
                }
                for s in [-1, 1] {
                    let root = expr::sqrt(expr::num(m));
                    let top = expr::add(vec![expr::num(minus_b), with_coeff(Q::int(s * k), root)]);
                    out.push(if two_a == 1 { top } else { expr::div(top, expr::num(two_a)) });
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// A simple number strictly between a and b (integers first, closest to 0).
fn test_point(a: f64, b: f64) -> Q {
    if a < 0.0 && b > 0.0 {
        return Q::ZERO;
    }
    for den in [1i128, 2, 4, 8, 16, 32, 64, 1024] {
        let lo = if a.is_finite() { (a * den as f64).floor() as i128 + 1 } else { (b * den as f64).ceil() as i128 - den };
        let hi = if b.is_finite() { (b * den as f64).ceil() as i128 - 1 } else { lo + den };
        let pick = if lo > 0 { lo } else if hi < 0 { hi } else { 0 };
        if (lo..=hi).contains(&pick) {
            if let Some(q) = Q::new(pick, den) {
                let x = q.to_f64();
                if x > a && x < b {
                    return q;
                }
            }
        }
    }
    Q::new(((a + b) / 2.0 * 1024.0).round() as i128, 1024).unwrap_or(Q::ZERO)
}

impl Rule for SignChart {
    fn name(&self) -> &'static str {
        "sign_chart"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["polynomial", "rational"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let Math::Ineq(p, rel, zero) = m else { return vec![] };
        if !zero.is_num(0) || !p.has_var(v) || p.vars().len() != 1 {
            return vec![];
        }
        // a sum with a fraction in it is made one fraction first
        if expr::terms(p).len() > 1 && expr::terms(p).iter().any(|t| crate::rules::move_term::var_in_denominator(t, v)) {
            return vec![];
        }
        let Some((n, d)) = poly::rational_from_expr(p, v) else { return vec![] };
        let fraction = d.deg().is_some_and(|k| k > 0);
        // a linear inequality is solved by moving and dividing
        if !fraction && n.deg().is_none_or(|k| k < 2) {
            return vec![];
        }
        let (Some(zs), Some(us)) = (exact_roots(&n), if fraction { exact_roots(&d) } else { Some(vec![]) }) else { return vec![] };
        let x = expr::var(v);
        let mut crits: Vec<Crit> = Vec::new();
        for (list, is_zero) in [(&us, false), (&zs, true)] {
            for e in list.iter() {
                let val = e.eval_f(&|_| f64::NAN);
                if !crits.iter().any(|c| (c.x - val).abs() < 1e-12) {
                    crits.push(Crit { at: e.clone(), x: val, zero: is_zero });
                }
            }
        }
        crits.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
        let sign_at = |t: &Q| -> Option<Q> { n.eval(t)?.div(&d.eval(t)?) };
        let mut work = Vec::new();
        let list = |cs: &[&Crit]| {
            let mut l = Line::new();
            for (i, c) in cs.iter().enumerate() {
                l = if i == 0 { l } else { l.t(", ") }.e(&x).t(" = ").e(&c.at);
            }
            l
        };
        let zero_list: Vec<&Crit> = crits.iter().filter(|c| c.zero).collect();
        let undef_list: Vec<&Crit> = crits.iter().filter(|c| !c.zero).collect();
        let top = if fraction { n.to_expr(v) } else { p.clone() };
        let factored = n.factor().map(|f| f.to_expr(v));
        let mut line = Line::new().e(&top);
        if let Some(f) = factored.filter(|f| *f != top && !fraction) {
            line = line.t(" = ").e(&f);
        }
        work.push(if zero_list.is_empty() { line.t(" is never 0.") } else { line.t(" is 0 at ").join(list(&zero_list)).t(".") });
        if fraction {
            let bottom = d.to_expr(v);
            work.push(if undef_list.is_empty() { Line::new().e(&bottom).t(" is never 0.") } else { Line::new().e(&bottom).t(" is 0 at ").join(list(&undef_list)).t(": undefined there.") });
        }
        // one test point per interval: its sign decides the interval
        let mut edges = vec![f64::NEG_INFINITY];
        edges.extend(crits.iter().map(|c| c.x));
        edges.push(f64::INFINITY);
        let mut inside: Vec<bool> = Vec::new();
        for (k, w) in edges.windows(2).enumerate() {
            let t = test_point(w[0], w[1]);
            let Some(val) = sign_at(&t) else { return vec![] };
            let holds = rel.holds_q(&val, &Q::ZERO);
            inside.push(holds);
            let lo = (k > 0).then(|| Bound { at: crits[k - 1].at.clone(), closed: false });
            let hi = (k < crits.len()).then(|| Bound { at: crits[k].at.clone(), closed: false });
            let piece = if crits.is_empty() { "It never changes sign".to_string() } else { print::inequalities(v, &[Interval { lo, hi }], Style::Ascii) };
            let sign = if val.is_neg() { "negative" } else { "positive" };
            work.push(Line::new().t(format!("{piece}: at {v} = {t} it is {val}, {sign}.")));
        }
        // runs of intervals (and the zeros between them) where the inequality holds
        let point_in = |c: &Crit| c.zero && !rel.strict();
        let mut ivs: Vec<Interval> = Vec::new();
        let mut open: Option<Option<Bound>> = None;
        for k in 0..=crits.len() {
            if inside[k] {
                if open.is_none() {
                    open = Some((k > 0).then(|| Bound { at: crits[k - 1].at.clone(), closed: point_in(&crits[k - 1]) }));
                }
            } else if let Some(lo) = open.take() {
                ivs.push(Interval { lo, hi: Some(Bound { at: crits[k - 1].at.clone(), closed: point_in(&crits[k - 1]) }) });
            }
            if k < crits.len() {
                let c = &crits[k];
                let next_in = inside.get(k + 1).copied().unwrap_or(false);
                // a zero that is a solution on its own, between two intervals that aren't
                if point_in(c) && !inside[k] && !next_in {
                    ivs.push(Interval { lo: Some(Bound { at: c.at.clone(), closed: true }), hi: Some(Bound { at: c.at.clone(), closed: true }) });
                }
                // an interval that stops at a point that isn't a solution
                if open.is_some() && next_in && !point_in(c) {
                    let lo = open.take().expect("open run");
                    ivs.push(Interval { lo, hi: Some(Bound { at: c.at.clone(), closed: false }) });
                }
            }
        }
        if let Some(lo) = open {
            ivs.push(Interval { lo, hi: None });
        }
        let result = match ivs.as_slice() {
            [] => Math::NoSolution,
            [Interval { lo: None, hi: None }] => Math::AllReals,
            _ => Math::Intervals(v.to_string(), ivs),
        };
        let variant = if fraction { "rational" } else { "polynomial" };
        let what = if fraction { "is 0 or undefined" } else { "is 0" };
        let says = Line::new().t("Find where ").e(p).t(format!(" {what}, then test its sign in each interval."));
        vec![Move { rule: "sign_chart", variant, result, says, work }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn reads_the_sign_chart() {
        assert_eq!(test_moves(&SignChart, "solve x^2 - 5x + 6 < 0"), vec!["2 < x < 3"]);
        assert_eq!(test_moves(&SignChart, "solve x^2 - 5x + 6 >= 0"), vec!["x <= 2 or x >= 3"]);
        assert_eq!(test_moves(&SignChart, "solve (x - 1)/(x + 2) >= 0"), vec!["x < -2 or x >= 1"]);
        assert_eq!(test_moves(&SignChart, "solve (x - 1)^2 <= 0"), vec!["x = 1"]);
        assert_eq!(test_moves(&SignChart, "solve x^2 + 1 > 0"), vec!["every real number"]);
        assert!(test_moves(&SignChart, "solve 2x - 4 > 0").is_empty());
    }
}
