//! The idea machine for a prime-giving function, evolved the Goldbach way:
//! Nuome breeds formulas f(n) from n, A(n) (where the n-th prime is, by
//! Cipolla's series, rounded), whole constants, +, -, *, floor division and
//! remainder; the fitness is how many of f's values are prime, with each
//! value kept near the n-th prime (within `band`) and no value given twice,
//! so a constant prime or a formula that wanders off cannot win. Learned on
//! one range with a forward check on the next, then tested on n it never
//! saw and far beyond, against chance (a number near x is prime with
//! chance about 1/ln x). Last, why the winner works: which small divisors
//! its values avoid.

use crate::evolve::Rng;
use crate::prime_formula::is_prime;
use crate::prime_function::primes_to;

#[derive(Clone)]
enum G {
    N,
    A,
    C(i64),
    Add(Box<G>, Box<G>),
    Sub(Box<G>, Box<G>),
    Mul(Box<G>, Box<G>),
    Div(Box<G>, Box<G>),
    Mod(Box<G>, Box<G>),
}

const CAP: i128 = 1 << 62;

/// Where the n-th prime is: Cipolla's series to second order, rounded.
pub fn approx(n: i128) -> i128 {
    let x = n.max(6) as f64;
    let l = x.ln();
    let ll = l.ln();
    (x * (l + ll - 1.0 + (ll - 2.0) / l - (ll * ll - 6.0 * ll + 11.0) / (2.0 * l * l))).round() as i128
}

impl G {
    fn eval(&self, n: i128) -> Option<i128> {
        let v = match self {
            G::N => n,
            G::A => approx(n),
            G::C(c) => *c as i128,
            G::Add(a, b) => a.eval(n)? + b.eval(n)?,
            G::Sub(a, b) => a.eval(n)? - b.eval(n)?,
            G::Mul(a, b) => a.eval(n)?.checked_mul(b.eval(n)?)?,
            G::Div(a, b) => {
                let d = b.eval(n)?;
                if d <= 0 {
                    return None;
                }
                a.eval(n)?.div_euclid(d)
            }
            G::Mod(a, b) => {
                let d = b.eval(n)?;
                if d <= 0 {
                    return None;
                }
                a.eval(n)?.rem_euclid(d)
            }
        };
        (v.abs() < CAP).then_some(v)
    }
    fn size(&self) -> usize {
        match self {
            G::N | G::A | G::C(_) => 1,
            G::Add(a, b) | G::Sub(a, b) | G::Mul(a, b) | G::Div(a, b) | G::Mod(a, b) => 1 + a.size() + b.size(),
        }
    }
    fn show(&self) -> String {
        match self {
            G::N => "n".into(),
            G::A => "A".into(),
            G::C(c) => c.to_string(),
            G::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            G::Sub(a, b) => format!("({} - {})", a.show(), b.show()),
            G::Mul(a, b) => format!("{}*{}", a.show(), b.show()),
            G::Div(a, b) => format!("floor({} / {})", a.show(), b.show()),
            G::Mod(a, b) => format!("({} mod {})", a.show(), b.show()),
        }
    }
    fn at(&mut self, k: usize) -> &mut G {
        if k == 0 {
            return self;
        }
        match self {
            G::Add(a, b) | G::Sub(a, b) | G::Mul(a, b) | G::Div(a, b) | G::Mod(a, b) => {
                let s = a.size();
                if k <= s {
                    a.at(k - 1)
                } else {
                    b.at(k - 1 - s)
                }
            }
            _ => self,
        }
    }
}

fn random_g(r: &mut Rng, depth: usize) -> G {
    if depth == 0 || r.below(3) == 0 {
        return match r.below(4) {
            0 => G::N,
            1 | 2 => G::A,
            _ => G::C(r.below(61) as i64 - 10),
        };
    }
    let a = Box::new(random_g(r, depth - 1));
    let b = Box::new(random_g(r, depth - 1));
    match r.below(5) {
        0 => G::Add(a, b),
        1 => G::Sub(a, b),
        2 => G::Mul(a, b),
        3 => G::Div(a, b),
        _ => G::Mod(a, b),
    }
}

pub struct Settings {
    pub generations: usize,
    pub population: usize,
    pub seed: u64,
    /// how far from the n-th prime a value may be, as a share of it
    pub band: f64,
}

/// Primality: the sieve up to its end, Miller-Rabin past it.
struct Primes {
    sieve: Vec<bool>,
    list: Vec<u64>,
}

impl Primes {
    fn new(limit: usize) -> Primes {
        let list = primes_to(limit);
        let mut sieve = vec![false; limit + 1];
        for &p in &list {
            sieve[p as usize] = true;
        }
        Primes { sieve, list }
    }
    fn is(&self, v: i128) -> bool {
        if v < 2 {
            return false;
        }
        if (v as usize) < self.sieve.len() {
            self.sieve[v as usize]
        } else {
            is_prime(v as u64)
        }
    }
}

/// On the n of `ns`: (points that stayed in the band, primes among the new
/// values, exact hits of the n-th prime, the chance a random number there
/// is prime, summed), or None if a value cannot be computed.
fn measure(g: &G, ns: &[i128], ps: &Primes, band: f64) -> Option<(usize, usize, usize, f64)> {
    let (mut inside, mut primes, mut exact, mut chance) = (0, 0, 0, 0.0);
    let mut last = i128::MIN;
    for &n in ns {
        let v = g.eval(n)?;
        let p = ps.list[(n - 1) as usize] as i128;
        if ((v - p) as f64).abs() > band * p as f64 {
            continue;
        }
        inside += 1;
        chance += 1.0 / (p as f64).ln();
        // a value given again counts as a miss
        if v == last {
            continue;
        }
        last = v;
        if ps.is(v) {
            primes += 1;
            if v == p {
                exact += 1;
            }
        }
    }
    Some((inside, primes, exact, chance))
}

fn invent(s: &Settings, ps: &Primes, learn: &[i128], forward: &[i128]) -> Vec<(f64, G)> {
    let fit = |g: &G| -> f64 {
        let (Some(a), Some(b)) = (measure(g, learn, ps, s.band), measure(g, forward, ps, s.band)) else { return f64::INFINITY };
        // nearly every value must stay near the n-th prime
        if (a.0 as f64) < 0.98 * learn.len() as f64 || (b.0 as f64) < 0.98 * forward.len() as f64 {
            return f64::INFINITY;
        }
        // the share of primes, the forward check counting double (Goldbach's way), and Occam
        let ra = a.1 as f64 / learn.len() as f64;
        let rb = b.1 as f64 / forward.len() as f64;
        1.0 - (ra + 2.0 * rb) / 3.0 + 0.002 * g.size() as f64
    };
    let mut r = Rng(s.seed);
    let pop_n = s.population;
    let mut pop: Vec<G> = (0..pop_n).map(|_| random_g(&mut r, 4)).collect();
    // the seed of every search: A itself is in the first generation
    pop[0] = G::A;
    for _ in 0..s.generations {
        let mut scored: Vec<(f64, G)> = pop.drain(..).map(|g| (fit(&g), g)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next: Vec<G> = scored.iter().take(pop_n / 10).map(|x| x.1.clone()).collect();
        while next.len() < pop_n {
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            let mut child = scored[best].1.clone();
            if r.below(4) == 0 {
                // crossover: a branch from another good one
                let mut other = scored[r.below(pop_n / 4)].1.clone();
                let k2 = r.below(other.size());
                let branch = other.at(k2).clone();
                let k = r.below(child.size());
                *child.at(k) = branch;
            } else {
                let k = r.below(child.size());
                match child.at(k) {
                    G::C(c) if r.below(2) == 0 => *c += r.below(7) as i64 - 3,
                    node => *node = random_g(&mut r, 2),
                }
            }
            if child.size() <= 15 {
                next.push(child);
            }
        }
        pop = next;
    }
    let mut last: Vec<(f64, G)> = pop.into_iter().map(|g| (fit(&g), g)).filter(|x| x.0.is_finite()).collect();
    last.sort_by(|a, b| a.0.total_cmp(&b.0));
    last.dedup_by(|a, b| a.1.show() == b.1.show());
    last
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let far = 1_000_000usize;
    let lf = (far as f64).ln();
    let ps = Primes::new((far as f64 * (lf + lf.ln())) as usize + 1000);
    let range = |a: i128, b: i128, step: usize| -> Vec<i128> { (a..b).step_by(step).collect() };
    let learn = range(1_000, 20_000, 7);
    let forward = range(20_000, 30_000, 5);
    let test = range(30_000, 100_000, 1);
    let far_ns = range(100_000, far as i128 + 1, 13);
    let mut r = String::new();
    r.push_str(&format!(
        "Nuome evolves a prime-giving function, the Goldbach way\n\nbuilding blocks: n, A = where the n-th prime is (Cipolla's series, rounded), whole constants, +, -, *, floor division, remainder\nfitness: the share of f(n) that are prime, every value within {:.1}% of the n-th prime and no value given twice; learned on n = 1,000..20,000 with a forward check on n = 20,000..30,000 counting double; a price per symbol\nsearch: {} formulas a generation, {} generations, seed {}\n",
        s.band * 100.0,
        s.population,
        s.generations,
        s.seed
    ));
    let found = invent(s, &ps, &learn, &forward);
    let line = |name: &str, g: &G| -> String {
        let cell = |ns: &[i128]| match measure(g, ns, &ps, s.band) {
            Some((inside, primes, exact, chance)) => {
                let share = primes as f64 / ns.len() as f64;
                format!("{:>6.2}% prime ({:.2}x chance, {} exact n-th)", share * 100.0, share / (chance / inside.max(1) as f64), exact)
            }
            None => "cannot be computed".into(),
        };
        let title = if name.is_empty() { g.show() } else { format!("{name}: {}", g.show()) };
        format!("  {title}\n      unseen n = 30k..100k: {}\n      far n = 100k..1M:     {}\n", cell(&test), cell(&far_ns))
    };
    r.push_str("\nfor comparison:\n");
    r.push_str(&line("A itself", &G::A));
    let wheel6 = G::Add(Box::new(G::Mul(Box::new(G::C(6)), Box::new(G::Div(Box::new(G::A), Box::new(G::C(6)))))), Box::new(G::C(1)));
    r.push_str(&line("a hand-made wheel (6k + 1 near A)", &wheel6));
    r.push_str("\nNuome's evolved formulas (best first):\n");
    for (_, g) in found.iter().take(5) {
        r.push_str(&line("", g));
    }
    // why the winner works: which small divisors its values avoid
    if let Some((_, best)) = found.first() {
        r.push_str(&format!("\nwhy the best works ({}): its values on the unseen n, by remainder:\n", best.show()));
        for m in [2i128, 3, 5, 7] {
            let mut counts = vec![0usize; m as usize];
            let mut total = 0;
            for &n in &test {
                if let Some(v) = best.eval(n) {
                    counts[v.rem_euclid(m) as usize] += 1;
                    total += 1;
                }
            }
            let zero = counts[0] as f64 / total.max(1) as f64;
            r.push_str(&format!(
                "  mod {m}: divisible by {m} in {:.2}% of values (a random number: {:.1}%){}\n",
                zero * 100.0,
                100.0 / m as f64,
                if zero < 0.001 { format!(" -> it never gives a multiple of {m}") } else { String::new() }
            ));
        }
        r.push_str(
            "  a function that only knows where the primes are can raise its odds only by avoiding small divisors (a wheel):\n  odd numbers 2x, also not divisible by 3 3x, by 5 too 3.75x, by 7 too 4.4x chance; it cannot know which of the rest\n  are prime without testing them\n",
        );
    }
    r.push_str(&format!("\n({:.1} s)\n", t0.elapsed().as_secs_f64()));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_is_close_to_the_nth_prime() {
        let ps = primes_to(1_400_000);
        let a = approx(100_000) as f64;
        assert!((a - ps[99_999] as f64).abs() / a < 0.002);
    }

    #[test]
    fn a_wheel_avoids_small_divisors() {
        let ps = Primes::new(2_000_000);
        let ns: Vec<i128> = (30_000..31_000).collect();
        let wheel = G::Add(Box::new(G::Mul(Box::new(G::C(6)), Box::new(G::Div(Box::new(G::A), Box::new(G::C(6)))))), Box::new(G::C(1)));
        let plain = measure(&G::A, &ns, &ps, 0.01).unwrap();
        let w = measure(&wheel, &ns, &ps, 0.01).unwrap();
        // 6k + 1 is never even nor a multiple of 3: about three times the primes
        assert!(w.1 > 2 * plain.1, "{} vs {}", w.1, plain.1);
    }
}
