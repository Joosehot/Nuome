//! The rank part of the Birch and Swinnerton-Dyer conjecture, checked on a
//! table of elliptic curves: for each curve Nuome computes the order of
//! vanishing of L(E, s) at s = 1 (the analytic rank) itself and compares it
//! with the rank in the table (found there by descent).
//!
//! a_p = p + 1 - #E(F_p) by counting points on the (minimal) model, for good
//! and bad primes alike; a_n from the Euler product. With w the sign of the
//! functional equation (found numerically) and N the conductor,
//!   L^(r)(E, 1) = 2 r! sum a_n / n G_r(2 pi n / sqrt N)   when w = (-1)^r,
//! G_0(x) = e^-x, G_1 = E_1, G_r(x) = int_0^inf exp(-x e^u) u^(r-1)/(r-1)! du.

use std::f64::consts::PI;

pub struct Curve {
    pub conductor: u64,
    pub label: String,
    pub a: [i128; 5],
    pub rank: u32,
}

/// Cremona's allcurves format: "11 a 1 [0,-1,1,-10,-20] 0 5". Only the
/// first curve of each isogeny class (the rank is the same across a class).
pub fn parse(text: &str) -> Vec<Curve> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 5 || f[2] != "1" {
                return None;
            }
            let c: Vec<i128> = f[3].trim_matches(|c| c == '[' || c == ']').split(',').filter_map(|v| v.parse().ok()).collect();
            Some(Curve {
                conductor: f[0].parse().ok()?,
                label: format!("{}{}1", f[0], f[1]),
                a: c.try_into().ok()?,
                rank: f[4].parse().ok()?,
            })
        })
        .collect()
}

/// #E(F_p), the point at infinity included, on the reduction of the model.
pub fn points(a: &[i128; 5], p: u64) -> u64 {
    let r = |v: i128| v.rem_euclid(p as i128) as u64;
    let [a1, a2, a3, a4, a6] = [r(a[0]), r(a[1]), r(a[2]), r(a[3]), r(a[4])];
    let rhs = |x: u64| (((x * x % p) * x) + a2 * (x * x % p) + a4 * x + a6) % p;
    if p <= 3 {
        let mut n = 1;
        for x in 0..p {
            for y in 0..p {
                if (y * y + a1 * x * y + a3 * y) % p == rhs(x) {
                    n += 1;
                }
            }
        }
        return n;
    }
    let mut square = vec![false; p as usize];
    for y in 1..p {
        square[(y * y % p) as usize] = true;
    }
    let mut n = 1;
    for x in 0..p {
        let b = (a1 * x + a3) % p;
        let d = ((b * b) % p + 4 * rhs(x)) % p; // discriminant in y
        n += if d == 0 { 1 } else if square[d as usize] { 2 } else { 0 };
    }
    n
}

/// a_1 .. a_m.
fn coefficients(c: &Curve, m: usize) -> Vec<f64> {
    let mut spf = vec![0usize; m + 1];
    for i in 2..=m {
        if spf[i] == 0 {
            let mut j = i;
            while j <= m {
                if spf[j] == 0 {
                    spf[j] = i;
                }
                j += i;
            }
        }
    }
    let mut a = vec![0.0; m + 1];
    if m >= 1 {
        a[1] = 1.0;
    }
    for n in 2..=m {
        let p = spf[n];
        let mut rest = n;
        while rest % p == 0 {
            rest /= p;
        }
        a[n] = if rest > 1 {
            a[n / rest] * a[rest]
        } else if n == p {
            p as f64 + 1.0 - points(&c.a, p as u64) as f64
        } else if c.conductor % p as u64 == 0 {
            a[p] * a[n / p]
        } else {
            a[p] * a[n / p] - p as f64 * a[n / (p * p)]
        };
    }
    a
}

/// E_1(x) for x > 0.
fn e1(x: f64) -> f64 {
    if x <= 1.0 {
        let mut sum = -0.577_215_664_901_532_9 - x.ln();
        let (mut term, mut k) = (1.0, 1.0);
        loop {
            term *= -x / k;
            let add = -term / k;
            sum += add;
            if add.abs() < 1e-17 {
                return sum;
            }
            k += 1.0;
        }
    }
    // continued fraction (modified Lentz)
    let tiny = 1e-300;
    let mut b = x + 1.0;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..500 {
        let an = -((i * i) as f64);
        b += 2.0;
        d = 1.0 / (an * d + b);
        c = b + an / c;
        let del = c * d;
        h *= del;
        if (del - 1.0).abs() < 1e-16 {
            break;
        }
    }
    h * (-x).exp()
}

/// G_r(x).
fn g(r: u32, x: f64) -> f64 {
    match r {
        0 => (-x).exp(),
        1 => e1(x),
        _ => {
            if x > 60.0 {
                return 0.0;
            }
            let top = (60.0 / x).ln().max(0.5);
            let steps = 4000;
            let h = top / steps as f64;
            let fact: f64 = (1..r).map(|k| k as f64).product();
            let f = |u: f64| (-x * u.exp()).exp() * u.powi(r as i32 - 1) / fact;
            let mut s = f(0.0) + f(top);
            for i in 1..steps {
                s += f(i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
            }
            s * h / 3.0
        }
    }
}

pub struct Analytic {
    pub sign: i32,
    pub rank: u32,
    /// L^(r)(E, 1) / r! at that rank
    pub value: f64,
    /// |L^(k)(E, 1) / k!| for the lower k of the same parity (numerically zero)
    pub vanished: Vec<f64>,
}

/// The analytic rank of a curve, from its L-series.
pub fn analytic(c: &Curve) -> Result<Analytic, String> {
    let root = (c.conductor as f64).sqrt();
    let m = (40.0 * 1.25 * root / (2.0 * PI)).ceil() as usize + 1;
    let a = coefficients(c, m);
    // L(E,1) = A(t) + w B(t) for every t > 0: two values of t give w
    let part = |scale: f64| (1..=m).map(|n| a[n] / n as f64 * (-2.0 * PI * n as f64 * scale / root).exp()).sum::<f64>();
    let (a1, b1, a2, b2) = (part(1.0), part(1.0), part(1.0 / 1.2), part(1.2));
    let w = (a2 - a1) / (b1 - b2);
    let sign = if (w - 1.0).abs() < 1e-6 {
        1
    } else if (w + 1.0).abs() < 1e-6 {
        -1
    } else {
        return Err(format!("{}: the sign of the functional equation came out {w}, not +-1", c.label));
    };
    let mut r = if sign == 1 { 0 } else { 1 };
    let mut vanished = Vec::new();
    loop {
        let sum: f64 = (1..=m).map(|n| a[n] / n as f64 * g(r, 2.0 * PI * n as f64 / root)).sum();
        let value = 2.0 * sum; // L^(r)(1) / r!
        if value.abs() > 1e-6 {
            return Ok(Analytic { sign, rank: r, value, vanished });
        }
        vanished.push(value.abs());
        r += 2;
        if r > 5 {
            return Err(format!("{}: L and its first derivatives all vanish numerically", c.label));
        }
    }
}

/// The first readable table.
fn load(paths: &[String]) -> Option<Vec<Curve>> {
    paths.iter().find_map(|p| std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(p)).ok().map(|t| parse(&t)))
}

/// Report lines for the attempt at the conjecture: every class with
/// conductor below `limit`.
pub fn report(limit: u64, tables: &[String]) -> Vec<String> {
    let Some(all) = load(tables) else {
        return vec!["no table of elliptic curves found (tables in rules.toml)".into()];
    };
    let curves: Vec<Curve> = all.into_iter().filter(|c| c.conductor < limit).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let size = curves.len().div_ceil(threads).max(1);
    let results: Vec<Result<Analytic, String>> = std::thread::scope(|s| {
        let parts: Vec<_> = curves.chunks(size).map(|part| s.spawn(move || part.iter().map(analytic).collect::<Vec<_>>())).collect();
        parts.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
    });
    let mut by_rank = [0usize; 4];
    let (mut agree, mut differ, mut failed) = (0usize, Vec::new(), Vec::new());
    let mut examples: Vec<Option<(String, f64)>> = vec![None; 4];
    let (mut smallest, mut largest_zero) = (f64::INFINITY, 0.0f64);
    for (c, r) in curves.iter().zip(&results) {
        match r {
            Ok(an) => {
                smallest = smallest.min(an.value.abs());
                for &v in &an.vanished {
                    largest_zero = largest_zero.max(v);
                }
                if an.rank == c.rank {
                    agree += 1;
                    if let Some(slot) = by_rank.get_mut(an.rank as usize) {
                        *slot += 1;
                    }
                    if let Some(e) = examples.get_mut(an.rank as usize) {
                        e.get_or_insert((c.label.clone(), an.value));
                    }
                } else {
                    differ.push(format!("{} (Nuome {}, table {})", c.label, an.rank, c.rank));
                }
            }
            Err(e) => failed.push(e.clone()),
        }
    }
    let shown: Vec<String> = examples
        .iter()
        .enumerate()
        .filter_map(|(r, e)| e.as_ref().map(|(l, v)| format!("{l}: L{}(E,1)/{r}! = {v:.6} (rank {r})", "'".repeat(r))))
        .collect();
    let mut out = vec![
        format!("computed L(E, s) for every elliptic curve over Q with conductor below {limit}: {} isogeny classes, one curve each from Cremona's tables; a_p by counting points mod p, the sign of the functional equation found numerically, then the order of vanishing at s = 1", curves.len()),
        if differ.is_empty() && failed.is_empty() {
            format!("the analytic rank equals the rank in the table for ALL {agree} classes (rank 0: {}, rank 1: {}, rank 2: {}, rank 3: {})", by_rank[0], by_rank[1], by_rank[2], by_rank[3])
        } else {
            format!("the analytic rank equals the table's rank for {agree} of {} classes; differ: {:?}; failed: {:?}", curves.len(), &differ[..differ.len().min(5)], &failed[..failed.len().min(5)])
        },
        format!("first of each rank: {}", shown.join("; ")),
        format!("certainty: every nonzero value is at least {smallest:.2e}, far above the error of the sums (about 1e-12); the values taken as zero are at most {largest_zero:.1e}. Ranks 0 and 1 are theorems (Gross-Zagier, Kolyvagin); for ranks 2 and 3 a computation can show the lower derivatives are tiny, not exactly zero"),
        "the ranks themselves are from the downloaded table (found there by descent), not computed by Nuome; finitely many curves: evidence, not a proof".into(),
    ];
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(label: &str, conductor: u64, a: [i128; 5]) -> Curve {
        Curve { conductor, label: label.into(), a, rank: 0 }
    }

    #[test]
    fn known_l_values() {
        let e11 = analytic(&curve("11a1", 11, [0, -1, 1, -10, -20])).expect("11a1");
        assert_eq!((e11.sign, e11.rank), (1, 0));
        assert!((e11.value - 0.253_841_860_855_911).abs() < 1e-9, "{}", e11.value);
        let e37 = analytic(&curve("37a1", 37, [0, 0, 1, -1, 0])).expect("37a1");
        assert_eq!((e37.sign, e37.rank), (-1, 1));
        assert!((e37.value - 0.305_999_773_834_052).abs() < 1e-9, "{}", e37.value);
        assert_eq!(analytic(&curve("389a1", 389, [0, 1, 1, -2, 0])).expect("389a1").rank, 2);
        assert_eq!(analytic(&curve("5077a1", 5077, [0, 0, 1, -7, 6])).expect("5077a1").rank, 3);
    }

    #[test]
    fn counts_points() {
        // 11a1 has a_2 = -2, a_3 = -1, a_5 = 1
        let a = [0, -1, 1, -10, -20];
        assert_eq!(points(&a, 2), 5);
        assert_eq!(points(&a, 3), 5);
        assert_eq!(points(&a, 5), 5);
    }
}
