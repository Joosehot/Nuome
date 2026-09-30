//! Checking the Riemann hypothesis up to a height T the way it is done in the
//! literature (Brent 1979, van de Lune et al. 1986, Gourdon 2004):
//!
//! 1. Z(t) is real on the critical line and |Z(t)| = |zeta(1/2 + it)|, so a
//!    sign change of Z is a zero of zeta exactly on the line. Every value of
//!    Z comes with an error bound (Euler-Maclaurin below t = 200, Gabcke's
//!    bound on the Riemann-Siegel remainder above), and a sign is used only
//!    when |Z| is larger than that bound.
//! 2. The Gram points g_n (theta(g_n) = n pi) split the line into Gram blocks;
//!    Rosser's rule (a block of k Gram intervals holds at least k sign
//!    changes) is checked for every block, which gives at least n + 1 zeros
//!    on the line up to g_n.
//! 3. Turing's method in Brent's form: K further blocks obeying Rosser's rule,
//!    K >= 0.0061 ln^2(g_p) + 0.08 ln(g_p), give N(g_n) <= n + 1 for ALL zeros
//!    in the strip. Together: exactly n + 1 zeros up to g_n, every one on the
//!    line and simple.
//!
//! Floating-point rounding is bounded by an estimate, not interval arithmetic.

use std::f64::consts::PI;

/// Lehman's bound behind Turing's method needs heights above 168 pi.
pub const LOWEST_HEIGHT: f64 = 600.0;

/// A sign is used only when |Z| is at least this many times its error bound.
pub const SAFETY: f64 = 2.0;

/// theta(t) = arg Gamma(1/4 + it/2) - (t/2) ln pi by Stirling's series; for
/// t >= 10 the first omitted term, 511/(1216512 t^9), is below 5e-10.
pub fn theta(t: f64) -> f64 {
    t / 2.0 * (t / (2.0 * PI)).ln() - t / 2.0 - PI / 8.0
        + 1.0 / (48.0 * t)
        + 7.0 / (5760.0 * t.powi(3))
        + 31.0 / (80640.0 * t.powi(5))
        + 127.0 / (430080.0 * t.powi(7))
}

#[derive(Clone, Copy)]
struct C(f64, f64);

impl C {
    fn add(self, o: C) -> C {
        C(self.0 + o.0, self.1 + o.1)
    }
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn scale(self, k: f64) -> C {
        C(self.0 * k, self.1 * k)
    }
    fn div(self, o: C) -> C {
        let d = o.0 * o.0 + o.1 * o.1;
        C((self.0 * o.0 + self.1 * o.1) / d, (self.1 * o.0 - self.0 * o.1) / d)
    }
    fn abs(self) -> f64 {
        self.0.hypot(self.1)
    }
}

/// n^(-s) for s = 1/2 + it.
fn n_pow(n: f64, t: f64) -> C {
    let a = t * n.ln();
    C(a.cos(), -a.sin()).scale(n.powf(-0.5))
}

/// B_2k / (2k)! = (-1)^(k+1) 2 zeta(2k) / (2 pi)^(2k).
fn bernoulli_over_factorial(k: i32) -> f64 {
    let z = match k {
        1 => PI * PI / 6.0,
        2 => PI.powi(4) / 90.0,
        _ => (1..=200).map(|n| (n as f64).powi(-2 * k)).sum::<f64>(),
    };
    let sign = if k % 2 == 1 { 1.0 } else { -1.0 };
    sign * 2.0 * z / (2.0 * PI).powi(2 * k)
}

/// Z(t) by Euler-Maclaurin summation of zeta(1/2 + it), with a rigorous bound
/// on the truncation (the next term times |s + 2m + 1| / (sigma + 2m + 1)).
fn z_euler_maclaurin(t: f64) -> (f64, f64) {
    let s = C(0.5, t);
    let big = (t / PI).ceil().max(20.0);
    let m = 20;
    let mut zeta = C(0.0, 0.0);
    for n in 1..big as usize {
        zeta = zeta.add(n_pow(n as f64, t));
    }
    let n_s = n_pow(big, t);
    zeta = zeta.add(n_s.scale(big).div(C(s.0 - 1.0, s.1)));
    zeta = zeta.add(n_s.scale(0.5));
    let mut rising = s; // s (s+1) ... (s+2k-2)
    let mut power = n_s.scale(1.0 / big); // N^(-s-2k+1)
    for k in 1..=m {
        zeta = zeta.add(rising.mul(power).scale(bernoulli_over_factorial(k)));
        let j = 2.0 * k as f64;
        rising = rising.mul(C(s.0 + j - 1.0, s.1)).mul(C(s.0 + j, s.1));
        power = power.scale(1.0 / (big * big));
    }
    let next = rising.mul(power).abs() * bernoulli_over_factorial(m + 1).abs();
    let tail = next * C(s.0 + 2.0 * m as f64 + 1.0, s.1).abs() / (0.5 + 2.0 * m as f64 + 1.0);
    let th = theta(t);
    let z = th.cos() * zeta.0 - th.sin() * zeta.1;
    // each argument t ln n is off by a few ulps; the weights sum to < 2 sqrt(N);
    // theta's omitted term and rounding turn into at most 1e-9 |zeta|
    let rounding = 1e-15 * (t * big.ln() + th.abs()) * 2.0 * big.sqrt() + 1e-15 * big + 1e-9 * zeta.abs();
    (z, tail + rounding)
}

/// C0(p) = cos(2 pi (p^2 - p - 1/16)) / cos(2 pi p), smooth through the
/// removable points p = 1/4 and 3/4.
fn c0(p: f64) -> f64 {
    let f = |p: f64| (2.0 * PI * (p * p - p - 1.0 / 16.0)).cos() / (2.0 * PI * p).cos();
    if (2.0 * PI * p).cos().abs() < 1e-6 {
        (f(p - 1e-5) + f(p + 1e-5)) / 2.0
    } else {
        f(p)
    }
}

/// Z(t) by the Riemann-Siegel formula with the first correction term, for
/// t >= 200, where Gabcke (1979) bounds the remainder by 0.127 (t/2 pi)^(-3/4).
fn z_riemann_siegel(t: f64) -> (f64, f64) {
    let tau = t / (2.0 * PI);
    let a = tau.sqrt();
    let n = a.floor() as usize;
    let th = theta(t);
    let mut sum = 0.0;
    for k in 1..=n {
        sum += (th - t * (k as f64).ln()).cos() / (k as f64).sqrt();
    }
    let sign = if n % 2 == 1 { 1.0 } else { -1.0 };
    let z = 2.0 * sum + sign * tau.powf(-0.25) * c0(a - n as f64);
    // each cosine argument is off by a few ulps of theta; the weights sum to < 4 sqrt(n)
    let rounding = 1e-15 * th.abs() * 4.0 * a + 1e-12 + 1e-9 * tau.powf(-0.25);
    (z, 0.127 * tau.powf(-0.75) + rounding)
}

/// Z(t) and a bound on the error of that value. Near a zero, where the
/// Riemann-Siegel bound is too coarse to fix the sign, Euler-Maclaurin
/// (slower, with a much smaller bound) is used instead.
pub fn z(t: f64) -> (f64, f64) {
    if t >= 200.0 {
        let (v, e) = z_riemann_siegel(t);
        if v.abs() > SAFETY * e {
            return (v, e);
        }
    }
    z_euler_maclaurin(t)
}

/// The Gram point g_n (theta(g_n) = n pi) by Newton's method from `near`.
fn gram(n: i64, near: f64) -> f64 {
    let mut g = near;
    for _ in 0..60 {
        let step = (theta(g) - n as f64 * PI) / (0.5 * (g / (2.0 * PI)).ln());
        g -= step;
        if step.abs() < 1e-13 * g {
            break;
        }
    }
    g
}

/// What a verification found.
pub struct Verified {
    /// the height g_n reached
    pub height: f64,
    /// its Gram index n
    pub index: i64,
    /// zeros found on the line up to g_n (equal to n + 1)
    pub zeros: u64,
    pub first: Vec<f64>,
    pub blocks: u64,
    pub longest_block: i64,
    pub bad_gram_points: u64,
    pub turing_blocks: u64,
    pub turing_needed: f64,
    /// the smallest |Z| used, as a multiple of its error bound
    pub margin: f64,
    pub evaluations: u64,
}

/// Verify the hypothesis up to at least `t_max`.
pub fn verify(t_max: f64) -> Result<Verified, String> {
    let t_max = t_max.max(LOWEST_HEIGHT);
    let evaluations = std::cell::Cell::new(0u64);
    let mut margin = f64::INFINITY;
    let signed = |t: f64, margin: &mut f64| -> Option<f64> {
        evaluations.set(evaluations.get() + 1);
        let (v, e) = z(t);
        if v.abs() > SAFETY * e {
            *margin = margin.min(v.abs() / e);
            Some(v.signum())
        } else {
            None
        }
    };
    // Gram points and the signs of Z there, from g_-1
    let mut g = vec![gram(-1, 10.0)];
    let mut sign = Vec::new();
    let at = |n: i64| (n + 1) as usize;
    let ensure = |n: i64, g: &mut Vec<f64>, sign: &mut Vec<f64>, margin: &mut f64| -> Result<(), String> {
        while g.len() <= at(n) {
            let last = *g.last().expect("g_-1");
            let k = g.len() as i64 - 1;
            g.push(gram(k, last + 2.0 * PI / (last / (2.0 * PI)).ln()));
        }
        while sign.len() <= at(n) {
            let i = sign.len();
            match signed(g[i], margin) {
                Some(s) => sign.push(s),
                None => return Err(format!("the sign of Z at the Gram point g_{} = {:.6} could not be fixed within its error bound", i as i64 - 1, g[i])),
            }
        }
        Ok(())
    };
    let good = |n: i64, sign: &[f64]| (if n % 2 == 0 { 1.0 } else { -1.0 }) * sign[at(n)] > 0.0;
    ensure(-1, &mut g, &mut sign, &mut margin)?;
    if !good(-1, &sign) {
        return Err("g_-1 is not a good Gram point".into());
    }
    let (mut a, mut found, mut blocks, mut longest, mut bad) = (-1i64, 0u64, 0u64, 1i64, 0u64);
    let mut end: Option<(i64, u64)> = None; // (n, zeros found up to g_n)
    let mut past = 0u64;
    loop {
        // the next Gram block [g_a, g_b]
        let mut b = a + 1;
        loop {
            ensure(b, &mut g, &mut sign, &mut margin)?;
            if good(b, &sign) {
                break;
            }
            b += 1;
        }
        let k = b - a;
        bad += (k - 1) as u64;
        longest = longest.max(k);
        let mut changes = 0u64;
        if k == 1 {
            changes = 1; // good points of opposite parity have opposite signs
        } else {
            for level in 1..=10 {
                let parts = 1i64 << level;
                let mut last = sign[at(a)];
                changes = 0;
                for i in a..b {
                    for r in 1..=parts {
                        let t = if r == parts { g[at(i + 1)] } else { g[at(i)] + (g[at(i + 1)] - g[at(i)]) * r as f64 / parts as f64 };
                        let s = if r == parts { Some(sign[at(i + 1)]) } else { signed(t, &mut margin) };
                        if let Some(s) = s {
                            if s != last {
                                changes += 1;
                                last = s;
                            }
                        }
                    }
                }
                if changes >= k as u64 {
                    break;
                }
            }
            if changes < k as u64 {
                return Err(format!("Rosser's rule could not be confirmed in the Gram block from g_{a} = {:.3} to g_{b} = {:.3}", g[at(a)], g[at(b)]));
            }
        }
        blocks += 1;
        match end {
            None => {
                found += changes;
                if g[at(b)] >= t_max {
                    end = Some((b, found));
                }
            }
            Some(_) => past += 1,
        }
        a = b;
        if let Some((_, _)) = end {
            let needed = 0.0061 * g[at(a)].ln().powi(2) + 0.08 * g[at(a)].ln();
            // twice what Brent's theorem asks for, as a margin
            if past as f64 >= 2.0 * needed {
                let (n, zeros) = end.expect("set");
                if zeros != (n + 1) as u64 {
                    return Err(format!("found {zeros} sign changes up to g_{n}, but Turing's bound allows only {}: a computation error", n + 1));
                }
                // the first zeros, by bisection between sign changes
                let mut first = Vec::new();
                let mut t = 10.0;
                while first.len() < 5 {
                    if z(t).0.signum() != z(t + 0.05).0.signum() {
                        let (mut lo, mut hi) = (t, t + 0.05);
                        for _ in 0..60 {
                            let mid = (lo + hi) / 2.0;
                            if z(mid).0.signum() == z(lo).0.signum() {
                                lo = mid;
                            } else {
                                hi = mid;
                            }
                        }
                        first.push((lo + hi) / 2.0);
                    }
                    t += 0.05;
                }
                return Ok(Verified {
                    height: g[at(n)],
                    index: n,
                    zeros,
                    first,
                    blocks,
                    longest_block: longest,
                    bad_gram_points: bad,
                    turing_blocks: past,
                    turing_needed: needed,
                    margin,
                    evaluations: evaluations.get(),
                });
            }
        }
    }
}

/// Report lines for the attempt at the Riemann hypothesis.
pub fn report(t_max: f64) -> Vec<String> {
    match verify(t_max) {
        Ok(v) => {
            let first: Vec<String> = v.first.iter().map(|z| format!("{z:.6}")).collect();
            vec![
                format!("verified the hypothesis up to height {:.2} (the Gram point g_{}) by Turing's method, the standard way it is checked", v.height, v.index),
                format!("found {} sign changes of Z(t): {} zeros of zeta exactly on the line Re(s) = 1/2 (first: t = {})", v.zeros, v.zeros, first.join(", ")),
                format!("Rosser's rule held in all {} Gram blocks ({} bad Gram points, longest block {} intervals)", v.blocks, v.bad_gram_points, v.longest_block),
                format!("Turing's method (Brent's form) with {} blocks past that point, {:.1} needed, bounds ALL zeros in the strip up to that height by {}", v.turing_blocks, v.turing_needed, v.index + 1),
                format!("so the strip 0 < Re(s) < 1 holds exactly {} zeros up to height {:.2}, and every one is on the line and simple: the hypothesis is TRUE up to that height", v.zeros, v.height),
                format!("certainty: every sign used was fixed with an error bound (Gabcke's bound on the Riemann-Siegel remainder, and Euler-Maclaurin below t = 200 and near zeros) and only when |Z| was at least {SAFETY} times that bound; the smallest |Z| used was {:.1} times its bound; {} values of Z; floating-point rounding is estimated, not interval arithmetic", v.margin, v.evaluations),
                "above that height nothing is checked: the hypothesis is about every height, so this is a verified finite range, not a proof".into(),
            ]
        }
        Err(e) => vec![format!("the check stopped: {e}")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gram_points_and_z() {
        assert!((gram(0, 18.0) - 17.845_599_7).abs() < 1e-6);
        assert!((gram(-1, 10.0) - 9.666_908).abs() < 1e-5);
        // the first zero is at 14.134725...
        assert!(z(14.1347).0.signum() != z(14.1348).0.signum());
        // both formulas agree where they overlap
        let (a, ea) = z_euler_maclaurin(250.0);
        let (b, eb) = z_riemann_siegel(250.0);
        assert!((a - b).abs() <= ea + eb, "{a} {b}");
    }

    #[test]
    fn verifies_up_to_1000() {
        let v = verify(1000.0).expect("verified");
        assert!(v.height >= 1000.0);
        assert_eq!(v.zeros, (v.index + 1) as u64);
        assert!((v.first[0] - 14.134_725).abs() < 1e-5);
        assert!((v.first[1] - 21.022_040).abs() < 1e-5);
    }
}
