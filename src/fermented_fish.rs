//! Operation Fermented Fish: a constant whose chain of primes goes on, and
//! whose description is short. Joose's thought: the constant is not a known
//! number; it is found in the primes themselves. The question, exactly:
//!
//!   is there a constant c built from the primes by a short rule (their
//!   digits in a row, their continued fraction, a sum over them), or a
//!   simple transform of one, such that floor(c^(r^n)) (or floor(c r^n)) is
//!   prime for every n = 1, 2, ...?
//!
//! What is forced: a chain of this kind for r = 3 exists (Mills 1947), its
//! constant written from the primes themselves; the proof needs a prime
//! between x^3 and (x+1)^3 for large x (Ingham 1937). For r = 2 the same
//! argument needs a prime between x^2 and (x+1)^2, Legendre's conjecture,
//! which is open: so a chain floor(c^(2^n)) going on for ever is not known
//! to exist for any c. What the theorems do not say is whether some c
//! with a short description happens to be in Mills' set.
//!
//! The choice left, searched exactly: the constants built from the primes
//! (2.357111317... their digits in a row, Copeland-Erdos; [2; 3, 5, 7, ...]
//! their continued fraction; the sums over primes of 2^-p, 10^-p, 1/p^2,
//! 1/(p 2^p)) and their simple transforms c - 1, 2c, c / 2, c^2, sqrt c,
//! 1 + 1/c (kept above 1). Every one is written to D digits (fixed point), and the chain
//! x_(n+1) = x_n^r is followed with D digits kept (the error grows by r a
//! step, far below what is needed), each value tested by Miller-Rabin
//! until it fails or passes `max_digits` digits.
//!
//! The measure: chance, as in prime_instant. Among N candidates the
//! expected number of chains of k primes by luck is N times the product of
//! 1/ln p_i. A chain the search did not expect, from a constant whose
//! description is a few symbols, would be the catch; a chain within luck is
//! fish gone off.

use crate::bigprime::miller_rabin;
use num_bigint::BigUint;
use num_traits::{CheckedSub, One, Zero};

/// Digits of fixed point kept.
pub const D: u32 = 2500;

fn pow10(k: u32) -> BigUint {
    BigUint::from(10u32).pow(k)
}

/// floor(x^(1/k)) by Newton.
fn iroot(x: &BigUint, k: u32) -> BigUint {
    if x.is_zero() {
        return BigUint::zero();
    }
    let bits = x.bits();
    let mut r: BigUint = BigUint::one() << (bits / k as u64 + 1);
    let km1 = BigUint::from(k - 1);
    let kk = BigUint::from(k);
    loop {
        // r' = ((k-1) r + x / r^(k-1)) / k
        let next = (&km1 * &r + x / r.pow(k - 1)) / &kk;
        if next >= r {
            return r;
        }
        r = next;
    }
}

/// A named constant to D digits: the integer floor(c 10^D).
fn named(name: &str) -> Option<BigUint> {
    let scale = pow10(D);
    let sqrt = |k: u32| iroot(&(BigUint::from(k) * pow10(2 * D)), 2);
    Some(match name {
        "sqrt2" => sqrt(2),
        "sqrt3" => sqrt(3),
        "sqrt5" => sqrt(5),
        "sqrt7" => sqrt(7),
        "cbrt2" => iroot(&(BigUint::from(2u32) * pow10(3 * D)), 3),
        "phi" => (sqrt(5) + &scale) / BigUint::from(2u32),
        "e" => {
            // sum 1/k!
            let mut sum = BigUint::zero();
            let mut term = scale.clone();
            let mut k = 1u32;
            while !term.is_zero() {
                sum += &term;
                term /= BigUint::from(k);
                k += 1;
            }
            sum
        }
        "pi" => {
            // Machin: pi = 16 arctan(1/5) - 4 arctan(1/239), in fixed point with guard digits
            let guard = pow10(D + 10);
            let arctan_inv = |x: u32| -> BigUint {
                let x2 = BigUint::from(x * x);
                let mut term = &guard / BigUint::from(x);
                let (mut sum, mut k, mut plus) = (BigUint::zero(), 1u32, true);
                while !term.is_zero() {
                    let t = &term / BigUint::from(k);
                    if plus { sum += &t } else { sum -= &t }
                    term /= &x2;
                    k += 2;
                    plus = !plus;
                }
                sum
            };
            (BigUint::from(16u32) * arctan_inv(5) - BigUint::from(4u32) * arctan_inv(239)) / pow10(10)
        }
        "ln2" => {
            // sum 1/(k 2^k)
            let guard = pow10(D + 10);
            let mut term = &guard / BigUint::from(2u32);
            let (mut sum, mut k) = (BigUint::zero(), 1u32);
            while !term.is_zero() {
                sum += &term / BigUint::from(k);
                term /= BigUint::from(2u32);
                k += 1;
            }
            sum / pow10(10)
        }
        _ => return None,
    })
}

/// A simple transform of a constant (fixed point in, fixed point out), kept in (1, 100).
fn transform(name: &str, c: &BigUint) -> Option<BigUint> {
    let scale = pow10(D);
    let v = match name {
        "" => c.clone(),
        "-1" => c.checked_sub(&scale)?,
        "x2" => c * BigUint::from(2u32),
        "/2" => c / BigUint::from(2u32),
        "^2" => c * c / &scale,
        "sqrt" => iroot(&(c * &scale), 2),
        "1+1/" => &scale + &scale * &scale / c,
        _ => return None,
    };
    (v > &scale + BigUint::from(10u32).pow(D - 2) && v < &scale * BigUint::from(100u32)).then_some(v)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Mills(u32),
    Geometric(u32),
}

/// The chain of primes from c (fixed point) in `shape`, until a value fails or passes max_digits digits.
fn chain(c: &BigUint, shape: &Shape, max_digits: usize) -> Vec<BigUint> {
    let scale = pow10(D);
    let mut out = Vec::new();
    match *shape {
        Shape::Mills(r) => {
            // x = c^(r^n) in fixed point: x_1 = c^r, x_(n+1) = x_n^r / scale^(r-1)
            let mut x = c.pow(r) / scale.pow(r - 1);
            loop {
                let value = &x / &scale;
                if value.to_string().len() > max_digits {
                    break;
                }
                if value < BigUint::from(2u32) || !miller_rabin(&value) {
                    break;
                }
                out.push(value);
                x = x.pow(r) / scale.pow(r - 1);
            }
        }
        Shape::Geometric(r) => {
            let mut x = c * BigUint::from(r);
            for _ in 0..40 {
                let value = &x / &scale;
                if value < BigUint::from(2u32) || !miller_rabin(&value) {
                    break;
                }
                out.push(value);
                x *= BigUint::from(r);
            }
        }
    }
    out
}

fn bits(x: &BigUint) -> f64 {
    x.bits() as f64
}

pub struct Found {
    pub name: String,
    pub shape: Shape,
    pub primes: Vec<BigUint>,
}

impl Found {
    pub fn chance(&self) -> f64 {
        self.primes.iter().map(|p| { let b = bits(p); if b <= 2.0 { 1.0 } else { 1.0 / (b * std::f64::consts::LN_2) } }).product()
    }
}

/// The primes below `limit`.
fn primes_below(limit: usize) -> Vec<u32> {
    let mut sieve = vec![true; limit];
    let mut out = Vec::new();
    for i in 2..limit {
        if sieve[i] {
            out.push(i as u32);
            let mut j = i * i;
            while j < limit {
                sieve[j] = false;
                j += i;
            }
        }
    }
    out
}

/// A constant built from the primes by a short rule, to D digits (fixed point, times 10 or 100 where needed to stand above 1).
fn from_primes(name: &str) -> Option<BigUint> {
    let scale = pow10(D);
    let ps = primes_below(20_000);
    Some(match name {
        // 2.357111317192329... : the primes' digits in a row (Copeland-Erdos, times 10)
        "digits" => {
            let mut s = String::new();
            for p in &ps {
                s.push_str(&p.to_string());
                if s.len() > D as usize + 1 {
                    break;
                }
            }
            BigUint::parse_bytes(s[..D as usize + 1].as_bytes(), 10).unwrap()
        }
        // [2; 3, 5, 7, 11, ...]: the primes' continued fraction
        "cf" => {
            let (mut h0, mut h1) = (BigUint::one(), BigUint::from(2u32));
            let (mut k0, mut k1) = (BigUint::zero(), BigUint::one());
            for &a in &ps[1..] {
                let h2 = BigUint::from(a) * &h1 + &h0;
                let k2 = BigUint::from(a) * &k1 + &k0;
                h0 = h1;
                h1 = h2;
                k0 = k1;
                k1 = k2;
                if k1.bits() > (D as u64 + 20) * 4 {
                    break;
                }
            }
            &scale * h1 / k1
        }
        // sum 2^-p = 0.4146825... (the prime constant), times 10
        "2^-p" => {
            let guard = pow10(D + 10);
            let mut sum = BigUint::zero();
            for &p in &ps {
                if p as u64 > (D as u64 + 10) * 4 {
                    break;
                }
                sum += &guard >> p;
            }
            sum * BigUint::from(10u32) / pow10(10)
        }
        // sum 10^-p = 0.011010100010..., times 100
        "10^-p" => {
            let mut sum = BigUint::zero();
            for &p in &ps {
                if p > D {
                    break;
                }
                sum += pow10(D - p);
            }
            sum * BigUint::from(100u32)
        }
        // sum 1/p^2 over the primes below 20000 (the tail is below 1/20000: this constant is right to about 4 digits only), times 10
        "1/p^2" => {
            let guard = pow10(D + 10);
            let mut sum = BigUint::zero();
            for &p in &ps {
                sum += &guard / (BigUint::from(p) * BigUint::from(p));
            }
            sum * BigUint::from(10u32) / pow10(10)
        }
        // sum 1/(p 2^p), times 10
        "1/(p2^p)" => {
            let guard = pow10(D + 10);
            let mut sum = BigUint::zero();
            for &p in &ps {
                if p as u64 > (D as u64 + 10) * 4 {
                    break;
                }
                sum += (&guard >> p) / BigUint::from(p);
            }
            sum * BigUint::from(10u32) / pow10(10)
        }
        _ => return None,
    })
}

/// Every candidate constant with its name: the ones built from the primes and their transforms.
fn candidates(_max_q: u32) -> Vec<(String, BigUint)> {
    let mut out = Vec::new();
    for n in ["digits", "cf", "2^-p", "10^-p", "1/p^2", "1/(p2^p)"] {
        let c = from_primes(n).unwrap();
        for t in ["", "-1", "x2", "/2", "^2", "sqrt", "1+1/"] {
            if let Some(v) = transform(t, &c) {
                let label = match t { "" => n.to_string(), "-1" => format!("{n} - 1"), "x2" => format!("2 {n}"), "/2" => format!("{n} / 2"), "^2" => format!("{n}^2"), "sqrt" => format!("sqrt({n})"), _ => format!("1 + 1/{n}") };
                out.push((label, v));
            }
        }
    }
    out
}

pub fn report(max_q: u32, max_digits: usize) -> String {
    let t0 = std::time::Instant::now();
    let shapes = [Shape::Mills(2), Shape::Mills(3), Shape::Mills(4), Shape::Geometric(2), Shape::Geometric(3)];
    let cands = candidates(max_q);
    let n_tried = cands.len() * shapes.len();
    let mut found: Vec<Found> = Vec::new();
    for (name, c) in &cands {
        for shape in &shapes {
            let ps = chain(c, shape, max_digits);
            if ps.len() >= 2 {
                found.push(Found { name: name.clone(), shape: shape.clone(), primes: ps });
            }
        }
    }
    // the catch, if any: the chains least expected by luck
    found.sort_by(|a, b| (n_tried as f64 * a.chance()).total_cmp(&(n_tried as f64 * b.chance())));
    let mut out = String::new();
    out.push_str("Operation Fermented Fish: a constant found in the primes whose chain of primes goes on\n\n");
    out.push_str("question: a constant c built from the primes by a short rule, floor(c^(r^n)) or floor(c r^n) prime for every n?\n");
    out.push_str("forced: Mills (1947) for r = 3, the constant written from the primes (needs a prime between x^3 and (x+1)^3, Ingham 1937); for r = 2 the same needs Legendre's conjecture, open; nothing says a short c is or is not in Mills' set\n");
    let _ = max_q;
    out.push_str(&format!("the choice: 6 constants from the primes (digits in a row, continued fraction, sums of 2^-p, 10^-p, 1/p^2, 1/(p 2^p)) x 7 transforms, 5 shapes: {n_tried} chains, {D} digits kept, primes up to {max_digits} digits by Miller-Rabin\n\n"));
    out.push_str(&format!("  {:<14} {:<10} {:>5} {:>14}  primes\n", "c", "shape", "chain", "by luck"));
    for f in found.iter().take(12) {
        let shape = match f.shape { Shape::Mills(r) => format!("c^({r}^n)"), Shape::Geometric(r) => format!("c {r}^n") };
        let primes: Vec<String> = f.primes.iter().map(|p| { let s = p.to_string(); if s.len() > 12 { format!("{}...({} digits)", &s[..6], s.len()) } else { s } }).collect();
        out.push_str(&format!("  {:<14} {:<10} {:>5} {:>14}  {}\n", f.name, shape, f.primes.len(), format!("{:.3} expected", n_tried as f64 * f.chance()), primes.join(", ")));
    }
    let least = found.first().map(|f| n_tried as f64 * f.chance()).unwrap_or(f64::INFINITY);
    out.push_str(&format!("\nverdict ({:.0} s): the chain least expected by luck is expected {least:.3} times among the {n_tried} tried", t0.elapsed().as_secs_f64()));
    if least >= 0.05 {
        out.push_str(". Fish gone off: no constant built from the primes by these rules carries a chain beyond what luck gives this many tries. Mills' constant is built from the primes too, but from the chain's own primes, one digit per prime; a rule that names it shorter than that is not among these.\n");
    } else {
        out.push_str(": a catch to be checked, not claimed: each prime of the chain proven (not Miller-Rabin), the next value computed to more digits, and the chance recounted with every candidate ever tried.\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_are_right() {
        let s = |x: &BigUint| x.to_string();
        assert!(s(&from_primes("digits").unwrap()).starts_with("235711131719232931374143475359616771"));
        assert!(s(&from_primes("cf").unwrap()).starts_with("23130367364335829"));
        assert!(s(&from_primes("2^-p").unwrap()).starts_with("41468250985111166"));
        assert!(s(&named("pi").unwrap()).starts_with("314159265358979323846264338327950288"));
    }

    #[test]
    fn mills_constant_chains_through_the_fixed_point() {
        // A to 100 digits in fixed point: 2, 11, 1361, 2521008887, 16022236204009818131831320183 (28 digits of A give ...886 for the fourth: every prime costs its digits)
        let a = pow10(D) * BigUint::parse_bytes(b"1306377883863080690468614492602605712916784585156713644368053759966434053766826598821501403701197395", 10).unwrap() / pow10(99);
        let ps: Vec<String> = chain(&a, &Shape::Mills(3), 40).iter().map(|p| p.to_string()).collect();
        assert!(ps.starts_with(&["2".into(), "11".into(), "1361".into(), "2521008887".into(), "16022236204009818131831320183".into()]), "{ps:?}");
    }
}
