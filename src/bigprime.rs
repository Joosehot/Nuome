//! The first prime with D digits, found: 10^(D-1) + k for the smallest k
//! that passes. Candidates are sieved by every prime below `sieve_to`
//! without ever building the giant number (10^(D-1) mod p is a small
//! modular power), so the sieve works for any D, 100 million digits too.
//! The survivors are then tested in order with Miller-Rabin on the full
//! number (num-bigint), as far as that can be computed here; the first to
//! pass is the answer. Miller-Rabin with the first 12 prime bases decides
//! exactly below 3.3 * 10^24; above that a pass means "probable prime"
//! (no counterexample known for these bases), and the report says so.

use num_bigint::BigUint;
use num_traits::{One, Zero};

/// The smallest primes up to `limit`.
fn small_primes(limit: u64) -> Vec<u64> {
    let n = limit as usize;
    let mut comp = vec![false; n + 1];
    let mut out = Vec::new();
    for i in 2..=n {
        if !comp[i] {
            out.push(i as u64);
            let mut j = i * i;
            while j <= n {
                comp[j] = true;
                j += i;
            }
        }
    }
    out
}

fn pow_mod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1u128;
    let mut bb = (b % m) as u128;
    let mm = m as u128;
    while e > 0 {
        if e & 1 == 1 {
            r = r * bb % mm;
        }
        bb = bb * bb % mm;
        e >>= 1;
    }
    b = r as u64;
    b
}

/// What the sieve says about 10^(D-1) + k for k in 0..window: the smallest
/// prime factor below the sieve limit, or None if none divides it.
pub fn sieve(digits: u64, window: u64, sieve_to: u64) -> Vec<Option<u64>> {
    let mut out: Vec<Option<u64>> = vec![None; window as usize];
    for p in small_primes(sieve_to) {
        // 10^(D-1) mod p, then the first k with (x + k) = 0 mod p
        let r = pow_mod(10, digits - 1, p);
        let mut k = (p - r) % p;
        while k < window {
            // x + k equal to p itself would be prime, not divisible (only for tiny D)
            let tiny = digits <= 19 && 10u64.pow((digits - 1) as u32) + k == p;
            if out[k as usize].is_none() && !tiny {
                out[k as usize] = Some(p);
            }
            k += p;
        }
    }
    out
}

/// Miller-Rabin on n with the first 12 prime bases.
pub fn miller_rabin(n: &BigUint) -> bool {
    let two = BigUint::from(2u32);
    if *n < two {
        return false;
    }
    let one = BigUint::one();
    let n1 = n - &one;
    let mut d = n1.clone();
    let mut s = 0u32;
    while (&d % &two).is_zero() {
        d >>= 1;
        s += 1;
    }
    'base: for a in [2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let a = BigUint::from(a);
        if a >= *n {
            continue;
        }
        let mut x = a.modpow(&d, n);
        if x == one || x == n1 {
            continue;
        }
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == n1 {
                continue 'base;
            }
        }
        return false;
    }
    true
}

pub struct Found {
    pub k: u64,
    pub number: Option<BigUint>,
    /// candidates the sieve removed, and those Miller-Rabin removed, before it
    pub sieved_out: u64,
    pub tested_out: u64,
    pub exact: bool,
}

/// The first prime with `digits` digits, if Miller-Rabin can be run at that
/// size (digits <= `test_up_to`); otherwise the first sieve survivor.
pub fn first(digits: u64, sieve_to: u64, test_up_to: u64) -> Found {
    let ln_x = (digits - 1) as f64 * std::f64::consts::LN_10;
    // a candidate alone needs only a short window (a few in a hundred survive the sieve)
    let mut window = if digits > test_up_to { 10_000 } else { ((ln_x * 20.0) as u64).max(200) };
    loop {
        let marks = sieve(digits, window, sieve_to);
        let sieved_out = |upto: u64| marks[..upto as usize].iter().filter(|m| m.is_some()).count() as u64;
        if digits > test_up_to {
            if let Some(k) = marks.iter().position(|m| m.is_none()) {
                return Found { k: k as u64, number: None, sieved_out: sieved_out(k as u64), tested_out: 0, exact: false };
            }
        } else {
            let x = BigUint::from(10u32).pow((digits - 1) as u32);
            let mut tested_out = 0;
            for (k, m) in marks.iter().enumerate() {
                if m.is_some() {
                    continue;
                }
                let n = &x + BigUint::from(k as u64);
                if miller_rabin(&n) {
                    return Found { k: k as u64, sieved_out: sieved_out(k as u64), tested_out, exact: digits <= 24, number: Some(n) };
                }
                tested_out += 1;
            }
        }
        window *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_primes_with_few_digits() {
        // 11, 101, 1009, 10007, 1000000007
        for (d, k) in [(2, 1), (3, 1), (4, 9), (5, 7), (10, 7)] {
            let f = first(d, 1000, 5000);
            assert_eq!(f.k, k, "{d} digits");
        }
    }

    #[test]
    fn the_sieve_needs_no_giant_number() {
        // 10^99,999,999 + k: the sieve runs at 100 million digits
        let marks = sieve(100_000_000, 1000, 10_000);
        // k = 0 is even, divisible by 2
        assert_eq!(marks[0], Some(2));
        assert!(marks.iter().any(|m| m.is_none()));
    }
}

/// The smallest prime factor p of 10^(D-1) + k with from <= p < to, if any:
/// a segmented sieve of the primes in [from, to) on every core, each prime
/// tried by 10^(D-1) mod p (a small modular power: the number is never built).
pub fn deep_factor(digits: u64, k: u64, from: u64, to: u64) -> Option<u64> {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    let root = (to as f64).sqrt() as u64 + 1;
    let base = small_primes(root);
    const SEG: u64 = 1 << 22;
    let segments = (to - from).div_ceil(SEG) as usize;
    let next = AtomicUsize::new(0);
    let best = AtomicU64::new(u64::MAX);
    std::thread::scope(|sc| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |t| t.get()) {
            sc.spawn(|| loop {
                let s = next.fetch_add(1, Ordering::Relaxed);
                if s >= segments {
                    break;
                }
                let lo = from + s as u64 * SEG;
                let hi = (lo + SEG).min(to);
                // a smaller factor is already known: nothing here can beat it
                if lo >= best.load(Ordering::Relaxed) {
                    continue;
                }
                let mut comp = vec![false; (hi - lo) as usize];
                for &p in &base {
                    if p * p >= hi {
                        break;
                    }
                    let mut m = (lo.div_ceil(p) * p).max(p * p);
                    while m < hi {
                        comp[(m - lo) as usize] = true;
                        m += p;
                    }
                }
                for (i, c) in comp.iter().enumerate() {
                    let p = lo + i as u64;
                    if *c || p < 2 {
                        continue;
                    }
                    if (pow_mod(10, digits - 1, p) as u128 + k as u128) % p as u128 == 0 {
                        best.fetch_min(p, Ordering::Relaxed);
                        break;
                    }
                }
            });
        }
    });
    let b = best.load(Ordering::Relaxed);
    (b != u64::MAX).then_some(b)
}

#[cfg(test)]
mod deep_tests {
    use super::*;

    #[test]
    fn finds_a_known_factor() {
        // 10^9 + 9 = 1000000009 is prime, 10^9 + 1 = 7 * 11 * 13 * 19 * 52579
        assert_eq!(deep_factor(10, 1, 20, 100_000), Some(52579));
        assert_eq!(deep_factor(10, 9, 2, 40_000), None);
    }
}
