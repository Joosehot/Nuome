//! Operation Fermented Fish, the checker: as the generator was invented (a
//! shape and the number that completes it), a checker is invented: a shape
//! c(A, N) over the number N to be tested and a constant A, true exactly when
//! N is prime.
//!
//! The shapes come from a grammar, none is given: a value built from N, the
//! digits 2 and 3, a constant A inside floor(A h(N)), and +, x, minus one,
//! factorial, and a^e mod N; and a predicate on the value: = 0 mod N,
//! = 1 mod N, or odd. Every shape up to a size is enumerated.
//!
//! The number: for a shape with A, A is solved from the data the way the
//! generator's was, by intervals: each N in the fitting range allows A in a
//! union of intervals (floor(A h) must fall in the right residue class),
//! and the intersection over N = 2.. is kept as pieces until it is empty.
//! A shape without A has nothing to solve.
//!
//! The measure, the one that separates a checker from a lookup table:
//!   - the fit: right on every N in 2..fit;
//!   - the test: right on N the fit never saw (fit+1..test), and on the
//!     known liars (the Carmichael numbers, which fool Fermat's test);
//!   - the bits: of the constant A against the numbers it was fitted on;
//!     a bit per number is a table;
//!   - the cost: multiplications per test, a^e mod N paying 2 log2 e.
//! A shape right on the unseen numbers with a short constant and a cost
//! below one multiplication per bit of N would be the catch.

use num_bigint::BigUint;
use num_traits::{CheckedSub, One, Zero};

#[derive(Clone, Debug, PartialEq)]
pub enum V {
    N,
    C(u32),
    /// floor(A h(N)), A solved from the data
    FloorA(Box<V>),
    Add(Box<V>, Box<V>),
    Mul(Box<V>, Box<V>),
    Dec(Box<V>),
    /// base^exp mod N
    PowMod(Box<V>, Box<V>),
    Fact(Box<V>),
    /// 2^x, exact (inside floor(A h) only: the table shapes)
    Exp(Box<V>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pred {
    ZeroModN,
    OneModN,
    Odd,
}

#[derive(Clone, Debug)]
pub struct Shape {
    pub value: V,
    pub pred: Pred,
}

impl V {
    fn has_a(&self) -> bool {
        match self {
            V::N | V::C(_) => false,
            V::FloorA(_) => true,
            V::Add(a, b) | V::Mul(a, b) | V::PowMod(a, b) => a.has_a() || b.has_a(),
            V::Dec(a) | V::Fact(a) | V::Exp(a) => a.has_a(),
        }
    }
    fn count_a(&self) -> usize {
        match self {
            V::N | V::C(_) => 0,
            V::FloorA(h) => 1 + h.count_a(),
            V::Add(a, b) | V::Mul(a, b) | V::PowMod(a, b) => a.count_a() + b.count_a(),
            V::Dec(a) | V::Fact(a) | V::Exp(a) => a.count_a(),
        }
    }
    fn has_n(&self) -> bool {
        match self {
            V::N => true,
            V::C(_) => false,
            V::FloorA(h) => h.has_n(),
            V::Add(a, b) | V::Mul(a, b) | V::PowMod(a, b) => a.has_n() || b.has_n(),
            V::Dec(a) | V::Fact(a) | V::Exp(a) => a.has_n(),
        }
    }
    pub fn show(&self) -> String {
        match self {
            V::N => "N".into(),
            V::C(c) => c.to_string(),
            V::FloorA(h) => format!("floor(A {})", h.show()),
            V::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            V::Mul(a, b) => format!("{} {}", a.show(), b.show()),
            V::Dec(a) => format!("({} - 1)", a.show()),
            V::PowMod(a, b) => format!("{}^{} mod N", a.show(), b.show()),
            V::Fact(a) => format!("({})!", a.show()),
            V::Exp(a) => format!("2^{}", a.show()),
        }
    }
    /// Multiplications the value costs at N (a^e mod N: 2 log2 e; a factorial: its argument).
    fn cost(&self, n: u64) -> f64 {
        match self {
            V::N | V::C(_) => 0.0,
            V::FloorA(h) => 1.0 + h.cost(n),
            V::Add(a, b) => a.cost(n) + b.cost(n),
            V::Mul(a, b) => 1.0 + a.cost(n) + b.cost(n),
            V::Dec(a) => a.cost(n),
            V::PowMod(a, e) => {
                let ev = e.eval_plain(n).map(|x| x as f64).unwrap_or(n as f64);
                2.0 * ev.max(2.0).log2() + a.cost(n) + e.cost(n)
            }
            V::Fact(a) => a.eval_plain(n).map(|x| x as f64).unwrap_or(n as f64) + a.cost(n),
            V::Exp(a) => a.eval_plain(n).map(|x| x as f64).unwrap_or(n as f64) + a.cost(n),
        }
    }
    /// The exact whole value at N with no A, for the table shapes (factorials to 2000, powers of two to 4000 bits).
    fn eval_big(&self, n: u64) -> Option<BigUint> {
        Some(match self {
            V::N => BigUint::from(n),
            V::C(c) => BigUint::from(*c),
            V::FloorA(_) => return None,
            V::Add(a, b) => a.eval_big(n)? + b.eval_big(n)?,
            V::Mul(a, b) => a.eval_big(n)? * b.eval_big(n)?,
            V::Dec(a) => a.eval_big(n)?.checked_sub(&BigUint::one())?,
            V::PowMod(a, e) => BigUint::from(powmod(a.eval_plain(n)? % n.max(1), e.eval_plain(n)?, n)),
            V::Fact(a) => {
                let k = a.eval_plain(n)?;
                if k > 2000 {
                    return None;
                }
                (1..=k).fold(BigUint::one(), |acc, i| acc * BigUint::from(i))
            }
            V::Exp(a) => {
                let k = a.eval_plain(n)?;
                if k > 4000 {
                    return None;
                }
                BigUint::one() << k
            }
        })
    }
    /// The whole value at N with no A (None when too large or undefined); factorials and products are taken mod N where the predicate is mod N.
    fn eval_plain(&self, n: u64) -> Option<u64> {
        Some(match self {
            V::N => n,
            V::C(c) => *c as u64,
            V::FloorA(_) => return None,
            V::Add(a, b) => a.eval_plain(n)?.checked_add(b.eval_plain(n)?)?,
            V::Mul(a, b) => a.eval_plain(n)?.checked_mul(b.eval_plain(n)?)?,
            V::Dec(a) => a.eval_plain(n)?.checked_sub(1)?,
            V::PowMod(a, e) => powmod(a.eval_plain(n)? % n.max(1), e.eval_plain(n)?, n),
            V::Fact(a) => {
                let k = a.eval_plain(n)?;
                if k > 20 {
                    return None;
                }
                (1..=k).product()
            }
            V::Exp(a) => {
                let k = a.eval_plain(n)?;
                if k > 62 {
                    return None;
                }
                1u64 << k
            }
        })
    }
    /// The value mod m at N (m = N or 2), with factorials reduced as they go; None when A is inside or undefined.
    fn eval_mod(&self, n: u64, m: u64) -> Option<u64> {
        Some(match self {
            V::N => n % m,
            V::C(c) => *c as u64 % m,
            V::FloorA(_) => return None,
            V::Add(a, b) => (a.eval_mod(n, m)? + b.eval_mod(n, m)?) % m,
            V::Mul(a, b) => mulmod(a.eval_mod(n, m)?, b.eval_mod(n, m)?, m),
            V::Dec(a) => (a.eval_mod(n, m)? + m - 1) % m,
            V::PowMod(a, e) => {
                // computed mod N by definition, then reduced mod m
                let base = a.eval_mod(n, n)?;
                let exp = e.eval_plain(n)?;
                powmod(base, exp, n) % m
            }
            V::Fact(a) => {
                let k = a.eval_plain(n)?;
                if k > 30_000 {
                    return None;
                }
                let mut f = 1u64 % m;
                for i in 1..=k {
                    f = mulmod(f, i % m, m);
                }
                f
            }
            V::Exp(a) => powmod(2, a.eval_plain(n)?, m),
        })
    }
}

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m.max(1) as u128) as u64
}

fn powmod(mut b: u64, mut e: u64, m: u64) -> u64 {
    if m <= 1 {
        return 0;
    }
    let mut r = 1 % m;
    b %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, b, m);
        }
        b = mulmod(b, b, m);
        e >>= 1;
    }
    r
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    // deterministic Miller-Rabin for n < 3.3e24 with these bases
    let d0 = n - 1;
    let s = d0.trailing_zeros();
    let d = d0 >> s;
    'outer: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

/// Carmichael numbers below 10^7: the liars that fool Fermat's test for every base coprime to them.
const CARMICHAEL: [u64; 24] = [561, 1105, 1729, 2465, 2821, 6601, 8911, 10585, 15841, 29341, 41041, 46657, 52633, 62745, 63973, 75361, 101101, 115921, 126217, 162401, 172081, 188461, 252601, 278545];

/// Every shape up to `size` nodes with N present and A at most once.
pub fn shapes(size: usize) -> Vec<Shape> {
    let mut by_size: Vec<Vec<V>> = vec![Vec::new(), vec![V::N, V::C(2), V::C(3)]];
    for s in 2..=size {
        let mut v = Vec::new();
        for x in &by_size[s - 1] {
            v.push(V::Dec(Box::new(x.clone())));
            if !x.has_a() {
                v.push(V::Fact(Box::new(x.clone())));
                v.push(V::FloorA(Box::new(x.clone())));
                v.push(V::FloorA(Box::new(V::Exp(Box::new(x.clone())))));
            }
        }
        for i in 1..s - 1 {
            let j = s - 1 - i;
            for x in &by_size[i] {
                for y in &by_size[j] {
                    if x.count_a() + y.count_a() <= 1 {
                        v.push(V::Add(Box::new(x.clone()), Box::new(y.clone())));
                        v.push(V::Mul(Box::new(x.clone()), Box::new(y.clone())));
                        if !y.has_a() && !x.has_a() {
                            v.push(V::PowMod(Box::new(x.clone()), Box::new(y.clone())));
                        }
                    }
                }
            }
        }
        by_size.push(v);
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for v in by_size.into_iter().flatten() {
        if !v.has_n() || v.count_a() > 1 || !seen.insert(v.show()) {
            continue;
        }
        for pred in [Pred::ZeroModN, Pred::OneModN, Pred::Odd] {
            out.push(Shape { value: v.clone(), pred });
        }
    }
    out
}

/// A fixed-point piece [lo, hi) of A, D digits.
const D: u32 = 400;
fn scale() -> BigUint {
    BigUint::from(10u32).pow(D)
}

pub struct Judged {
    pub shape: Shape,
    /// mistakes in the fit range (shapes with A: how far the pieces lasted instead)
    pub fit_wrong: usize,
    pub fit_reached: u64,
    pub test_right: usize,
    pub test_n: usize,
    /// of the unseen primes and composites: (primes right, primes), (composites right, composites)
    pub primes: (usize, usize),
    pub composites: (usize, usize),
    pub liars_caught: usize,
    pub cost_bits: f64,
    /// bits of the constant the fit needed (0 without A)
    pub constant_bits: f64,
    pub a: Option<BigUint>,
}

/// The whole number floor(A h) at N for a fixed-point A.
fn floor_a(a: &BigUint, h: &BigUint) -> BigUint {
    a * h / scale()
}

/// Tally one unseen N: (primes right, primes, composites right, composites) by whether the verdict matched.
fn tally(t: &mut (usize, usize, usize, usize), n: u64, verdict: Option<bool>) {
    let prime = is_prime(n);
    if prime { t.1 += 1 } else { t.3 += 1 }
    if let Some(v) = verdict {
        if v == prime {
            if prime { t.0 += 1 } else { t.2 += 1 }
        }
    }
}

fn predicate(pred: Pred, value_mod: u64) -> bool {
    match pred {
        Pred::ZeroModN => value_mod == 0,
        Pred::OneModN => value_mod == 1,
        Pred::Odd => value_mod == 1,
    }
}

/// Solve A for a shape floor(A h(N)) [pred] by intersecting, over N = 2..fit, the pieces of [0, 1) that put floor(A h(N)) in the wanted class.
/// Returns (pieces left, the last N satisfied).
fn solve_a(h: &V, pred: Pred, fit: u64) -> (Vec<(BigUint, BigUint)>, u64) {
    let s = scale();
    let mut pieces: Vec<(BigUint, BigUint)> = vec![(BigUint::zero(), s.clone())];
    let mut reached = 1;
    for n in 2..=fit {
        let Some(hb) = h.eval_big(n) else { break };
        if hb.is_zero() {
            break;
        }
        let want = is_prime(n);
        let m = match pred { Pred::Odd => 2u64, _ => n };
        let mut next = Vec::new();
        for (lo, hi) in &pieces {
            // floor(A h) runs over the whole numbers v in [floor(lo h / s), floor(hi h / s)]
            let v0 = lo * &hb / &s;
            let v1 = hi * &hb / &s;
            let mut v = v0.clone();
            let mut count = 0;
            while v <= v1 && count < 4096 {
                count += 1;
                let vm = (&v % m).try_into().unwrap_or(0u64);
                let ok = predicate(pred, vm) == want;
                if ok {
                    // A in [v/h, (v+1)/h) intersected with the piece
                    let a0 = (&v * &s + &hb - BigUint::one()) / &hb; // ceil(v s / h)
                    let a1 = ((&v + BigUint::one()) * &s + &hb - BigUint::one()) / &hb;
                    let (plo, phi) = (a0.max(lo.clone()), a1.min(hi.clone()));
                    if plo < phi {
                        next.push((plo, phi));
                    }
                }
                v += BigUint::one();
            }
            if next.len() > 2000 {
                break;
            }
        }
        if next.is_empty() {
            return (pieces, reached);
        }
        next.truncate(2000);
        pieces = next;
        reached = n;
    }
    (pieces, reached)
}

pub fn judge(shape: &Shape, fit: u64, test: u64) -> Judged {
    let m_of = |n: u64| match shape.pred { Pred::Odd => 2, _ => n };
    if let Some((h, _)) = find_floor_a(&shape.value) {
        // only the bare floor(A h) shapes are solved; A inside a bigger value is left
        if matches!(&shape.value, V::FloorA(_)) {
            let (pieces, reached) = solve_a(h, shape.pred, fit);
            let a = pieces.first().map(|(lo, hi)| (lo + hi) / BigUint::from(2u32));
            let mut right = 0;
            let mut liars = 0;
            let mut t = (0usize, 0usize, 0usize, 0usize);
            if let Some(a) = &a {
                for n in fit + 1..=test {
                    let verdict = h.eval_big(n).map(|hv| { let v = floor_a(a, &hv); let vm: u64 = (&v % m_of(n)).try_into().unwrap_or(0); predicate(shape.pred, vm) });
                    if verdict == Some(is_prime(n)) {
                        right += 1;
                    }
                    tally(&mut t, n, verdict);
                }
                for &c in &CARMICHAEL {
                    if let Some(hv) = h.eval_big(c) {
                        let v = floor_a(a, &hv);
                        let vm: u64 = (&v % m_of(c)).try_into().unwrap_or(0);
                        if !predicate(shape.pred, vm) {
                            liars += 1;
                        }
                    }
                }
            }
            // the constant's bits: the width of the last piece says how many digits A needs
            let width = pieces.first().map(|(lo, hi)| hi - lo).unwrap_or_else(BigUint::one);
            let constant_bits = (D as f64 - width.to_string().len() as f64).max(0.0) * 10f64.log2();
            return Judged { shape: shape.clone(), fit_wrong: (fit - reached) as usize, fit_reached: reached, test_right: right, test_n: (test - fit) as usize, primes: (t.0, t.1), composites: (t.2, t.3), liars_caught: liars, cost_bits: shape.value.cost(test) / (test as f64).log2(), constant_bits, a };
        }
    }
    let mut wrong = 0;
    for n in 2..=fit {
        match shape.value.eval_mod(n, m_of(n)) {
            Some(vm) => {
                if predicate(shape.pred, vm) != is_prime(n) {
                    wrong += 1;
                }
            }
            None => wrong += 1,
        }
    }
    let mut right = 0;
    let mut t = (0usize, 0usize, 0usize, 0usize);
    // shapes wrong on more than a twentieth of the fit are not worth the unseen run
    if wrong * 20 <= fit as usize {
        for n in fit + 1..=test {
            let verdict = shape.value.eval_mod(n, m_of(n)).map(|vm| predicate(shape.pred, vm));
            if verdict == Some(is_prime(n)) {
                right += 1;
            }
            tally(&mut t, n, verdict);
        }
    }
    let liars = CARMICHAEL.iter().filter(|&&c| shape.value.eval_mod(c, m_of(c)).map_or(false, |vm| !predicate(shape.pred, vm))).count();
    Judged { shape: shape.clone(), fit_wrong: wrong, fit_reached: fit, test_right: right, test_n: (test - fit) as usize, primes: (t.0, t.1), composites: (t.2, t.3), liars_caught: liars, cost_bits: shape.value.cost(test) / (test as f64).log2(), constant_bits: 0.0, a: None }
}

fn find_floor_a(v: &V) -> Option<(&V, ())> {
    match v {
        V::FloorA(h) => Some((h, ())),
        V::Add(a, b) | V::Mul(a, b) | V::PowMod(a, b) => find_floor_a(a).or_else(|| find_floor_a(b)),
        V::Dec(a) | V::Fact(a) => find_floor_a(a),
        _ => None,
    }
}

pub fn report(size: usize, fit: u64, test: u64) -> String {
    let t0 = std::time::Instant::now();
    let all = shapes(size);
    let mut out = String::new();
    out.push_str("Operation Fermented Fish, the checker: a shape c(A, N) and a number A, true exactly when N is prime\n\n");
    out.push_str("question: a checker invented as the generator was, a shape and a constant; is any one right on numbers it never saw, with a short constant, cheaper than a multiplication per bit?\n");
    out.push_str(&format!("the choice: every value of up to {size} symbols from N, 2, 3, floor(A h), +, x, -1, !, a^e mod N, with the predicates = 0 mod N, = 1 mod N, odd: {} shapes; A solved on N = 2..{fit}, judged on {}..{test} and {} Carmichael liars\n\n", all.len(), fit + 1, CARMICHAEL.len()));
    let mut judged: Vec<Judged> = all.iter().map(|s| judge(s, fit, test)).collect();
    // the measure: right on the unseen primes and right on the unseen composites, both (a coin or a constant answer gets a half)
    let balanced = |j: &Judged| -> f64 {
        let pr = if j.primes.1 > 0 { j.primes.0 as f64 / j.primes.1 as f64 } else { 0.0 };
        let cr = if j.composites.1 > 0 { j.composites.0 as f64 / j.composites.1 as f64 } else { 0.0 };
        (pr + cr) / 2.0
    };
    judged.sort_by(|a, b| balanced(b).partial_cmp(&balanced(a)).unwrap().then(a.cost_bits.partial_cmp(&b.cost_bits).unwrap()));
    out.push_str(&format!("  {:<30} {:<9} {:>9} {:>13} {:>13} {:>7} {:>9} {:>7}\n", "shape", "predicate", "fit wrong", "unseen primes", "unseen comp.", "liars", "cost/bit", "A bits"));
    for j in judged.iter().take(20) {
        let pred = match j.shape.pred { Pred::ZeroModN => "= 0 mod N", Pred::OneModN => "= 1 mod N", Pred::Odd => "odd" };
        let fitc = if j.a.is_some() { format!("to N={}", j.fit_reached) } else { j.fit_wrong.to_string() };
        out.push_str(&format!("  {:<30} {:<9} {:>9} {:>13} {:>13} {:>7} {:>9.2} {:>7.0}\n", j.shape.value.show(), pred, fitc, format!("{}/{}", j.primes.0, j.primes.1), format!("{}/{}", j.composites.0, j.composites.1), format!("{}/{}", j.liars_caught, CARMICHAEL.len()), j.cost_bits, j.constant_bits));
    }
    let best = judged.iter().filter(|j| j.a.is_none()).max_by(|a, b| balanced(a).partial_cmp(&balanced(b)).unwrap());
    let cheapest_exact = judged.iter().filter(|j| j.a.is_none() && j.primes.0 == j.primes.1 && j.composites.0 == j.composites.1 && j.primes.1 > 0).min_by(|a, b| a.cost_bits.partial_cmp(&b.cost_bits).unwrap());
    out.push_str(&format!("\nverdict ({:.0} s): ", t0.elapsed().as_secs_f64()));
    match (best, cheapest_exact) {
        (_, Some(e)) => out.push_str(&format!("{} [{:?}] is right on every unseen number ({} primes, {} composites) at {:.2} multiplications per bit; the cheapest exact checker found", e.shape.value.show(), e.shape.pred, e.primes.1, e.composites.1, e.cost_bits)),
        (Some(b), None) => out.push_str(&format!("no shape is right on every unseen number; the best, {} [{:?}], is right on {}/{} unseen primes and {}/{} composites, catching {}/{} liars, at {:.2} multiplications per bit", b.shape.value.show(), b.shape.pred, b.primes.0, b.primes.1, b.composites.0, b.composites.1, b.liars_caught, CARMICHAEL.len(), b.cost_bits)),
        (None, None) => out.push_str("no shape without a constant fits the range"),
    }
    let tables: Vec<&Judged> = judged.iter().filter(|j| j.a.is_some()).collect();
    if let Some(t) = tables.iter().max_by_key(|j| j.fit_reached) {
        out.push_str(&format!("; the shapes with a constant are tables: the best, {}, holds to N = {} and needs {:.0} bits of A for it (a bit a number), then {}/{} unseen primes and {}/{} composites right, a coin's share", t.shape.value.show(), t.fit_reached, t.constant_bits, t.primes.0, t.primes.1, t.composites.0, t.composites.1));
    }
    out.push_str(".\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fermat_and_wilson_are_in_the_grammar_and_judged_right() {
        let fit = 300;
        let fermat = Shape { value: V::PowMod(Box::new(V::C(2)), Box::new(V::Dec(Box::new(V::N)))), pred: Pred::OneModN };
        let j = judge(&fermat, fit, 2000);
        // 2^(N-1) = 1 mod N: right on primes, wrong on the base-2 pseudoprimes 341, 561, 645, 1105, 1387, 1729, 1905 ... and on N = 2
        assert!(j.fit_wrong >= 1 && j.fit_wrong <= 3, "{}", j.fit_wrong);
        let wilson = Shape { value: V::Dec(Box::new(V::Fact(Box::new(V::Dec(Box::new(V::N)))))), pred: Pred::ZeroModN };
        // (N-1)! - 1 = 0 mod N is not Wilson; (N-1)! + 1 is: built as ((N-1)! + 1)
        let wilson = Shape { value: V::Add(Box::new(V::Fact(Box::new(V::Dec(Box::new(V::N))))), Box::new(V::Dec(Box::new(V::C(2))))), pred: Pred::ZeroModN };
        let j = judge(&wilson, fit, 1000);
        assert_eq!(j.fit_wrong, 0, "Wilson");
        assert_eq!(j.test_right, 700);
        let _ = wilson;
        assert!(shapes(4).iter().any(|s| s.value.show() == "2^(N - 1) mod N"));
    }

    #[test]
    fn the_constant_shape_is_a_table() {
        // floor(A 2^N) odd exactly when N is prime: A exists (the prime constant) and costs a bit a number
        let s = Shape { value: V::FloorA(Box::new(V::PowMod(Box::new(V::C(2)), Box::new(V::N)))), pred: Pred::Odd };
        let _ = s;
        let table = Shape { value: V::FloorA(Box::new(V::Exp(Box::new(V::N)))), pred: Pred::Odd };
        let j = judge(&table, 60, 200);
        assert!(j.a.is_some());
        assert_eq!(j.fit_reached, 60, "the prime constant fits every N");
        assert!(j.constant_bits >= 50.0, "{}", j.constant_bits);
        // and it knows nothing past the fit: about half right on the unseen numbers is what a coin does
        assert!(j.test_right < 130, "{}", j.test_right);
    }
}
