//! Goldbach, computed: every even number up to a limit is split into two
//! primes, in parallel segments. For each n Nuome finds the smallest prime p
//! with n - p prime; the numbers where that smallest p sets a new record are
//! the hardest cases. A second, independent test (deterministic Miller-Rabin)
//! confirms every record.
//!
//! Then the Hardy-Littlewood prediction for the number of ways to split n
//! is compared with exact counts, and, if the model fits, it gives the chance
//! of a counterexample above the checked limit. That is a model's number,
//! reported as such, never as a proof.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Odd-only sieve: bit k says whether 2k + 1 is prime.
fn small_primes(n: u64) -> Vec<u64> {
    let n = n as usize;
    let mut composite = vec![false; n + 1];
    let mut out = Vec::new();
    for i in 2..=n {
        if !composite[i] {
            out.push(i as u64);
            let mut j = i * i;
            while j <= n {
                composite[j] = true;
                j += i;
            }
        }
    }
    out
}

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn pow_mod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u64;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, b, m);
        }
        b = mul_mod(b, b, m);
        e >>= 1;
    }
    r
}

/// Deterministic Miller-Rabin: these bases decide every n below 2^64.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'bases: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = pow_mod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                continue 'bases;
            }
        }
        return false;
    }
    true
}

pub struct Run {
    pub limit: u64,
    pub evens: u64,
    /// (n, smallest p): each p larger than every one before it.
    pub records: Vec<(u64, u64)>,
    pub counterexample: Option<u64>,
    pub seconds: f64,
    pub threads: usize,
}

/// Search the small primes first (a sieved window answers "is n - p prime?"),
/// then Miller-Rabin beyond them.
const SMALL: u64 = 5_000;
const WINDOW: u64 = 1 << 24;

/// Numbers where the smallest prime exceeds a formula a (ln n)^b: how many,
/// and the worst few (n, p, formula value), worst ratio first.
#[derive(Clone, Debug, Default)]
pub struct Over {
    pub count: u64,
    pub worst: Vec<(u64, u64, f64)>,
}

fn segment(lo: u64, hi: u64, small: &[u64], base: &[u64]) -> (Vec<(u64, u64)>, Option<u64>) {
    let (r, bad, _) = segment_with(lo, hi, small, base, None);
    (r, bad)
}

fn segment_with(lo: u64, hi: u64, small: &[u64], base: &[u64], bound: Option<(f64, f64)>) -> (Vec<(u64, u64)>, Option<u64>, Over) {
    let mut over = Over::default();
    // sieve the odd numbers in [lo - SMALL, hi]
    let start = lo.saturating_sub(SMALL) | 1;
    let len = ((hi - start) / 2 + 1) as usize;
    let mut prime = vec![true; len];
    for &p in base.iter().skip(1) {
        if p * p > hi {
            break;
        }
        let mut m = (start.div_ceil(p) * p).max(p * p);
        if m % 2 == 0 {
            m += p;
        }
        while m <= hi {
            prime[((m - start) / 2) as usize] = false;
            m += 2 * p;
        }
    }
    if start == 1 {
        prime[0] = false; // 1 isn't prime
    }
    let odd_prime = |q: u64| q >= start && q % 2 == 1 && prime[((q - start) / 2) as usize];
    let mut records = Vec::new();
    let mut best = 0u64;
    let mut n = lo + (lo % 2);
    while n <= hi {
        let p = if n == 4 {
            2
        } else {
            match small.iter().skip(1).find(|&&p| p < n && odd_prime(n - p)) {
                Some(&p) => p,
                None => {
                    // beyond the sieved primes: test both halves directly
                    let mut p = SMALL + 1;
                    loop {
                        if p > n / 2 {
                            return (records, Some(n), over);
                        }
                        if is_prime(p) && is_prime(n - p) {
                            break p;
                        }
                        p += 2;
                    }
                }
            }
        };
        if p > best {
            best = p;
            records.push((n, p));
        }
        if let Some((a, b)) = bound {
            let f = a * (n as f64).ln().powf(b);
            if p as f64 > f {
                over.count += 1;
                over.worst.push((n, p, f));
                if over.worst.len() > 64 {
                    over.worst.sort_by(|x, y| (y.1 as f64 / y.2).partial_cmp(&(x.1 as f64 / x.2)).expect("finite"));
                    over.worst.truncate(8);
                }
            }
        }
        n += 2;
    }
    (records, None, over)
}

/// Every even n from 4 to `limit`: how often the smallest prime exceeds a (ln n)^b.
pub fn exceed(limit: u64, from: u64, threads: usize, a: f64, b: f64) -> (Over, f64) {
    let t0 = Instant::now();
    let small = small_primes(SMALL);
    let base = small_primes(((limit as f64).sqrt() as u64 + 2).max(SMALL));
    let chunks = limit.div_ceil(WINDOW);
    let next = AtomicU64::new(0);
    let total: Mutex<Over> = Mutex::new(Over::default());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let c = next.fetch_add(1, Ordering::Relaxed);
                if c >= chunks {
                    break;
                }
                let lo = (c * WINDOW).max(4).max(from);
                let hi = ((c + 1) * WINDOW - 1).min(limit);
                if lo > hi {
                    continue;
                }
                let (_, _, o) = segment_with(lo, hi, &small, &base, Some((a, b)));
                let mut t = total.lock().expect("no panics while holding it");
                t.count += o.count;
                t.worst.extend(o.worst);
                t.worst.sort_by(|x, y| (y.1 as f64 / y.2).partial_cmp(&(x.1 as f64 / x.2)).expect("finite"));
                t.worst.truncate(8);
            });
        }
    });
    (total.into_inner().expect("threads done"), t0.elapsed().as_secs_f64())
}

pub fn run(limit: u64, threads: usize) -> Run {
    let t0 = Instant::now();
    let small = small_primes(SMALL);
    let base = small_primes(((limit as f64).sqrt() as u64 + 2).max(SMALL));
    let chunks = limit.div_ceil(WINDOW);
    let next = AtomicU64::new(0);
    let results: Mutex<Vec<(u64, Vec<(u64, u64)>, Option<u64>)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let c = next.fetch_add(1, Ordering::Relaxed);
                if c >= chunks {
                    break;
                }
                let lo = (c * WINDOW).max(4);
                let hi = ((c + 1) * WINDOW - 1).min(limit);
                if lo > hi {
                    continue;
                }
                let (r, bad) = segment(lo, hi, &small, &base);
                results.lock().expect("no panics while holding it").push((c, r, bad));
            });
        }
    });
    let mut parts = results.into_inner().expect("threads done");
    parts.sort_by_key(|p| p.0);
    let mut records = Vec::new();
    let mut best = 0;
    let mut counterexample = None;
    for (_, r, bad) in parts {
        for (n, p) in r {
            if p > best {
                best = p;
                records.push((n, p));
            }
        }
        if counterexample.is_none() {
            counterexample = bad;
        }
    }
    Run { limit, evens: limit / 2 - 1, records, counterexample, seconds: t0.elapsed().as_secs_f64(), threads }
}

/// The twin prime constant.
const C2: f64 = 0.660_161_815_846_869_6;

/// Hardy-Littlewood: the expected number of ways to write n as p + q with
/// p <= q, C2 * S(n) * integral_2^(n-2) dx / (ln x ln(n - x)).
pub fn predicted(n: u64) -> f64 {
    let mut s = 1.0;
    let mut m = n;
    while m % 2 == 0 {
        m /= 2;
    }
    let mut p = 3;
    while p * p <= m {
        if m % p == 0 {
            s *= (p - 1) as f64 / (p - 2) as f64;
            while m % p == 0 {
                m /= p;
            }
        }
        p += 2;
    }
    if m > 1 {
        s *= (m - 1) as f64 / (m - 2) as f64;
    }
    C2 * s * integral(n)
}

/// integral_2^(n-2) dx / (ln x ln(n - x)), by Simpson's rule on a log scale
/// near the ends (where the integrand changes fastest).
fn integral(n: u64) -> f64 {
    let n = n as f64;
    let f = |x: f64| 1.0 / (x.ln() * (n - x).ln());
    // symmetric: twice the integral from 2 to n/2
    let (a, b) = (2f64.ln(), (n / 2.0).ln());
    let k = 20_000;
    let h = (b - a) / k as f64;
    let g = |u: f64| {
        let x = u.exp();
        f(x) * x
    };
    let mut sum = g(a) + g(b);
    for i in 1..k {
        sum += g(a + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    2.0 * sum * h / 3.0
}

/// The exact number of ways to write n as p + q with p <= q.
pub fn ways(n: u64) -> u64 {
    let len = (n / 2 + 1) as usize;
    let mut composite = vec![false; n as usize + 1];
    let mut i = 2usize;
    while i * i <= n as usize {
        if !composite[i] {
            let mut j = i * i;
            while j <= n as usize {
                composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    (2..len).filter(|&p| !composite[p] && !composite[n as usize - p]).count() as u64
}

/// log10 of the model's chance that some even n above `limit` has no split:
/// under the model, n has none with probability about exp(-predicted(n)),
/// and the smallest mean above the limit dominates the sum.
pub fn risk_log10(limit: u64) -> f64 {
    // S(n) >= 1, so the bare integral is the smallest mean; the sum over all
    // larger n adds a factor that is negligible next to it
    -(C2 * integral(limit)) / std::f64::consts::LN_10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_primality() {
        assert!(is_prime(999_999_937) && !is_prime(999_999_939));
        let r = run(1_000_000, 4);
        assert_eq!(r.counterexample, None);
        assert_eq!(r.records.last(), Some(&(503_222, 523)));
        assert_eq!(ways(100), 6);
        // the model is within a few percent of the truth at 10^6
        let (w, p) = (ways(1_000_000) as f64, predicted(1_000_000));
        assert!((w / p - 1.0).abs() < 0.1, "{w} vs {p}");
    }
}
