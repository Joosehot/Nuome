//! The mad attempt: an expression that knows a prime at once. Asked in the
//! open-problem way (CLAUDE.md), abstractly: Nuome claims nothing it has not
//! measured.
//!
//! The question: is there an expression f(n), evaluated in time that does
//! not grow with the prime it names, whose values f(1), f(2), ... are all
//! prime?
//!
//! What is forced (proved): no polynomial with whole coefficients gives only
//! primes (Goldbach 1752), nor any u 2^n + v (Fermat's little theorem); see
//! prime_formula. What is known to give only primes hides the primes in a
//! constant: Mills (1947) proved a real A with floor(A^(3^n)) prime for every
//! n, A = 1.3063778838630806904686144926... — but A is computed from the
//! primes it names, digit by digit.
//!
//! The choice left: an expression of a shape the theorems do not exclude,
//! floor(c^(r^n)) (Mills' shape) or floor(c r^n) (a geometric one), with a
//! constant c that Nuome writes itself, in L digits. The search is exact and
//! short: every c = m / 10^L in [1, 2) for L up to a bound, every r in 2..4,
//! the chain of primes f(1), f(2), ... counted while it lasts.
//!
//! The measure that decides whether the expression knows anything: chance.
//! A number near x is prime with chance about 1/ln x, so a chain of primes
//! p_1..p_k happens to a random constant with chance about the product of
//! 1/ln p_i; over the N constants searched, the expected number of chains
//! that long by luck is N times that. A chain found when that expectation
//! is well below 1 would be knowledge; one found when it is above 1 is luck.
//! The bits of the primes against the bits of the constant say what the
//! constant stores (Mills' constant: a lookup table written as a formula).

use crate::bigprime::miller_rabin;
use num_bigint::BigUint;
use num_traits::Zero;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// floor(c^(r^n))
    Mills,
    /// floor(c r^n)
    Geometric,
}

/// The primes floor(c^(r^n)) (or floor(c r^n)) for n = 1, 2, ... while they are prime; c = m / 10^l.
fn chain(shape: Shape, m: u64, l: u32, r: u32, max_len: usize) -> Vec<BigUint> {
    let ten_l = BigUint::from(10u32).pow(l);
    let m = BigUint::from(m);
    let mut out = Vec::new();
    let mut power = r; // r^n
    for n in 1..=max_len {
        let value = match shape {
            // floor(m^(r^n) / 10^(l r^n))
            Shape::Mills => {
                if power > 20_000 {
                    break;
                }
                m.pow(power) / ten_l.pow(power)
            }
            // floor(m r^n / 10^l)
            Shape::Geometric => (&m * BigUint::from(r).pow(n as u32)) / &ten_l,
        };
        if value < BigUint::from(2u32) || !miller_rabin(&value) {
            break;
        }
        out.push(value);
        if shape == Shape::Mills {
            power = power.saturating_mul(r);
        }
    }
    out
}

fn bits(x: &BigUint) -> f64 {
    if x.is_zero() { 0.0 } else { x.bits() as f64 }
}

pub struct Best {
    pub shape: Shape,
    pub m: u64,
    pub l: u32,
    pub r: u32,
    pub primes: Vec<BigUint>,
}

impl Best {
    /// The chance a random constant gives a chain this long: the product of 1/ln p over the chain (2 and 3 counted as certain).
    pub fn chance(&self) -> f64 {
        self.primes.iter().map(|p| { let b = bits(p); if b <= 2.0 { 1.0 } else { 1.0 / (b * std::f64::consts::LN_2) } }).product()
    }
    /// Constants searched at this L: 10^L of them, times three bases, times two shapes.
    pub fn searched(&self) -> f64 {
        10f64.powi(self.l as i32) * 6.0
    }
    /// How many chains this long the search expects by luck.
    pub fn expected_by_luck(&self) -> f64 {
        self.searched() * self.chance()
    }
    pub fn bits_out(&self) -> f64 {
        self.primes.iter().map(bits).sum()
    }
    pub fn bits_in(&self) -> f64 {
        self.l as f64 * 10f64.log2() + 2.0
    }
    pub fn constant(&self) -> String {
        format!("{}.{:0width$}", self.m / 10u64.pow(self.l), self.m % 10u64.pow(self.l), width = self.l as usize)
    }
}

/// The exact search: every constant of up to `max_digits` digits, every r in 2..=4, both shapes; the longest chain (by bits) per L.
pub fn search(max_digits: u32, max_len: usize) -> Vec<Best> {
    let mut bests = Vec::new();
    for l in 1..=max_digits {
        let mut best_l: Option<Best> = None;
        for shape in [Shape::Mills, Shape::Geometric] {
            for r in 2..=4u32 {
                let lo = 10u64.pow(l);
                for m in lo..2 * lo {
                    let ps = chain(shape, m, l, r, max_len);
                    if ps.is_empty() {
                        continue;
                    }
                    let cand = Best { shape, m, l, r, primes: ps };
                    if best_l.as_ref().map_or(true, |b| cand.bits_out() > b.bits_out()) {
                        best_l = Some(cand);
                    }
                }
            }
        }
        if let Some(b) = best_l {
            bests.push(b);
        }
    }
    bests
}

/// The report, in the open-problem order: the question, what is forced, the choice, the search, the verdict.
pub fn report(max_digits: u32) -> String {
    let t0 = std::time::Instant::now();
    let mut out = String::new();
    out.push_str("The mad attempt: an expression that knows a prime at once\n\n");
    out.push_str("question: f(n) evaluated in time that does not grow with the prime, f(1), f(2), ... all prime?\n");
    out.push_str("forced: no polynomial (Goldbach 1752) and no u 2^n + v (Fermat) gives only primes; Mills (1947): floor(A^(3^n)) is prime for every n, with A = 1.30637788386308069... computed from the primes themselves\n");
    out.push_str(&format!("the choice left: floor(c^(r^n)) and floor(c r^n), c = m/10^L in [1, 2), L = 1..{max_digits}, r = 2..4; searched exactly\n\n"));
    out.push_str(&format!("  {:<3} {:<8} {:>2} {:<10} {:>5} {:>8} {:>7} {:>14}  primes\n", "L", "shape", "r", "c", "chain", "bits out", "bits in", "by luck"));
    let bests = search(max_digits, 8);
    let mut least = f64::INFINITY;
    for b in &bests {
        let expected = b.expected_by_luck();
        least = least.min(expected);
        let shape = match b.shape { Shape::Mills => "c^(r^n)", Shape::Geometric => "c r^n" };
        let primes: Vec<String> = b.primes.iter().map(|p| { let s = p.to_string(); if s.len() > 12 { format!("{}...({} digits)", &s[..6], s.len()) } else { s } }).collect();
        out.push_str(&format!("  {:<3} {:<8} {:>2} {:<10} {:>5} {:>8.0} {:>7.0} {:>14}  {}\n", b.l, shape, b.r, b.constant(), b.primes.len(), b.bits_out(), b.bits_in(), format!("{expected:.2} expected"), primes.join(", ")));
    }
    out.push_str(&format!("\nverdict ({:.0} s): among the constants searched, the longest chains are expected by luck {least:.2} times over at the least", t0.elapsed().as_secs_f64()));
    if least >= 0.05 {
        out.push_str(". Luck, not knowledge: a chain this long turns up among this many constants by chance, and finding the constant was a search over the primes, the sieve again in another dress. An expression of these shapes knows a prime only as far as its constant stores it (Mills' constant stores them all). If a formula that knows more than it stores exists, it is not among these shapes.\n");
    } else {
        out.push_str(": far rarer than luck allows, which no known reason explains. To be checked before anything is claimed: the chain with Miller-Rabin replaced by a proof, and whether it goes on.\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mills_constant_gives_its_primes() {
        // A = 1.306377883863 to 12 digits: floor(A^3) = 2, floor(A^9) = 11, floor(A^27) = 1361
        let ps = chain(Shape::Mills, 1_306_377_883_863, 12, 3, 4);
        let s: Vec<String> = ps.iter().map(|p| p.to_string()).collect();
        assert!(s.starts_with(&["2".to_string(), "11".to_string(), "1361".to_string()]), "{s:?}");
    }

    #[test]
    fn the_short_chains_are_luck() {
        for b in search(3, 8) {
            assert!(b.expected_by_luck() >= 0.05, "L={} chain {} expected {}", b.l, b.primes.len(), b.expected_by_luck());
        }
    }
}
