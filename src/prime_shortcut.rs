//! The idea machine that looks for a shortcut: a primality test cheaper
//! than the known ones, evolved the way the supergenius and the prime rule
//! were found. Nuome breeds tests from n, small constants, + - * mod div,
//! floor(sqrt n), gcd, modular powers a^e mod m, comparisons, and/or/not
//! and "for every / for some d from a to b". Every test pays for its work
//! in multiplications: a^e mod m costs about 2 log2 e of them, a loop one
//! per pass, gcd one per step. Fitness: mistakes on 2..3000 and on every
//! known liar (Carmichael numbers and base-2 Fermat pseudoprimes, which
//! fool the cheap tests), then the cost relative to log2 n, then length.
//! Tested on numbers it never saw: 3001..200,000 and every liar below
//! 10^7. A shortcut would be a test with no mistakes and a cost well
//! below log2 n multiplications per number.

use crate::evolve::Rng;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum T {
    N,
    D,
    C(i64),
    Sqrt,
    Add(Box<T>, Box<T>),
    Sub(Box<T>, Box<T>),
    Mul(Box<T>, Box<T>),
    Mod(Box<T>, Box<T>),
    Div(Box<T>, Box<T>),
    Gcd(Box<T>, Box<T>),
    /// a^e mod m
    Pow(Box<T>, Box<T>, Box<T>),
    Eq(Box<T>, Box<T>),
    Lt(Box<T>, Box<T>),
    Not(Box<T>),
    And(Box<T>, Box<T>),
    Or(Box<T>, Box<T>),
    All(Box<T>, Box<T>, Box<T>),
    Any(Box<T>, Box<T>, Box<T>),
}

struct Ctx {
    n: i128,
    cost: u64,
    limit: u64,
}

const CAP: i128 = 1 << 62;

fn bits(e: i128) -> u64 {
    (128 - e.max(1).leading_zeros()) as u64
}

fn pow_mod(a: i128, mut e: i128, m: i128) -> i128 {
    let mut r: i128 = 1 % m;
    let mut b = a.rem_euclid(m);
    while e > 0 {
        if e & 1 == 1 {
            r = r * b % m;
        }
        b = b * b % m;
        e >>= 1;
    }
    r
}

impl T {
    fn eval(&self, c: &mut Ctx, d: Option<i128>) -> Option<i128> {
        let b = |v: bool| v as i128;
        let v = match self {
            T::N => c.n,
            T::D => d?,
            T::C(k) => *k as i128,
            T::Sqrt => {
                c.cost += 1;
                (c.n.max(0) as f64).sqrt().floor() as i128
            }
            T::Add(x, y) => x.eval(c, d)? + y.eval(c, d)?,
            T::Sub(x, y) => x.eval(c, d)? - y.eval(c, d)?,
            T::Mul(x, y) => {
                c.cost += 1;
                x.eval(c, d)?.checked_mul(y.eval(c, d)?)?
            }
            T::Mod(x, y) => {
                let (v, m) = (x.eval(c, d)?, y.eval(c, d)?);
                c.cost += 1;
                if m <= 0 {
                    0
                } else {
                    v.rem_euclid(m)
                }
            }
            T::Div(x, y) => {
                let (v, m) = (x.eval(c, d)?, y.eval(c, d)?);
                c.cost += 1;
                if m <= 0 {
                    0
                } else {
                    v.div_euclid(m)
                }
            }
            T::Gcd(x, y) => {
                let (mut p, mut q) = (x.eval(c, d)?.abs(), y.eval(c, d)?.abs());
                while q != 0 {
                    c.cost += 1;
                    (p, q) = (q, p % q);
                }
                p
            }
            T::Pow(a, e, m) => {
                let (a, e, m) = (a.eval(c, d)?, e.eval(c, d)?, m.eval(c, d)?);
                if m <= 0 || e < 0 || m >= (1 << 62) {
                    return Some(0);
                }
                c.cost += 2 * bits(e);
                pow_mod(a, e, m)
            }
            T::Eq(x, y) => b(x.eval(c, d)? == y.eval(c, d)?),
            T::Lt(x, y) => b(x.eval(c, d)? < y.eval(c, d)?),
            T::Not(x) => b(x.eval(c, d)? == 0),
            T::And(x, y) => b(x.eval(c, d)? != 0 && y.eval(c, d)? != 0),
            T::Or(x, y) => b(x.eval(c, d)? != 0 || y.eval(c, d)? != 0),
            T::All(lo, hi, body) | T::Any(lo, hi, body) => {
                if d.is_some() {
                    return None;
                }
                let (lo, hi) = (lo.eval(c, None)?, hi.eval(c, None)?);
                let mut x = lo;
                let all = matches!(self, T::All(..));
                while x <= hi {
                    c.cost += 1;
                    if c.cost > c.limit {
                        return None;
                    }
                    let v = body.eval(c, Some(x))? != 0;
                    if all && !v {
                        return Some(0);
                    }
                    if !all && v {
                        return Some(1);
                    }
                    x += 1;
                }
                b(all)
            }
        };
        if c.cost > c.limit {
            return None;
        }
        (v.abs() < CAP).then_some(v)
    }
    fn kids(&mut self) -> Vec<&mut T> {
        match self {
            T::N | T::D | T::C(_) | T::Sqrt => vec![],
            T::Not(x) => vec![x],
            T::Add(x, y) | T::Sub(x, y) | T::Mul(x, y) | T::Mod(x, y) | T::Div(x, y) | T::Gcd(x, y) | T::Eq(x, y) | T::Lt(x, y) | T::And(x, y) | T::Or(x, y) => vec![x, y],
            T::Pow(a, b, c) | T::All(a, b, c) | T::Any(a, b, c) => vec![a, b, c],
        }
    }
    fn size(&self) -> usize {
        1 + self.clone().kids().iter().map(|k| k.size()).sum::<usize>()
    }
    fn at(&mut self, k: usize) -> &mut T {
        if k == 0 {
            return self;
        }
        let mut k = k - 1;
        let sizes: Vec<usize> = self.clone().kids().iter().map(|c| c.size()).collect();
        for (i, s) in sizes.iter().enumerate() {
            if k < *s {
                return self.kids().into_iter().nth(i).expect("kid").at(k);
            }
            k -= s;
        }
        self
    }
    /// Where node k sits: (inside a loop body, inside a loop at all).
    fn context(&self, k: usize, inside: bool, in_q: bool) -> (bool, bool) {
        if k == 0 {
            return (inside, in_q);
        }
        let mut k = k - 1;
        let quant = matches!(self, T::All(..) | T::Any(..));
        for (i, c) in self.clone().kids().iter().enumerate() {
            let s = c.size();
            if k < s {
                return c.context(k, inside || (quant && i == 2), in_q || quant);
            }
            k -= s;
        }
        (inside, in_q)
    }
    pub(crate) fn show(&self) -> String {
        match self {
            T::N => "n".into(),
            T::D => "d".into(),
            T::C(k) => k.to_string(),
            T::Sqrt => "floor(sqrt n)".into(),
            T::Add(x, y) => format!("({} + {})", x.show(), y.show()),
            T::Sub(x, y) => format!("({} - {})", x.show(), y.show()),
            T::Mul(x, y) => format!("{}*{}", x.show(), y.show()),
            T::Mod(x, y) => format!("({} mod {})", x.show(), y.show()),
            T::Div(x, y) => format!("floor({} / {})", x.show(), y.show()),
            T::Gcd(x, y) => format!("gcd({}, {})", x.show(), y.show()),
            T::Pow(a, e, m) => format!("{}^{} mod {}", a.show(), e.show(), m.show()),
            T::Eq(x, y) => format!("{} = {}", x.show(), y.show()),
            T::Lt(x, y) => format!("{} < {}", x.show(), y.show()),
            T::Not(x) => format!("not ({})", x.show()),
            T::And(x, y) => format!("({} and {})", x.show(), y.show()),
            T::Or(x, y) => format!("({} or {})", x.show(), y.show()),
            T::All(a, b, c) => format!("[for every d from {} to {}: {}]", a.show(), b.show(), c.show()),
            T::Any(a, b, c) => format!("[for some d from {} to {}: {}]", a.show(), b.show(), c.show()),
        }
    }
}

fn leaf(r: &mut Rng, inside: bool) -> T {
    match r.below(if inside { 8 } else { 6 }) {
        0 | 1 => T::N,
        2 => T::Sqrt,
        3..=5 => T::C(r.below(11) as i64),
        _ => T::D,
    }
}

fn random_t(r: &mut Rng, depth: usize, inside: bool, q: bool) -> T {
    if depth == 0 || r.below(4) == 0 {
        return leaf(r, inside);
    }
    if q && r.below(6) == 0 {
        let lo = Box::new(random_t(r, 1, false, false));
        let hi = Box::new(random_t(r, 1, false, false));
        let body = Box::new(random_t(r, depth - 1, true, false));
        return if r.below(2) == 0 { T::All(lo, hi, body) } else { T::Any(lo, hi, body) };
    }
    let g = |r: &mut Rng| Box::new(random_t(r, depth - 1, inside, q));
    match r.below(16) {
        0 => T::Add(g(r), g(r)),
        1 => T::Sub(g(r), g(r)),
        2 => T::Mul(g(r), g(r)),
        3 => T::Mod(g(r), g(r)),
        4 => T::Div(g(r), g(r)),
        5 => T::Gcd(g(r), g(r)),
        6..=8 => T::Pow(g(r), g(r), g(r)),
        9 | 10 => T::Eq(g(r), g(r)),
        11 => T::Lt(g(r), g(r)),
        12 => T::Not(g(r)),
        13 => T::And(g(r), g(r)),
        _ => T::Or(g(r), g(r)),
    }
}

/// The truth: Miller-Rabin with 12 bases, exact for every n < 3.3e24.
fn is_prime(n: i128) -> bool {
    n >= 2 && crate::prime_formula::is_prime(n as u64)
}

/// The liars below `limit`: Carmichael numbers and base-2 Fermat pseudoprimes.
pub fn liars(limit: usize) -> (Vec<i128>, Vec<i128>) {
    // smallest prime factor sieve, for Korselt's criterion
    let mut spf = vec![0u32; limit + 1];
    for i in 2..=limit {
        if spf[i] == 0 {
            let mut j = i;
            while j <= limit {
                if spf[j] == 0 {
                    spf[j] = i as u32;
                }
                j += i;
            }
        }
    }
    let (mut carmichael, mut psp) = (Vec::new(), Vec::new());
    for n in (3..=limit).step_by(2) {
        if spf[n] as usize == n {
            continue;
        }
        if pow_mod(2, n as i128 - 1, n as i128) == 1 {
            psp.push(n as i128);
        }
        // Korselt: square-free, and p - 1 divides n - 1 for every prime p | n
        let (mut m, mut ok, mut count) = (n, true, 0);
        while m > 1 {
            let p = spf[m] as usize;
            m /= p;
            count += 1;
            if m % p == 0 || (n - 1) % (p - 1) != 0 {
                ok = false;
                break;
            }
        }
        if ok && count >= 3 {
            carmichael.push(n as i128);
        }
    }
    (carmichael, psp)
}

/// (mistakes, total cost) over `ns`, or None if the test breaks or runs too long.
pub(crate) fn judge(t: &T, ns: &[i128], limit_per: u64) -> Option<(usize, u64)> {
    let mut wrong = 0;
    let mut cost = 0;
    for &n in ns {
        let mut c = Ctx { n, cost: 0, limit: limit_per };
        let v = t.eval(&mut c, None)? != 0;
        cost += c.cost;
        if v != is_prime(n) {
            wrong += 1;
        }
    }
    Some((wrong, cost))
}

pub struct Settings {
    pub population: usize,
    pub generations: usize,
    pub islands: usize,
    pub seed: u64,
}

pub(crate) fn evolve(s: &Settings, seed: u64, learn: &[i128], liars: &[i128]) -> Vec<(f64, T)> {
    let log_mean = learn.iter().map(|&n| (n as f64).log2().max(1.0)).sum::<f64>() / learn.len() as f64;
    let fit = |t: &T| -> f64 {
        let (Some(a), Some(b)) = (judge(t, learn, 20_000), judge(t, liars, 20_000)) else { return f64::INFINITY };
        let mistakes = (a.0 as f64 + 3.0 * b.0 as f64) / (learn.len() as f64 + 3.0 * liars.len() as f64);
        // cost as a share of log2 n multiplications per number
        let cost = a.1 as f64 / learn.len() as f64 / log_mean;
        mistakes + 0.002 * cost + 0.002 * t.size() as f64
    };
    let mut r = Rng(seed);
    let mut pop: Vec<T> = (0..s.population).map(|_| random_t(&mut r, 5, false, true)).collect();
    for _ in 0..s.generations {
        let mut scored: Vec<(f64, T)> = pop.drain(..).map(|t| (fit(&t), t)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next: Vec<T> = scored.iter().take(s.population / 10).map(|x| x.1.clone()).collect();
        while next.len() < s.population {
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            let mut child = scored[best].1.clone();
            let k = r.below(child.size());
            let (inside, in_q) = child.context(k, false, false);
            match child.at(k) {
                T::C(c) if r.below(2) == 0 => *c = (*c + r.below(5) as i64 - 2).max(0),
                node => *node = random_t(&mut r, 3, inside, !in_q),
            }
            if child.size() <= 22 {
                next.push(child);
            }
        }
        pop = next;
    }
    let mut last: Vec<(f64, T)> = pop.into_iter().map(|t| (fit(&t), t)).filter(|x| x.0.is_finite()).collect();
    last.sort_by(|a, b| a.0.total_cmp(&b.0));
    last
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let learn: Vec<i128> = (2..=3000).collect();
    let (carm_small, psp_small) = liars(1_000_000);
    let mut liar_learn: Vec<i128> = carm_small.iter().chain(psp_small.iter()).copied().collect();
    liar_learn.sort();
    liar_learn.dedup();
    let runs: Vec<Vec<(f64, T)>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..s.islands)
            .map(|i| {
                let (learn, liar_learn) = (&learn, &liar_learn);
                sc.spawn(move || evolve(s, s.seed + i as u64 * 104_729, learn, liar_learn))
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("island")).collect()
    });
    let mut all: Vec<(f64, T)> = runs.into_iter().flat_map(|v| v.into_iter().take(15)).collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0));
    all.dedup_by(|a, b| a.1 == b.1);

    // the test: numbers never seen, and every liar below 10^7
    let unseen: Vec<i128> = (3001..=200_000).collect();
    let (carm, psp) = liars(10_000_000);
    let mut liar_test: Vec<i128> = carm.iter().chain(psp.iter()).copied().filter(|&n| n > 1_000_000).collect();
    liar_test.sort();
    liar_test.dedup();
    let mut out = String::new();
    out.push_str(&format!(
        "Nuome looks for a shortcut: a primality test cheaper than the known ones\n\nbuilding blocks: n, constants 0..10, + - * mod div, floor(sqrt n), gcd, a^e mod m (costs ~2 log2 e multiplications), =, <, not, and, or, \"for every / for some d\" (one multiplication a pass)\nlearned on 2..3000 and on every liar below 10^6 ({} Carmichael numbers, {} base-2 Fermat pseudoprimes); cost measured in multiplications per number, as a share of log2 n\n{} islands of {} tests, {} generations each\n\n",
        carm_small.len(),
        psp_small.len(),
        s.islands,
        s.population,
        s.generations
    ));
    out.push_str(&format!("tested on 3001..200,000 (never seen) and on the {} liars between 10^6 and 10^7 (never seen):\n", liar_test.len()));
    let mut shown = 0;
    for (_, t) in &all {
        if shown == 8 {
            break;
        }
        let Some(u) = judge(t, &unseen, 5_000_000) else { continue };
        let l = judge(t, &liar_test, 5_000_000);
        let log_mean = unseen.iter().map(|&n| (n as f64).log2()).sum::<f64>() / unseen.len() as f64;
        let per = u.1 as f64 / unseen.len() as f64;
        out.push_str(&format!(
            "  {}\n      unseen numbers: {} mistakes; liars: {}; cost {:.1} multiplications per number = {:.2} x log2 n\n",
            t.show(),
            u.0,
            l.map_or("breaks".into(), |x| format!("{} of {} wrong", x.0, liar_test.len())),
            per,
            per / log_mean
        ));
        shown += 1;
    }
    out.push_str(
        "\nthe known shortcuts, for comparison: Fermat (3^(n-1) mod n = 1) ~2 x log2 n but fooled by liars; Miller-Rabin ~2 x log2 n per base;\n\
         Baillie-PSW ~6 x log2 n, no liar known; trial division ~sqrt(n) / ln(sqrt n) divisions. A shortcut would be no mistakes at well under 1 x log2 n.\n",
    );
    out.push_str(&format!("\n({:.1} s)\n", t0.elapsed().as_secs_f64()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_liars_are_the_known_ones() {
        let (carm, psp) = liars(10_000);
        assert_eq!(&carm[..4], &[561, 1105, 1729, 2465]);
        assert_eq!(&psp[..3], &[341, 561, 645]);
    }

    #[test]
    fn fermat_is_cheap_but_lied_to() {
        // 3^(n-1) mod n = 1, or n = 3
        let fermat = T::Or(
            Box::new(T::Eq(Box::new(T::Pow(Box::new(T::C(3)), Box::new(T::Sub(Box::new(T::N), Box::new(T::C(1)))), Box::new(T::N))), Box::new(T::C(1)))),
            Box::new(T::Eq(Box::new(T::N), Box::new(T::C(3)))),
        );
        let ns: Vec<i128> = (2..=1000).collect();
        let (wrong, cost) = judge(&fermat, &ns, 1_000_000).unwrap();
        // base 3 is fooled by the six base-3 pseudoprimes below 1000: 91 121 286 671 703 949
        assert_eq!(wrong, 6);
        assert!(cost / 999 < 25);
        // 1105 = 5 * 13 * 17 is a Carmichael number: it fools Fermat for base 3
        assert_eq!(judge(&fermat, &[1105], 1_000_000).unwrap().0, 1);
    }
}
