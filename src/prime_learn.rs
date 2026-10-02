//! The idea machine that learns what a prime is from the first hundred
//! numbers, the Goldbach way. Nuome is told only which of 1..100 are prime;
//! it breeds rules from n, a counter d, small constants, +, -, *, mod,
//! floor(sqrt n), =, <, not, and, or, and three counting forms: "for every
//! d from a to b", "for some d from a to b", "how many d from a to b".
//! Fitness: mistakes on 1..60, then a forward check on 61..100 counting
//! double; a price per symbol and a price per bit of every constant
//! (Occam, by description length: "a divisor up to 7" fits 1..100 too, but
//! 7 is a number the data chose, and it fails at 121 = 11 * 11); and a
//! small price for the work a rule does. The winner is then tested on every
//! number it never saw.

use crate::evolve::Rng;

#[derive(Clone, Debug, PartialEq)]
enum T {
    N,
    D,
    C(i64),
    Sqrt,
    Add(Box<T>, Box<T>),
    Sub(Box<T>, Box<T>),
    Mul(Box<T>, Box<T>),
    Mod(Box<T>, Box<T>),
    Eq(Box<T>, Box<T>),
    Lt(Box<T>, Box<T>),
    Not(Box<T>),
    And(Box<T>, Box<T>),
    Or(Box<T>, Box<T>),
    /// for every d in lo..=hi, body
    All(Box<T>, Box<T>, Box<T>),
    /// for some d in lo..=hi, body
    Any(Box<T>, Box<T>, Box<T>),
    /// how many d in lo..=hi make body true
    Count(Box<T>, Box<T>, Box<T>),
}

/// Evaluation: n, the counter if inside a counting form, and the work done.
struct Ctx {
    n: i64,
    work: u64,
    limit: u64,
}

impl T {
    fn eval(&self, c: &mut Ctx, d: Option<i64>) -> Option<i64> {
        let b = |v: bool| v as i64;
        Some(match self {
            T::N => c.n,
            T::D => d?,
            T::C(k) => *k,
            T::Sqrt => (c.n.max(0) as f64).sqrt().floor() as i64,
            T::Add(x, y) => x.eval(c, d)?.checked_add(y.eval(c, d)?)?,
            T::Sub(x, y) => x.eval(c, d)?.checked_sub(y.eval(c, d)?)?,
            T::Mul(x, y) => x.eval(c, d)?.checked_mul(y.eval(c, d)?)?,
            T::Mod(x, y) => {
                // mod 0 (or less) is 0: a rule that tries it is wrong there, not broken
                let m = y.eval(c, d)?;
                let v = x.eval(c, d)?;
                if m <= 0 {
                    0
                } else {
                    v.rem_euclid(m)
                }
            }
            T::Eq(x, y) => b(x.eval(c, d)? == y.eval(c, d)?),
            T::Lt(x, y) => b(x.eval(c, d)? < y.eval(c, d)?),
            T::Not(x) => b(x.eval(c, d)? == 0),
            T::And(x, y) => b(x.eval(c, d)? != 0 && y.eval(c, d)? != 0),
            T::Or(x, y) => b(x.eval(c, d)? != 0 || y.eval(c, d)? != 0),
            T::All(lo, hi, body) | T::Any(lo, hi, body) | T::Count(lo, hi, body) => {
                // one counter: no counting form inside another
                if d.is_some() {
                    return None;
                }
                let (lo, hi) = (lo.eval(c, None)?, hi.eval(c, None)?);
                let mut count = 0i64;
                let mut x = lo;
                while x <= hi {
                    c.work += 1;
                    if c.work > c.limit {
                        return None;
                    }
                    let v = body.eval(c, Some(x))? != 0;
                    match self {
                        T::All(..) if !v => return Some(0),
                        T::Any(..) if v => return Some(1),
                        _ => {}
                    }
                    count += v as i64;
                    x += 1;
                }
                match self {
                    T::All(..) => 1,
                    T::Any(..) => 0,
                    _ => count,
                }
            }
        })
    }
    fn kids(&mut self) -> Vec<&mut T> {
        match self {
            T::N | T::D | T::C(_) | T::Sqrt => vec![],
            T::Not(x) => vec![x],
            T::Add(x, y) | T::Sub(x, y) | T::Mul(x, y) | T::Mod(x, y) | T::Eq(x, y) | T::Lt(x, y) | T::And(x, y) | T::Or(x, y) => vec![x, y],
            T::All(a, b, c) | T::Any(a, b, c) | T::Count(a, b, c) => vec![a, b, c],
        }
    }
    fn size(&self) -> usize {
        1 + self.clone().kids().iter().map(|k| k.size()).sum::<usize>()
    }
    /// bits to write every constant: log2(|c| + 2) each
    fn const_bits(&self) -> f64 {
        match self {
            T::C(k) => ((k.unsigned_abs() + 2) as f64).log2(),
            _ => self.clone().kids().iter().map(|k| k.const_bits()).sum(),
        }
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
    fn show(&self) -> String {
        match self {
            T::N => "n".into(),
            T::D => "d".into(),
            T::C(k) => k.to_string(),
            T::Sqrt => "floor(sqrt n)".into(),
            T::Add(x, y) => format!("({} + {})", x.show(), y.show()),
            T::Sub(x, y) => format!("({} - {})", x.show(), y.show()),
            T::Mul(x, y) => format!("{}*{}", x.show(), y.show()),
            T::Mod(x, y) => format!("({} mod {})", x.show(), y.show()),
            T::Eq(x, y) => format!("{} = {}", x.show(), y.show()),
            T::Lt(x, y) => format!("{} < {}", x.show(), y.show()),
            T::Not(x) => format!("not ({})", x.show()),
            T::And(x, y) => format!("({} and {})", x.show(), y.show()),
            T::Or(x, y) => format!("({} or {})", x.show(), y.show()),
            T::All(a, b, c) => format!("[for every d from {} to {}: {}]", a.show(), b.show(), c.show()),
            T::Any(a, b, c) => format!("[for some d from {} to {}: {}]", a.show(), b.show(), c.show()),
            T::Count(a, b, c) => format!("#[d from {} to {}: {}]", a.show(), b.show(), c.show()),
        }
    }
}

/// A leaf: d only inside a counting form's body.
fn leaf(r: &mut Rng, inside: bool) -> T {
    match r.below(if inside { 7 } else { 5 }) {
        0 | 1 => T::N,
        2 => T::Sqrt,
        3 | 4 => T::C(r.below(11) as i64),
        _ => T::D,
    }
}

/// A random rule, by the grammar: `inside` a counting form's body (d may
/// appear), `q` whether a counting form may start here (never nested).
fn random_t(r: &mut Rng, depth: usize, inside: bool, q: bool) -> T {
    if depth == 0 || r.below(4) == 0 {
        return leaf(r, inside);
    }
    if q && r.below(3) == 0 {
        // bounds are plain (no d), the body may use d
        let lo = Box::new(random_t(r, 1, false, false));
        let hi = Box::new(random_t(r, 1, false, false));
        let body = Box::new(random_t(r, depth - 1, true, false));
        return match r.below(3) {
            0 => T::All(lo, hi, body),
            1 => T::Any(lo, hi, body),
            _ => T::Count(lo, hi, body),
        };
    }
    let a = Box::new(random_t(r, depth - 1, inside, q));
    let b = Box::new(random_t(r, depth - 1, inside, q));
    match r.below(11) {
        0 => T::Add(a, b),
        1 => T::Sub(a, b),
        2 => T::Mul(a, b),
        3 | 4 => T::Mod(a, b),
        5 | 6 => T::Eq(a, b),
        7 => T::Lt(a, b),
        8 => T::Not(a),
        9 => T::And(a, b),
        _ => T::Or(a, b),
    }
}

impl T {
    /// Where node k sits: (inside a body, inside a counting form at all).
    fn context(&self, k: usize, inside: bool, in_q: bool) -> (bool, bool) {
        if k == 0 {
            return (inside, in_q);
        }
        let mut k = k - 1;
        let quant = matches!(self, T::All(..) | T::Any(..) | T::Count(..));
        for (i, c) in self.clone().kids().iter().enumerate() {
            let s = c.size();
            if k < s {
                let body = quant && i == 2;
                return c.context(k, inside || body, in_q || quant);
            }
            k -= s;
        }
        (inside, in_q)
    }
}

fn is_prime(n: i64) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

/// (mistakes, work) on the numbers `ns`, or None if the rule breaks.
fn judge(t: &T, ns: &[i64], limit: u64) -> Option<(usize, u64)> {
    let mut c = Ctx { n: 0, work: 0, limit };
    let mut wrong = 0;
    for &n in ns {
        c.n = n;
        let v = t.eval(&mut c, None)? != 0;
        if v != is_prime(n) {
            wrong += 1;
        }
    }
    Some((wrong, c.work))
}

pub struct Settings {
    pub population: usize,
    pub generations: usize,
    pub seed: u64,
    /// independent evolutions run side by side (islands)
    pub islands: usize,
    /// learned on 0..=learn_to, forward check on learn_to+1..=upto
    pub learn_to: i64,
    pub upto: i64,
}

/// One island's evolution: its last generation, best first, and its log.
fn evolve(s: &Settings, seed: u64, early: &[i64], late: &[i64]) -> (Vec<(f64, T)>, Vec<String>) {
    let all: Vec<i64> = early.iter().chain(late).copied().collect();
    let fit = |t: &T| -> f64 {
        let (Some(a), Some(b)) = (judge(t, early, 400_000), judge(t, late, 400_000)) else { return f64::INFINITY };
        let mistakes = (a.0 as f64 + 2.0 * b.0 as f64) / (early.len() as f64 + 2.0 * late.len() as f64);
        let work = (a.1 + b.1) as f64 / all.len() as f64;
        mistakes + 0.003 * t.size() as f64 + 0.002 * t.const_bits() + 0.0001 * work
    };
    let mut r = Rng(seed);
    let mut pop: Vec<T> = (0..s.population).map(|_| random_t(&mut r, 5, false, true)).collect();
    let mut log = Vec::new();
    for g in 0..s.generations {
        let mut scored: Vec<(f64, T)> = pop.drain(..).map(|t| (fit(&t), t)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        if g % 100 == 0 || g + 1 == s.generations {
            let best = &scored[0].1;
            let m = judge(best, &all, 400_000).map_or(all.len(), |x| x.0);
            log.push(format!("generation {:>3}: {} mistakes: {}", g, m, best.show()));
        }
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
            if r.below(3) == 0 {
                let mut other = scored[r.below(s.population / 4)].1.clone();
                let k2 = r.below(other.size());
                let (in_b2, in_q2) = other.context(k2, false, false);
                let branch = other.at(k2).clone();
                let k = r.below(child.size());
                let (in_b, in_q) = child.context(k, false, false);
                // a branch only where it fits the grammar
                let has_q = matches!(branch, T::All(..) | T::Any(..) | T::Count(..)) || format!("{:?}", branch).contains("All(") || format!("{:?}", branch).contains("Any(") || format!("{:?}", branch).contains("Count(");
                let has_d = format!("{:?}", branch).contains("D");
                if (!has_d || in_b) && (!has_q || !in_q) && (in_b2 == in_b || !has_d) && (in_q2 || !in_q || !has_q) {
                    *child.at(k) = branch;
                }
            } else {
                let k = r.below(child.size());
                let (inside, in_q) = child.context(k, false, false);
                match child.at(k) {
                    T::C(c) if r.below(2) == 0 => *c = (*c + r.below(5) as i64 - 2).max(0),
                    node => *node = random_t(&mut r, 3, inside, !in_q),
                }
            }
            if child.size() <= 25 {
                next.push(child);
            }
        }
        pop = next;
    }
    let mut last: Vec<(f64, T)> = pop.into_iter().map(|t| (fit(&t), t)).filter(|x| x.0.is_finite()).collect();
    last.sort_by(|a, b| a.0.total_cmp(&b.0));
    (last, log)
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let early: Vec<i64> = (0..=s.learn_to).collect();
    let late: Vec<i64> = (s.learn_to + 1..=s.upto).collect();
    // the islands, side by side, each from its own seed
    let runs: Vec<(Vec<(f64, T)>, Vec<String>)> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..s.islands).map(|i| {
            let (early, late) = (&early, &late);
            sc.spawn(move || evolve(s, s.seed + i as u64 * 7919, early, late))
        }).collect();
        hs.into_iter().map(|h| h.join().expect("island")).collect()
    });
    let mut log = Vec::new();
    let mut last: Vec<(f64, T)> = Vec::new();
    for (i, (pop, l)) in runs.into_iter().enumerate() {
        log.push(format!("  island {:>2}: {}", i + 1, l.last().cloned().unwrap_or_default()));
        last.extend(pop.into_iter().take(20));
    }
    // the best of all islands, by fitness alone (the unseen numbers play no part)
    last.sort_by(|a, b| a.0.total_cmp(&b.0));
    last.dedup_by(|a, b| a.1 == b.1);
    let unseen: Vec<i64> = (s.upto + 1..=10_000).collect();
    let far: Vec<i64> = (999_000..=1_001_000).collect();
    let mut out = String::new();
    out.push_str(&format!(
        "Nuome invents a function for the primes in 0..{}\n\ntold only which of 0..{} are prime ({} of them: {}); building blocks: n, a counter d, constants 0..10, +, -, *, mod, floor(sqrt n), =, <, not, and, or, and \"for every / for some / how many d from a to b\"\nfitness: mistakes on 0..{} plus a forward check on {}..{} counting double; a price per symbol and per bit of each constant; a little for the work\n{} islands of {} rules, {} generations each, seeds from {}\n\nthe best of each island at the end:\n{}\n",
        s.upto,
        s.upto,
        (0..=s.upto).filter(|&n| is_prime(n)).count(),
        (0..=s.upto).filter(|&n| is_prime(n)).map(|n| n.to_string()).collect::<Vec<_>>().join(" "),
        s.learn_to,
        s.learn_to + 1,
        s.upto,
        s.islands,
        s.population,
        s.generations,
        s.seed,
        log.join("\n")
    ));
    out.push_str("\nthe best rules, tested on numbers never seen:\n");
    for (_, t) in last.iter().take(5) {
        let learned = judge(t, &(0..=s.upto).collect::<Vec<_>>(), u64::MAX).map_or(s.upto as usize + 1, |x| x.0);
        let first_wrong = unseen.iter().find(|&&n| judge(t, &[n], 50_000_000).is_none_or(|x| x.0 > 0));
        let u = judge(t, &unseen, u64::MAX);
        let f = judge(t, &far, u64::MAX);
        out.push_str(&format!(
            "  {}\n      0..{}: {} mistakes; {}..10,000 (never seen): {}; 999,000..1,001,000: {}; work per number near a million: {}\n",
            t.show(),
            s.upto,
            learned,
            s.upto + 1,
            match (u, first_wrong) {
                (Some((0, _)), _) => "no mistakes".to_string(),
                (Some((w, _)), Some(n)) => format!("{w} mistakes, the first at {n}"),
                _ => "breaks".into(),
            },
            match f {
                Some((0, _)) => "no mistakes".to_string(),
                Some((w, _)) => format!("{w} mistakes"),
                None => "breaks".into(),
            },
            f.map_or("-".into(), |x| format!("{:.0}", x.1 as f64 / far.len() as f64))
        ));
    }
    out.push_str(&format!("\n({:.1} s)\n", t0.elapsed().as_secs_f64()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_definition_is_exact_and_the_overfit_fails_at_121() {
        // n has exactly two divisors
        let def = T::Eq(Box::new(T::Count(Box::new(T::C(1)), Box::new(T::N), Box::new(T::Eq(Box::new(T::Mod(Box::new(T::N), Box::new(T::D))), Box::new(T::C(0)))))), Box::new(T::C(2)));
        let ns: Vec<i64> = (1..=2000).collect();
        assert_eq!(judge(&def, &ns, u64::MAX).unwrap().0, 0);
        // no divisor from 2 to 7, and more than 1 (or a prime up to 7): right on 1..100, wrong at 121
        let small = T::And(
            Box::new(T::Lt(Box::new(T::C(1)), Box::new(T::N))),
            Box::new(T::All(Box::new(T::C(2)), Box::new(T::C(7)), Box::new(T::Or(Box::new(T::Eq(Box::new(T::D), Box::new(T::N))), Box::new(T::Not(Box::new(T::Eq(Box::new(T::Mod(Box::new(T::N), Box::new(T::D))), Box::new(T::C(0)))))))))),
        );
        assert_eq!(judge(&small, &(1..=100).collect::<Vec<_>>(), u64::MAX).unwrap().0, 0);
        assert_eq!(judge(&small, &[121], u64::MAX).unwrap().0, 1);
        // the definition costs fewer constant bits than the overfit
        assert!(def.const_bits() < small.const_bits());
    }
}
