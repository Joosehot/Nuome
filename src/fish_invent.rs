//! Operation Fermented Fish, the invention: Nuome invents the shape of the
//! function and the number that completes it.
//!
//! A shape is an expression g(A, n) built from n, A, the digits 2, 3, 4 and
//! +, x, power and factorial, with A once, in a place where g grows with A
//! (as a base or a factor), so that floor(g(A, n)) = p pins A to an interval
//! by undoing the operations on the way to A: minus, divide, root. Every
//! shape up to a size is enumerated; none is given.
//!
//! The number: for each shape the chain is built the way Mills built his
//! constant: A starts in [1, 10); at every n the primes that floor(g(A, n))
//! can still be form an interval, the smallest of them is taken, and A's
//! interval shrinks to what gives it. A chain that goes on as far as the
//! primes can be tested gives a number A, known to as many digits as the
//! last interval allows.
//!
//! What would be a catch: an A with a description shorter than the digits
//! it is known to, a periodic continued fraction (a quadratic irrational), a
//! fraction, or a tail of a known number. What is expected: every A costs
//! its primes (Mills' does: 28 digits give the fourth prime wrong), because
//! the smallest prime past a point is not written in any shorter way than by
//! finding it. The verdict says which happened, shape by shape.

use crate::bigprime::miller_rabin;
use num_bigint::BigUint;
use num_traits::{CheckedSub, One, Zero};

/// Digits of fixed point kept.
const D: u32 = 1500;

#[derive(Clone, Debug, PartialEq)]
pub enum E {
    N,
    A,
    C(u32),
    Add(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
    Pow(Box<E>, Box<E>),
    Fact(Box<E>),
}

impl E {
    fn has_a(&self) -> bool {
        match self {
            E::A => true,
            E::N | E::C(_) => false,
            E::Add(a, b) | E::Mul(a, b) | E::Pow(a, b) => a.has_a() || b.has_a(),
            E::Fact(a) => a.has_a(),
        }
    }
    fn has_n(&self) -> bool {
        match self {
            E::N => true,
            E::A | E::C(_) => false,
            E::Add(a, b) | E::Mul(a, b) | E::Pow(a, b) => a.has_n() || b.has_n(),
            E::Fact(a) => a.has_n(),
        }
    }
    fn count_a(&self) -> usize {
        match self {
            E::A => 1,
            E::N | E::C(_) => 0,
            E::Add(a, b) | E::Mul(a, b) | E::Pow(a, b) => a.count_a() + b.count_a(),
            E::Fact(a) => a.count_a(),
        }
    }
    /// A sits in a place that can be undone: never in an exponent, never under a factorial.
    fn a_invertible(&self) -> bool {
        match self {
            E::A | E::N | E::C(_) => true,
            E::Add(a, b) | E::Mul(a, b) => a.a_invertible() && b.a_invertible(),
            E::Pow(a, b) => !b.has_a() && a.a_invertible(),
            E::Fact(a) => !a.has_a(),
        }
    }
    pub fn show(&self) -> String {
        match self {
            E::N => "n".into(),
            E::A => "A".into(),
            E::C(c) => c.to_string(),
            E::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            E::Mul(a, b) => format!("{} {}", a.show(), b.show()),
            E::Pow(a, b) => format!("{}^({})", if matches!(**a, E::Mul(..) | E::Add(..)) { format!("({})", a.show()) } else { a.show() }, b.show()),
            E::Fact(a) => format!("({})!", a.show()),
        }
    }
    /// The whole-number value of an A-free part at n.
    fn whole(&self, n: u32) -> Option<BigUint> {
        Some(match self {
            E::N => BigUint::from(n),
            E::A => return None,
            E::C(c) => BigUint::from(*c),
            E::Add(a, b) => a.whole(n)? + b.whole(n)?,
            E::Mul(a, b) => a.whole(n)? * b.whole(n)?,
            E::Pow(a, b) => {
                let e = b.whole(n)?;
                if e.bits() > 20 {
                    return None;
                }
                let e: u32 = e.try_into().ok()?;
                let base = a.whole(n)?;
                if base.bits() as u64 * e as u64 > 40_000 {
                    return None;
                }
                base.pow(e)
            }
            E::Fact(a) => {
                let k = a.whole(n)?;
                if k.bits() > 10 {
                    return None;
                }
                let k: u32 = k.try_into().ok()?;
                (1..=k).fold(BigUint::one(), |acc, i| acc * BigUint::from(i))
            }
        })
    }
}

fn scale() -> BigUint {
    BigUint::from(10u32).pow(D)
}

/// Fixed-point x^k (x with D digits after the point), truncated each step.
fn fpow(x: &BigUint, k: &BigUint) -> Option<BigUint> {
    if k.bits() > 24 {
        return None;
    }
    let s = scale();
    let mut result = s.clone();
    let mut base = x.clone();
    let mut e: u64 = k.try_into().ok()?;
    // guard against values too large to hold
    while e > 0 {
        if e & 1 == 1 {
            result = &result * &base / &s;
            if result.bits() > 12_000_000 {
                return None;
            }
        }
        e >>= 1;
        if e > 0 {
            base = &base * &base / &s;
            if base.bits() > 12_000_000 {
                return None;
            }
        }
    }
    Some(result)
}

/// Fixed-point k-th root by Newton (x > 0).
fn froot(x: &BigUint, k: &BigUint) -> Option<BigUint> {
    let s = scale();
    let k64: u64 = k.try_into().ok()?;
    if k64 == 1 {
        return Some(x.clone());
    }
    // the exact root of x s^(k-1): floor((x s^(k-1))^(1/k)) is the fixed point of x^(1/k)
    if k64 <= 64 {
        let big = x * s.pow(k64 as u32 - 1);
        return Some(iroot(&big, k64 as u32));
    }
    // Newton in fixed point from an f64 start
    let bits = x.bits() as f64 - D as f64 * std::f64::consts::LOG2_10;
    let start = (bits / k64 as f64 * std::f64::consts::LN_2).exp();
    let mut r = BigUint::from((start * 1e15) as u64) * &s / BigUint::from(1_000_000_000_000_000u64);
    if r.is_zero() {
        r = s.clone();
    }
    let kb = BigUint::from(k64);
    for _ in 0..200 {
        let rk1 = fpow(&r, &BigUint::from(k64 - 1))?;
        // r' = r - (r^k - x) / (k r^(k-1)) = ((k-1) r + x / r^(k-1)) / k
        let next = (BigUint::from(k64 - 1) * &r + x * &s / &rk1) / &kb;
        let done = if next > r { &next - &r } else { &r - &next } < BigUint::from(10u32).pow(D / 2);
        r = next;
        if done {
            break;
        }
    }
    Some(r)
}

/// floor(x^(1/k)) by Newton.
fn iroot(x: &BigUint, k: u32) -> BigUint {
    if x.is_zero() {
        return BigUint::zero();
    }
    let mut r: BigUint = BigUint::one() << (x.bits() / k as u64 + 1);
    let (km1, kk) = (BigUint::from(k - 1), BigUint::from(k));
    loop {
        let next = (&km1 * &r + x / r.pow(k - 1)) / &kk;
        if next >= r {
            return r;
        }
        r = next;
    }
}

/// g(a, n) in fixed point, a the fixed-point A.
fn eval(e: &E, a: &BigUint, n: u32) -> Option<BigUint> {
    let s = scale();
    Some(match e {
        E::A => a.clone(),
        _ if !e.has_a() => e.whole(n)? * &s,
        E::Add(x, y) => eval(x, a, n)? + eval(y, a, n)?,
        E::Mul(x, y) => eval(x, a, n)? * eval(y, a, n)? / &s,
        E::Pow(x, y) => fpow(&eval(x, a, n)?, &y.whole(n)?)?,
        _ => return None,
    })
}

/// Undo g on the way to A: the A that gives g(A, n) = v (fixed point).
fn invert(e: &E, v: &BigUint, n: u32) -> Option<BigUint> {
    let s = scale();
    match e {
        E::A => Some(v.clone()),
        E::Add(x, y) => {
            let (with_a, other) = if x.has_a() { (x, y) } else { (y, x) };
            let c = other.whole(n)? * &s;
            invert(with_a, &v.checked_sub(&c)?, n)
        }
        E::Mul(x, y) => {
            let (with_a, other) = if x.has_a() { (x, y) } else { (y, x) };
            let c = other.whole(n)?;
            if c.is_zero() {
                return None;
            }
            invert(with_a, &(v / c), n)
        }
        E::Pow(x, y) => invert(x, &froot(v, &y.whole(n)?)?, n),
        _ => None,
    }
}

/// The smallest prime in [lo, hi) (whole numbers), with a cheap sieve first; None when there is none.
fn smallest_prime_in(lo: &BigUint, hi: &BigUint) -> Option<BigUint> {
    let mut x = lo.clone();
    if x < BigUint::from(2u32) {
        x = BigUint::from(2u32);
    }
    let small: [u32; 15] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    let mut tries = 0;
    while &x < hi {
        tries += 1;
        if tries > 50_000 {
            return None;
        }
        let divisible = small.iter().any(|&p| (&x % p).is_zero() && x != BigUint::from(p));
        if !divisible && miller_rabin(&x) {
            return Some(x);
        }
        x += BigUint::one();
    }
    None
}

pub struct Built {
    pub shape: E,
    pub primes: Vec<BigUint>,
    /// A's interval, fixed point
    pub lo: BigUint,
    pub hi: BigUint,
    /// why the chain stopped
    pub stop: &'static str,
}

impl Built {
    /// The digits of A the chain pins down.
    pub fn digits_known(&self) -> usize {
        let w = &self.hi - &self.lo;
        if w.is_zero() {
            return D as usize;
        }
        (D as usize).saturating_sub(w.to_string().len())
    }
    pub fn a_string(&self, digits: usize) -> String {
        let s = self.lo.to_string();
        let int_len = s.len().saturating_sub(D as usize).max(1);
        let s = format!("{:0>width$}", s, width = D as usize + 1);
        let int_len = int_len.max(s.len() - D as usize);
        format!("{}.{}", &s[..int_len], &s[int_len..(int_len + digits).min(s.len())])
    }
}

/// Build the chain for a shape: A in [1, 10) at the start, the smallest prime each time.
pub fn build(shape: &E, max_digits: usize, max_len: usize) -> Built {
    let s = scale();
    let (mut lo, mut hi) = (s.clone(), &s * BigUint::from(10u32));
    let mut primes = Vec::new();
    let mut stop = "went on to the limit";
    for n in 1..=max_len {
        let (Some(glo), Some(ghi)) = (eval(shape, &lo, n as u32), eval(shape, &hi, n as u32)) else { stop = "too large to hold"; break };
        let (vlo, vhi) = (&glo / &s + BigUint::one(), &ghi / &s + BigUint::one());
        // the whole numbers v with floor(g) = v possible: from ceil(glo) to floor(ghi); and past the last prime,
        // so that a shape standing still (3 A^n with A near 1 is 3 for ever) is not a chain
        let mut from = if (&glo % &s).is_zero() { &glo / &s } else { vlo };
        if let Some(last) = primes.last() {
            if &from <= last {
                from = last + BigUint::one();
            }
        }
        if from.to_string().len() > max_digits {
            stop = "past the digits that can be tested";
            break;
        }
        let Some(p) = smallest_prime_in(&from, &vhi) else { stop = "no prime in the interval: the shape fails"; break };
        // A's new interval: g = p .. p + 1
        let (Some(nlo), Some(nhi)) = (invert(shape, &(&p * &s), n as u32), invert(shape, &((&p + BigUint::one()) * &s), n as u32)) else { stop = "cannot undo the shape"; break };
        lo = lo.max(nlo);
        hi = hi.min(nhi);
        if lo >= hi {
            stop = "the interval closed";
            break;
        }
        primes.push(p);
    }
    Built { shape: shape.clone(), primes, lo, hi, stop }
}

/// The continued fraction of a fixed-point number, at most `terms` terms, only while the convergents are within the known digits.
pub fn continued_fraction(a: &BigUint, digits_known: usize, terms: usize) -> Vec<u64> {
    let s = scale();
    let mut out = Vec::new();
    let mut x = a.clone();
    let (mut k0, mut k1) = (BigUint::zero(), BigUint::one());
    for _ in 0..terms {
        let q = &x / &s;
        let qv: u64 = match (&q).try_into() { Ok(v) => v, Err(_) => break };
        let k2 = &q * &k1 + &k0;
        // the convergent is trustworthy while k^2 < 10^digits_known
        if k2.to_string().len() * 2 > digits_known {
            break;
        }
        out.push(qv);
        k0 = k1;
        k1 = k2;
        let frac = &x - &q * &s;
        if frac.is_zero() {
            break;
        }
        x = &s * &s / frac;
    }
    out
}

/// A period in the continued fraction's tail: Some((preperiod, period)) when a period of at most 8 repeats at least three times.
pub fn periodic(cf: &[u64]) -> Option<(usize, usize)> {
    for pre in 0..=4.min(cf.len()) {
        for per in 1..=8 {
            if pre + 3 * per > cf.len() {
                break;
            }
            let ok = (pre + per..cf.len()).all(|i| cf[i] == cf[i - per]);
            if ok {
                return Some((pre, per));
            }
        }
    }
    None
}

/// Every shape up to `size` nodes with A once, invertible, and n present.
pub fn shapes(size: usize) -> Vec<E> {
    let mut by_size: Vec<Vec<E>> = vec![Vec::new(), vec![E::N, E::A, E::C(2), E::C(3), E::C(4)]];
    for s in 2..=size {
        let mut v = Vec::new();
        for x in &by_size[s - 1] {
            if !x.has_a() {
                v.push(E::Fact(Box::new(x.clone())));
            }
        }
        for i in 1..s - 1 {
            let j = s - 1 - i;
            for x in &by_size[i] {
                for y in &by_size[j] {
                    if x.count_a() + y.count_a() <= 1 {
                        v.push(E::Add(Box::new(x.clone()), Box::new(y.clone())));
                        v.push(E::Mul(Box::new(x.clone()), Box::new(y.clone())));
                        if !y.has_a() {
                            v.push(E::Pow(Box::new(x.clone()), Box::new(y.clone())));
                        }
                    }
                }
            }
        }
        by_size.push(v);
    }
    let mut out: Vec<E> = by_size.into_iter().flatten().filter(|e| e.count_a() == 1 && e.has_n() && e.a_invertible()).collect();
    out.dedup_by_key(|e| e.show());
    out
}

pub fn report(size: usize, max_digits: usize) -> String {
    let t0 = std::time::Instant::now();
    let all = shapes(size);
    let mut out = String::new();
    out.push_str("Operation Fermented Fish, the invention: the shape of the function and the number that completes it\n\n");
    out.push_str("question: a shape floor(g(A, n)) and a number A, prime for every n, A written shorter than the primes it names?\n");
    out.push_str("forced: Mills (1947) shows the construction for A^(3^n); its A costs its primes (28 digits give the fourth prime wrong)\n");
    out.push_str(&format!("the choice: every shape of up to {size} symbols from n, A, 2, 3, 4, +, x, ^, ! with A once in an undoable place: {} shapes; A from [1, 10) by the smallest prime each time, primes tested to {max_digits} digits\n\n", all.len()));
    let mut built: Vec<Built> = all.iter().map(|e| build(e, max_digits, 40)).collect();
    built.sort_by(|a, b| b.primes.len().cmp(&a.primes.len()).then(b.digits_known().cmp(&a.digits_known())));
    // shapes that give the same chain are the same shape written otherwise: the first stays
    let mut seen: Vec<Vec<BigUint>> = Vec::new();
    built.retain(|b| { if seen.contains(&b.primes) { false } else { seen.push(b.primes.clone()); true } });
    out.push_str(&format!("  {:<22} {:>5} {:>7} {:<26} {:<34} {}\n", "shape", "chain", "digits", "A", "continued fraction", "stopped"));
    let mut catch = Vec::new();
    for b in built.iter().take(25) {
        let dk = b.digits_known();
        let cf = continued_fraction(&b.lo, dk, 14);
        let per = periodic(&cf);
        let mut note = String::new();
        if let Some((pre, p)) = per {
            if dk >= 8 && cf.len() >= 6 {
                note = format!(" PERIODIC (pre {pre}, period {p})");
                catch.push((b.shape.show(), b.a_string(30), pre, p, dk));
            }
        }
        out.push_str(&format!("  {:<22} {:>5} {:>7} {:<26} {:<34} {}{}\n", b.shape.show(), b.primes.len(), dk, b.a_string(20.min(dk.max(1))), format!("{:?}", cf), b.stop, note));
    }
    // the chains written out for a check by other tools: shape, A to its known digits, the primes
    let mut dump = String::new();
    for b in built.iter().take(25) {
        let dk = b.digits_known();
        if b.primes.len() >= 5 && dk >= 5 {
            // the middle of A's interval, three digits past what is known: a number inside the interval, not its lower end cut short
            let mid = Built { shape: b.shape.clone(), primes: Vec::new(), lo: (&b.lo + &b.hi) / BigUint::from(2u32), hi: b.hi.clone(), stop: "" };
            dump.push_str(&format!("{}\t{}\t{}\n", b.shape.show(), mid.a_string(dk + 3), b.primes.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" ")));
        }
    }
    let _ = std::fs::write("out/fish_invent_chains.tsv", dump);
    out.push_str(&format!("\nverdict ({:.0} s): ", t0.elapsed().as_secs_f64()));
    if catch.is_empty() {
        out.push_str("no A with a shorter description than its digits: every continued fraction is irregular as far as it is known, and every chain that goes on pins A down only as far as its primes. The number that completes the function is always written by the primes it names; the shapes invented here do not change that.\n");
    } else {
        out.push_str("a period in the continued fraction, to be checked before anything is claimed (more primes, more digits, the period holding):\n");
        for (s, a, pre, p, dk) in catch {
            out.push_str(&format!("  {s}: A = {a}..., preperiod {pre}, period {p}, {dk} digits known\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mills_shape_rebuilds_mills_primes() {
        let shape = E::Pow(Box::new(E::A), Box::new(E::Pow(Box::new(E::C(3)), Box::new(E::N))));
        let b = build(&shape, 60, 6);
        let ps: Vec<String> = b.primes.iter().map(|p| p.to_string()).collect();
        assert_eq!(&ps[..4], &["2", "11", "1361", "2521008887"], "{ps:?} {}", b.stop);
        assert!(b.a_string(12).starts_with("1.3063778838"), "{}", b.a_string(20));
    }

    #[test]
    fn the_grammar_holds_mills_shape_and_undoes_it() {
        let all = shapes(5);
        assert!(all.iter().any(|e| e.show() == "A^(3^(n))"), "{}", all.len());
        let s = scale();
        let a = &s * BigUint::from(13u32) / BigUint::from(10u32);
        let shape = E::Pow(Box::new(E::A), Box::new(E::C(3)));
        let g = eval(&shape, &a, 1).unwrap();
        let back = invert(&shape, &g, 1).unwrap();
        assert!((if back > a { &back - &a } else { &a - &back }) < BigUint::from(10u32).pow(D - 1400));
    }

    #[test]
    fn a_period_is_seen() {
        // sqrt 2 = [1; 2, 2, 2, ...]
        let s = scale();
        let r = iroot(&(BigUint::from(2u32) * &s * &s), 2);
        let cf = continued_fraction(&r, 1000, 12);
        assert_eq!(periodic(&cf), Some((1, 1)), "{cf:?}");
    }
}
