//! The idea machine for a prime formula: Nuome builds formulas f(n) itself
//! (from n, whole constants, +, -, *, squares and 2^x; no formula given) and
//! scores them by how many of f(0), f(1), ... are prime, the Goldbach way:
//! learned on n = 0..29 with a forward check inside, then tested on n >= 30,
//! which the search never saw, compared with chance (a number near x is
//! prime with chance about 1 / ln x) and with the known formulas. Then the
//! general part: why no formula of these kinds can give only primes.

use crate::evolve::Rng;

#[derive(Clone)]
enum F {
    N,
    C(i64),
    Add(Box<F>, Box<F>),
    Sub(Box<F>, Box<F>),
    Mul(Box<F>, Box<F>),
    Sq(Box<F>),
    Pow2(Box<F>),
}

const CAP: i128 = 1 << 62;

impl F {
    /// f(n) exactly, or None when it leaves |v| < 2^62.
    fn eval(&self, n: i128) -> Option<i128> {
        let v = match self {
            F::N => n,
            F::C(c) => *c as i128,
            F::Add(a, b) => a.eval(n)? + b.eval(n)?,
            F::Sub(a, b) => a.eval(n)? - b.eval(n)?,
            F::Mul(a, b) => a.eval(n)?.checked_mul(b.eval(n)?)?,
            F::Sq(a) => a.eval(n)?.checked_mul(a.eval(n)?)?,
            F::Pow2(a) => {
                let e = a.eval(n)?;
                if !(0..62).contains(&e) {
                    return None;
                }
                1i128 << e
            }
        };
        (v.abs() < CAP).then_some(v)
    }
    fn size(&self) -> usize {
        match self {
            F::N | F::C(_) => 1,
            F::Add(a, b) | F::Sub(a, b) | F::Mul(a, b) => 1 + a.size() + b.size(),
            F::Sq(a) | F::Pow2(a) => 1 + a.size(),
        }
    }
    fn show(&self) -> String {
        match self {
            F::N => "n".into(),
            F::C(c) => c.to_string(),
            F::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            F::Sub(a, b) => format!("({} - {})", a.show(), b.show()),
            F::Mul(a, b) => format!("{}*{}", a.show(), b.show()),
            F::Sq(a) => format!("({})^2", a.show()),
            F::Pow2(a) => format!("2^({})", a.show()),
        }
    }
    fn at(&mut self, k: usize) -> &mut F {
        if k == 0 {
            return self;
        }
        match self {
            F::Add(a, b) | F::Sub(a, b) | F::Mul(a, b) => {
                let s = a.size();
                if k <= s {
                    a.at(k - 1)
                } else {
                    b.at(k - 1 - s)
                }
            }
            F::Sq(a) | F::Pow2(a) => a.at(k - 1),
            _ => self,
        }
    }
}

fn random_f(r: &mut Rng, depth: usize) -> F {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(2) == 0 { F::N } else { F::C(r.below(81) as i64 - 20) };
    }
    let a = Box::new(random_f(r, depth - 1));
    match r.below(5) {
        0 => F::Add(a, Box::new(random_f(r, depth - 1))),
        1 => F::Sub(a, Box::new(random_f(r, depth - 1))),
        2 => F::Mul(a, Box::new(random_f(r, depth - 1))),
        3 => F::Sq(a),
        _ => F::Pow2(a),
    }
}

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    (a as u128 * b as u128 % m as u128) as u64
}

/// Miller-Rabin with the first 12 prime bases: exact for every n < 3.3e24.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n % p == 0 {
            return n == p;
        }
    }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'base: for a in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = 1u64;
        let (mut b, mut e) = (a, d);
        while e > 0 {
            if e & 1 == 1 {
                x = mulmod(x, b, n);
            }
            b = mulmod(b, b, n);
            e >>= 1;
        }
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'base;
            }
        }
        return false;
    }
    true
}

/// Over n in the range: (values that fit, primes, chance = sum of 1 / ln f);
/// None when f is not increasing or not above 1 (a formula must grow, or it
/// repeats one prime).
fn score(f: &F, ns: std::ops::Range<i128>) -> Option<(usize, usize, f64)> {
    let (mut fit, mut primes, mut chance, mut last) = (0, 0, 0.0, i128::MIN);
    for n in ns {
        let Some(v) = f.eval(n) else { break };
        if v <= last || v < 2 {
            return None;
        }
        last = v;
        fit += 1;
        if is_prime(v as u64) {
            primes += 1;
        }
        chance += 1.0 / (v as f64).ln().max(1.0);
    }
    Some((fit, primes, chance))
}

pub struct Settings {
    pub generations: usize,
    pub seed: u64,
}

/// The genetic search: the last generation, best first.
fn invent(s: &Settings) -> Vec<(f64, F)> {
    let fit = |f: &F| -> f64 {
        // the forward check of the Goldbach search: n = 0..19 and then n = 20..29
        let (Some(early), Some(late)) = (score(f, 0..20), score(f, 20..30)) else { return f64::INFINITY };
        if early.0 < 20 || late.0 < 10 || f.eval(29).is_none_or(|v| v < 1000) {
            return f64::INFINITY;
        }
        1.0 - (early.1 as f64 / 20.0 + 2.0 * late.1 as f64 / 10.0) / 3.0 + 0.004 * f.size() as f64
    };
    let mut r = Rng(s.seed);
    let mut pop: Vec<F> = (0..400).map(|_| random_f(&mut r, 4)).collect();
    for _ in 0..s.generations {
        let mut scored: Vec<(f64, F)> = pop.drain(..).map(|f| (fit(&f), f)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next: Vec<F> = scored.iter().take(40).map(|x| x.1.clone()).collect();
        while next.len() < 400 {
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            let mut child = scored[best].1.clone();
            let k = r.below(child.size());
            match child.at(k) {
                F::C(c) if r.below(2) == 0 => *c += r.below(7) as i64 - 3,
                node => *node = random_f(&mut r, 2),
            }
            if child.size() <= 13 {
                next.push(child);
            }
        }
        pop = next;
    }
    let mut last: Vec<(f64, F)> = pop.into_iter().map(|f| (fit(&f), f)).filter(|x| x.0.is_finite()).collect();
    last.sort_by(|a, b| a.0.total_cmp(&b.0));
    last
}

pub fn report(s: &Settings) -> String {
    let mut out = vec!["Nuome's idea machine for a prime formula: a formula f(n) that gives primes, built from n, whole constants, +, -, *, squares and 2^x (no formula given)".to_string(), String::new()];
    let last = invent(s);
    let line = |name: &str, f: &F| -> Option<(String, f64)> {
        let (tf, tp, tc) = score(f, 0..30)?;
        let (uf, up, uc) = score(f, 30..200).unwrap_or((0, 0, 0.0));
        let unseen = if uf >= 10 {
            format!("{up}/{uf} prime on the unseen n = 30..{} ({:.1}x chance)", 29 + uf, up as f64 / uc)
        } else {
            format!("only {uf} unseen values fit below 2^62, too few to test")
        };
        Some((format!("   {name}f(n) = {}: {tp}/{tf} prime on the learned n = 0..29 ({:.1}x chance); {unseen}", f.show(), tp as f64 / tc), if uf >= 10 { up as f64 / uc } else { 0.0 }))
    };
    let euler = F::Add(Box::new(F::Add(Box::new(F::Sq(Box::new(F::N))), Box::new(F::N))), Box::new(F::C(41)));
    let mersenne = F::Sub(Box::new(F::Pow2(Box::new(F::Add(Box::new(F::N), Box::new(F::C(2)))))), Box::new(F::C(1)));
    let mersenne0 = F::Sub(Box::new(F::Pow2(Box::new(F::N))), Box::new(F::C(1)));
    out.push("formulas Nuome built (best first, each shape once):".into());
    let mut shown: Vec<String> = Vec::new();
    let mut best: Option<F> = None;
    for (_, f) in &last {
        if shown.len() >= 6 {
            break;
        }
        let text = f.show();
        let key: String = text.chars().filter(|c| !c.is_ascii_digit()).collect();
        if shown.contains(&key) {
            continue;
        }
        shown.push(key);
        if let Some((l, _)) = line("", f) {
            // the same as a known formula moved along: f(n) = g(n + s)
            let same = (-60i128..=60).find_map(|s| {
                [("Euler's n^2 + n + 41", &euler), ("2^n - 1", &mersenne0)].into_iter().find(|(_, g)| (0..30).all(|n| n + s >= 0 && f.eval(n).is_some() && f.eval(n) == g.eval(n + s))).map(|(name, _)| (name, s))
            });
            out.push(match same {
                Some((name, s)) => format!("{l}; a rediscovery: {name} with n moved by {s}"),
                None => format!("{l}; not a moved copy of Euler's formula or 2^n - 1 (the only two compared)"),
            });
            if best.is_none() {
                best = Some(f.clone());
            }
        }
    }
    out.push("known formulas, scored the same way:".into());
    for (name, f) in [("Euler 1772: ", &euler), ("Mersenne: ", &mersenne)] {
        match line(name, f) {
            Some((l, _)) => out.push(l),
            None => out.push(format!("   {name}{} does not grow from n = 0", f.show())),
        }
    }
    // where the winner first fails, with the factor
    if let Some(f) = &best {
        if let Some((n, v)) = (0..200).map_while(|n| f.eval(n).map(|v| (n, v))).find(|&(_, v)| !is_prime(v as u64)) {
            let factor = (2..=1_000_000u64).find(|d| (v as u64) % d == 0 && (v as u64) != *d);
            out.push(format!(
                "   Nuome's best first gives a composite at n = {n}: f({n}) = {v}{}",
                factor.map_or(" (composite by Miller-Rabin)".to_string(), |d| format!(" = {d} * {}", v as u64 / d))
            ));
        }
    }
    out.push(String::new());
    out.push("general proofs (for every formula of these kinds; no number is checked):".into());
    out.push("  theorem 1 (no polynomial gives only primes; Goldbach 1752): let f have whole coefficients and grow. If f(a) = p is prime, then f(a + kp) = f(a) = 0 mod p for every k, because (a + kp)^j = a^j mod p term by term; f(a + p) > p since f grows, so f(a + p) is a multiple of p larger than p, not prime. QED".into());
    out.push("  theorem 2 (no u 2^n + v gives only primes, u, v whole, u > 0): if f(a) = u 2^a + v = p is an odd prime, then by Fermat's little theorem 2^(p-1) = 1 mod p, so f(a + p - 1) = u 2^a 2^(p-1) + v = f(a) = 0 mod p, and f(a + p - 1) > p, not prime. QED".into());
    out.push("  so a formula that names a prime at once cannot be one of these shapes: every polynomial and every u 2^n + v formula gives composites, at the latest at n = a + p. What a formula can do is make primes likelier than chance, as the numbers above show, and then each value still needs its primality test".into());
    out.push(String::new());
    out.push("for the prize: a number with 100 million digits is prime with chance about 1 / ln(10^(10^8)) = 1 / 2.3e8; a formula that is k times better than chance gives k / 2.3e8 per value, and each value with 100 million digits still needs a test of the Lucas-Lehmer kind (Proth's theorem for k 2^n + 1, the Lucas-Lehmer test for 2^n - 1), which is the slow part".into());
    out.join("\n")
}

/// Primes among f(n), n in from..from+len, and the chance sum (1 / ln f).
fn primes_in(f: &F, from: i128, len: i128) -> Option<(usize, f64)> {
    let (mut p, mut c) = (0, 0.0);
    for n in from..from + len {
        let v = f.eval(n)?;
        if is_prime(v as u64) {
            p += 1;
        }
        c += 1.0 / (v as f64).ln();
    }
    Some((p, c))
}

/// The range for a prime with `digits` digits from Nuome's best formula:
/// Nuome measures how much likelier than chance its values are prime, turns
/// that into windows of n for 50%, 95% and 99%, first checks those windows
/// on sizes it can test (does the 95% window hold a prime 95% of the time?),
/// then gives the window at the asked size.
pub fn range(s: &Settings, digits: u64) -> String {
    let mut out = Vec::new();
    let Some((_, f)) = invent(s).into_iter().next() else { return "the search found no formula".into() };
    out.push(format!("Nuome's formula: f(n) = {}", f.show()));
    // 1. how fast it grows: f(n) ~ a n^d, from two large n
    let (n1, n2) = (100_000_000i128, 200_000_000i128);
    let (Some(v1), Some(v2)) = (f.eval(n1), f.eval(n2)) else { return format!("{}\nf grows too fast to measure below 2^62", out.join("\n")) };
    let d = ((v2 as f64).ln() - (v1 as f64).ln()) / 2f64.ln();
    let d_round = d.round();
    if (d - d_round).abs() > 0.01 || d_round < 1.0 {
        return format!("{}\nf grows like n^{d:.2}, not a polynomial; no window computed", out.join("\n"));
    }
    let a = v2 as f64 / (n2 as f64).powf(d_round);
    out.push(format!("growth, measured: f(n) is about {a:.3} n^{d_round}"));
    // 2. how much likelier than chance, measured by Nuome at three sizes
    let mut ks = Vec::new();
    for start in [1_000_000i128, 10_000_000, 100_000_000] {
        if let Some((p, c)) = primes_in(&f, start, 20_000) {
            ks.push((start, p as f64 / c));
        }
    }
    let k = ks.iter().map(|x| x.1).sum::<f64>() / ks.len() as f64;
    out.push(format!(
        "likelier than chance, measured on 20000 values each: {}; Nuome uses the mean k = {k:.2}",
        ks.iter().map(|(n, k)| format!("{k:.2}x from n = {n}")).collect::<Vec<_>>().join(", ")
    ));
    // a value with D digits is prime with chance k / (D ln 10); a window of L
    // values holds none with chance exp(-L q), so L = -ln(1 - conf) / q
    let window = |dg: f64, conf: f64| -(1.0f64 - conf).ln() / (k / (dg * 10f64.ln()));
    // 3. check the windows where every value can be tested
    out.push("check first, on sizes Nuome can test (500 windows each, spread out): how often does each window hold a prime?".into());
    let mut all_ok = true;
    for dg in [10u32, 14, 18] {
        let start = (10f64.powf((dg as f64 - 1.0 - a.log10()) / d_round)).ceil() as i128;
        let mut cells = Vec::new();
        for conf in [0.5, 0.95, 0.99] {
            let len = window(dg as f64, conf).ceil() as i128;
            let hits = (0..500).filter(|j| primes_in(&f, start + j * 2000, len).is_some_and(|x| x.0 > 0)).count();
            let rate = hits as f64 / 500.0;
            // the whole-number window promises a little more than conf
            let promised = 1.0 - (-(len as f64) * k / (dg as f64 * 10f64.ln())).exp();
            // within 3 standard errors of what it promises
            let ok = (rate - promised).abs() <= 3.0 * (promised * (1.0 - promised) / 500.0).sqrt() + 0.01;
            all_ok &= ok;
            cells.push(format!("{:.0}% window ({len} values, promises {:.1}%): {:.1}%{}", conf * 100.0, promised * 100.0, rate * 100.0, if ok { "" } else { " MISS" }));
        }
        out.push(format!("   {dg} digits (n from {start}): {}", cells.join("; ")));
    }
    out.push(format!("   {}", if all_ok { "the windows hold what they promise at every tested size" } else { "the windows do NOT hold what they promise everywhere, so the prediction below is unreliable" }));
    // 4. the asked size
    let dg = digits as f64;
    // the first power of ten from which f(n) has at least that many digits
    let exp10 = ((dg - 1.0 - a.log10()) / d_round).ceil();
    let ln_f = dg * 10f64.ln();
    out.push(format!("the range for a prime with {digits} digits: f(n) has {digits} digits from n = 10^{exp10:.0} on (a {:.0}-digit n); there a value is prime with chance about 1 in {:.3e}", exp10.floor() + 1.0, ln_f / k));
    for conf in [0.5, 0.95, 0.99] {
        out.push(format!("   n from 10^{exp10:.0} to 10^{exp10:.0} + {:.3e}: at least one prime with chance {:.0}%", window(dg, conf), conf * 100.0));
    }
    out.push("not proved: that f gives infinitely many primes is open (Landau 1912); the windows rest on the measured k and the Bateman-Horn picture, which the check above supports at the tested sizes. The window says how many values to test, not which one is prime; each value still needs a primality test, and for a number of this form and size no proof method fast enough is known".into());
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miller_rabin_and_euler() {
        assert!(is_prime(2_305_843_009_213_693_951)); // 2^61 - 1
        assert!(!is_prime(3_215_031_751)); // a strong pseudoprime to 2, 3, 5, 7
        let euler = F::Add(Box::new(F::Add(Box::new(F::Sq(Box::new(F::N))), Box::new(F::N))), Box::new(F::C(41)));
        assert_eq!(score(&euler, 0..40).map(|s| s.1), Some(40));
    }
}
