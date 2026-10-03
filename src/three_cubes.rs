//! Sums of three cubes: x^3 + y^3 + z^3 = k.
//!
//! A solution is checked in an instant (three cubes and a sum); finding one
//! can take a planet's worth of computers. Below 1000 the open k are 114,
//! 390, 627, 633, 732, 921 and 975 (k = 4 or 5 mod 9 has no solution at all).
//!
//! The search is Booker's (2019). Let z be the coordinate smallest in size
//! and d = x + y. Then x^3 + y^3 = d (x^2 - xy + y^2) = k - z^3, so d divides
//! k - z^3, that is z^3 = k (mod d); x and y have opposite signs, which
//! gives 3 d z^2 <= |z|^3 + k, so 0 < d <= (|z| + k / z^2) / 3 (a negative
//! d is the same as a positive one for -k, so both signs of k are run).
//! Given d and z, x and y are the roots of t^2 - d t + (d^2 - q)/3 with
//! q = (k - z^3)/d: a solution exists exactly when (4q - d^2)/3 is a
//! perfect square s^2 with s = d (mod 2), and then x, y = (d +- s)/2.
//! So for every d up to B/3 the z with z^3 = k (mod d) are walked, one
//! residue class at a time, down to -B: every solution whose smallest
//! coordinate is at most B in size is found, in time about B log B
//! (except d = 0, the family t^3 + (-t)^3 + c^3 = c^3 when k is a cube).

use num_bigint::BigInt;

/// Known solutions, as published (each is checked by the tests).
pub const KNOWN: [(i64, [&str; 3], &str); 4] = [
    (3, ["569936821221962380720", "-569936821113563493509", "-472715493453327032"], "Booker, Sutherland 2019"),
    (30, ["2220422932", "-2218888517", "-283059965"], "Beck, Pine, Tarrant, Yarbrough Jensen 1999"),
    (33, ["8866128975287528", "-8778405442862239", "-2736111468807040"], "Booker 2019"),
    (42, ["-80538738812075974", "80435758145817515", "12602123297335631"], "Booker, Sutherland 2019"),
];

/// The open k below 1000.
pub const OPEN: [i64; 7] = [114, 390, 627, 633, 732, 921, 975];

/// Does x^3 + y^3 + z^3 equal k? Exact, any size.
pub fn check(k: i64, xyz: [&str; 3]) -> Option<bool> {
    let mut sum = BigInt::from(0);
    for v in xyz {
        let v: BigInt = v.parse().ok()?;
        sum += &v * &v * &v;
    }
    Some(sum == BigInt::from(k))
}

/// k = 4 or 5 (mod 9) has no solution: cubes are 0, 1 or -1 mod 9.
pub fn possible(k: i64) -> bool {
    !matches!(k.rem_euclid(9), 4 | 5)
}

fn pow_mod(b: u64, mut e: u64, m: u64) -> u64 {
    let (mut r, mut b, m) = (1u128, (b % m) as u128, m as u128);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % m;
        }
        b = b * b % m;
        e >>= 1;
    }
    r as u64
}

fn inv_mod(a: u64, m: u64) -> u64 {
    let (mut t, mut nt, mut r, mut nr) = (0i128, 1i128, m as i128, a as i128);
    while nr != 0 {
        let q = r / nr;
        (t, nt) = (nt, t - q * nt);
        (r, nr) = (nr, r - q * nr);
    }
    t.rem_euclid(m as i128) as u64
}

/// The cube roots of a modulo a prime p (a not divisible by p, p > 3).
fn cube_roots_prime(a: u64, p: u64) -> Vec<u64> {
    let a = a % p;
    if p % 3 == 2 {
        // cubing is a bijection: the inverse of 3 mod p - 1 is (2p - 1)/3
        return vec![pow_mod(a, (2 * p - 1) / 3, p)];
    }
    if pow_mod(a, (p - 1) / 3, p) != 1 {
        return vec![];
    }
    // p - 1 = 3^s t with 3 not dividing t (Adleman-Manders-Miller)
    let (mut s, mut t) = (0u32, p - 1);
    while t % 3 == 0 {
        t /= 3;
        s += 1;
    }
    let c = (2..p).find(|&c| pow_mod(c, (p - 1) / 3, p) != 1).expect("a non-cube");
    let g = pow_mod(c, t, p); // generates the 3-part, order 3^s
    // x0 = a^e with 3e = 1 (mod t): x0^3 = a w, w in the 3-part
    let e = if t == 1 { 0 } else { inv_mod(3 % t, t) };
    let x0 = pow_mod(a, e, p);
    let w = (x0 as u128 * x0 as u128 % p as u128 * x0 as u128 % p as u128 * inv_mod(a, p) as u128 % p as u128) as u64;
    // discrete log of w^-1 base g, digit by digit (Pohlig-Hellman)
    let target = inv_mod(w, p);
    let gamma = pow_mod(g, 3u64.pow(s - 1), p);
    let g_inv = inv_mod(g, p);
    let mut l = 0u64;
    for i in 0..s {
        let y = (pow_mod(g_inv, l, p) as u128 * target as u128 % p as u128) as u64;
        let h = pow_mod(y, 3u64.pow(s - 1 - i), p);
        let digit = (0..3u64).find(|&dd| pow_mod(gamma, dd, p) == h).expect("digit");
        l += digit * 3u64.pow(i);
    }
    // w^-1 is a cube in the 3-part, so 3 divides l
    let x = (x0 as u128 * pow_mod(g, l / 3, p) as u128 % p as u128) as u64;
    let omega = gamma; // a primitive cube root of 1
    let x1 = (x as u128 * omega as u128 % p as u128) as u64;
    let x2 = (x1 as u128 * omega as u128 % p as u128) as u64;
    vec![x, x1, x2]
}

/// The cube roots of k modulo p^e: roots mod p, then each lifted by trying
/// the p ways to extend it (only small p appear with e > 1).
fn cube_roots_prime_power(k: i64, p: u64, e: u32) -> Vec<u64> {
    let kp = k.rem_euclid(p as i64) as u64;
    let mut roots: Vec<u64> = if p <= 3 || kp == 0 || p < 64 {
        (0..p).filter(|&r| (r as u128).pow(3) % p as u128 == kp as u128).collect()
    } else {
        cube_roots_prime(kp, p)
    };
    let mut m = p;
    for _ in 1..e {
        let next = m * p;
        let kn = k.rem_euclid(next as i64) as u128;
        roots = roots
            .iter()
            .flat_map(|&r| (0..p).map(move |j| r + j * m))
            .filter(|&r| (r as u128).pow(3) % next as u128 == kn)
            .collect();
        m = next;
        if roots.is_empty() {
            break;
        }
    }
    roots
}

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

fn isqrt(n: i128) -> i128 {
    let mut s = (n as f64).sqrt() as i128;
    while s * s > n {
        s -= 1;
    }
    while (s + 1) * (s + 1) <= n {
        s += 1;
    }
    s
}

fn icbrt_floor(n: i128) -> i128 {
    let mut c = (n as f64).cbrt().round() as i128;
    while c * c * c > n {
        c -= 1;
    }
    while (c + 1) * (c + 1) * (c + 1) <= n {
        c += 1;
    }
    c
}

/// Squares mod 64, 63, 65, 11: a quick no before the square root.
struct Squares([bool; 64], [bool; 63], [bool; 65], [bool; 11]);

impl Squares {
    fn new() -> Self {
        let mut s = Squares([false; 64], [false; 63], [false; 65], [false; 11]);
        for i in 0..64 {
            s.0[i * i % 64] = true;
            s.1[i * i % 63] = true;
            s.2[i * i % 65] = true;
            s.3[i * i % 11] = true;
        }
        s.2[64 * 64 % 65] = true;
        s
    }
    fn maybe(&self, n: i128) -> bool {
        self.0[(n & 63) as usize] && self.1[(n % 63) as usize] && self.2[(n % 65) as usize] && self.3[(n % 11) as usize]
    }
}

/// For a given d and one residue r of z mod d: walk z = r (mod d) from the
/// top of the range down to -b and report every (x, y, z).
fn walk(k: i64, d: i64, r: i64, z_top: i128, z_bottom: i128, sq: &Squares, found: &mut Vec<[i128; 3]>) -> u64 {
    let d128 = d as i128;
    // the largest z <= z_top with z = r (mod d)
    let mut z = z_top - (z_top - r as i128).rem_euclid(d128);
    let mut steps = 0;
    while z >= z_bottom {
        steps += 1;
        let m = k as i128 - z * z * z;
        let q = m / d128;
        let t = 4 * q - d128 * d128;
        if t >= 0 && t % 3 == 0 {
            let t3 = t / 3;
            if sq.maybe(t3) {
                let s = isqrt(t3);
                if s * s == t3 && (d128 + s) % 2 == 0 {
                    found.push([(d128 + s) / 2, (d128 - s) / 2, z]);
                }
            }
        }
        z -= d128;
    }
    steps
}

pub struct Search {
    pub k: i64,
    pub bound: i64,
    pub solutions: Vec<[i128; 3]>,
    pub steps: u64,
    pub seconds: f64,
}

/// Every solution of x^3 + y^3 + z^3 = k whose smallest coordinate is at
/// most `bound` in size (the trivial ones too).
pub fn search(k: i64, bound: i64, threads: usize) -> Search {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Mutex;
    assert!(bound <= 2_000_000_000_000, "z^3 must fit in i128 with room to spare");
    let t0 = std::time::Instant::now();
    let d_max = bound / 3 + k.abs() + 1;
    let primes = small_primes(((d_max as f64).sqrt() as u64 + 2).max(10));
    let sq = Squares::new();
    let all = Mutex::new(Vec::new());
    let steps = AtomicU64::new(0);
    // work: small d split by z ranges (most of the work is there), big d in chunks
    const SPLIT_D: i64 = 4096;
    const CHUNK: i64 = 1 << 15;
    let mut tasks: Vec<(i64, i64, i128, i128)> = Vec::new(); // (d_lo, d_hi, z_hi, z_lo); z_lo > z_hi means whole range
    let piece = 200_000_000i128;
    for d in 1..=SPLIT_D.min(d_max) {
        let mut hi = 0i128;
        while hi > -(bound as i128) {
            let lo = (hi - piece * d as i128).max(-(bound as i128));
            tasks.push((d, d + 1, hi, lo));
            hi = lo - 1;
        }
        tasks.push((d, d + 1, i128::MAX, 1)); // the z > 0 part
    }
    let mut lo = SPLIT_D + 1;
    while lo <= d_max {
        tasks.push((lo, (lo + CHUNK).min(d_max + 1), i128::MIN, i128::MIN));
        lo += CHUNK;
    }
    let next = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                let mut found = Vec::new();
                let mut my_steps = 0u64;
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= tasks.len() {
                        break;
                    }
                    let (d_lo, d_hi, z_hi, z_lo) = tasks[i];
                    // factor every d in [d_lo, d_hi) by a small segmented sieve
                    let n = (d_hi - d_lo) as usize;
                    let mut rest: Vec<u64> = (d_lo..d_hi).map(|d| d as u64).collect();
                    let mut factors: Vec<Vec<(u64, u32)>> = vec![Vec::new(); n];
                    for &p in &primes {
                        if p * p >= d_hi as u64 && p as i64 >= d_lo {
                            break;
                        }
                        let start = ((d_lo as u64).div_ceil(p) * p) as i64;
                        let mut m = start;
                        while m < d_hi {
                            let j = (m - d_lo) as usize;
                            let mut e = 0;
                            while rest[j] % p == 0 {
                                rest[j] /= p;
                                e += 1;
                            }
                            factors[j].push((p, e));
                            m += p as i64;
                        }
                    }
                    for j in 0..n {
                        if rest[j] > 1 {
                            factors[j].push((rest[j], 1));
                        }
                    }
                    for (j, f) in factors.iter().enumerate() {
                        let d = d_lo + j as i64;
                        for sign in [1i64, -1] {
                            let kk = sign * k;
                            // cube roots of kk mod d, by the Chinese remainder theorem
                            let mut roots: Vec<(u64, u64)> = vec![(0, 1)]; // (root, modulus)
                            for &(p, e) in f {
                                let pe = p.pow(e);
                                let rp = cube_roots_prime_power(kk, p, e);
                                if rp.is_empty() {
                                    roots.clear();
                                    break;
                                }
                                let mut next_roots = Vec::with_capacity(roots.len() * rp.len());
                                for &(r, m) in &roots {
                                    let inv = inv_mod(m % pe, pe);
                                    for &a in &rp {
                                        // x = r (mod m), x = a (mod pe)
                                        let tt = ((a + pe - r % pe) % pe) as u128 * inv as u128 % pe as u128;
                                        next_roots.push(((r as u128 + m as u128 * tt) as u64, m * pe));
                                    }
                                }
                                roots = next_roots;
                            }
                            // the z range for this d: z^3 <= kk - d^3/4 from the square being >= 0
                            let lim = kk as i128 - (d as i128).pow(3) / 4;
                            let top = icbrt_floor(lim);
                            let (top, bottom) = if z_hi == i128::MIN {
                                (top, -(bound as i128))
                            } else if z_hi == i128::MAX {
                                if top < z_lo {
                                    continue;
                                }
                                (top, z_lo)
                            } else {
                                (top.min(z_hi), z_lo)
                            };
                            if top < bottom {
                                continue;
                            }
                            for &(r, _) in &roots {
                                my_steps += walk(kk, d, r as i64, top, bottom, &sq, &mut found);
                                for s in found.drain(..) {
                                    let s = if sign == 1 { s } else { [-s[0], -s[1], -s[2]] };
                                    all.lock().unwrap().push(s);
                                }
                            }
                        }
                    }
                }
                steps.fetch_add(my_steps, Ordering::Relaxed);
            });
        }
    });
    let mut solutions: Vec<[i128; 3]> = all.into_inner().unwrap().into_iter().map(|mut s| {
        s.sort_by_key(|v| std::cmp::Reverse(v.abs()));
        s
    }).collect();
    solutions.sort_by_key(|s| (s[2].abs(), s[0].abs(), s[0]));
    solutions.dedup();
    Search { k, bound, solutions, steps: steps.load(Ordering::Relaxed), seconds: t0.elapsed().as_secs_f64() }
}

fn group(n: i128) -> String {
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

pub fn report(ks: &[i64], bound: i64) -> String {
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let mut out = String::from("Sums of three cubes: x^3 + y^3 + z^3 = k\n\nThe known solutions, checked exactly:\n");
    for (k, xyz, who) in KNOWN {
        let ok = check(k, xyz) == Some(true);
        out.push_str(&format!("  {k:>3} = ({})^3 + ({})^3 + ({})^3   {}  ({who})\n", xyz[0], xyz[1], xyz[2], if ok { "correct" } else { "WRONG" }));
    }
    out.push_str(&format!(
        "\nSearch (Booker's method, {threads} threads): every solution whose smallest coordinate is at most {} in size\n",
        group(bound as i128)
    ));
    for &k in ks {
        if !possible(k) {
            out.push_str(&format!("  k = {k}: none can exist ({k} = {} mod 9, cubes are 0, 1, -1 mod 9)\n", k.rem_euclid(9)));
            continue;
        }
        let s = search(k, bound, threads);
        out.push_str(&format!("  k = {k}: {} solutions ({} z values tried, {:.1} s)\n", s.solutions.len(), group(s.steps as i128), s.seconds));
        for v in s.solutions.iter().take(12) {
            let ok = (v[0].pow(3) + v[1].pow(3) + v[2].pow(3)) == k as i128 || check(k, [&v[0].to_string(), &v[1].to_string(), &v[2].to_string()]) == Some(true);
            out.push_str(&format!("      ({})^3 + ({})^3 + ({})^3   {}\n", group(v[0]), group(v[1]), group(v[2]), if ok { "checked" } else { "WRONG" }));
        }
        let c = icbrt_floor(k as i128);
        if c * c * c == k as i128 {
            out.push_str(&format!("      and the endless family t^3 + (-t)^3 + {c}^3
"));
        }
        if s.solutions.is_empty() && OPEN.contains(&k) {
            out.push_str(&format!("      none with smallest coordinate up to {} (searches by Booker and Sutherland have already gone far beyond this)\n", group(bound as i128)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_solutions_are_correct() {
        for (k, xyz, _) in KNOWN {
            assert_eq!(check(k, xyz), Some(true), "{k}");
        }
        assert_eq!(check(30, ["1", "2", "3"]), Some(false));
    }

    #[test]
    fn cube_roots_mod_primes_are_right() {
        for p in small_primes(3000).into_iter().filter(|&p| p > 3) {
            for a in [2u64, 5, 7, 30, 114, 975] {
                if a % p == 0 {
                    continue;
                }
                let mut want: Vec<u64> = (0..p).filter(|&r| r * r % p * r % p == a % p).collect();
                let mut got = cube_roots_prime(a, p);
                want.sort();
                got.sort();
                assert_eq!(got, want, "a = {a}, p = {p}");
            }
        }
    }

    #[test]
    fn the_search_agrees_with_brute_force() {
        // every solution with all coordinates within 60, found by brute force,
        // must be found by the search with bound 60
        for k in 1..=40i64 {
            if !possible(k) {
                continue;
            }
            let found = search(k, 60, 4).solutions;
            for x in -60i128..=60 {
                for y in -60i128..=x {
                    for z in -60i128..=y {
                        // x = -y (d = 0) is the family t^3 + (-t)^3 + c^3 = c^3, not searched
                        let family = x + y == 0 || x + z == 0 || y + z == 0;
                        if !family && x * x * x + y * y * y + z * z * z == k as i128 {
                            let mut s = [x, y, z];
                            s.sort_by_key(|v| std::cmp::Reverse(v.abs()));
                            // the search sorts by size; compare as sets
                            let mut a = s;
                            a.sort();
                            assert!(found.iter().any(|f| {
                                let mut b = *f;
                                b.sort();
                                b == a
                            }), "k = {k}: missed {x} {y} {z}");
                        }
                    }
                }
            }
            for f in &found {
                assert_eq!(f[0].pow(3) + f[1].pow(3) + f[2].pow(3), k as i128);
            }
        }
    }
}
