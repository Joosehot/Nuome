//! Check Goldbach on a window of even numbers anywhere below 3.3 * 10^24,
//! beyond the 64-bit range. For each even n it finds the smallest prime p
//! with n - p prime.
//!
//!   goldbach_window <start> <count> [threads]
//!   goldbach_window 1e20 1000000
//!   goldbach_window 2795935116574469638 1
//!
//! Candidates n - p are first sieved by the primes below SIEVE (fast); the
//! survivors are tested with Miller-Rabin on the first 13 primes as bases,
//! which is deterministic below 3.3 * 10^24 (Sorenson and Webster, 2015).
//! A window is split into chunks that run in parallel; the result is the
//! same whatever the thread count.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

const SIEVE: u64 = 200_000;
const PMAX: u64 = 30_000;
const CHUNK: u64 = 1 << 18;
const BASES: [u128; 13] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41];

fn primes_below(n: u64) -> Vec<u64> {
    let mut composite = vec![false; n as usize + 1];
    let mut out = Vec::new();
    for i in 2..=n as usize {
        if !composite[i] {
            out.push(i as u64);
            let mut j = i * i;
            while j <= n as usize {
                composite[j] = true;
                j += i;
            }
        }
    }
    out
}

/// a * b mod m for m < 2^100: b is taken 27 bits at a time from the top,
/// so no intermediate product passes 128 bits.
fn mul_mod(a: u128, b: u128, m: u128) -> u128 {
    const K: u32 = 27;
    let a = a % m;
    let mut r = 0u128;
    let mut shift = 4 * K; // 108 bits covers every b below 2^100
    while shift > 0 {
        shift -= K;
        let digit = (b >> shift) & ((1u128 << K) - 1);
        r = (r << K) % m;
        r = (r + (a * digit) % m) % m;
    }
    r
}

fn pow_mod(mut b: u128, mut e: u128, m: u128) -> u128 {
    let mut r = 1u128;
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

/// Deterministic for n < 3.3 * 10^24 with the first 13 primes as bases.
fn is_prime(n: u128) -> bool {
    if n < 2 {
        return false;
    }
    for &p in &BASES {
        if n % p == 0 {
            return n == p;
        }
    }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'bases: for &a in &BASES {
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

struct Found {
    worst_n: u128,
    worst_p: u64,
    counterexample: Option<u128>,
}

/// Even n in [lo, hi]: the largest "smallest prime" and where it occurs.
fn chunk(lo: u128, hi: u128, small: &[u64], sieve_primes: &[u64]) -> Found {
    // sieve the odd numbers in [lo - PMAX, hi] by the primes below SIEVE
    let start = (lo - PMAX as u128) | 1;
    let len = ((hi - start) / 2 + 1) as usize;
    let mut maybe = vec![true; len];
    for &q in sieve_primes.iter().skip(1) {
        let q = q as u128;
        let mut m = ((start + q - 1) / q) * q;
        if m % 2 == 0 {
            m += q;
        }
        while m <= hi {
            maybe[((m - start) / 2) as usize] = false;
            m += 2 * q;
        }
    }
    let mut f = Found { worst_n: 0, worst_p: 0, counterexample: None };
    let mut n = lo + (lo % 2);
    while n <= hi {
        let mut got = None;
        for &p in small.iter().skip(1) {
            let q = n - p as u128;
            if maybe[((q - start) / 2) as usize] && is_prime(q) {
                got = Some(p);
                break;
            }
        }
        let p = match got {
            Some(p) => p,
            None => {
                // past the sieved window: test both halves directly
                let mut p = PMAX + 1;
                loop {
                    if p as u128 > n / 2 {
                        f.counterexample = Some(n);
                        return f;
                    }
                    if is_prime(p as u128) && is_prime(n - p as u128) {
                        break p;
                    }
                    p += 2;
                }
            }
        };
        if p > f.worst_p {
            f.worst_p = p;
            f.worst_n = n;
        }
        n += 2;
    }
    f
}

fn parse_big(s: &str) -> Option<u128> {
    // "10^20", "1e20", "3e19", or plain digits
    let s = s.replace('_', "");
    if let Some((b, e)) = s.split_once('^') {
        let (b, e): (u128, u32) = (b.parse().ok()?, e.parse().ok()?);
        return b.checked_pow(e);
    }
    if let Some((b, e)) = s.split_once('e') {
        let (b, e): (u128, u32) = (b.parse().ok()?, e.parse().ok()?);
        return b.checked_mul(10u128.checked_pow(e)?);
    }
    s.parse().ok()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: goldbach_window <start> <count> [threads]\n  e.g. goldbach_window 1e20 1000000");
        std::process::exit(2);
    }
    let start = parse_big(&args[1]).expect("start: a number like 1e20, 10^20 or 2795935116574469638");
    let count = parse_big(&args[2]).expect("count: how many even numbers") as u64;
    let threads = args.get(3).and_then(|t| t.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    if start > 3_300_000_000_000_000_000_000_000 || start < 1_000_000 {
        eprintln!("start must be between 10^6 and 3.3 * 10^24 (where the primality test is deterministic)");
        std::process::exit(2);
    }
    let lo = start + (start % 2);
    let hi = lo + 2 * (count.max(1) as u128 - 1);
    let small = primes_below(PMAX);
    let sieve_primes = primes_below(SIEVE);
    let t0 = Instant::now();
    let chunks = (count.max(1)).div_ceil(CHUNK);
    let next = AtomicU64::new(0);
    let parts: Mutex<Vec<(u64, Found)>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let c = next.fetch_add(1, Ordering::Relaxed);
                if c >= chunks {
                    break;
                }
                let a = lo + 2 * (c * CHUNK) as u128;
                let b = (a + 2 * (CHUNK as u128 - 1)).min(hi);
                let f = chunk(a, b, &small, &sieve_primes);
                parts.lock().expect("no panics while holding it").push((c, f));
            });
        }
    });
    let secs = t0.elapsed().as_secs_f64();
    let mut parts = parts.into_inner().expect("threads done");
    parts.sort_by_key(|p| p.0);
    if let Some(bad) = parts.iter().find_map(|(_, f)| f.counterexample) {
        println!("COUNTEREXAMPLE: {bad} is not a sum of two primes");
        return;
    }
    let (worst_n, worst_p) = parts.iter().map(|(_, f)| (f.worst_n, f.worst_p)).max_by_key(|&(n, p)| (p, std::cmp::Reverse(n))).expect("one chunk");
    let rate = count as f64 / secs;
    println!("window: {count} even numbers from {lo} to {hi}");
    println!("result: every one is a sum of two primes (checked in {secs:.1} s, {rate:.0} numbers/s, {threads} threads)");
    println!("hardest in the window: {worst_n} = {worst_p} + {}", worst_n - worst_p as u128);
    let full = 5e20 - 5e17; // even numbers from 10^18 to 10^21
    let years = full / rate / (365.25 * 86400.0);
    println!("at this rate, all even numbers from 10^18 to 10^21 would take {years:.3e} years on this machine");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primality_beyond_64_bits() {
        assert!(is_prime(18_446_744_073_709_551_557)); // the largest prime below 2^64
        assert!(is_prime((1u128 << 89) - 1)); // the Mersenne prime 2^89 - 1
        assert!(!is_prime(((1u128 << 61) - 1) * ((1u128 << 31) - 1))); // two Mersenne primes multiplied
        let m = (1u128 << 89) - 1;
        assert_eq!(mul_mod(m - 1, m - 1, m), 1);
    }

    #[test]
    fn finds_a_known_record() {
        // 503222 = 523 + 502699 is the hardest case below a million
        let (small, sieve) = (primes_below(PMAX), primes_below(SIEVE));
        let f = chunk(503_000, 503_400, &small, &sieve);
        assert_eq!((f.worst_n, f.worst_p), (503_222, 523));
    }
}
