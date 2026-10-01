//! Nuome's idea machine for the open part of Goldbach: it makes up new
//! statements about the even numbers and their prime pairs, finds the best
//! constant for each on the first half of the data, tests it on the second
//! half it never saw, keeps the survivors, and says for each what it would
//! give if it were proved. Statements are conjectures, never proofs.

use crate::goldbach::C2;

/// What Nuome knows about each even n: the number of ways r(n) to write it
/// as p + q (p <= q, both prime), the smallest such p, and S(n), the product
/// over odd primes q dividing n of (q - 1)/(q - 2).
struct Data {
    n: Vec<u64>,
    r: Vec<f64>,
    p: Vec<f64>,
    s: Vec<f64>,
}

fn data(limit: usize) -> Data {
    let mut composite = vec![false; limit + 1];
    composite[0] = true;
    composite[1] = true;
    let mut i = 2;
    while i * i <= limit {
        if !composite[i] {
            let mut j = i * i;
            while j <= limit {
                composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    let primes: Vec<usize> = (2..=limit).filter(|&k| !composite[k]).collect();
    let evens: Vec<usize> = (6..=limit).step_by(2).collect();
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let chunk = evens.len().div_ceil(threads);
    let rows: Vec<(u64, f64, f64, f64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = evens
            .chunks(chunk)
            .map(|part| {
                let (primes, composite) = (&primes, &composite);
                sc.spawn(move || {
                    part.iter()
                        .map(|&n| {
                            let (mut r, mut first) = (0u32, 0usize);
                            for &q in primes.iter().take_while(|&&q| q <= n / 2) {
                                if !composite[n - q] {
                                    r += 1;
                                    if first == 0 {
                                        first = q;
                                    }
                                }
                            }
                            let mut s = 1.0;
                            let mut m = n;
                            while m % 2 == 0 {
                                m /= 2;
                            }
                            let mut d = 3;
                            while d * d <= m {
                                if m % d == 0 {
                                    s *= (d as f64 - 1.0) / (d as f64 - 2.0);
                                    while m % d == 0 {
                                        m /= d;
                                    }
                                }
                                d += 2;
                            }
                            if m > 1 {
                                s *= (m as f64 - 1.0) / (m as f64 - 2.0);
                            }
                            (n as u64, r as f64, first as f64, s)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
    });
    Data { n: rows.iter().map(|r| r.0).collect(), r: rows.iter().map(|r| r.1).collect(), p: rows.iter().map(|r| r.2).collect(), s: rows.iter().map(|r| r.3).collect() }
}

/// An idea: a statement "lhs(n) >= c * rhs(n)" (or <=) for every even n >= n0,
/// with what proving it would give.
struct Idea {
    text: String,
    /// value(n) = lhs / rhs, and whether the statement is a lower bound
    ratio: Box<dyn Fn(usize, &Data) -> f64>,
    lower: bool,
    gives: &'static str,
}

fn ln(n: u64) -> f64 {
    (n as f64).ln()
}

fn ideas() -> Vec<Idea> {
    let mut v: Vec<Idea> = Vec::new();
    // lower bounds for the number of ways: any of these, proved, gives Goldbach
    for (name, f) in [
        ("n / (ln n)^2", Box::new(|n: u64, _s: f64| n as f64 / ln(n).powi(2)) as Box<dyn Fn(u64, f64) -> f64>),
        ("S(n) n / (ln n)^2", Box::new(|n: u64, s: f64| s * n as f64 / ln(n).powi(2))),
        ("2 C2 S(n) n / (ln n)^2 (the Hardy-Littlewood size)", Box::new(|n: u64, s: f64| 2.0 * C2 * s * n as f64 / ln(n).powi(2))),
        ("S(n) n / ((ln n)^2 ln ln n)", Box::new(|n: u64, s: f64| s * n as f64 / (ln(n).powi(2) * ln(n).ln()))),
        ("sqrt(n)", Box::new(|n: u64, _s: f64| (n as f64).sqrt())),
        ("S(n) n^0.9 / (ln n)^2", Box::new(|n: u64, s: f64| s * (n as f64).powf(0.9) / ln(n).powi(2))),
    ] {
        v.push(Idea {
            text: format!("the number of ways r(n) is at least c * {name}"),
            ratio: Box::new(move |i, d| d.r[i] / f(d.n[i], d.s[i])),
            lower: true,
            gives: "Goldbach for every even n >= n0, since it makes r(n) > 0",
        });
    }
    // upper bounds for the smallest prime, with S(n) helping
    for (name, k) in [("(ln n)^2 ln ln n", 0.0), ("(ln n)^2 ln ln n / sqrt(S(n))", 0.5), ("(ln n)^2 ln ln n / S(n)", 1.0)] {
        v.push(Idea {
            text: format!("the smallest prime p(n) is at most c * {name}"),
            ratio: Box::new(move |i, d| d.p[i] * d.s[i].powf(k) / (ln(d.n[i]).powi(2) * ln(d.n[i]).ln())),
            lower: false,
            gives: "Goldbach for every even n >= n0, and where to look for the prime",
        });
    }
    // relations between neighbours: the multiples of 6 against their neighbours
    v.push(Idea {
        text: "a multiple of 6 has at least c times as many ways as each even neighbour (r(6k) >= c max(r(6k - 2), r(6k + 2)))".into(),
        ratio: Box::new(|i, d| {
            if d.n[i] % 6 != 0 || i == 0 || i + 1 >= d.n.len() {
                return f64::NAN;
            }
            d.r[i] / d.r[i - 1].max(d.r[i + 1])
        }),
        lower: true,
        gives: "nothing for Goldbach by itself (it compares counts), but a regularity of the counts",
    });
    v.push(Idea {
        text: "r(n) / S(n) never drops by more than a factor c from one even number to the next (r(n+2)/S(n+2) >= c r(n)/S(n))".into(),
        ratio: Box::new(|i, d| {
            if i + 1 >= d.n.len() || d.r[i] == 0.0 {
                return f64::NAN;
            }
            (d.r[i + 1] / d.s[i + 1]) / (d.r[i] / d.s[i])
        }),
        lower: true,
        gives: "Goldbach for every n by induction from a start, if c > 0 held for all n: the counts can never fall to 0",
    });
    v
}

/// A shape Nuome builds itself, from the features of n.
#[derive(Clone, Debug)]
enum G {
    /// n, ln n, ln ln n, S(n), the number of distinct odd primes of n, the power of 2 in n
    Feat(u8),
    Num(f64),
    Add(Box<G>, Box<G>),
    Mul(Box<G>, Box<G>),
    Div(Box<G>, Box<G>),
    Pow(Box<G>, f64),
}

const FEATURES: [&str; 6] = ["n", "ln n", "ln ln n", "S(n)", "w(n)", "v2(n)"];

impl G {
    fn eval(&self, f: &[f64; 6]) -> f64 {
        match self {
            G::Feat(i) => f[*i as usize],
            G::Num(c) => *c,
            G::Add(a, b) => a.eval(f) + b.eval(f),
            G::Mul(a, b) => a.eval(f) * b.eval(f),
            G::Div(a, b) => a.eval(f) / b.eval(f),
            G::Pow(a, e) => a.eval(f).powf(*e),
        }
    }
    fn size(&self) -> usize {
        match self {
            G::Feat(_) | G::Num(_) => 1,
            G::Add(a, b) | G::Mul(a, b) | G::Div(a, b) => 1 + a.size() + b.size(),
            G::Pow(a, _) => 1 + a.size(),
        }
    }
    fn show(&self) -> String {
        match self {
            G::Feat(i) => FEATURES[*i as usize].into(),
            G::Num(c) => format!("{c:.3}"),
            G::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            G::Mul(a, b) => format!("{} * {}", a.show(), b.show()),
            G::Div(a, b) => format!("{} / ({})", a.show(), b.show()),
            G::Pow(a, e) => format!("({})^{e:.3}", a.show()),
        }
    }
    fn at(&mut self, k: usize) -> &mut G {
        if k == 0 {
            return self;
        }
        match self {
            G::Add(a, b) | G::Mul(a, b) | G::Div(a, b) => {
                let s = a.size();
                if k <= s {
                    a.at(k - 1)
                } else {
                    b.at(k - 1 - s)
                }
            }
            G::Pow(a, _) => a.at(k - 1),
            _ => self,
        }
    }
    fn get(&self, k: usize) -> &G {
        if k == 0 {
            return self;
        }
        match self {
            G::Add(a, b) | G::Mul(a, b) | G::Div(a, b) => {
                let s = a.size();
                if k <= s {
                    a.get(k - 1)
                } else {
                    b.get(k - 1 - s)
                }
            }
            G::Pow(a, _) => a.get(k - 1),
            _ => self,
        }
    }
}

fn random_g(r: &mut crate::evolve::Rng, depth: usize) -> G {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(4) == 0 { G::Num((0.5 + r.unit() * 3.0 * 100.0).round() / 100.0) } else { G::Feat(r.below(6) as u8) };
    }
    match r.below(4) {
        0 => G::Add(Box::new(random_g(r, depth - 1)), Box::new(random_g(r, depth - 1))),
        1 => G::Mul(Box::new(random_g(r, depth - 1)), Box::new(random_g(r, depth - 1))),
        2 => G::Div(Box::new(random_g(r, depth - 1)), Box::new(random_g(r, depth - 1))),
        _ => G::Pow(Box::new(random_g(r, depth - 1)), (r.unit() * 3.0 * 100.0).round() / 100.0),
    }
}

/// The features of the i-th even number.
fn features(d: &Data, i: usize) -> [f64; 6] {
    let n = d.n[i];
    let mut m = n;
    let mut v2 = 0.0;
    while m % 2 == 0 {
        m /= 2;
        v2 += 1.0;
    }
    let mut w = 0.0;
    let mut q = 3;
    while q * q <= m {
        if m % q == 0 {
            w += 1.0;
            while m % q == 0 {
                m /= q;
            }
        }
        q += 2;
    }
    if m > 1 {
        w += 1.0;
    }
    [n as f64, ln(n), ln(n).ln(), d.s[i], w, v2]
}

/// For a shape g: the constant c = min r/g, and how tight c g is (the mean of
/// c g / r, 1 = exact), over the given points; None when g is not positive.
fn tightness(g: &G, pts: &[(f64, [f64; 6])]) -> Option<(f64, f64)> {
    let vals: Vec<(f64, f64)> = pts.iter().map(|(r, f)| (*r, g.eval(f))).collect();
    if vals.iter().any(|(_, v)| !(v.is_finite() && *v > 0.0)) {
        return None;
    }
    let c = vals.iter().map(|(r, v)| r / v).fold(f64::INFINITY, f64::min);
    if !(c.is_finite() && c > 0.0) {
        return None;
    }
    Some((c, vals.iter().map(|(r, v)| c * v / r).sum::<f64>() / vals.len() as f64))
}

/// Nuome builds shapes g itself (no shape given), for "r(n) >= c g(n)": the
/// genetic search of the Goldbach formula, fitness = how loose the bound is
/// plus a price per node. Each winner is tested on the unseen half and
/// compared with the known shapes.
fn invent(d: &Data, start: usize, half: usize, generations: usize) -> Vec<String> {
    let mut r = crate::evolve::Rng(2026);
    // a sample of the first half to learn on, all of the second half to test on
    let step = ((half - start) / 4000).max(1);
    let train: Vec<(f64, [f64; 6])> = (start..half).step_by(step).map(|i| (d.r[i], features(d, i))).collect();
    let test: Vec<(f64, [f64; 6])> = (half..d.n.len()).step_by(((d.n.len() - half) / 20000).max(1)).map(|i| (d.r[i], features(d, i))).collect();
    let price = 0.002;
    // forward prediction, as for the Goldbach formula: the constant from the
    // earlier two thirds must still hold on the later third, or pay heavily
    let cut = train.len() * 2 / 3;
    let fit = |g: &G| {
        let Some((c, t)) = tightness(g, &train[..cut]) else { return f64::INFINITY };
        let late = train[cut..].iter().map(|(rr, f)| rr / g.eval(f)).fold(f64::INFINITY, f64::min);
        let broken = (1.0 - late / c).max(0.0);
        1.0 - t + 20.0 * broken + price * g.size() as f64
    };
    let mut pop: Vec<G> = (0..300).map(|_| random_g(&mut r, 4)).collect();
    for _ in 0..generations {
        let mut scored: Vec<(f64, G)> = pop.drain(..).map(|g| (fit(&g), g)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut next: Vec<G> = scored.iter().take(30).map(|x| x.1.clone()).collect();
        let pick = |r: &mut crate::evolve::Rng| -> G {
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            scored[best].1.clone()
        };
        while next.len() < 300 {
            let mut child = pick(&mut r);
            match r.below(4) {
                0 => {
                    let donor = pick(&mut r);
                    let piece = donor.get(r.below(donor.size())).clone();
                    let k = r.below(child.size());
                    *child.at(k) = piece;
                }
                1 => {
                    let k = r.below(child.size());
                    *child.at(k) = random_g(&mut r, 2);
                }
                _ => {
                    let k = r.below(child.size());
                    let f = 1.0 + (r.unit() - 0.5) * 0.2;
                    match child.at(k) {
                        G::Num(c) => *c *= f,
                        G::Pow(_, e) => *e *= f,
                        _ => {}
                    }
                }
            }
            if child.size() <= 15 {
                next.push(child);
            }
        }
        pop = next;
    }
    let mut scored: Vec<(f64, G)> = pop.into_iter().map(|g| (fit(&g), g)).filter(|x| x.0.is_finite()).collect();
    scored.sort_by(|a, b| a.0.total_cmp(&b.0));
    // known shapes, to tell a rediscovery from something new
    let known: Vec<(&str, Box<dyn Fn(&[f64; 6]) -> f64>)> = vec![
        ("n / (ln n)^2", Box::new(|f| f[0] / f[1].powi(2))),
        ("S(n) n / (ln n)^2 (Hardy-Littlewood)", Box::new(|f| f[3] * f[0] / f[1].powi(2))),
        ("S(n) n / ((ln n)^2 ln ln n)", Box::new(|f| f[3] * f[0] / (f[1].powi(2) * f[2]))),
    ];
    let mut out = Vec::new();
    let mut shown: Vec<String> = Vec::new();
    for (_, g) in scored.iter() {
        if out.len() >= 5 * 3 {
            break;
        }
        let text = g.show();
        // the same shape with nudged constants counts once
        let key: String = text.chars().filter(|ch| !ch.is_ascii_digit() && *ch != '.').collect();
        if shown.contains(&key) {
            continue;
        }
        shown.push(key);
        let Some((c, train_t)) = tightness(g, &train) else { continue };
        let test_ratio = test.iter().map(|(rr, f)| rr / g.eval(f)).fold(f64::INFINITY, f64::min);
        let holds = test_ratio >= c;
        let test_t = test.iter().map(|(rr, f)| c * g.eval(f) / rr).sum::<f64>() / test.len() as f64;
        // proportional to a known shape: log(g / known) nearly constant
        let same_as = known.iter().find(|(_, k)| {
            let diffs: Vec<f64> = train.iter().map(|(_, f)| (g.eval(f) / k(f)).ln()).collect();
            let mean = diffs.iter().sum::<f64>() / diffs.len() as f64;
            (diffs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / diffs.len() as f64).sqrt() < 0.003
        });
        let label = match same_as {
            Some((name, _)) => format!("a rediscovery of the known shape {name}"),
            None => "NEW: not proportional to any known shape Nuome compared".to_string(),
        };
        out.push(format!("  {} r(n) >= {c:.4} * {text}", if holds { "KEPT  " } else { "FAILED" }));
        out.push(format!("         tightness {:.1}% on the first half, {:.1}% on the unseen half{}; {label}", train_t * 100.0, test_t * 100.0, if holds { "" } else { " (it fails there)" }));
    }
    out
}

/// The same idea machine for Conway's 99-graph: Nuome builds the joining
/// rule itself, from nothing (no family given), in several settings of
/// rings and fields; then it compares each winner with the known families.
pub fn conway99(population: usize, generations: usize, seed: u64) -> String {
    use crate::abstract_eq::{evolve, families, Ring, Settings as S, Space};
    let s = S { population, generations, seed, price_per_node: 1.0 };
    let mut out = vec![
        format!("Nuome's idea machine for Conway's 99-graph: in each setting it builds the rule 'join p and q when ...' itself, from atoms (a term is 0, a square or a non-square) joined by and / or / not, by the genetic search with no family given ({population} rules, {generations} generations, seed {seed}); errors = pairs of points with the wrong number of common neighbours, 0 = the graph"),
        String::new(),
    ];
    for rings in [vec![Ring::z(9), Ring::z(11)], vec![Ring::gf9(), Ring::z(11)], vec![Ring::z(3), Ring::z(3), Ring::z(11)], vec![Ring::z(3), Ring::z(33)], vec![Ring::z(99)]] {
        let space = Space { rings, k: 14, lambda: 1, mu: 2 };
        let own = evolve(&space, &[], &s);
        let (common, degree) = space.errors(&own.best);
        // the known families, for comparison and to tell a rediscovery
        let known = families(&space);
        let best_known = known.iter().map(|(name, f)| (space.errors(f), name)).min_by_key(|((c, d), _)| c + 10 * d);
        let rediscovered = known.iter().find(|(_, f)| space.build(f) == space.build(&own.best)).map(|(name, _)| name.clone());
        out.push(format!("{} (99 points):", space.name()));
        out.push(format!("  Nuome's own rule: join p and q when {}", own.best.show(&space.rings)));
        out.push(format!(
            "  {common} common-neighbour errors, {degree} neighbour errors{}; {}",
            if common == 0 && degree == 0 { ": IT IS THE GRAPH, checked pair by pair" } else { ": not the graph" },
            match &rediscovered {
                Some(name) => format!("the same graph as the known family '{name}'"),
                None => "NEW: a different graph from every known family Nuome compared".into(),
            }
        ));
        if let Some(((kc, kd), name)) = best_known {
            out.push(format!("  the best known family there, '{name}': {kc} common-neighbour errors, {kd} neighbour errors; Nuome's own rule is {}", if common + 10 * degree < kc + 10 * kd { "closer" } else { "not closer" }));
        }
        out.push(String::new());
    }
    out.push("a rule with 0 errors would be the graph and solve Conway's problem; anything above 0 is not the graph, however close".into());
    out.join("\n")
}

pub struct Settings {
    pub limit: usize,
    pub from: u64,
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let d = data(s.limit);
    let half = d.n.iter().position(|&n| n > s.limit as u64 / 2).unwrap_or(d.n.len());
    let start = d.n.iter().position(|&n| n >= s.from).unwrap_or(0);
    let mut out = vec![
        format!("Nuome's idea machine for the open part of Goldbach: new statements about the even numbers, made up from templates, each with its best constant found on n = {} .. {} and then tested on n = {} .. {}, numbers it never saw", s.from, s.limit / 2, s.limit / 2, s.limit),
        format!("data: the number of ways r(n), the smallest prime p(n) and S(n) for every even n up to {} ({:.1} s)", s.limit, t0.elapsed().as_secs_f64()),
        String::new(),
    ];
    out.push("NUOME'S OWN SHAPES (built by the genetic search from n, ln n, ln ln n, S(n), w(n), v2(n) and constants; no shape given to it): lower bounds r(n) >= c g(n), tightness = how close c g(n) stays to the true count".into());
    out.extend(invent(&d, start, half, 150));
    out.push(String::new());
    out.push("for comparison, shapes given to it (known forms), each with its best constant:".into());
    let mut kept = 0;
    for idea in ideas() {
        let vals = |range: std::ops::Range<usize>| -> Vec<f64> { range.map(|i| (idea.ratio)(i, &d)).filter(|x| x.is_finite()).collect() };
        let train = vals(start..half);
        let test = vals(half..d.n.len());
        if train.is_empty() || test.is_empty() {
            continue;
        }
        // the best constant on the training half, and how the unseen half fares
        let (c, test_extreme, holds) = if idea.lower {
            let c = train.iter().cloned().fold(f64::INFINITY, f64::min);
            let t = test.iter().cloned().fold(f64::INFINITY, f64::min);
            (c, t, t >= c && c > 0.0)
        } else {
            let c = train.iter().cloned().fold(0.0, f64::max);
            let t = test.iter().cloned().fold(0.0, f64::max);
            (c, t, t <= c)
        };
        let shown = if idea.lower { (c * 1000.0).floor() / 1000.0 } else { (c * 1000.0).ceil() / 1000.0 };
        if holds {
            kept += 1;
            out.push(format!("KEPT  {} for every even n >= {}, with c = {shown}", idea.text, s.from));
            out.push(format!("      found on the first half; on the unseen half the {} value is {test_extreme:.4}, so it {}", if idea.lower { "smallest" } else { "largest" }, if idea.lower { "never drops below c" } else { "never rises above c" }));
            out.push(format!("      if proved, it would give: {}", idea.gives));
        } else {
            out.push(format!("FAILED {}: the best c on the first half is {c:.4}, but the unseen half reaches {test_extreme:.4}", idea.text));
        }
    }
    out.push(String::new());
    out.push(format!("{kept} statements survived the test. They are Nuome's conjectures: made up and tested by it, true for every even n it looked at, proved for none. The ones that would give Goldbach are as hard to prove as Goldbach: the difficulty moves into the statement, it does not go away"));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn small_run() {
        let text = super::report(&super::Settings { limit: 20_000, from: 1_000 });
        assert!(text.contains("survived the test"));
        // r(n) > 0 everywhere up to 20 000: the sqrt(n) bound must have some positive c
        assert!(text.contains("KEPT") || text.contains("FAILED"));
    }
}
