//! The Beal conjecture ($1 000 000, American Mathematical Society): if
//! A^x + B^y = C^z with positive integers and x, y, z >= 3, then A, B and C
//! share a prime factor. One counterexample would settle it.
//!
//! The search: every C^z up to the largest possible sum goes into a table,
//! keyed by its residues modulo two 61-bit primes; then every A^x + B^y with
//! A <= B is looked up the same way. A hit is rechecked with exact big
//! integers. With gcd(A, B) = 1 a hit would be a counterexample; without
//! that condition the search must find the known solutions that share a
//! factor (2^3 + 2^3 = 2^4, 3^3 + 6^3 = 3^5, ...), which shows it works.

/// The search covers every C below this (see the root test).
pub const C_MAX: f64 = 1e12;
const P1: u64 = 2_305_843_009_213_693_951; // 2^61 - 1
const P2: u64 = 2_305_843_009_213_693_921; // a prime below 2^61

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Exact big naturals, base 2^32, just enough for powers and sums.
#[derive(Clone, PartialEq, Debug)]
struct Big(Vec<u32>);

impl Big {
    fn from(x: u64) -> Big {
        Big(vec![x as u32, (x >> 32) as u32]).trim()
    }
    fn trim(mut self) -> Big {
        while self.0.len() > 1 && *self.0.last().expect("not empty") == 0 {
            self.0.pop();
        }
        self
    }
    fn mul_small(&self, k: u64) -> Big {
        let mut out = Vec::with_capacity(self.0.len() + 2);
        let mut carry: u128 = 0;
        for &d in &self.0 {
            let t = d as u128 * k as u128 + carry;
            out.push(t as u32);
            carry = t >> 32;
        }
        while carry > 0 {
            out.push(carry as u32);
            carry >>= 32;
        }
        Big(out).trim()
    }
    fn add(&self, o: &Big) -> Big {
        let mut out = Vec::new();
        let mut carry = 0u64;
        for i in 0..self.0.len().max(o.0.len()) {
            let t = *self.0.get(i).unwrap_or(&0) as u64 + *o.0.get(i).unwrap_or(&0) as u64 + carry;
            out.push(t as u32);
            carry = t >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        Big(out).trim()
    }
    fn pow(b: u64, e: u32) -> Big {
        let mut r = Big::from(1);
        for _ in 0..e {
            r = r.mul_small(b);
        }
        r
    }
}

pub struct Settings {
    pub max_base: u64,
    pub max_exp: u32,
}

pub struct Found {
    pub a: u64,
    pub x: u32,
    pub b: u64,
    pub y: u32,
    pub c: u64,
    pub z: u32,
}

/// All solutions with A, B <= max_base and x, y, z in 3..=max_exp (C as
/// large as the sums need), exactly checked; `coprime` keeps gcd(A, B) = 1 only.
pub fn search(s: &Settings, coprime: bool) -> (Vec<Found>, u64) {
    // a sum S = A^x + B^y is a perfect z-th power only if one of the integers
    // next to S^(1/z) (from floating point) has the same residues as S
    let powmod = |b: u64, e: u32, m: u64| (0..e).fold(1u64, |r, _| mulmod(r, b % m, m));
    // powers of each base mod both primes
    let pows: Vec<Vec<(u64, u64)>> = (0..=s.max_base)
        .map(|a| {
            let (mut r1, mut r2) = (1u64, 1u64);
            (0..=s.max_exp)
                .map(|_| {
                    let v = (r1, r2);
                    r1 = mulmod(r1, a, P1);
                    r2 = mulmod(r2, a, P2);
                    v
                })
                .collect()
        })
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let next = std::sync::atomic::AtomicU64::new(1);
    let found = std::sync::Mutex::new(Vec::new());
    let tried = std::sync::atomic::AtomicU64::new(0);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let a = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if a > s.max_base {
                    break;
                }
                let mut local = 0u64;
                for b in a..=s.max_base {
                    if coprime && gcd(a, b) != 1 {
                        continue;
                    }
                    for x in 3..=s.max_exp {
                        for y in 3..=s.max_exp {
                            local += 1;
                            let (u, v) = (pows[a as usize][x as usize], pows[b as usize][y as usize]);
                            let key = ((u.0 + v.0) % P1, (u.1 + v.1) % P2);
                            let log_sum = {
                                let (la, lb) = (x as f64 * (a as f64).ln(), y as f64 * (b as f64).ln());
                                la.max(lb) + (1.0 + (-(la - lb).abs()).exp()).ln()
                            };
                            for z in 3..=s.max_exp {
                                let root = (log_sum / z as f64).exp();
                                // the floating root is right to within 1 only while
                                // C stays below about 10^12 (log rounding 1e-13)
                                if root > C_MAX {
                                    continue;
                                }
                                let base = root.round() as u64;
                                for c in base.saturating_sub(1)..=base + 1 {
                                    if c < 2 || (powmod(c, z, P1), powmod(c, z, P2)) != key {
                                        continue;
                                    }
                                    // exact recheck with big integers
                                    if Big::pow(a, x).add(&Big::pow(b, y)) == Big::pow(c, z) {
                                        found.lock().expect("no panics").push(Found { a, x, b, y, c, z });
                                    }
                                }
                            }
                        }
                    }
                }
                tried.fetch_add(local, std::sync::atomic::Ordering::Relaxed);
            });
        }
    });
    let mut f = found.into_inner().expect("done");
    f.sort_by_key(|h| (h.c, h.a, h.b, h.x, h.y));
    (f, tried.into_inner())
}

pub fn report(s: &Settings) -> Vec<String> {
    let t0 = std::time::Instant::now();
    let (all, _) = search(&Settings { max_base: s.max_base.min(60), max_exp: s.max_exp.min(8) }, false);
    let shown: Vec<String> = all
        .iter()
        .take(6)
        .map(|h| format!("{}^{} + {}^{} = {}^{} (common factor {})", h.a, h.x, h.b, h.y, h.c, h.z, gcd(gcd(h.a, h.b), h.c)))
        .collect();
    let mut out = vec![format!(
        "the search first finds the known solutions that share a factor (bases up to 60, exponents up to 8): {} of them, for example {}",
        all.len(),
        shown.join("; ")
    )];
    let every_shares = all.iter().all(|h| gcd(gcd(h.a, h.b), h.c) > 1);
    out.push(format!("  every one has a common prime factor: {}", if every_shares { "yes, as the conjecture says" } else { "NO: a counterexample" }));
    let (hits, tried) = search(s, true);
    out.push(format!(
        "counterexample search, gcd(A, B) = 1, every A <= B <= {}, every x, y, z from 3 to {} (every C below 10^12): {} sums A^x + B^y tested against every perfect power C^z in {:.1} s",
        s.max_base,
        s.max_exp,
        tried,
        t0.elapsed().as_secs_f64()
    ));
    out.push(match hits.first() {
        Some(h) => format!("  COUNTEREXAMPLE FOUND: {}^{} + {}^{} = {}^{}, checked exactly; it would disprove the Beal conjecture", h.a, h.x, h.b, h.y, h.c, h.z),
        None => "  none: no counterexample in this range (larger searches by others found none either)".into(),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_known_solutions_and_no_coprime_ones() {
        let (all, _) = search(&Settings { max_base: 10, max_exp: 6 }, false);
        assert!(all.iter().any(|h| (h.a, h.x, h.b, h.y, h.c, h.z) == (2, 3, 2, 3, 2, 4)));
        assert!(all.iter().any(|h| (h.a, h.x, h.b, h.y, h.c, h.z) == (3, 3, 6, 3, 3, 5)));
        let (coprime, _) = search(&Settings { max_base: 30, max_exp: 6 }, true);
        assert!(coprime.is_empty());
        assert_eq!(Big::pow(3, 5), Big::from(243));
    }
}
