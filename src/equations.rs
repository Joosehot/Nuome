//! Equations for a graph's lines, found the way the Goldbach formula was
//! (src/discover.rs, src/evolve.rs):
//!
//! 1. families: fixed shapes of equation, each with its best constants;
//! 2. evolution: equation trees by the same genetic search (the population
//!    seeded with the fitted families, parents by a tournament of three,
//!    crossover, a new subtree, or a nudged constant; a price per node);
//! 3. the printed equation rechecked, pair by pair.
//!
//! The points sit on a grid of `rows` x `cols`; point p is (x_p, y_p). An
//! equation is a term t(p, q) and a modulus M: p and q are joined when
//! t(p, q) = 0 or t(q, p) = 0 (mod M), which makes the rule symmetric.
//! Arithmetic is in Z_M, so a product is 0 when one factor is: a product
//! says "or" ((x_p - x_q)(y_p - y_q) = 0 is "same row or same column").

use crate::conway::{violations_with, Set};
use crate::evolve::Rng;

#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    /// 0 x_p, 1 y_p, 2 x_q, 3 y_q, 4 p, 5 q
    Var(u8),
    Num(i64),
    Add(Box<Term>, Box<Term>),
    Sub(Box<Term>, Box<Term>),
    Mul(Box<Term>, Box<Term>),
}

use Term::*;

fn b(t: Term) -> Box<Term> {
    Box::new(t)
}

impl Term {
    fn eval(&self, v: &[i64; 6], m: i64) -> i64 {
        match self {
            Var(i) => v[*i as usize].rem_euclid(m),
            Num(c) => c.rem_euclid(m),
            Add(a, c) => (a.eval(v, m) + c.eval(v, m)) % m,
            Sub(a, c) => (a.eval(v, m) - c.eval(v, m)).rem_euclid(m),
            Mul(a, c) => (a.eval(v, m) * c.eval(v, m)) % m,
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Var(_) | Num(_) => 1,
            Add(a, c) | Sub(a, c) | Mul(a, c) => 1 + a.size() + c.size(),
        }
    }
    pub fn show(&self) -> String {
        match self {
            Var(i) => ["x_p", "y_p", "x_q", "y_q", "p", "q"][*i as usize].to_string(),
            Num(c) => c.to_string(),
            Add(a, c) => format!("({} + {})", a.show(), c.show()),
            Sub(a, c) => format!("({} - {})", a.show(), c.show()),
            Mul(a, c) => format!("{} * {}", a.show(), c.show()),
        }
    }
    fn at(&mut self, k: usize) -> &mut Term {
        if k == 0 {
            return self;
        }
        match self {
            Add(a, c) | Sub(a, c) | Mul(a, c) => {
                let s = a.size();
                if k <= s {
                    a.at(k - 1)
                } else {
                    c.at(k - 1 - s)
                }
            }
            _ => self,
        }
    }
    fn get(&self, k: usize) -> &Term {
        if k == 0 {
            return self;
        }
        match self {
            Add(a, c) | Sub(a, c) | Mul(a, c) => {
                let s = a.size();
                if k <= s {
                    a.get(k - 1)
                } else {
                    c.get(k - 1 - s)
                }
            }
            _ => self,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Equation {
    pub term: Term,
    pub modulus: i64,
}

impl Equation {
    pub fn show(&self) -> String {
        format!("{} = 0 (mod {})", self.term.show(), self.modulus)
    }
}

/// The problem: a grid of points and the counts asked for.
#[derive(Clone, Copy)]
pub struct Target {
    pub rows: usize,
    pub cols: usize,
    pub k: u32,
    pub lambda: u32,
    pub mu: u32,
}

impl Target {
    fn n(&self) -> usize {
        self.rows * self.cols
    }
    fn vars(&self, p: usize, q: usize) -> [i64; 6] {
        let (c, pi, qi) = (self.cols, p as i64, q as i64);
        [(p / c) as i64, (p % c) as i64, (q / c) as i64, (q % c) as i64, pi, qi]
    }
    /// The graph the equation draws.
    pub fn build(&self, e: &Equation) -> Vec<Set> {
        let n = self.n();
        let mut g = vec![0 as Set; n];
        for p in 0..n {
            for q in p + 1..n {
                let joined = e.term.eval(&self.vars(p, q), e.modulus) == 0 || e.term.eval(&self.vars(q, p), e.modulus) == 0;
                if joined {
                    g[p] |= 1 << q;
                    g[q] |= 1 << p;
                }
            }
        }
        g
    }
    /// (common-neighbour counts off, neighbours off) of the drawn graph.
    pub fn errors(&self, e: &Equation) -> (u32, u32) {
        let g = self.build(e);
        let degree = g.iter().map(|x| (x.count_ones() as i32 - self.k as i32).unsigned_abs()).sum();
        (violations_with(&g, self.lambda, self.mu), degree)
    }
    /// The fitness: both kinds of error (a wrong neighbour count weighs 10)
    /// plus a price per node of the equation.
    pub fn fitness(&self, e: &Equation, price: f64) -> f64 {
        let (common, degree) = self.errors(e);
        common as f64 + 10.0 * degree as f64 + price * e.term.size() as f64
    }
}

/// The fixed families, each a shape with one constant c (0..modulus).
pub fn families() -> Vec<(&'static str, fn(i64) -> Term)> {
    fn d(i: u8, j: u8) -> Term {
        Sub(b(Var(i)), b(Var(j)))
    }
    vec![
        ("same row or same column: (x_p - x_q)(y_p - y_q)", |_| Mul(b(d(0, 2)), b(d(1, 3)))),
        ("difference of labels: p - q - c", |c| Sub(b(d(4, 5)), b(Num(c)))),
        ("square of the difference: (p - q)^2 - c", |c| Sub(b(Mul(b(d(4, 5)), b(d(4, 5)))), b(Num(c)))),
        ("row difference times column difference minus c: (x_p - x_q)(y_p - y_q) - c", |c| Sub(b(Mul(b(d(0, 2)), b(d(1, 3)))), b(Num(c)))),
        ("distance: (x_p - x_q)^2 + (y_p - y_q)^2 - c", |c| Sub(b(Add(b(Mul(b(d(0, 2)), b(d(0, 2)))), b(Mul(b(d(1, 3)), b(d(1, 3)))))), b(Num(c)))),
        ("dot product: x_p x_q + y_p y_q - c", |c| Sub(b(Add(b(Mul(b(Var(0)), b(Var(2)))), b(Mul(b(Var(1)), b(Var(3)))))), b(Num(c)))),
        ("line: y_q - y_p - c (x_q - x_p)", |c| Sub(b(d(3, 1)), b(Mul(b(Num(c)), b(d(2, 0)))))),
    ]
}

/// Each family with its best constant and modulus (every modulus 2..=n and
/// every constant below it tried), best first.
pub fn fit_families(t: &Target, price: f64) -> Vec<(&'static str, Equation, f64)> {
    let n = t.n() as i64;
    let mut out: Vec<(&'static str, Equation, f64)> = families()
        .into_iter()
        .map(|(name, shape)| {
            let candidates: Vec<Equation> = (2..=n).flat_map(|m| (0..m).map(move |c| Equation { term: shape(c), modulus: m })).collect();
            let best = parallel_best(&candidates, |e| t.fitness(e, price));
            (name, candidates[best.0].clone(), best.1)
        })
        .collect();
    out.sort_by(|a, c| a.2.total_cmp(&c.2));
    out
}

/// The index and value of the smallest score, computed on every thread.
fn parallel_best<T: Sync>(items: &[T], score: impl Fn(&T) -> f64 + Sync) -> (usize, f64) {
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let size = items.len().div_ceil(threads).max(1);
    let scores: Vec<f64> = std::thread::scope(|sc| {
        let hs: Vec<_> = items.chunks(size).map(|part| {
            let score = &score;
            sc.spawn(move || part.iter().map(score).collect::<Vec<_>>())
        }).collect();
        hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
    });
    scores.iter().enumerate().fold((0, f64::INFINITY), |best, (i, &s)| if s < best.1 { (i, s) } else { best })
}

fn random_term(r: &mut Rng, depth: usize, m: i64) -> Term {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(4) == 0 { Num(r.below(m as usize) as i64) } else { Var(r.below(6) as u8) };
    }
    match r.below(3) {
        0 => Add(b(random_term(r, depth - 1, m)), b(random_term(r, depth - 1, m))),
        1 => Sub(b(random_term(r, depth - 1, m)), b(random_term(r, depth - 1, m))),
        _ => Mul(b(random_term(r, depth - 1, m)), b(random_term(r, depth - 1, m))),
    }
}

pub struct Settings {
    pub population: usize,
    pub generations: usize,
    pub seed: u64,
    pub price_per_node: f64,
}

pub struct Evolved {
    pub best: Equation,
    pub fitness: f64,
    /// the best fitness after each tenth of the generations
    pub history: Vec<f64>,
}

/// Evolve equations; the population starts with the fitted families.
pub fn evolve(t: &Target, seeds: &[Equation], s: &Settings) -> Evolved {
    let n = t.n();
    let mut r = Rng(s.seed.max(1));
    let mut pop: Vec<Equation> = seeds.to_vec();
    while pop.len() < s.population {
        let m = 2 + r.below(n - 1) as i64;
        pop.push(Equation { term: random_term(&mut r, 3, m), modulus: m });
    }
    let mut history = Vec::new();
    let mut scored: Vec<(f64, Equation)> = Vec::new();
    for generation in 0..s.generations {
        let scores = {
            let threads = std::thread::available_parallelism().map_or(4, |x| x.get());
            let size = pop.len().div_ceil(threads).max(1);
            std::thread::scope(|sc| {
                let hs: Vec<_> = pop.chunks(size).map(|part| sc.spawn(move || part.iter().map(|e| t.fitness(e, s.price_per_node)).collect::<Vec<_>>())).collect();
                hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect::<Vec<f64>>()
            })
        };
        scored = scores.into_iter().zip(pop.drain(..)).collect();
        scored.sort_by(|a, c| a.0.total_cmp(&c.0));
        if generation % (s.generations / 10).max(1) == 0 {
            history.push(scored[0].0);
        }
        if generation + 1 == s.generations {
            break;
        }
        let elite = s.population / 10;
        let mut next: Vec<Equation> = scored.iter().take(elite.max(1)).map(|(_, e)| e.clone()).collect();
        let pick = |r: &mut Rng| -> Equation {
            // tournament of three
            let mut best = r.below(scored.len());
            for _ in 0..2 {
                let c = r.below(scored.len());
                if scored[c].0 < scored[best].0 {
                    best = c;
                }
            }
            scored[best].1.clone()
        };
        while next.len() < s.population {
            let mut child = pick(&mut r);
            match r.below(4) {
                0 => {
                    // crossover: a subtree of another parent replaces one of ours
                    let donor = pick(&mut r);
                    let piece = donor.term.get(r.below(donor.term.size())).clone();
                    let k = r.below(child.term.size());
                    *child.term.at(k) = piece;
                }
                1 => {
                    let k = r.below(child.term.size());
                    *child.term.at(k) = random_term(&mut r, 2, child.modulus);
                }
                _ => {
                    // nudge a constant, or the modulus
                    let k = r.below(child.term.size() + 1);
                    if k == child.term.size() {
                        child.modulus = (child.modulus + if r.below(2) == 0 { 1 } else { -1 }).clamp(2, n as i64);
                    } else if let Num(c) = child.term.at(k) {
                        *c += if r.below(2) == 0 { 1 } else { -1 };
                    }
                }
            }
            if child.term.size() <= 15 {
                next.push(child);
            }
        }
        pop = next;
    }
    let (fitness, best) = scored.into_iter().next().expect("a population");
    history.push(fitness);
    Evolved { best, fitness, history }
}

/// Report lines: families, evolution, and the printed equation rechecked.
pub fn report(t: &Target, s: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    let fitted = fit_families(t, s.price_per_node);
    out.push(format!("equation families on a {} x {} grid, each with its best constant and modulus (fitness = common-neighbour counts off + 10 per neighbour off + {} per node; 0 errors = the graph):", t.rows, t.cols, s.price_per_node));
    for (name, e, f) in &fitted {
        let (common, degree) = t.errors(e);
        out.push(format!("    {name}: best {} ({common} common-neighbour errors, {degree} neighbour errors, fitness {f:.1})", e.show()));
    }
    let seeds: Vec<Equation> = fitted.iter().map(|x| x.1.clone()).collect();
    let e = evolve(t, &seeds, s);
    let steps: Vec<String> = e.history.iter().map(|h| format!("{h:.0}")).collect();
    out.push(format!(
        "  evolved like the Goldbach formula ({} equations, {} generations, seed {}, seeded with the families, tournament of three, crossover, new subtrees, nudged constants): fitness {}",
        s.population, s.generations, s.seed, steps.join(" -> ")
    ));
    // recheck the equation exactly as printed
    let (common, degree) = t.errors(&e.best);
    out.push(format!("  best equation: join p and q when {} (either order); rechecked as printed, pair by pair: {common} common-neighbour errors, {degree} neighbour errors{}", e.best.show(), if common == 0 && degree == 0 { ": IT IS THE GRAPH" } else { ": not the graph" }));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_point_graph_has_an_equation() {
        let t = Target { rows: 3, cols: 3, k: 4, lambda: 1, mu: 2 };
        let fitted = fit_families(&t, 0.01);
        let e = &fitted[0].1;
        assert_eq!(t.errors(e), (0, 0), "{}", e.show()); // same row or same column
        let s = Settings { population: 30, generations: 10, seed: 3, price_per_node: 0.01 };
        let a = evolve(&t, &[e.clone()], &s);
        let b = evolve(&t, &[e.clone()], &s);
        assert_eq!(a.best, b.best); // seeded
        assert_eq!(t.errors(&a.best), (0, 0));
    }
}
