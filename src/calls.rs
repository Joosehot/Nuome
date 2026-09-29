//! Named operations on numbers: gcd, n choose k, the mean of a list,
//! compound growth... A problem like "gcd of 48 and 18" is a `Call` node;
//! rules turn it into the arithmetic a person would write, step by step.
//!
//! The values here are the *reference*: straightforward direct
//! computations that the checks compare the worked answer against. They
//! share no code with the rules that produce the steps.

use crate::q::{gcd, Q};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Named {
    Gcd,
    Lcm,
    /// a mod b: the remainder, 0 <= r < |b|.
    Mod,
    Factorial,
    /// n choose k.
    Choose,
    /// ordered selections of k from n.
    Perm,
    Mean,
    Median,
    Mode,
    Range,
    Variance,
    StdDev,
    SampleVariance,
    SampleStdDev,
    /// a + (a+1) + ... + b.
    SumTo,
    /// (n, t1, t2, t3, ...): the sum of the first n terms of the sequence.
    SeriesSum,
    /// (n, t1, t2, t3, ...): the n-th term.
    NthTerm,
    /// (t1, t2, t3, ...): the sum to infinity of a geometric sequence.
    InfiniteSum,
    /// (principal, rate per period, periods).
    Compound,
    /// (from, to): the change as a percentage of `from`.
    PercentChange,
    /// (part, whole): part as a percentage of whole.
    WhatPercent,
    /// (amount, percent): amount raised by percent (negative lowers it).
    Raise,
    // proofs (main)
    /// (f(n), start, n): f(start) + f(start + 1) + ... + f(n), with f written in the letter n.
    Series,
    /// (d, e): the statement "d divides e".
    Divides,
}

impl Named {
    pub fn name(self) -> &'static str {
        match self {
            Named::Gcd => "gcd",
            Named::Lcm => "lcm",
            Named::Mod => "mod",
            Named::Factorial => "factorial",
            Named::Choose => "C",
            Named::Perm => "P",
            Named::Mean => "mean",
            Named::Median => "median",
            Named::Mode => "mode",
            Named::Range => "range",
            Named::Variance => "variance",
            Named::StdDev => "sd",
            Named::SampleVariance => "sample variance",
            Named::SampleStdDev => "sample sd",
            Named::SumTo => "sum",
            Named::SeriesSum => "sum of terms",
            Named::NthTerm => "term",
            Named::InfiniteSum => "sum to infinity",
            Named::Compound => "compound",
            Named::PercentChange => "percent change",
            Named::WhatPercent => "percent",
            Named::Raise => "raise",
            Named::Series => "sum",
            Named::Divides => "divides",
        }
    }
    /// Is the answer a percentage (printed with %)?
    pub fn is_percent(self) -> bool {
        matches!(self, Named::PercentChange | Named::WhatPercent)
    }
}

/// The arguments as exact numbers, when they all are.
pub fn nums(args: &[crate::expr::Expr]) -> Option<Vec<Q>> {
    args.iter().map(|a| a.as_num()).collect()
}

/// The arguments as whole numbers, when they all are.
pub fn ints(args: &[crate::expr::Expr]) -> Option<Vec<i128>> {
    args.iter().map(|a| a.as_num().filter(|q| q.is_int()).map(|q| q.num())).collect()
}

/// Why a call has no answer, when the reason is about its numbers.
pub fn why_not(f: Named, a: &[Q]) -> Option<String> {
    match f {
        Named::Mode if eval_q(f, a).is_none() => Some("no value appears more often than all the others, so there is no single mode".into()),
        Named::SeriesSum | Named::NthTerm if a.len() < 4 => Some("give at least three terms: two terms never decide whether a sequence is arithmetic or geometric".into()),
        Named::SeriesSum | Named::NthTerm if sequence_kind(&a[1..]).is_none() => Some("the terms have neither a common difference nor a common ratio".into()),
        Named::InfiniteSum => match sequence_kind(a) {
            Some(Kind::Geometric(r)) if r.abs() >= Q::ONE => Some(format!("the common ratio is {r}; a sum to infinity only exists when it is between -1 and 1")),
            Some(Kind::Geometric(_)) => None,
            _ => Some("a sum to infinity needs a geometric sequence (a common ratio between -1 and 1)".into()),
        },
        Named::Mod if a.get(1).is_some_and(|m| m.is_zero()) => Some("mod 0 is undefined".into()),
        Named::Factorial if a.first().is_some_and(|n| !n.is_int() || n.is_neg()) => Some("factorials are for whole numbers 0, 1, 2, ...".into()),
        _ => None,
    }
}

fn int(q: &Q) -> Option<i128> {
    q.is_int().then(|| q.num())
}

fn sorted(xs: &[Q]) -> Vec<Q> {
    let mut v = xs.to_vec();
    v.sort();
    v
}

fn mean(xs: &[Q]) -> Option<Q> {
    if xs.is_empty() {
        return None;
    }
    xs.iter().try_fold(Q::ZERO, |a, x| a.add(x))?.div(&Q::int(xs.len() as i128))
}

fn variance(xs: &[Q], sample: bool) -> Option<Q> {
    let m = mean(xs)?;
    let ss = xs.iter().try_fold(Q::ZERO, |a, x| {
        let d = x.sub(&m)?;
        a.add(&d.mul(&d)?)
    })?;
    let n = xs.len() as i128 - i128::from(sample);
    if n <= 0 {
        return None;
    }
    ss.div(&Q::int(n))
}

/// Is the list arithmetic (constant difference) or geometric (constant
/// ratio)? Needs at least three terms: two never decide it.
pub enum Kind {
    Arithmetic(Q),
    Geometric(Q),
}

pub fn sequence_kind(ts: &[Q]) -> Option<Kind> {
    if ts.len() < 3 {
        return None;
    }
    let d = ts[1].sub(&ts[0])?;
    if ts.windows(2).all(|w| w[1].sub(&w[0]) == Some(d)) {
        return Some(Kind::Arithmetic(d));
    }
    if ts.iter().any(|t| t.is_zero()) {
        return None;
    }
    let r = ts[1].div(&ts[0])?;
    if ts.windows(2).all(|w| w[1].div(&w[0]) == Some(r)) {
        return Some(Kind::Geometric(r));
    }
    None
}

/// The exact value, when there is one.
pub fn eval_q(f: Named, a: &[Q]) -> Option<Q> {
    match f {
        Named::Gcd | Named::Lcm => {
            let ns: Vec<i128> = a.iter().map(int).collect::<Option<_>>()?;
            if ns.is_empty() {
                return None;
            }
            let mut acc = ns[0].abs();
            for &n in &ns[1..] {
                acc = if f == Named::Gcd {
                    gcd(acc, n)
                } else {
                    if acc == 0 || n == 0 {
                        return Some(Q::ZERO);
                    }
                    (acc / gcd(acc, n)).checked_mul(n.abs())?
                };
            }
            Some(Q::int(acc))
        }
        Named::Mod => {
            let (x, m) = (int(a.first()?)?, int(a.get(1)?)?);
            (m != 0).then(|| Q::int(x.rem_euclid(m)))
        }
        Named::Factorial => {
            let n = int(a.first()?)?;
            if !(0..=33).contains(&n) {
                return None;
            }
            (1..=n).try_fold(1i128, |p, k| p.checked_mul(k)).map(Q::int)
        }
        Named::Choose | Named::Perm => {
            let (n, k) = (int(a.first()?)?, int(a.get(1)?)?);
            if k < 0 || n < 0 || k > n {
                return Some(Q::ZERO);
            }
            let mut r = 1i128;
            for i in 0..k {
                r = r.checked_mul(n - i)?;
                if f == Named::Choose {
                    r /= i + 1; // exact: a product of i+1 consecutive integers is divisible by (i+1)!
                }
            }
            Some(Q::int(r))
        }
        Named::Mean => mean(a),
        Named::Median => {
            let s = sorted(a);
            let n = s.len();
            match n {
                0 => None,
                _ if n % 2 == 1 => Some(s[n / 2]),
                _ => s[n / 2 - 1].add(&s[n / 2])?.div(&Q::int(2)),
            }
        }
        Named::Mode => {
            let s = sorted(a);
            let mut best: Vec<(Q, usize)> = Vec::new();
            for x in &s {
                match best.iter_mut().find(|(v, _)| v == x) {
                    Some((_, c)) => *c += 1,
                    None => best.push((*x, 1)),
                }
            }
            let top = best.iter().map(|(_, c)| *c).max()?;
            let tops: Vec<&(Q, usize)> = best.iter().filter(|(_, c)| *c == top).collect();
            (tops.len() == 1 && top > 1).then(|| tops[0].0)
        }
        Named::Range => {
            let s = sorted(a);
            s.last()?.sub(s.first()?)
        }
        Named::Variance => variance(a, false),
        Named::SampleVariance => variance(a, true),
        Named::StdDev => variance(a, false)?.sqrt(),
        Named::SampleStdDev => variance(a, true)?.sqrt(),
        Named::SumTo => {
            let (x, y) = (int(a.first()?)?, int(a.get(1)?)?);
            if y < x || y - x > 10_000_000 {
                return None;
            }
            let mut s = 0i128;
            for k in x..=y {
                s = s.checked_add(k)?;
            }
            Some(Q::int(s))
        }
        Named::SeriesSum | Named::NthTerm => {
            let n = int(a.first()?)?;
            if !(1..=100_000).contains(&n) {
                return None;
            }
            let ts = &a[1..];
            let kind = sequence_kind(ts)?;
            let mut term = ts[0];
            let mut sum = Q::ZERO;
            for i in 1..=n {
                sum = sum.add(&term)?;
                if i == n {
                    break;
                }
                term = match kind {
                    Kind::Arithmetic(d) => term.add(&d)?,
                    Kind::Geometric(r) => term.mul(&r)?,
                };
            }
            Some(if f == Named::NthTerm { term } else { sum })
        }
        Named::InfiniteSum => None, // a limit: checked numerically
        Named::Compound => {
            let (p, r, n) = (a.first()?, a.get(1)?, int(a.get(2)?)?);
            if !(0..=10_000).contains(&n) {
                return None;
            }
            let g = Q::ONE.add(r)?;
            let mut acc = *p;
            for _ in 0..n {
                acc = acc.mul(&g)?;
            }
            Some(acc)
        }
        Named::PercentChange => {
            let (x, y) = (a.first()?, a.get(1)?);
            y.sub(x)?.div(x)?.mul(&Q::int(100))
        }
        Named::WhatPercent => a.first()?.div(a.get(1)?)?.mul(&Q::int(100)),
        // a sum with a letter in it, and a statement: evaluated in expr.rs / not a number
        Named::Series | Named::Divides => None,
        Named::Raise => {
            let (x, p) = (a.first()?, a.get(1)?);
            x.mul(&Q::ONE.add(&p.div(&Q::int(100))?)?)
        }
    }
}

/// The value as a float, for what isn't rational (square roots, sums to
/// infinity, compound growth too large for exact fractions).
pub fn eval_f(f: Named, a: &[f64]) -> f64 {
    match f {
        Named::StdDev | Named::SampleStdDev => {
            let n = a.len() as f64;
            let m = a.iter().sum::<f64>() / n;
            let ss: f64 = a.iter().map(|x| (x - m) * (x - m)).sum();
            (ss / if f == Named::SampleStdDev { n - 1.0 } else { n }).sqrt()
        }
        Named::InfiniteSum => {
            // add terms until they stop mattering
            if a.len() < 3 || a[0] == 0.0 {
                return f64::NAN;
            }
            let r = a[1] / a[0];
            if (a[2] / a[1] - r).abs() > 1e-12 || r.abs() >= 1.0 {
                return f64::NAN;
            }
            let (mut s, mut t) = (0.0, a[0]);
            for _ in 0..100_000 {
                s += t;
                t *= r;
                if t.abs() < 1e-18 {
                    break;
                }
            }
            s
        }
        Named::Compound => {
            let (p, r, n) = (a[0], a[1], a[2]);
            let mut acc = p;
            for _ in 0..n.max(0.0) as u64 {
                acc *= 1.0 + r;
            }
            acc
        }
        _ => {
            let qs: Option<Vec<Q>> = a.iter().map(|x| to_q(*x)).collect();
            qs.and_then(|qs| eval_q(f, &qs)).map_or(f64::NAN, |q| q.to_f64())
        }
    }
}

/// A float that is exactly a short decimal back to a rational (for the
/// float fallback of list operations).
fn to_q(x: f64) -> Option<Q> {
    for places in 0..=9u32 {
        let scale = 10f64.powi(places as i32);
        let n = (x * scale).round();
        if ((n / scale) - x).abs() < 1e-12 && n.abs() < 1e15 {
            return Q::new(n as i128, 10i128.pow(places));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qs(v: &[i128]) -> Vec<Q> {
        v.iter().map(|&n| Q::int(n)).collect()
    }

    #[test]
    fn reference_values() {
        assert_eq!(eval_q(Named::Gcd, &qs(&[48, 18])), Some(Q::int(6)));
        assert_eq!(eval_q(Named::Lcm, &qs(&[4, 6])), Some(Q::int(12)));
        assert_eq!(eval_q(Named::Mod, &qs(&[-7, 5])), Some(Q::int(3)));
        assert_eq!(eval_q(Named::Choose, &qs(&[10, 3])), Some(Q::int(120)));
        assert_eq!(eval_q(Named::Median, &qs(&[5, 1, 3, 9])), Some(Q::int(4)));
        assert_eq!(eval_q(Named::Mode, &qs(&[1, 2, 2, 3])), Some(Q::int(2)));
        assert_eq!(eval_q(Named::Mode, &qs(&[1, 2, 3])), None);
        assert_eq!(eval_q(Named::SumTo, &qs(&[1, 100])), Some(Q::int(5050)));
        assert_eq!(eval_q(Named::SeriesSum, &qs(&[4, 3, 7, 11])), Some(Q::int(3 + 7 + 11 + 15)));
        assert_eq!(eval_q(Named::NthTerm, &qs(&[5, 2, 6, 18])), Some(Q::int(162)));
        let sum = eval_f(Named::InfiniteSum, &[1.0, 0.5, 0.25]);
        assert!((sum - 2.0).abs() < 1e-12);
    }
}
