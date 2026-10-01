//! Mersenne primes and the EFF prize for a prime of 100 million digits.
//!
//! Lucas-Lehmer: for an odd prime p, M = 2^p - 1 is prime exactly when
//! s_{p-2} = 0 (mod M), where s_0 = 4 and s_{i+1} = s_i^2 - 2. Squaring is
//! done by a number-theoretic transform over the prime 2^64 - 2^32 + 1, on
//! 16-bit digits: exact, no rounding. Reduction mod 2^p - 1 folds the bits
//! above p back onto the bottom, since 2^p = 1 there.

const P: u64 = 0xFFFF_FFFF_0000_0001;

fn reduce(x: u128) -> u64 {
    // 2^64 = 2^32 - 1 and 2^96 = -1 (mod P)
    let (lo, hi) = (x as u64, (x >> 64) as u64);
    let (hi_hi, hi_lo) = (hi >> 32, hi & 0xFFFF_FFFF);
    let (mut t, borrow) = lo.overflowing_sub(hi_hi);
    if borrow {
        t = t.wrapping_sub(0xFFFF_FFFF);
    }
    let (mut r, carry) = t.overflowing_add(hi_lo * 0xFFFF_FFFF);
    if carry {
        r = r.wrapping_add(0xFFFF_FFFF);
    }
    if r >= P {
        r - P
    } else {
        r
    }
}

fn mul(a: u64, b: u64) -> u64 {
    reduce(a as u128 * b as u128)
}

fn add(a: u64, b: u64) -> u64 {
    let (s, c) = a.overflowing_add(b);
    let s = if c { s.wrapping_add(0xFFFF_FFFF) } else { s };
    if s >= P {
        s - P
    } else {
        s
    }
}

fn sub(a: u64, b: u64) -> u64 {
    if a >= b {
        a - b
    } else {
        P - (b - a)
    }
}

fn pow(mut b: u64, mut e: u64) -> u64 {
    let mut r = 1;
    while e > 0 {
        if e & 1 == 1 {
            r = mul(r, b);
        }
        b = mul(b, b);
        e >>= 1;
    }
    r
}

/// In-place NTT of length a power of two (the inverse includes 1/n).
fn ntt(a: &mut [u64], inverse: bool) {
    let n = a.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        // 7 generates the multiplicative group
        let mut w = pow(7, (P - 1) / len as u64);
        if inverse {
            w = pow(w, P - 2);
        }
        let half = len / 2;
        let mut ws = Vec::with_capacity(half);
        let mut x = 1;
        for _ in 0..half {
            ws.push(x);
            x = mul(x, w);
        }
        for start in (0..n).step_by(len) {
            for k in 0..half {
                let u = a[start + k];
                let v = mul(a[start + k + half], ws[k]);
                a[start + k] = add(u, v);
                a[start + k + half] = sub(u, v);
            }
        }
        len <<= 1;
    }
    if inverse {
        let inv = pow(n as u64, P - 2);
        for x in a.iter_mut() {
            *x = mul(*x, inv);
        }
    }
}

/// A residue mod 2^p - 1 as 16-bit digits, least significant first.
pub struct Residue {
    p: usize,
    digits: Vec<u64>,
    size: usize,
}

impl Residue {
    pub fn new(p: usize, value: u64) -> Residue {
        let count = p.div_ceil(16);
        let mut digits = vec![0u64; count];
        let mut v = value;
        for d in digits.iter_mut() {
            *d = v & 0xFFFF;
            v >>= 16;
        }
        let size = (2 * count).next_power_of_two();
        Residue { p, digits, size }
    }
    /// Bits start.. start+16 of a 16-bit-digit number.
    fn bits16(d: &[u64], start: usize) -> u64 {
        let (i, s) = (start / 16, start % 16);
        let lo = d.get(i).copied().unwrap_or(0) >> s;
        let hi = if s == 0 { 0 } else { d.get(i + 1).copied().unwrap_or(0) << (16 - s) };
        (lo | hi) & 0xFFFF
    }
    /// x mod 2^p - 1 for a number of up to 2p bits, as 16-bit digits.
    fn fold(&mut self, wide: &[u64]) {
        let (p, count) = (self.p, self.digits.len());
        let top = p % 16;
        let mut out: Vec<u64> = (0..count).map(|i| {
            let low = wide.get(i).copied().unwrap_or(0);
            if i + 1 == count && top != 0 { low & ((1 << top) - 1) } else { low }
        }).collect();
        let mut carry = 0u64;
        for (i, o) in out.iter_mut().enumerate() {
            let s = *o + Self::bits16(wide, p + 16 * i) + carry;
            *o = s & 0xFFFF;
            carry = s >> 16;
        }
        // bits past p (the top digit's spill and the carry) wrap to the bottom
        loop {
            let spill = if top != 0 { (out[count - 1] >> top) + (carry << (16 - top)) } else { carry };
            if spill == 0 {
                break;
            }
            if top != 0 {
                out[count - 1] &= (1 << top) - 1;
            }
            let mut add_in = spill;
            for o in out.iter_mut() {
                if add_in == 0 {
                    break;
                }
                let s = *o + add_in;
                *o = s & 0xFFFF;
                add_in = s >> 16;
            }
            carry = add_in;
        }
        self.digits = out;
    }
    /// s -> s^2 - 2 (mod 2^p - 1).
    pub fn step(&mut self) {
        let mut a = vec![0u64; self.size];
        a[..self.digits.len()].copy_from_slice(&self.digits);
        ntt(&mut a, false);
        for x in a.iter_mut() {
            *x = mul(*x, *x);
        }
        ntt(&mut a, true);
        // carries into 16-bit digits
        let mut wide = Vec::with_capacity(a.len() + 4);
        let mut carry: u128 = 0;
        for &x in &a {
            let s = x as u128 + carry;
            wide.push((s & 0xFFFF) as u64);
            carry = s >> 16;
        }
        while carry > 0 {
            wide.push((carry & 0xFFFF) as u64);
            carry >>= 16;
        }
        self.fold(&wide);
        self.minus_two();
    }
    fn minus_two(&mut self) {
        let mut borrow = 2u64;
        for d in self.digits.iter_mut() {
            if borrow == 0 {
                break;
            }
            if *d >= borrow {
                *d -= borrow;
                borrow = 0;
            } else {
                *d = *d + 0x10000 - borrow;
                borrow = 1;
            }
        }
        if borrow > 0 {
            // went below 0: add 2^p - 1, i.e. subtract 1 more and keep p bits
            let mut b = 1u64;
            for d in self.digits.iter_mut() {
                if b == 0 {
                    break;
                }
                if *d >= b {
                    *d -= b;
                    b = 0;
                } else {
                    *d = *d + 0x10000 - b;
                }
            }
            let top = self.p % 16;
            if top != 0 {
                let last = self.digits.len() - 1;
                self.digits[last] &= (1 << top) - 1;
            }
        }
    }
    /// 0 mod 2^p - 1 (the digits are 0, or all p bits are 1).
    pub fn is_zero(&self) -> bool {
        if self.digits.iter().all(|&d| d == 0) {
            return true;
        }
        let top = self.p % 16;
        self.digits.iter().enumerate().all(|(i, &d)| d == if i + 1 == self.digits.len() && top != 0 { (1 << top) - 1 } else { 0xFFFF })
    }
}

/// Is 2^p - 1 prime? (p prime)
pub fn lucas_lehmer(p: usize) -> bool {
    if p == 2 {
        return true;
    }
    let mut s = Residue::new(p, 4);
    for _ in 0..p - 2 {
        s.step();
    }
    s.is_zero()
}

fn is_prime(n: u64) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0)
}

/// The Mersenne prime exponents known below 5000 (OEIS A000043).
pub const KNOWN: [usize; 20] = [2, 3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127, 521, 607, 1279, 2203, 2281, 3217, 4253, 4423];

pub struct Settings {
    pub check_below: usize,
    pub timed_steps: usize,
    pub factor_exponents: usize,
    pub factor_bits: u32,
}

/// 2^e mod q, square-and-multiply from the top bit (q < 2^63).
pub(crate) fn pow2_mod(e: u64, q: u64) -> u64 {
    let mut r = 1u64;
    for i in (0..64 - e.leading_zeros()).rev() {
        r = ((r as u128 * r as u128) % q as u128) as u64;
        if e >> i & 1 == 1 {
            r = if r >= q - r { r - (q - r) } else { 2 * r };
        }
    }
    r
}

/// 2^e mod q again, from the bottom bit with a separate running power (an
/// independent recheck of a factor).
fn pow2_mod_check(mut e: u64, q: u64) -> u64 {
    let (mut r, mut b) = (1u128, 2u128 % q as u128);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % q as u128;
        }
        b = b * b % q as u128;
        e >>= 1;
    }
    r as u64
}

/// The smallest factor q < 2^bits of 2^p - 1 (p an odd prime), if any. Every
/// factor has the form q = 2kp + 1 with q = 1 or 7 mod 8 (proved in the
/// general part), so only those q are tried; q with a small prime factor are
/// skipped, since that prime would be a smaller factor of the same form.
pub fn trial_factor(p: u64, bits: u32) -> Option<u64> {
    const SMALL: [u64; 24] = [3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97];
    let limit = 1u64 << bits;
    let mut k = 1u64;
    loop {
        let q = 2 * k * p + 1;
        if q >= limit {
            return None;
        }
        k += 1;
        if (q % 8 != 1 && q % 8 != 7) || SMALL.iter().any(|&s| q % s == 0 && q != s) {
            continue;
        }
        if pow2_mod(p, q) == 1 {
            return Some(q);
        }
    }
}

/// The first `count` prime exponents with 100 million digits, each with its
/// smallest factor below 2^bits if trial factoring finds one (rechecked).
pub fn candidates(count: usize, bits: u32) -> Vec<(u64, Option<u64>)> {
    let log2 = std::f64::consts::LOG10_2;
    let first = ((1e8 - 1.0) / log2).ceil() as u64;
    let exps: Vec<u64> = (first..).filter(|&q| is_prime(q) && ((q as f64 * log2).floor() as u64 + 1) >= 100_000_000).take(count).collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots = std::sync::Mutex::new(vec![None; exps.len()]);
    std::thread::scope(|sc| {
        for _ in 0..std::thread::available_parallelism().map_or(4, |t| t.get()) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if i >= exps.len() {
                    break;
                }
                let f = trial_factor(exps[i], bits).filter(|&q| pow2_mod_check(exps[i], q) == 1);
                slots.lock().expect("no panics")[i] = f;
            });
        }
    });
    exps.into_iter().zip(slots.into_inner().expect("done")).collect()
}

pub fn report(s: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    // 1. find every Mersenne prime exponent below the limit
    let candidates: Vec<usize> = (2..s.check_below).filter(|&p| is_prime(p as u64)).collect();
    let found: Vec<usize> = {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let hits = std::sync::Mutex::new(Vec::new());
        std::thread::scope(|sc| {
            for _ in 0..std::thread::available_parallelism().map_or(4, |t| t.get()) {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= candidates.len() {
                        break;
                    }
                    // largest first, so the long ones start early
                    let p = candidates[candidates.len() - 1 - i];
                    if lucas_lehmer(p) {
                        hits.lock().expect("no panics").push(p);
                    }
                });
            }
        });
        let mut h = hits.into_inner().expect("done");
        h.sort();
        h
    };
    let known: Vec<usize> = KNOWN.iter().copied().filter(|&p| p < s.check_below).collect();
    out.push(format!(
        "Lucas-Lehmer on every prime exponent p < {} ({} of them), squaring by an exact number-theoretic transform: 2^p - 1 is prime for p = {:?}: {}",
        s.check_below,
        candidates.len(),
        found,
        if found == known { "exactly the known Mersenne primes (OEIS A000043)" } else { "DIFFERENT from the known list" }
    ));
    // 2. the prize: the smallest prime exponent with 100 million digits
    let log2 = std::f64::consts::LOG10_2;
    let first = ((1e8 - 1.0) / log2).ceil() as u64;
    let p = (first..).find(|&q| is_prime(q) && ((q as f64 * log2).floor() as u64 + 1) >= 100_000_000).expect("a prime");
    out.push(format!(
        "the EFF prize ($150,000, a prime with at least 100 million digits): 2^p - 1 has 100 million digits from p = {first}; the first prime exponent is p = {p} ({} digits)",
        (p as f64 * log2).floor() as u64 + 1
    ));
    // 2b. trial factoring of the first prize-size prime exponents: a factor
    // settles 2^p - 1 as composite in a fraction of a second
    if s.factor_exponents > 0 {
        let t0 = std::time::Instant::now();
        let exps: Vec<u64> = (p..).filter(|&q| is_prime(q)).take(s.factor_exponents).collect();
        let results: Vec<Option<u64>> = {
            let next = std::sync::atomic::AtomicUsize::new(0);
            let slots = std::sync::Mutex::new(vec![None; exps.len()]);
            std::thread::scope(|sc| {
                for _ in 0..std::thread::available_parallelism().map_or(4, |t| t.get()) {
                    sc.spawn(|| loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if i >= exps.len() {
                            break;
                        }
                        let f = trial_factor(exps[i], s.factor_bits);
                        slots.lock().expect("no panics")[i] = f;
                    });
                }
            });
            slots.into_inner().expect("done")
        };
        let factored: Vec<(u64, u64)> = exps.iter().zip(&results).filter_map(|(&e, f)| f.map(|q| (e, q))).collect();
        let rechecked = factored.iter().all(|&(e, q)| pow2_mod_check(e, q) == 1 && (q - 1) % (2 * e) == 0);
        let survivors: Vec<u64> = exps.iter().zip(&results).filter(|(_, f)| f.is_none()).map(|(&e, _)| e).collect();
        // heuristic (Wagstaff): the chance of a factor between 2^a and 2^b is about 1 - a/b, with 2^a = 2p the smallest possible
        let a = ((2 * p + 1) as f64).log2();
        let expected = 1.0 - a / s.factor_bits as f64;
        out.push(format!(
            "trial factoring, the first {} prime exponents from p = {p}, every factor 2kp + 1 below 2^{} ({:.1} s): {} of the {} numbers 2^p - 1 have a factor, so they are proved composite (each factor rechecked by a second power routine: {}); expected about {:.0}% by the 1 - a/b heuristic, found {:.0}%",
            exps.len(),
            s.factor_bits,
            t0.elapsed().as_secs_f64(),
            factored.len(),
            exps.len(),
            if rechecked { "all agree" } else { "DISAGREE" },
            100.0 * expected,
            100.0 * factored.len() as f64 / exps.len() as f64
        ));
        out.push(format!(
            "  first factors found: {}",
            factored.iter().take(6).map(|(e, q)| format!("2^{e} - 1 = {q} * ...")).collect::<Vec<_>>().join("; ")
        ));
        out.push(format!(
            "  {} survive, each needs a full Lucas-Lehmer test; the first: {}",
            survivors.len(),
            survivors.iter().take(8).map(|e| e.to_string()).collect::<Vec<_>>().join(", ")
        ));
    }
    // 3. time squarings at a sixteenth of that size (a full-size transform
    // needs over a gigabyte) and scale by n log n
    let small = ((p / 16)..).find(|&q| is_prime(q)).expect("a prime") as usize;
    let t0 = std::time::Instant::now();
    let mut r = Residue::new(small, 4);
    for _ in 0..s.timed_steps {
        r.step();
    }
    let per_small = t0.elapsed().as_secs_f64() / s.timed_steps as f64;
    let (n_small, n_big) = (Residue::new(small, 4).size as f64, (2 * (p as usize).div_ceil(16)).next_power_of_two() as f64);
    let per = per_small * (n_big * n_big.log2()) / (n_small * n_small.log2());
    let years = per * (p - 2) as f64 / (365.25 * 86400.0);
    // heuristic: about e^gamma = 1.78 Mersenne primes per doubling of the exponent
    let exponents = (p as f64) / (p as f64).ln();
    let tests = exponents / 1.78 / 2.0; // about half survive trial factoring
    out.push(format!(
        "timed: one squaring mod 2^{small} - 1 took {per_small:.2} s here (one thread); scaled by n log n to p = {p}, one squaring takes about {per:.1} s, so one Lucas-Lehmer test ({} squarings) takes about {years:.0} years; heuristically about 1.8 Mersenne primes lie between p and 2p, among about {:.1e} prime exponents, so on the order of {tests:.0e} tests per prime found",
        p - 2,
        exponents
    ));
    out.push(format!(
        "so this machine cannot win it: about {years:.0} years for one test, and millions of tests expected. The GIMPS project (thousands of computers, and code about a hundred times faster per test, using irrational-base FFTs on graphics cards) has not found a 100-million-digit prime yet"
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntt_squares_exactly() {
        let mut a = vec![0u64; 8];
        a[0] = 0xFFFF;
        a[1] = 0x1234;
        ntt(&mut a, false);
        for x in a.iter_mut() {
            *x = mul(*x, *x);
        }
        ntt(&mut a, true);
        // (0x1234 * 2^16 + 0xFFFF)^2 by digits: a0^2, 2 a0 a1, a1^2
        assert_eq!(&a[..3], &[0xFFFF * 0xFFFF, 2 * 0xFFFF * 0x1234, 0x1234 * 0x1234]);
    }

    #[test]
    fn known_mersenne_primes() {
        for p in [3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127, 521, 607] {
            assert!(lucas_lehmer(p), "{p}");
        }
        for p in [11, 23, 29, 37, 41, 43, 47, 53, 59, 67, 257] {
            assert!(!lucas_lehmer(p), "{p}");
        }
    }
}
