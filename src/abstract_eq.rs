//! Abstract equations for a graph's lines. The points are the elements of a
//! product of finite rings (Z_n, or finite fields such as GF(9)); a formula
//! is logic (and, or, not) over atoms, and an atom says something about a
//! term in one coordinate's own arithmetic: the term is 0, a nonzero square,
//! or a non-square. p and q are joined when the formula holds for (p, q) or
//! for (q, p).
//!
//! Examples: the 9-point graph is "p - q is a square" over GF(9) (the Paley
//! graph); a product construction is "(dx is a square and dy = 0) or (dx = 0
//! and dy is a square)".
//!
//! The search is the Goldbach formula's: families with their best choices,
//! then formula trees evolved by the same genetic search (seeded, tournament
//! of three, crossover, new subtrees, nudged constants, a price per node),
//! then the printed formula rechecked pair by pair. The chain lifts the
//! small graph's formula into a bigger product and continues there.

use crate::conway::{violations_with, Set};
use crate::evolve::Rng;

/// A finite commutative ring given by its tables.
#[derive(Clone)]
pub struct Ring {
    pub name: String,
    size: usize,
    add: Vec<u8>,
    mul: Vec<u8>,
    neg: Vec<u8>,
    /// nonzero squares
    square: Vec<bool>,
}

impl Ring {
    fn from(name: String, size: usize, add: impl Fn(usize, usize) -> usize, mul: impl Fn(usize, usize) -> usize) -> Ring {
        let add_t: Vec<u8> = (0..size * size).map(|i| add(i / size, i % size) as u8).collect();
        let mul_t: Vec<u8> = (0..size * size).map(|i| mul(i / size, i % size) as u8).collect();
        let neg = (0..size).map(|x| (0..size).find(|&y| add_t[x * size + y] == 0).expect("a ring has negatives") as u8).collect();
        let mut square = vec![false; size];
        for x in 1..size {
            let s = mul_t[x * size + x] as usize;
            if s != 0 {
                square[s] = true;
            }
        }
        Ring { name, size, add: add_t, mul: mul_t, neg, square }
    }
    /// The integers mod n.
    pub fn z(n: usize) -> Ring {
        Ring::from(if [2, 3, 5, 7, 11, 13].contains(&n) { format!("GF({n})") } else { format!("Z{n}") }, n, |a, b| (a + b) % n, |a, b| (a * b) % n)
    }
    /// GF(9) = GF(3)[i] with i^2 = -1; the element a + b i is 3a + b.
    pub fn gf9() -> Ring {
        let split = |x: usize| (x / 3, x % 3);
        Ring::from(
            "GF(9)".into(),
            9,
            |x, y| {
                let ((a, b), (c, d)) = (split(x), split(y));
                3 * ((a + c) % 3) + (b + d) % 3
            },
            |x, y| {
                let ((a, b), (c, d)) = (split(x), split(y));
                // (a + b i)(c + d i) = (ac - bd) + (ad + bc) i
                3 * ((a * c + 2 * b * d) % 3) + (a * d + b * c) % 3
            },
        )
    }
    fn add(&self, x: u8, y: u8) -> u8 {
        self.add[x as usize * self.size + y as usize]
    }
    fn mul(&self, x: u8, y: u8) -> u8 {
        self.mul[x as usize * self.size + y as usize]
    }
}

/// A term in one coordinate: built from p's and q's value there and constants.
#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    P,
    Q,
    Num(u8),
    Add(Box<Term>, Box<Term>),
    Sub(Box<Term>, Box<Term>),
    Mul(Box<Term>, Box<Term>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Zero,
    Square,
    NonSquare,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Formula {
    Atom(usize, Term, Kind),
    And(Box<Formula>, Box<Formula>),
    Or(Box<Formula>, Box<Formula>),
    Not(Box<Formula>),
}

fn bx<T>(t: T) -> Box<T> {
    Box::new(t)
}

impl Term {
    fn eval(&self, r: &Ring, p: u8, q: u8) -> u8 {
        match self {
            Term::P => p,
            Term::Q => q,
            Term::Num(c) => *c % r.size as u8,
            Term::Add(a, b) => r.add(a.eval(r, p, q), b.eval(r, p, q)),
            Term::Sub(a, b) => r.add(a.eval(r, p, q), r.neg[b.eval(r, p, q) as usize]),
            Term::Mul(a, b) => r.mul(a.eval(r, p, q), b.eval(r, p, q)),
        }
    }
    fn size(&self) -> usize {
        match self {
            Term::P | Term::Q | Term::Num(_) => 1,
            Term::Add(a, b) | Term::Sub(a, b) | Term::Mul(a, b) => 1 + a.size() + b.size(),
        }
    }
    fn show(&self, c: usize) -> String {
        match self {
            Term::P => format!("p{c}"),
            Term::Q => format!("q{c}"),
            Term::Num(n) => n.to_string(),
            Term::Add(a, b) => format!("({} + {})", a.show(c), b.show(c)),
            Term::Sub(a, b) => format!("({} - {})", a.show(c), b.show(c)),
            Term::Mul(a, b) => format!("{} * {}", a.show(c), b.show(c)),
        }
    }
}

fn diff() -> Term {
    Term::Sub(bx(Term::P), bx(Term::Q))
}

impl Formula {
    fn holds(&self, rings: &[Ring], p: &[u8], q: &[u8]) -> bool {
        match self {
            Formula::Atom(c, t, k) => {
                let v = t.eval(&rings[*c], p[*c], q[*c]);
                match k {
                    Kind::Zero => v == 0,
                    Kind::Square => rings[*c].square[v as usize],
                    Kind::NonSquare => v != 0 && !rings[*c].square[v as usize],
                }
            }
            Formula::And(a, b) => a.holds(rings, p, q) && b.holds(rings, p, q),
            Formula::Or(a, b) => a.holds(rings, p, q) || b.holds(rings, p, q),
            Formula::Not(a) => !a.holds(rings, p, q),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Formula::Atom(_, t, _) => 1 + t.size(),
            Formula::And(a, b) | Formula::Or(a, b) => 1 + a.size() + b.size(),
            Formula::Not(a) => 1 + a.size(),
        }
    }
    pub fn show(&self, rings: &[Ring]) -> String {
        match self {
            Formula::Atom(c, t, k) => {
                let what = match k {
                    Kind::Zero => "= 0",
                    Kind::Square => "is a square",
                    Kind::NonSquare => "is a non-square",
                };
                format!("[{} {what} in {}]", t.show(*c + 1), rings[*c].name)
            }
            Formula::And(a, b) => format!("({} and {})", a.show(rings), b.show(rings)),
            Formula::Or(a, b) => format!("({} or {})", a.show(rings), b.show(rings)),
            Formula::Not(a) => format!("not {}", a.show(rings)),
        }
    }
    fn nodes(&self) -> usize {
        match self {
            Formula::Atom(..) => 1,
            Formula::And(a, b) | Formula::Or(a, b) => 1 + a.nodes() + b.nodes(),
            Formula::Not(a) => 1 + a.nodes(),
        }
    }
    fn at(&mut self, k: usize) -> &mut Formula {
        if k == 0 {
            return self;
        }
        match self {
            Formula::And(a, b) | Formula::Or(a, b) => {
                let s = a.nodes();
                if k <= s {
                    a.at(k - 1)
                } else {
                    b.at(k - 1 - s)
                }
            }
            Formula::Not(a) => a.at(k - 1),
            _ => self,
        }
    }
    fn get(&self, k: usize) -> &Formula {
        if k == 0 {
            return self;
        }
        match self {
            Formula::And(a, b) | Formula::Or(a, b) => {
                let s = a.nodes();
                if k <= s {
                    a.get(k - 1)
                } else {
                    b.get(k - 1 - s)
                }
            }
            Formula::Not(a) => a.get(k - 1),
            _ => self,
        }
    }
}

/// The points and the counts asked for.
#[derive(Clone)]
pub struct Space {
    pub rings: Vec<Ring>,
    pub k: u32,
    pub lambda: u32,
    pub mu: u32,
}

impl Space {
    pub fn n(&self) -> usize {
        self.rings.iter().map(|r| r.size).product()
    }
    pub fn name(&self) -> String {
        self.rings.iter().map(|r| r.name.clone()).collect::<Vec<_>>().join(" x ")
    }
    fn coords(&self, p: usize) -> Vec<u8> {
        let mut rest = p;
        let mut c = vec![0; self.rings.len()];
        for (i, r) in self.rings.iter().enumerate().rev() {
            c[i] = (rest % r.size) as u8;
            rest /= r.size;
        }
        c
    }
    pub fn build(&self, f: &Formula) -> Vec<Set> {
        let n = self.n();
        let coords: Vec<Vec<u8>> = (0..n).map(|p| self.coords(p)).collect();
        let mut g = vec![0 as Set; n];
        for p in 0..n {
            for q in p + 1..n {
                if f.holds(&self.rings, &coords[p], &coords[q]) || f.holds(&self.rings, &coords[q], &coords[p]) {
                    g[p] |= 1 << q;
                    g[q] |= 1 << p;
                }
            }
        }
        g
    }
    pub fn errors(&self, f: &Formula) -> (u32, u32) {
        let g = self.build(f);
        let degree = g.iter().map(|x| (x.count_ones() as i32 - self.k as i32).unsigned_abs()).sum();
        (violations_with(&g, self.lambda, self.mu), degree)
    }
    pub fn fitness(&self, f: &Formula, price: f64) -> f64 {
        let (common, degree) = self.errors(f);
        common as f64 + 10.0 * degree as f64 + price * f.size() as f64
    }
}

fn atom(c: usize, k: Kind) -> Formula {
    Formula::Atom(c, diff(), k)
}

/// The families: Paley-type ("the difference is a square"), rook-type, and
/// the product constructions, over every coordinate or pair of them.
pub fn families(space: &Space) -> Vec<(String, Formula)> {
    let d = space.rings.len();
    let mut out = Vec::new();
    for c in 0..d {
        out.push((format!("difference in coordinate {} is a square (Paley)", c + 1), atom(c, Kind::Square)));
        out.push((format!("difference in coordinate {} is a non-square", c + 1), atom(c, Kind::NonSquare)));
    }
    for a in 0..d {
        for b in a + 1..d {
            let (sa, sb, za, zb) = (atom(a, Kind::Square), atom(b, Kind::Square), atom(a, Kind::Zero), atom(b, Kind::Zero));
            out.push((format!("same {} or same {} (rook)", a + 1, b + 1), Formula::Or(bx(Formula::And(bx(za.clone()), bx(Formula::Not(bx(zb.clone()))))), bx(Formula::And(bx(zb.clone()), bx(Formula::Not(bx(za.clone()))))))));
            out.push((format!("product: square in {} and 0 in {}, or 0 in {} and square in {}", a + 1, b + 1, a + 1, b + 1), Formula::Or(bx(Formula::And(bx(sa.clone()), bx(zb.clone()))), bx(Formula::And(bx(za.clone()), bx(sb.clone()))))));
            out.push((format!("both squares or both non-squares in {} and {}", a + 1, b + 1), Formula::Or(bx(Formula::And(bx(sa.clone()), bx(sb.clone()))), bx(Formula::And(bx(atom(a, Kind::NonSquare)), bx(atom(b, Kind::NonSquare)))))));
            out.push((format!("square in {} and 0 in {}, or square in both", a + 1, b + 1), Formula::Or(bx(Formula::And(bx(sa.clone()), bx(zb))), bx(Formula::And(bx(sa), bx(sb))))));
        }
    }
    out
}

fn random_term(r: &mut Rng, depth: usize, size: usize) -> Term {
    if depth == 0 || r.below(3) == 0 {
        return match r.below(4) {
            0 => Term::Num(r.below(size) as u8),
            1 => Term::P,
            2 => Term::Q,
            _ => diff(),
        };
    }
    match r.below(3) {
        0 => Term::Add(bx(random_term(r, depth - 1, size)), bx(random_term(r, depth - 1, size))),
        1 => Term::Sub(bx(random_term(r, depth - 1, size)), bx(random_term(r, depth - 1, size))),
        _ => Term::Mul(bx(random_term(r, depth - 1, size)), bx(random_term(r, depth - 1, size))),
    }
}

fn random_atom(r: &mut Rng, space: &Space) -> Formula {
    let c = r.below(space.rings.len());
    let kind = [Kind::Zero, Kind::Square, Kind::NonSquare][r.below(3)];
    Formula::Atom(c, random_term(r, 2, space.rings[c].size), kind)
}

fn random_formula(r: &mut Rng, space: &Space, depth: usize) -> Formula {
    if depth == 0 || r.below(3) == 0 {
        return random_atom(r, space);
    }
    match r.below(5) {
        0 | 1 => Formula::And(bx(random_formula(r, space, depth - 1)), bx(random_formula(r, space, depth - 1))),
        2 | 3 => Formula::Or(bx(random_formula(r, space, depth - 1)), bx(random_formula(r, space, depth - 1))),
        _ => Formula::Not(bx(random_formula(r, space, depth - 1))),
    }
}

/// Mutate an atom in place: a new kind, a new coordinate, or a new term.
fn nudge_atom(f: &mut Formula, r: &mut Rng, space: &Space) {
    if let Formula::Atom(c, t, k) = f {
        match r.below(3) {
            0 => *k = [Kind::Zero, Kind::Square, Kind::NonSquare][r.below(3)],
            1 => *c = r.below(space.rings.len()),
            _ => *t = random_term(r, 2, space.rings[*c].size),
        }
    }
}

pub struct Settings {
    pub population: usize,
    pub generations: usize,
    pub seed: u64,
    pub price_per_node: f64,
}

pub struct Evolved {
    pub best: Formula,
    pub fitness: f64,
    pub history: Vec<f64>,
}

fn scores(space: &Space, pop: &[Formula], price: f64) -> Vec<f64> {
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let size = pop.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = pop.chunks(size).map(|part| sc.spawn(move || part.iter().map(|f| space.fitness(f, price)).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
    })
}

/// Evolve formulas, starting from the seeds.
pub fn evolve(space: &Space, seeds: &[Formula], s: &Settings) -> Evolved {
    let mut r = Rng(s.seed.max(1));
    let mut pop: Vec<Formula> = seeds.iter().take(s.population).cloned().collect();
    while pop.len() < s.population {
        pop.push(random_formula(&mut r, space, 3));
    }
    let mut history = Vec::new();
    let mut scored: Vec<(f64, Formula)> = Vec::new();
    for generation in 0..s.generations {
        let sc = scores(space, &pop, s.price_per_node);
        scored = sc.into_iter().zip(pop.drain(..)).collect();
        scored.sort_by(|a, b| a.0.total_cmp(&b.0));
        if generation % (s.generations / 10).max(1) == 0 {
            history.push(scored[0].0);
        }
        if generation + 1 == s.generations || scored[0].0 < 1e-9 + s.price_per_node * scored[0].1.size() as f64 {
            break;
        }
        let elite = (s.population / 10).max(1);
        let mut next: Vec<Formula> = scored.iter().take(elite).map(|(_, f)| f.clone()).collect();
        let pick = |r: &mut Rng| -> Formula {
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
                    let donor = pick(&mut r);
                    let piece = donor.get(r.below(donor.nodes())).clone();
                    let k = r.below(child.nodes());
                    *child.at(k) = piece;
                }
                1 => {
                    let k = r.below(child.nodes());
                    *child.at(k) = random_formula(&mut r, space, 2);
                }
                _ => {
                    // nudge an atom: a new kind, coordinate or term
                    let atoms: Vec<usize> = (0..child.nodes()).filter(|&k| matches!(child.get(k), Formula::Atom(..))).collect();
                    let k = atoms[r.below(atoms.len())];
                    nudge_atom(child.at(k), &mut r, space);
                }
            }
            if child.size() <= 40 {
                next.push(child);
            }
        }
        pop = next;
    }
    let (fitness, best) = scored.into_iter().next().expect("a population");
    history.push(fitness);
    Evolved { best, fitness, history }
}

/// A formula of a smaller space moved into a bigger one whose first
/// coordinates are the same rings, in evolved forms: as it is, and joined by
/// and / or with each family atom on every new coordinate.
pub fn lift(f: &Formula, from: usize, to: &Space) -> Vec<Formula> {
    let mut out = vec![f.clone()];
    for c in from..to.rings.len() {
        for kind in [Kind::Zero, Kind::Square, Kind::NonSquare] {
            let a = atom(c, kind);
            out.push(Formula::And(bx(f.clone()), bx(a.clone())));
            out.push(Formula::Or(bx(f.clone()), bx(a.clone())));
            out.push(Formula::Or(bx(Formula::And(bx(f.clone()), bx(atom(c, Kind::Zero)))), bx(Formula::And(bx(a.clone()), bx(Formula::Not(bx(f.clone())))))));
            out.push(Formula::Or(bx(Formula::And(bx(f.clone()), bx(a.clone()))), bx(Formula::And(bx(Formula::Not(bx(f.clone()))), bx(Formula::Not(bx(a)))))));
        }
    }
    out
}

fn result_lines(space: &Space, e: &Evolved, s: &Settings, seeded: &str) -> Vec<String> {
    let steps: Vec<String> = e.history.iter().map(|h| format!("{h:.0}")).collect();
    let (common, degree) = space.errors(&e.best);
    vec![
        format!("  evolved ({} formulas, {} generations, seed {}, {seeded}): fitness {}", s.population, s.generations, s.seed, steps.join(" -> ")),
        format!(
            "  best formula: join p and q when {}; rechecked as printed, pair by pair: {common} common-neighbour errors, {degree} neighbour errors{}",
            e.best.show(&space.rings),
            if common == 0 && degree == 0 { ": IT IS THE GRAPH" } else { ": not the graph" }
        ),
    ]
}

/// Families, then evolution, on one space.
pub fn search(space: &Space, s: &Settings) -> (Vec<String>, Evolved) {
    let mut out = Vec::new();
    let fams = families(space);
    let formulas: Vec<Formula> = fams.iter().map(|x| x.1.clone()).collect();
    let sc = scores(space, &formulas, s.price_per_node);
    let mut ranked: Vec<(f64, &(String, Formula))> = sc.into_iter().zip(fams.iter()).collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.push(format!("{} ({} points): {} families; best three:", space.name(), space.n(), fams.len()));
    for (_, (name, f)) in ranked.iter().take(3) {
        let (c, d) = space.errors(f);
        out.push(format!("    {name}: {c} common-neighbour errors, {d} neighbour errors"));
    }
    let e = evolve(space, &formulas, s);
    out.extend(result_lines(space, &e, s, "seeded with the families"));
    (out, e)
}

/// The chain: the small space's best formula, lifted into the big space,
/// seeds the big search beside the big space's own families.
pub fn chain(small: &Space, big: &Space, s: &Settings) -> Vec<String> {
    let (mut out, first) = search(small, s);
    let lifted = lift(&first.best, small.rings.len(), big);
    let mut seeds = lifted.clone();
    seeds.extend(families(big).into_iter().map(|x| x.1));
    let sc = scores(big, &lifted, s.price_per_node);
    let best_lift = sc.iter().enumerate().fold((0, f64::INFINITY), |b, (i, &v)| if v < b.1 { (i, v) } else { b }).0;
    let (lc, ld) = big.errors(&lifted[best_lift]);
    out.push(format!(
        "lifted into {} ({} points): {} evolved forms of the small formula (with and / or / exclusive combinations on the new coordinate); best: {} ({lc} common-neighbour errors, {ld} neighbour errors)",
        big.name(), big.n(), lifted.len(), lifted[best_lift].show(&big.rings)
    ));
    let e = evolve(big, &seeds, s);
    out.extend(result_lines(big, &e, s, "seeded with the lifted forms and the families"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf9_is_a_field_and_paley_gives_the_nine_point_graph() {
        let f = Ring::gf9();
        // every nonzero element has an inverse, and there are 4 nonzero squares
        assert!((1..9u8).all(|x| (1..9u8).any(|y| f.mul(x, y) == 1)));
        assert_eq!(f.square.iter().filter(|&&s| s).count(), 4);
        let space = Space { rings: vec![Ring::gf9()], k: 4, lambda: 1, mu: 2 };
        assert_eq!(space.errors(&atom(0, Kind::Square)), (0, 0));
        // and the rook form on GF(3) x GF(3)
        let grid = Space { rings: vec![Ring::z(3), Ring::z(3)], k: 4, lambda: 1, mu: 2 };
        let (_, e) = search(&grid, &Settings { population: 20, generations: 5, seed: 1, price_per_node: 0.01 });
        assert_eq!(grid.errors(&e.best), (0, 0));
    }
}
