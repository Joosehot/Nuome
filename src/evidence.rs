//! Attempts at open problems that can be computed on, never proved: every
//! case up to a limit (from rules.toml) is worked out and the result is
//! reported as evidence for those cases only. A finite check can refute a
//! conjecture (a counterexample) but never prove it for all numbers.

/// Every even number 4..=limit as a sum of two primes. Returns report lines.
pub fn goldbach(limit: u64) -> Vec<String> {
    let n = limit as usize;
    let mut composite = vec![false; n + 1];
    let mut primes = Vec::new();
    for i in 2..=n {
        if !composite[i] {
            primes.push(i);
            let mut j = i * i;
            while j <= n {
                composite[j] = true;
                j += i;
            }
        }
    }
    let is_prime = |k: usize| k >= 2 && !composite[k];
    // the hardest case: the even number whose smallest usable prime is largest
    let (mut worst_n, mut worst_p) = (4, 2);
    let mut e = 4;
    while e <= n {
        match primes.iter().find(|&&p| is_prime(e - p)) {
            Some(&p) => {
                if p > worst_p {
                    (worst_n, worst_p) = (e, p);
                }
            }
            None => return vec![format!("counterexample: {e} is not a sum of two primes, so the conjecture is false")],
        }
        e += 2;
    }
    vec![
        format!("checked every even number from 4 to {limit}: each is a sum of two primes ({} primes used)", primes.len()),
        format!("hardest case in that range: {worst_n} = {worst_p} + {}, where no smaller prime works", worst_n - worst_p),
        "this covers those numbers only; it is evidence, not a proof for all even numbers".into(),
    ]
}

/// Every starting number 1..=limit run until it reaches 1. Returns report lines.
pub fn collatz(limit: u64) -> Vec<String> {
    let n = limit as usize;
    let mut steps = vec![0u32; n + 1];
    let (mut longest_n, mut longest, mut peak_n, mut peak) = (1usize, 0u32, 1usize, 1u128);
    for start in 2..=n {
        let mut x = start as u128;
        let mut count = 0u32;
        let mut top = x;
        // run until the orbit drops to a number already known to reach 1
        while x >= start as u128 {
            x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
            count += 1;
            top = top.max(x);
            if count > 100_000 {
                return vec![format!("{start} did not reach a smaller number within 100000 steps: a possible counterexample, needs a closer look")];
            }
        }
        steps[start] = count + steps[x as usize];
        if steps[start] > longest {
            (longest_n, longest) = (start, steps[start]);
        }
        if top > peak {
            (peak_n, peak) = (start, top);
        }
    }
    vec![
        format!("checked every starting number from 1 to {limit}: each reaches 1"),
        format!("longest run in that range: {longest_n} takes {longest} steps; highest point: {peak_n} climbs to {peak}"),
        "this covers those numbers only; it is evidence, not a proof for all numbers".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_ranges() {
        assert!(goldbach(1000)[0].starts_with("checked every even number from 4 to 1000"));
        assert!(collatz(30)[1].contains("27 takes 111 steps"));
        // 29 zeros of zeta with 0 < t <= 100 (the known count)
        assert!(riemann(100.0)[0].contains("found 29 sign changes"));
    }
}

/// The Riemann-Siegel theta function.
fn theta(t: f64) -> f64 {
    t / 2.0 * (t / (2.0 * std::f64::consts::PI)).ln() - t / 2.0 - std::f64::consts::PI / 8.0 + 1.0 / (48.0 * t) + 7.0 / (5760.0 * t.powi(3))
}

/// Hardy's Z function by the Riemann-Siegel formula (main sum plus the
/// first correction term): real on the critical line, |Z(t)| = |zeta(1/2 + it)|,
/// so every sign change of Z is a zero of zeta exactly on the line.
pub fn hardy_z(t: f64) -> f64 {
    let a = (t / (2.0 * std::f64::consts::PI)).sqrt();
    let n = a.floor() as usize;
    let th = theta(t);
    let mut sum = 0.0;
    for k in 1..=n {
        sum += (th - t * (k as f64).ln()).cos() / (k as f64).sqrt();
    }
    let p = a - n as f64;
    let c0 = (2.0 * std::f64::consts::PI * (p * p - p - 1.0 / 16.0)).cos() / (2.0 * std::f64::consts::PI * p).cos();
    let sign = if n % 2 == 1 { 1.0 } else { -1.0 };
    2.0 * sum + sign * (t / (2.0 * std::f64::consts::PI)).powf(-0.25) * c0
}

/// Zeros of zeta on the critical line up to height `t_max`, found as sign
/// changes of Z, against the Riemann-von Mangoldt count of all zeros in the
/// strip up to that height.
pub fn riemann(t_max: f64) -> Vec<String> {
    let step = 0.01;
    let mut zeros = Vec::new();
    let mut t = 10.0;
    let mut prev = hardy_z(t);
    while t < t_max {
        let next = hardy_z(t + step);
        if prev.signum() != next.signum() {
            // refine by bisection
            let (mut lo, mut hi) = (t, t + step);
            for _ in 0..50 {
                let mid = (lo + hi) / 2.0;
                if hardy_z(mid).signum() == hardy_z(lo).signum() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            zeros.push((lo + hi) / 2.0);
        }
        prev = next;
        t += step;
    }
    // Riemann-von Mangoldt: N(T) = theta(T)/pi + 1 + S(T), with S(T) small
    let expected = theta(t_max) / std::f64::consts::PI + 1.0;
    // the formula here keeps one correction term: near t = 14 it is good to about 0.01
    let first: Vec<String> = zeros.iter().take(5).map(|z| format!("{z:.2}")).collect();
    vec![
        format!("computed Z(t) (the Riemann-Siegel formula) for 10 <= t <= {t_max} and found {} sign changes: {} zeros of zeta exactly on the line Re(s) = 1/2", zeros.len(), zeros.len()),
        format!("first ones: t = {} (to about 0.01; the formula keeps one correction term)", first.join(", ")),
        format!("the Riemann-von Mangoldt formula puts about {expected:.1} zeros in the whole strip 0 < Re(s) < 1 up to that height, so none appear to be off the line there"),
        "making that last step rigorous needs Turing's method and careful error bounds, and it says nothing above that height: evidence, not a proof".into(),
    ]
}
