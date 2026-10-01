//! The idea machine for Mersenne primes: can Nuome build a function that
//! finds the next one at once? Two tries, each learned on known data and
//! tested on data it never saw:
//!
//! A. a formula n -> p_n for the exponent of the n-th Mersenne prime, learned
//!    on the first 26 and tested on the last 26;
//! B. a score of a prime p that puts the Mersenne exponents first, learned on
//!    p < 5000 and tested on 5000 < p < 10^7, measured by enrichment: how many
//!    times more Mersenne exponents the top tenth holds than chance would give.
//!
//! A function that finds the prime at once would need test error near 0 in A,
//! or an enrichment in B in the millions; the report says what came out.

use crate::evolve::Rng;

/// The exponents of all 52 known Mersenne primes (OEIS A000043, GIMPS).
pub const EXPONENTS: [u64; 52] = [
    2, 3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127, 521, 607, 1279, 2203, 2281, 3217, 4253, 4423, 9689, 9941, 11213, 19937, 21701, 23209, 44497, 86243,
    110503, 132049, 216091, 756839, 859433, 1257787, 1398269, 2976221, 3021377, 6972593, 13466917, 20996011, 24036583, 25964951, 30402457, 32582657,
    37156667, 42643801, 43112609, 57885161, 74207281, 77232917, 82589933, 136279841,
];

#[derive(Clone)]
enum E {
    Feat(usize),
    Num(f64),
    Add(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
    Div(Box<E>, Box<E>),
}

impl E {
    fn eval(&self, f: &[f64]) -> f64 {
        match self {
            E::Feat(i) => f[*i],
            E::Num(c) => *c,
            E::Add(a, b) => a.eval(f) + b.eval(f),
            E::Mul(a, b) => a.eval(f) * b.eval(f),
            E::Div(a, b) => a.eval(f) / b.eval(f),
        }
    }
    fn size(&self) -> usize {
        match self {
            E::Feat(_) | E::Num(_) => 1,
            E::Add(a, b) | E::Mul(a, b) | E::Div(a, b) => 1 + a.size() + b.size(),
        }
    }
    fn show(&self, names: &[&str]) -> String {
        match self {
            E::Feat(i) => names[*i].into(),
            E::Num(c) => format!("{c:.4}"),
            E::Add(a, b) => format!("({} + {})", a.show(names), b.show(names)),
            E::Mul(a, b) => format!("{} * {}", a.show(names), b.show(names)),
            E::Div(a, b) => format!("{} / ({})", a.show(names), b.show(names)),
        }
    }
    /// The k-th node in prefix order.
    fn at(&mut self, k: usize) -> &mut E {
        if k == 0 {
            return self;
        }
        match self {
            E::Add(a, b) | E::Mul(a, b) | E::Div(a, b) => {
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

fn random_e(r: &mut Rng, depth: usize, feats: usize) -> E {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(3) == 0 { E::Num((r.unit() * 4.0 - 1.0) * 100.0 / 100.0) } else { E::Feat(r.below(feats)) };
    }
    let (a, b) = (Box::new(random_e(r, depth - 1, feats)), Box::new(random_e(r, depth - 1, feats)));
    match r.below(3) {
        0 => E::Add(a, b),
        1 => E::Mul(a, b),
        _ => E::Div(a, b),
    }
}

/// The genetic search shared by A and B: lower fitness is better.
fn evolve(feats: usize, generations: usize, seed: u64, fit: &dyn Fn(&E) -> f64) -> E {
    let mut r = Rng(seed);
    let mut pop: Vec<E> = (0..300).map(|_| random_e(&mut r, 3, feats)).collect();
    for _ in 0..generations {
        let mut scored: Vec<(f64, E)> = pop.drain(..).map(|e| (fit(&e), e)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next: Vec<E> = scored.iter().take(30).map(|x| x.1.clone()).collect();
        while next.len() < 300 {
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            let mut child = scored[best].1.clone();
            let k = r.below(child.size());
            match r.below(3) {
                0 => *child.at(k) = random_e(&mut r, 2, feats),
                _ => {
                    if let E::Num(c) = child.at(k) {
                        *c *= 1.0 + (r.unit() - 0.5) * 0.1;
                        *c += (r.unit() - 0.5) * 0.01;
                    } else {
                        *child.at(k) = random_e(&mut r, 1, feats);
                    }
                }
            }
            if child.size() <= 11 {
                next.push(child);
            }
        }
        pop = next;
    }
    pop.into_iter().map(|e| (fit(&e), e)).filter(|x| x.0.is_finite()).min_by(|a, b| a.0.total_cmp(&b.0)).expect("a finite one").1
}

fn sieve(limit: usize) -> Vec<bool> {
    let mut is = vec![true; limit];
    is[0] = false;
    is[1] = false;
    let mut i = 2;
    while i * i < limit {
        if is[i] {
            for j in (i * i..limit).step_by(i) {
                is[j] = false;
            }
        }
        i += 1;
    }
    is
}

/// Does 2^p - 1 have a factor 2kp + 1 with k <= kmax (other than itself)?
fn small_factor(p: u64, kmax: u64) -> bool {
    let whole = if p < 63 { Some((1u64 << p) - 1) } else { None };
    (1..=kmax).any(|k| {
        let q = 2 * k * p + 1;
        (q % 8 == 1 || q % 8 == 7) && Some(q) != whole && crate::mersenne::pow2_mod(p, q) == 1
    })
}

pub struct Settings {
    pub generations: usize,
    pub seed: u64,
    /// Primes up to this are scored in B.
    pub limit: usize,
}

pub fn report(s: &Settings) -> String {
    let mut out = vec!["Nuome's idea machine for Mersenne primes: can a function find the next one at once?".to_string(), String::new()];
    // ---- A: where is the n-th one?
    let pts: Vec<(f64, [f64; 2])> = EXPONENTS.iter().enumerate().map(|(i, &p)| ((p as f64).log2(), [(i + 1) as f64, ((i + 1) as f64).ln()])).collect();
    let (train, test) = pts.split_at(26);
    let cut = 18;
    let err = |e: &E, set: &[(f64, [f64; 2])]| set.iter().map(|(y, f)| (e.eval(f) - y).abs()).fold(0.0, f64::max);
    let mean_err = |e: &E, set: &[(f64, [f64; 2])]| set.iter().map(|(y, f)| (e.eval(f) - y).abs()).sum::<f64>() / set.len() as f64;
    let fit_a = |e: &E| {
        let m = mean_err(e, &train[..cut]) + 2.0 * mean_err(e, &train[cut..]) + 0.01 * e.size() as f64;
        if m.is_finite() { m } else { f64::INFINITY }
    };
    let a = evolve(2, s.generations, s.seed, &fit_a);
    let names = ["n", "ln n"];
    out.push("A. a formula for the exponent of the n-th Mersenne prime (log2 p_n), built by the genetic search from n and ln n, learned on n = 1..26:".into());
    out.push(format!("   Nuome's formula: log2 p_n = {}", a.show(&names)));
    let (ta, te) = (mean_err(&a, train), mean_err(&a, test));
    out.push(format!(
        "   error in log2 p: mean {ta:.2} on the learned 26, mean {te:.2} and worst {:.2} on the unseen 27..52; a miss of {te:.2} in log2 means a factor of {:.1} in p",
        err(&a, test),
        2f64.powf(te)
    ));
    // the known heuristic (Wagstaff): p_(n+1) / p_n is about 2^(1/e^gamma), a straight line in log2
    let slope = 1.0 / 0.5772156649f64.exp();
    let icpt = train.iter().map(|(y, f)| y - slope * f[0]).sum::<f64>() / train.len() as f64;
    let wag = E::Add(Box::new(E::Mul(Box::new(E::Num(slope)), Box::new(E::Feat(0)))), Box::new(E::Num(icpt)));
    out.push(format!("   the known heuristic (Wagstaff, log2 p_n = n / e^gamma + c) on the unseen: mean {:.2}, worst {:.2}", mean_err(&wag, test), err(&wag, test)));
    let y53 = a.eval(&[53.0, 53f64.ln()]);
    let (lo, hi) = (2f64.powf(y53 - te), 2f64.powf(y53 + te));
    let count = (hi / hi.ln() - lo / lo.ln()).max(0.0);
    out.push(format!(
        "   Nuome's prediction for the 53rd: p near {:.0}; with its own unseen error the window is {:.2e}..{:.2e}, about {:.1e} prime exponents, so it narrows the search but does not find the prime",
        2f64.powf(y53),
        lo,
        hi,
        count
    ));
    out.push(String::new());
    // ---- B: which primes are Mersenne exponents?
    let is = sieve(s.limit);
    let mers: std::collections::HashSet<u64> = EXPONENTS.iter().copied().collect();
    let primes: Vec<u64> = (3..s.limit as u64).filter(|&p| is[p as usize]).collect();
    let names_b = ["p mod 3", "p mod 4", "p mod 5", "p mod 8", "p mod 12", "ones in p", "2p+1 prime", "no factor 2kp+1, k<=256", "ln p"];
    let feats: Vec<[f64; 9]> = {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let slots = std::sync::Mutex::new(vec![[0.0; 9]; primes.len()]);
        std::thread::scope(|sc| {
            for _ in 0..std::thread::available_parallelism().map_or(4, |t| t.get()) {
                sc.spawn(|| loop {
                    let i0 = next.fetch_add(4096, std::sync::atomic::Ordering::Relaxed);
                    if i0 >= primes.len() {
                        break;
                    }
                    let chunk: Vec<(usize, [f64; 9])> = (i0..(i0 + 4096).min(primes.len()))
                        .map(|i| {
                            let p = primes[i];
                            let q = (2 * p + 1) as usize;
                            let twin = if q < is.len() { is[q] } else { (3..).step_by(2).take_while(|d| d * d <= q).all(|d| q % d != 0) };
                            (i, [(p % 3) as f64, (p % 4) as f64, (p % 5) as f64, (p % 8) as f64, (p % 12) as f64, p.count_ones() as f64, twin as u8 as f64, !small_factor(p, 256) as u8 as f64, (p as f64).ln()])
                        })
                        .collect();
                    let mut g = slots.lock().expect("no panics");
                    for (i, f) in chunk {
                        g[i] = f;
                    }
                });
            }
        });
        slots.into_inner().expect("done")
    };
    let split = primes.partition_point(|&p| p < 5000);
    let enrich = |e: &E, range: std::ops::Range<usize>| -> (f64, usize, usize) {
        let mut vals: Vec<(f64, bool)> = range.map(|i| (e.eval(&feats[i]), mers.contains(&primes[i]))).collect();
        if vals.iter().any(|(v, _)| !v.is_finite()) {
            return (0.0, 0, 0);
        }
        let pos = vals.iter().filter(|x| x.1).count();
        vals.sort_by(|a, b| b.0.total_cmp(&a.0));
        let thr = vals[vals.len() / 10].0;
        let top: Vec<&(f64, bool)> = vals.iter().filter(|x| x.0 >= thr).collect();
        let hit = top.iter().filter(|x| x.1).count();
        let share = top.len() as f64 / vals.len() as f64;
        ((hit as f64 / pos.max(1) as f64) / share, hit, pos)
    };
    let fit_b = |e: &E| -enrich(e, 0..split).0 + 0.01 * e.size() as f64;
    let b = evolve(9, s.generations, s.seed + 1, &fit_b);
    let (etr, htr, ptr) = enrich(&b, 0..split);
    let (ete, hte, pte) = enrich(&b, split..primes.len());
    out.push(format!(
        "B. a score of a prime p that puts Mersenne exponents first, built from: {}; learned on the {} primes below 5000 ({} Mersenne exponents), tested on the {} primes from 5000 to {} ({} Mersenne exponents)",
        names_b.join(", "),
        split,
        ptr,
        primes.len() - split,
        s.limit,
        pte
    ));
    out.push(format!("   Nuome's score: {}", b.show(&names_b)));
    out.push(format!("   top tenth by the score holds {htr}/{ptr} learned and {hte}/{pte} unseen Mersenne exponents: {etr:.1}x and {ete:.1}x more than chance (1.0x = no signal)"));
    let sieve_only = E::Feat(7);
    let (es, hs, _) = enrich(&sieve_only, split..primes.len());
    out.push(format!("   the trial-factoring flag alone on the unseen: {hs}/{pte}, {es:.1}x"));
    // much of the gain can come from ln p (small p are likelier), which does
    // not help inside one window of the search; so score inside one decade
    let decade = primes.partition_point(|&p| p < 1_000_000)..primes.len();
    let (ed, hd, pd) = enrich(&b, decade.clone());
    let (fd, hf, _) = enrich(&sieve_only, decade);
    out.push(format!("   inside one decade only (10^6 to 10^7, where ln p hardly changes, as in the prize window): Nuome's score {hd}/{pd}, {ed:.1}x; the flag alone {hf}/{pd}, {fd:.1}x"));
    out.push(String::new());
    out.push("what this shows: neither function finds a Mersenne prime at once. A narrows where the next one lies to a window of millions of candidates; B can only reorder the candidates, by a factor of a few, and what carries the signal is whether 2^p - 1 has a small factor, which is the trial factoring GIMPS already does. A function that names the prime directly would need errors near 0 in A or an enrichment near the number of candidates in B; nothing here comes close, and none is known in mathematics".into());
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_factors_match_known() {
        // 2^11 - 1 = 23 * 89, 23 = 2*1*11 + 1; 2^13 - 1 is prime
        assert!(small_factor(11, 4));
        assert!(!small_factor(13, 400));
        assert!(!small_factor(7, 256)); // 127 itself does not count
    }
}
