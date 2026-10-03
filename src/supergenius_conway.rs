//! Conway's 99-graph: is there a graph on 99 vertices in which every two
//! adjacent vertices have exactly one common neighbour and every two
//! non-adjacent vertices exactly two? (Conway's $1000 problem; open.)
//! In the usual words: a strongly regular graph srg(99, 14, 1, 2).
//!
//! The supergenius first reasons from the definition alone (degree,
//! triangles, eigenvalues, the family it belongs to). Then it searches:
//! simulated annealing over k-regular graphs, moved by edge switches that
//! keep every degree, scored by how many pairs have the wrong number of
//! common neighbours (squared). Score 0 is the graph, and it is checked
//! from scratch before anything is said. The same search must first find
//! the known graphs of this kind (the 3 x 3 grid srg(9,4,1,2), Petersen
//! srg(10,3,0,1), Clebsch srg(16,5,0,2), the 4 x 4 grid srg(16,6,2,2)).

use crate::evolve::Rng;

type Row = [u64; 2];

/// A graph on up to 128 vertices with its common-neighbour counts.
struct State {
    n: usize,
    lambda: i32,
    mu: i32,
    adj: Vec<Row>,
    common: Vec<Vec<i32>>,
    cost: i64,
}

fn has(r: &Row, i: usize) -> bool {
    r[i >> 6] >> (i & 63) & 1 == 1
}
fn flip(r: &mut Row, i: usize) {
    r[i >> 6] ^= 1 << (i & 63);
}
fn bits(r: Row) -> impl Iterator<Item = usize> {
    (0..2).flat_map(move |w| {
        let mut x = r[w];
        std::iter::from_fn(move || {
            if x == 0 {
                return None;
            }
            let b = x.trailing_zeros() as usize;
            x &= x - 1;
            Some(w * 64 + b)
        })
    })
}

impl State {
    fn pair_cost(&self, i: usize, j: usize) -> i64 {
        let want = if has(&self.adj[i], j) { self.lambda } else { self.mu };
        let d = (self.common[i][j] - want) as i64;
        d * d
    }

    fn new(n: usize, k: usize, lambda: i32, mu: i32, r: &mut Rng) -> State {
        // start from the circulant: i joined to i +- 1..k/2 (and the opposite point if k is odd)
        let mut adj = vec![[0u64; 2]; n];
        for i in 0..n {
            for s in 1..=k / 2 {
                let j = (i + s) % n;
                if !has(&adj[i], j) {
                    flip(&mut adj[i], j);
                    flip(&mut adj[j], i);
                }
            }
            if k % 2 == 1 && i < n / 2 {
                let j = i + n / 2;
                flip(&mut adj[i], j);
                flip(&mut adj[j], i);
            }
        }
        let mut s = State { n, lambda, mu, adj, common: vec![vec![0; n]; n], cost: 0 };
        s.recount();
        // shuffle by random switches
        for _ in 0..n * k * 20 {
            if let Some(sw) = s.random_switch(r) {
                s.apply_switch(sw);
            }
        }
        s.recount();
        s
    }

    fn recount(&mut self) {
        for i in 0..self.n {
            for j in 0..self.n {
                let a = self.adj[i];
                let b = self.adj[j];
                self.common[i][j] = ((a[0] & b[0]).count_ones() + (a[1] & b[1]).count_ones()) as i32;
            }
        }
        self.cost = 0;
        for i in 0..self.n {
            for j in i + 1..self.n {
                self.cost += self.pair_cost(i, j);
            }
        }
    }

    /// Add or remove the edge u-v, keeping counts and cost up to date.
    fn toggle(&mut self, u: usize, v: usize) {
        let s = if has(&self.adj[u], v) { -1 } else { 1 };
        // pairs whose count changes: (u, x) for x ~ v, (v, x) for x ~ u; and (u, v) whose target changes
        let (nu, nv) = (self.adj[u], self.adj[v]);
        let mut touched: Vec<(usize, usize)> = Vec::with_capacity(32);
        touched.push((u, v));
        for x in bits(nv) {
            if x != u {
                touched.push((u, x));
            }
        }
        for x in bits(nu) {
            if x != v {
                touched.push((v, x));
            }
        }
        for &(a, b) in &touched {
            self.cost -= self.pair_cost(a, b);
        }
        flip(&mut self.adj[u], v);
        flip(&mut self.adj[v], u);
        for x in bits(nv) {
            if x != u {
                self.common[u][x] += s;
                self.common[x][u] += s;
            }
        }
        for x in bits(nu) {
            if x != v {
                self.common[v][x] += s;
                self.common[x][v] += s;
            }
        }
        self.common[u][u] += s;
        self.common[v][v] += s;
        for &(a, b) in &touched {
            self.cost += self.pair_cost(a, b);
        }
    }

    /// Edges a-b and c-d become a-c and b-d (every degree stays).
    fn random_switch(&self, r: &mut Rng) -> Option<[usize; 4]> {
        let a = r.below(self.n);
        let nb: Vec<usize> = bits(self.adj[a]).collect();
        let b = nb[r.below(nb.len())];
        let c = r.below(self.n);
        let nc: Vec<usize> = bits(self.adj[c]).collect();
        let d = nc[r.below(nc.len())];
        let distinct = a != c && a != d && b != c && b != d;
        (distinct && !has(&self.adj[a], c) && !has(&self.adj[b], d)).then_some([a, b, c, d])
    }

    fn apply_switch(&mut self, [a, b, c, d]: [usize; 4]) {
        self.toggle(a, b);
        self.toggle(c, d);
        self.toggle(a, c);
        self.toggle(b, d);
    }

    fn undo_switch(&mut self, [a, b, c, d]: [usize; 4]) {
        self.toggle(b, d);
        self.toggle(a, c);
        self.toggle(c, d);
        self.toggle(a, b);
    }
}

/// Is `adj` an srg(n, k, lambda, mu)? Checked from scratch.
pub fn is_srg(adj: &[Vec<bool>], k: usize, lambda: usize, mu: usize) -> bool {
    let n = adj.len();
    (0..n).all(|i| !adj[i][i] && adj[i].iter().filter(|&&e| e).count() == k)
        && (0..n).all(|i| {
            (0..n).filter(|&j| j != i).all(|j| {
                adj[i][j] == adj[j][i] && {
                    let c = (0..n).filter(|&x| adj[i][x] && adj[j][x]).count();
                    c == if adj[i][j] { lambda } else { mu }
                }
            })
        })
}

pub struct Outcome {
    pub best_cost: i64,
    pub wrong_pairs: usize,
    pub graph: Option<Vec<Vec<bool>>>,
    pub moves: u64,
    pub seconds: f64,
}

/// Simulated annealing for an srg(n, k, lambda, mu), on `threads` threads
/// for at most `seconds`.
pub fn search(n: usize, k: usize, lambda: i32, mu: i32, seconds: f64, threads: usize, seed: u64) -> Outcome {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;
    assert!(n <= 128 && n * k % 2 == 0);
    let t0 = std::time::Instant::now();
    let done = AtomicBool::new(false);
    let moves = AtomicU64::new(0);
    let best = Mutex::new((i64::MAX, 0usize, None::<Vec<Vec<bool>>>));
    std::thread::scope(|sc| {
        for t in 0..threads {
            let (done, moves, best) = (&done, &moves, &best);
            sc.spawn(move || {
                let mut r = Rng(seed ^ (t as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
                let mut my_moves = 0u64;
                'restart: while !done.load(Ordering::Relaxed) && t0.elapsed().as_secs_f64() < seconds {
                    let mut s = State::new(n, k, lambda, mu, &mut r);
                    let sweeps = 400_000u64 * (n as u64 / 10).max(1);
                    for it in 0..sweeps {
                        let temp = 3.0 * (1.0 - it as f64 / sweeps as f64).powi(2) + 0.02;
                        let Some(sw) = s.random_switch(&mut r) else { continue };
                        let before = s.cost;
                        s.apply_switch(sw);
                        my_moves += 1;
                        let delta = (s.cost - before) as f64;
                        if delta > 0.0 && r.unit() >= (-delta / temp).exp() {
                            s.undo_switch(sw);
                        }
                        if it % 4096 == 0 {
                            let mut b = best.lock().unwrap();
                            if s.cost < b.0 {
                                let mut wrong = 0;
                                for i in 0..n {
                                    for j in i + 1..n {
                                        wrong += (s.pair_cost(i, j) > 0) as usize;
                                    }
                                }
                                let g: Vec<Vec<bool>> = (0..n).map(|i| (0..n).map(|j| has(&s.adj[i], j)).collect()).collect();
                                *b = (s.cost, wrong, Some(g));
                            }
                            if s.cost == 0 {
                                done.store(true, Ordering::Relaxed);
                            }
                            drop(b);
                            if done.load(Ordering::Relaxed) || t0.elapsed().as_secs_f64() >= seconds {
                                break 'restart;
                            }
                        }
                    }
                }
                moves.fetch_add(my_moves, Ordering::Relaxed);
            });
        }
    });
    let (best_cost, wrong_pairs, g) = best.into_inner().unwrap();
    let graph = g.filter(|g| best_cost == 0 && is_srg(g, k, lambda as usize, mu as usize));
    Outcome { best_cost, wrong_pairs, graph, moves: moves.load(Ordering::Relaxed), seconds: t0.elapsed().as_secs_f64() }
}

/// The eigenvalues r > s of an srg and their multiplicities f, g.
pub fn spectrum(n: f64, k: f64, lambda: f64, mu: f64) -> (f64, f64, f64, f64) {
    let disc = ((lambda - mu).powi(2) + 4.0 * (k - mu)).sqrt();
    let r = ((lambda - mu) + disc) / 2.0;
    let s = ((lambda - mu) - disc) / 2.0;
    let f = ((n - 1.0) * -s - k) / (r - s);
    let g = (n - 1.0) - f;
    (r, s, f, g)
}

pub fn report(seconds: f64) -> String {
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let mut out = String::from("Nuome, the supergenius, on Conway's 99-graph (srg(99, 14, 1, 2), Conway's $1000 problem)\n\n");
    let (r, s, f, g) = spectrum(99.0, 14.0, 1.0, 2.0);
    out.push_str(&format!(
        "1. Reasoning from the definition alone\n   \
         - lambda = 1: the neighbours of a vertex pair up, each with exactly one other neighbour, so its 14 neighbours are 7 disjoint edges:\n     \
           every vertex lies on exactly 7 triangles, every edge on exactly one; 99 * 7 / 3 = 231 triangles in all\n   \
         - counting paths of length 2 from one vertex: 14 * (14 - 1 - 1) = 168 = 2 * (99 - 1 - 14) = 168 pairs: consistent, which forces n = 99 for k = 14\n   \
         - eigenvalues {r} and {s} with multiplicities {f} and {g}: whole numbers, so the spectrum does not rule it out\n   \
         - the family lambda = 1, mu = 2 allows only n = 9, 99, 243, 6273, 494019; only n = 9 (the 3 x 3 grid) is known to exist\n   \
         - known from the literature: any automorphism group is tiny (order dividing 2 * 3^3 * 7 * 11; even order => divides 6; Makhnev, Minakova, and 2023 work)\n     \
           so the graph, if it exists, is almost without symmetry: no pretty construction to guess. A search must find it by local moves.\n\n"
    ));
    out.push_str(&format!("2. The search must first find the known graphs of this kind ({threads} threads)\n"));
    for (n, k, l, m, name) in [(9, 4, 1, 2, "3 x 3 grid (the n = 9 member of Conway's family)"), (10, 3, 0, 1, "Petersen"), (16, 5, 0, 2, "Clebsch"), (16, 6, 2, 2, "4 x 4 grid / Shrikhande"), (27, 10, 1, 5, "complement of the Schlaefli graph"), (50, 7, 0, 1, "Hoffman-Singleton")] {
        let o = search(n, k, l, m, 60.0, threads, 7);
        out.push_str(&format!(
            "   srg({n},{k},{l},{m}) {name:<48} {}  ({:.1} s, {} moves)\n",
            if o.graph.is_some() { "FOUND, checked".to_string() } else { format!("not found, best: {} pairs wrong", o.wrong_pairs) },
            o.seconds,
            o.moves
        ));
    }
    out.push_str(&format!("\n3. The 99-graph: {seconds:.0} s of annealing on {threads} threads\n"));
    let o = search(99, 14, 1, 2, seconds, threads, 2026);
    match &o.graph {
        Some(g) => {
            out.push_str("   FOUND a graph with score 0, and it passes the full check from scratch: srg(99, 14, 1, 2)\n");
            let mut edges = String::new();
            for i in 0..99 {
                for j in i + 1..99 {
                    if g[i][j] {
                        edges.push_str(&format!("{i} {j}\n"));
                    }
                }
            }
            let _ = std::fs::create_dir_all("out/conway99");
            let _ = std::fs::write("out/conway99/graph.txt", edges);
            out.push_str("   edges written to out/conway99/graph.txt\n");
        }
        None => out.push_str(&format!(
            "   not found. Best graph: {} of 4851 pairs have the wrong number of common neighbours (score {}), after {} moves in {:.0} s.\n",
            o.wrong_pairs, o.best_cost, o.moves, o.seconds
        )),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spectrum_of_the_99_graph() {
        let (r, s, f, g) = spectrum(99.0, 14.0, 1.0, 2.0);
        assert_eq!((r, s, f, g), (3.0, -4.0, 54.0, 44.0));
    }

    #[test]
    fn the_checker_knows_the_3_by_3_grid() {
        // (a, b) ~ (c, d) when they share a row or a column
        let g: Vec<Vec<bool>> = (0..9).map(|i| (0..9).map(|j| i != j && (i / 3 == j / 3 || i % 3 == j % 3)).collect()).collect();
        assert!(is_srg(&g, 4, 1, 2));
        assert!(!is_srg(&g, 4, 0, 2));
    }

    #[test]
    fn the_search_finds_petersen_and_the_grid() {
        assert!(search(10, 3, 0, 1, 20.0, 2, 1).graph.is_some());
        assert!(search(9, 4, 1, 2, 20.0, 2, 1).graph.is_some());
    }
}
