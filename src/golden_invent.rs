//! Golden invents a function for a graph of Conway's kind (λ = 1, μ = 2),
//! from nothing: no known graph, no family, no example formula.
//!
//! The only material is what the conditions force (golden_graph::forced):
//! a vertex x, its k neighbours (i, s) — triangle i ∈ Z_q (q = k/2), corner
//! s ∈ {0, 1} — and the far vertices, the pairs {(i, s), (j, t)} with i ≠ j.
//! What is invented is the rule for which far vertices are adjacent:
//!
//!   f ~ g  when  E(f, g) + E(g, f) ≡ 0 (mod q)   (symmetric by construction),
//!
//! E a formula in i1 s1 j1 t1 i2 s2 j2 t2 built from +, -, ×, and constants
//! mod q. Formulas start random and evolve; a formula's fitness is how many
//! of the graph's conditions (degree k at every vertex; 1 common neighbour
//! for every adjacent pair, 2 for every other) its graph meets. A formula
//! whose graph meets all of them is checked again from scratch on every pair.

#[derive(Clone, Debug)]
pub enum Ex {
    Var(u8),
    Const(u64),
    Add(Box<Ex>, Box<Ex>),
    Sub(Box<Ex>, Box<Ex>),
    Mul(Box<Ex>, Box<Ex>),
}

const NAMES: [&str; 8] = ["i1", "s1", "j1", "t1", "i2", "s2", "j2", "t2"];

impl Ex {
    fn eval(&self, v: &[u64; 8], q: u64) -> u64 {
        match self {
            Ex::Var(i) => v[*i as usize] % q,
            Ex::Const(c) => c % q,
            Ex::Add(a, b) => (a.eval(v, q) + b.eval(v, q)) % q,
            Ex::Sub(a, b) => (a.eval(v, q) + q - b.eval(v, q)) % q,
            Ex::Mul(a, b) => a.eval(v, q) * b.eval(v, q) % q,
        }
    }
    fn size(&self) -> usize {
        match self {
            Ex::Var(_) | Ex::Const(_) => 1,
            Ex::Add(a, b) | Ex::Sub(a, b) | Ex::Mul(a, b) => 1 + a.size() + b.size(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Ex::Var(i) => NAMES[*i as usize].to_string(),
            Ex::Const(c) => c.to_string(),
            Ex::Add(a, b) => format!("({} + {})", a.text(), b.text()),
            Ex::Sub(a, b) => format!("({} - {})", a.text(), b.text()),
            Ex::Mul(a, b) => format!("{}·{}", a.text(), b.text()),
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

fn random_ex(r: &mut Rng, depth: u32, q: u64) -> Ex {
    if depth == 0 || r.below(10) < 3 {
        return if r.below(10) < 8 { Ex::Var(r.below(8) as u8) } else { Ex::Const(r.below(q)) };
    }
    let (a, b) = (Box::new(random_ex(r, depth - 1, q)), Box::new(random_ex(r, depth - 1, q)));
    match r.below(3) {
        0 => Ex::Add(a, b),
        1 => Ex::Sub(a, b),
        _ => Ex::Mul(a, b),
    }
}

/// A random subtree replaced (or a leaf nudged).
fn mutate(e: &Ex, r: &mut Rng, q: u64) -> Ex {
    if r.below(4) == 0 {
        return random_ex(r, 3, q);
    }
    match e {
        Ex::Var(_) => Ex::Var(r.below(8) as u8),
        Ex::Const(_) => Ex::Const(r.below(q)),
        Ex::Add(a, b) | Ex::Sub(a, b) | Ex::Mul(a, b) => {
            let (a, b) = if r.below(2) == 0 { (Box::new(mutate(a, r, q)), b.clone()) } else { (a.clone(), Box::new(mutate(b, r, q))) };
            match r.below(6) {
                0 => Ex::Add(a, b),
                1 => Ex::Sub(a, b),
                2 => Ex::Mul(a, b),
                _ => match e {
                    Ex::Add(..) => Ex::Add(a, b),
                    Ex::Sub(..) => Ex::Sub(a, b),
                    _ => Ex::Mul(a, b),
                },
            }
        }
    }
}

fn subtrees(e: &Ex) -> Vec<Ex> {
    let mut out = vec![e.clone()];
    if let Ex::Add(a, b) | Ex::Sub(a, b) | Ex::Mul(a, b) = e {
        out.extend(subtrees(a));
        out.extend(subtrees(b));
    }
    out
}

fn crossover(a: &Ex, b: &Ex, r: &mut Rng) -> Ex {
    let parts = subtrees(b);
    let piece = parts[r.below(parts.len() as u64) as usize].clone();
    fn put(e: &Ex, piece: &Ex, r: &mut Rng) -> Ex {
        if r.below(3) == 0 {
            return piece.clone();
        }
        match e {
            Ex::Add(x, y) => if r.below(2) == 0 { Ex::Add(Box::new(put(x, piece, r)), y.clone()) } else { Ex::Add(x.clone(), Box::new(put(y, piece, r))) },
            Ex::Sub(x, y) => if r.below(2) == 0 { Ex::Sub(Box::new(put(x, piece, r)), y.clone()) } else { Ex::Sub(x.clone(), Box::new(put(y, piece, r))) },
            Ex::Mul(x, y) => if r.below(2) == 0 { Ex::Mul(Box::new(put(x, piece, r)), y.clone()) } else { Ex::Mul(x.clone(), Box::new(put(y, piece, r))) },
            leaf => leaf.clone(),
        }
    }
    put(a, &piece, r)
}

/// The forced frame for degree k: far vertices as pairs of (triangle, corner).
pub struct Frame {
    pub k: usize,
    pub q: u64,
    /// far vertex: ((i, s), (j, t)), i < j
    pub far: Vec<((u64, u64), (u64, u64))>,
    pub v: usize,
}

impl Frame {
    pub fn new(k: usize) -> Frame {
        let q = (k / 2) as u64;
        let mut far = Vec::new();
        for i in 0..q {
            for j in i + 1..q {
                for s in 0..2 {
                    for t in 0..2 {
                        far.push(((i, s), (j, t)));
                    }
                }
            }
        }
        let v = 1 + k + far.len();
        Frame { k, q, far, v }
    }
    /// The whole graph a formula gives under a naming of the triangles and
    /// corners, as adjacency rows of bits (v <= 512).
    fn graph(&self, e: &Ex, nm: &Naming) -> Vec<[u64; 8]> {
        let q = self.q;
        self.graph_with(nm, |fg, gf| (e.eval(fg, q) + e.eval(gf, q)) % q == 0)
    }
    /// The graph any symmetric rule on the named far vertices gives: `adj(E-inputs for (f,g), for (g,f))`.
    pub fn graph_with(&self, nm: &Naming, adj: impl Fn(&[u64; 8], &[u64; 8]) -> bool) -> Vec<[u64; 8]> {
        let v = self.v;
        let mut r = vec![[0u64; 8]; v];
        let link = |a: usize, b: usize, r: &mut Vec<[u64; 8]>| {
            r[a][b / 64] |= 1 << (b % 64);
            r[b][a / 64] |= 1 << (a % 64);
        };
        let nb = |i: u64, s: u64| 1 + (2 * i + s) as usize;
        for i in 0..self.q {
            link(0, nb(i, 0), &mut r);
            link(0, nb(i, 1), &mut r);
            link(nb(i, 0), nb(i, 1), &mut r);
        }
        let base = 1 + self.k;
        for (f, &((i, s), (j, t))) in self.far.iter().enumerate() {
            link(base + f, nb(i, s), &mut r);
            link(base + f, nb(j, t), &mut r);
        }
        // the far vertices as the formula sees them: named, the smaller triangle name first
        let named: Vec<[u64; 4]> = self
            .far
            .iter()
            .map(|&((i, s), (j, t))| {
                let a = (nm.tri[i as usize], s ^ nm.flip[i as usize]);
                let b = (nm.tri[j as usize], t ^ nm.flip[j as usize]);
                let (a, b) = if a.0 <= b.0 { (a, b) } else { (b, a) };
                [a.0, a.1, b.0, b.1]
            })
            .collect();
        for f in 0..self.far.len() {
            for g in f + 1..self.far.len() {
                let (x, y) = (named[f], named[g]);
                // symmetric by construction: E(f, g) + E(g, f) ≡ 0
                let fg = [x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3]];
                let gf = [y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3]];
                if adj(&fg, &gf) {
                    link(base + f, base + g, &mut r);
                }
            }
        }
        r
    }
    /// (unmet conditions, all conditions): degree at every vertex, the common count at every pair.
    fn score(&self, e: &Ex, nm: &Naming) -> (u64, u64) {
        let q = self.q;
        self.score_with(nm, |fg, gf| (e.eval(fg, q) + e.eval(gf, q)) % q == 0)
    }
    pub fn score_with(&self, nm: &Naming, adj: impl Fn(&[u64; 8], &[u64; 8]) -> bool) -> (u64, u64) {
        let r = self.graph_with(nm, adj);
        let v = self.v;
        let mut bad = 0u64;
        for x in 0..v {
            let d: u32 = r[x].iter().map(|w| w.count_ones()).sum();
            bad += (d as i64 - self.k as i64).unsigned_abs();
        }
        for x in 0..v {
            for y in x + 1..v {
                let c: u32 = (0..8).map(|w| (r[x][w] & r[y][w]).count_ones()).sum();
                let want = if r[x][y / 64] >> (y % 64) & 1 == 1 { 1 } else { 2 };
                bad += (c as i64 - want).unsigned_abs();
            }
        }
        (bad, (v + v * (v - 1) / 2) as u64)
    }
    pub fn adjacency(&self, e: &Ex, nm: &Naming) -> Vec<Vec<bool>> {
        let q = self.q;
        self.adjacency_with(nm, |fg, gf| (e.eval(fg, q) + e.eval(gf, q)) % q == 0)
    }
    pub fn adjacency_with(&self, nm: &Naming, adj: impl Fn(&[u64; 8], &[u64; 8]) -> bool) -> Vec<Vec<bool>> {
        let r = self.graph_with(nm, adj);
        (0..self.v).map(|x| (0..self.v).map(|y| r[x][y / 64] >> (y % 64) & 1 == 1).collect()).collect()
    }
}

/// How the triangles and corners are named for the formula: the forced frame
/// gives no names, so the names are invented together with the formula.
#[derive(Clone, Debug)]
pub struct Naming {
    /// triangle i is called tri[i] (a permutation of Z_q)
    pub tri: Vec<u64>,
    /// the corners of triangle i are swapped when flip[i] = 1
    pub flip: Vec<u64>,
}

impl Naming {
    fn plain(q: u64) -> Naming {
        Naming { tri: (0..q).collect(), flip: vec![0; q as usize] }
    }
    pub fn shuffled(q: u64, seed: u64) -> Naming {
        let mut r = Rng(seed | 1);
        let mut n = Naming::plain(q);
        for _ in 0..q * 2 {
            n = n.mutate(&mut r);
        }
        n
    }
    pub fn mutate_seeded(&self, seed: u64) -> Naming {
        self.mutate(&mut Rng(seed | 1))
    }
    fn mutate(&self, r: &mut Rng) -> Naming {
        let mut n = self.clone();
        let q = n.tri.len() as u64;
        if r.below(2) == 0 {
            let (a, b) = (r.below(q) as usize, r.below(q) as usize);
            n.tri.swap(a, b);
        } else {
            let a = r.below(q) as usize;
            n.flip[a] ^= 1;
        }
        n
    }
}

pub struct Invented {
    pub best: Ex,
    pub naming: Naming,
    pub unmet: u64,
    pub conditions: u64,
    pub pairs_met_share: f64,
    pub verified: bool,
    pub tried: u64,
    pub seconds: f64,
    pub islands: usize,
}

/// Invent for at most `seconds` on every core: each island evolves its own
/// formulas and namings from random ones; the first that meets every
/// condition stops them all.
pub fn invent(k: usize, seconds: f64, seed: u64) -> Option<Invented> {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;
    if k % 2 != 0 || k < 4 {
        return None;
    }
    let fr = Frame::new(k);
    if fr.v > 512 {
        return None;
    }
    let t0 = std::time::Instant::now();
    let islands = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16);
    let done = AtomicBool::new(false);
    let tried = AtomicU64::new(0);
    let best: Mutex<Option<(u64, Ex, Naming)>> = Mutex::new(None);
    std::thread::scope(|sc| {
        for isl in 0..islands {
            let (fr, done, tried, best) = (&fr, &done, &tried, &best);
            sc.spawn(move || {
                let mut r = Rng((seed ^ (isl as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)) | 1);
                let pop_n = 200;
                let fit = |e: &Ex, nm: &Naming| fr.score(e, nm).0 * 64 + e.size() as u64; // shorter is better among equals
                let mut pop: Vec<(u64, Ex, Naming)> = (0..pop_n)
                    .map(|_| {
                        let e = random_ex(&mut r, 4, fr.q);
                        let mut nm = Naming::plain(fr.q);
                        for _ in 0..fr.q * 2 {
                            nm = nm.mutate(&mut r);
                        }
                        (fit(&e, &nm), e, nm)
                    })
                    .collect();
                pop.sort_by_key(|p| p.0);
                let mut my_tried = pop_n as u64;
                while !done.load(Ordering::Relaxed) && t0.elapsed().as_secs_f64() < seconds {
                    let mut next: Vec<(u64, Ex, Naming)> = pop[..8].to_vec();
                    while next.len() < pop_n {
                        let pick = |r: &mut Rng| -> usize {
                            let a = r.below(pop_n as u64) as usize;
                            let b = r.below(pop_n as u64) as usize;
                            a.min(b)
                        };
                        let pa = pick(&mut r);
                        let (e, nm) = match r.below(10) {
                            0 | 1 => {
                                let pb = pick(&mut r);
                                (crossover(&pop[pa].1, &pop[pb].1, &mut r), pop[pa].2.clone())
                            }
                            2 | 3 | 4 => (pop[pa].1.clone(), pop[pa].2.mutate(&mut r)),
                            _ => (mutate(&pop[pa].1, &mut r, fr.q), pop[pa].2.clone()),
                        };
                        if e.size() > 40 {
                            continue;
                        }
                        my_tried += 1;
                        next.push((fit(&e, &nm), e, nm));
                    }
                    next.sort_by_key(|p| p.0);
                    pop = next;
                    let mut b = best.lock().unwrap();
                    if b.as_ref().map_or(true, |x| pop[0].0 < x.0) {
                        *b = Some(pop[0].clone());
                    }
                    // progress every 5 minutes on long runs, from island 0
                    if isl == 0 && seconds > 600.0 {
                        static LAST: AtomicU64 = AtomicU64::new(0);
                        let slot = (t0.elapsed().as_secs_f64() / 300.0) as u64;
                        if slot > LAST.load(Ordering::Relaxed) {
                            LAST.store(slot, Ordering::Relaxed);
                            if let Some((fit, e, _)) = b.as_ref() {
                                eprintln!("[{} min] best error {} (formula size {}), {} tried by island 0: {}", slot * 5, fit / 64, e.size(), my_tried, e.text());
                            }
                        }
                    }
                    if pop[0].0 / 64 == 0 {
                        done.store(true, Ordering::Relaxed);
                    }
                }
                tried.fetch_add(my_tried, Ordering::Relaxed);
            });
        }
    });
    let (_, best, naming) = best.into_inner().unwrap()?;
    let (unmet, conditions) = fr.score(&best, &naming);
    let pairs = fr.v * (fr.v - 1) / 2;
    let adj = fr.adjacency(&best, &naming);
    let met_pairs = {
        let mut m = 0;
        for x in 0..fr.v {
            for y in x + 1..fr.v {
                let c = (0..fr.v).filter(|&z| adj[x][z] && adj[y][z]).count();
                m += (c == if adj[x][y] { 1 } else { 2 }) as usize;
            }
        }
        m
    };
    let verified = unmet == 0 && crate::golden_graph::check(&adj, k as u64, 1, 2).is_ok();
    Some(Invented { best, naming, unmet, conditions, pairs_met_share: met_pairs as f64 / pairs as f64, verified, tried: tried.load(Ordering::Relaxed), seconds: t0.elapsed().as_secs_f64(), islands })
}
