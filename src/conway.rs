//! Conway's 99-graph problem: is there a graph on 99 vertices, each with 14
//! neighbours, where adjacent vertices have exactly 1 common neighbour and
//! non-adjacent ones exactly 2 (a strongly regular graph srg(99, 14, 1, 2))?
//!
//! The same search runs for every srg(n, k, 1, 2), k even:
//! - lambda = 1 puts every edge in exactly one triangle, so the neighbours of
//!   vertex 0 form k/2 triangles;
//! - a vertex v at distance 2 from 0 has exactly 2 neighbours next to 0, from
//!   different triangles, and two vertices next to 0 from different
//!   triangles have exactly one common neighbour besides 0: the vertices at
//!   distance 2 ARE these pairs, n = 1 + k + (k choose 2) - k/2;
//! so every edge touching 0 or its neighbours is fixed, and the search only
//! decides the edges among the distance-2 vertices (84 of them for n = 99).
//! It branches on one undecided pair at a time and after each choice
//! propagates, to a fixed point: degrees (exactly k) and common neighbours
//! (exactly 1 for an edge, 2 for a non-edge), counted with bitsets. A
//! complete search that finds nothing proves no such graph exists.

/// One set of neighbours per vertex, n <= 128.
type Set = u128;

struct State {
    n: usize,
    k: u32,
    /// common neighbours of joined pairs and of the others
    lambda: u32,
    mu: u32,
    /// in the growing rule, the size of a block: every point has exactly
    /// one neighbour in each other block (0 = not used)
    block: usize,
    /// sure edges
    e: Vec<Set>,
    /// possible edges (not ruled out), sure ones included
    p: Vec<Set>,
    /// (u, v, true = an edge was set / false = a non-edge)
    trail: Vec<(u8, u8, bool)>,
}

impl State {
    fn edge(&mut self, u: usize, v: usize) -> bool {
        if self.e[u] >> v & 1 == 1 {
            return true;
        }
        if self.p[u] >> v & 1 == 0 {
            return false;
        }
        self.e[u] |= 1 << v;
        self.e[v] |= 1 << u;
        self.trail.push((u as u8, v as u8, true));
        true
    }

    fn non_edge(&mut self, u: usize, v: usize) -> bool {
        if self.p[u] >> v & 1 == 0 {
            return true;
        }
        if self.e[u] >> v & 1 == 1 {
            return false;
        }
        self.p[u] &= !(1 << v);
        self.p[v] &= !(1 << u);
        self.trail.push((u as u8, v as u8, false));
        true
    }

    fn undo(&mut self, to: usize) {
        while self.trail.len() > to {
            let (u, v, was_edge) = self.trail.pop().expect("longer than `to`");
            let (u, v) = (u as usize, v as usize);
            if was_edge {
                self.e[u] &= !(1 << v);
                self.e[v] &= !(1 << u);
            } else {
                self.p[u] |= 1 << v;
                self.p[v] |= 1 << u;
            }
        }
    }

    /// Apply every forced choice; false on a contradiction.
    fn propagate(&mut self) -> bool {
        loop {
            let before = self.trail.len();
            for v in 0..self.n {
                let (sure, possible) = (self.e[v].count_ones(), self.p[v].count_ones());
                if sure > self.k || possible < self.k {
                    return false;
                }
                let open = self.p[v] & !self.e[v];
                if open != 0 && (sure == self.k || possible == self.k) {
                    for w in bits(open) {
                        let ok = if sure == self.k { self.non_edge(v, w) } else { self.edge(v, w) };
                        if !ok {
                            return false;
                        }
                    }
                }
            }
            for u in 0..self.n {
                for v in u + 1..self.n {
                    let sure = (self.e[u] & self.e[v]).count_ones();
                    let possible = (self.p[u] & self.p[v]).count_ones();
                    let is_edge = self.e[u] >> v & 1 == 1;
                    let may_edge = self.p[u] >> v & 1 == 1;
                    let (l, m) = (self.lambda, self.mu);
                    if !is_edge && may_edge {
                        // undecided: an edge needs exactly lambda, a non-edge exactly mu
                        if sure > l.max(m) || possible < l.min(m) {
                            return false;
                        }
                        let ok = if sure > l || possible < l {
                            self.non_edge(u, v)
                        } else if sure > m || possible < m {
                            self.edge(u, v)
                        } else {
                            true
                        };
                        if !ok {
                            return false;
                        }
                        continue;
                    }
                    let target = if is_edge { l } else { m };
                    if sure > target || possible < target {
                        return false;
                    }
                    if sure == target {
                        // no other common neighbour: w next to one of them surely is not next to the other
                        for w in bits(self.e[u] & self.p[v] & !self.e[v]) {
                            if !self.non_edge(v, w) {
                                return false;
                            }
                        }
                        for w in bits(self.e[v] & self.p[u] & !self.e[u]) {
                            if !self.non_edge(u, w) {
                                return false;
                            }
                        }
                    } else if possible == target {
                        for w in bits(self.p[u] & self.p[v]) {
                            if !self.edge(u, w) || !self.edge(v, w) {
                                return false;
                            }
                        }
                    }
                }
            }
            if self.block > 0 {
                // exactly one neighbour in every other block
                let b = self.block;
                for v in 0..self.n {
                    for c in 0..self.n / b {
                        if c == v / b {
                            continue;
                        }
                        let mask: Set = (((1 as Set) << b) - 1) << (c * b);
                        let (sure, open) = (self.e[v] & mask, self.p[v] & mask);
                        if sure.count_ones() > 1 || open == 0 {
                            return false;
                        }
                        if sure.count_ones() == 1 {
                            for w in bits(open & !sure) {
                                if !self.non_edge(v, w) {
                                    return false;
                                }
                            }
                        } else if open.count_ones() == 1 && !self.edge(v, open.trailing_zeros() as usize) {
                            return false;
                        }
                    }
                }
            }
            if self.trail.len() == before {
                return true;
            }
        }
    }

    /// The undecided pair to branch on: at the vertex with the most sure
    /// edges that still has an undecided pair.
    fn choose(&self) -> Option<(usize, usize)> {
        let v = (0..self.n).filter(|&v| self.p[v] & !self.e[v] != 0).min_by_key(|&v| ((self.p[v] & !self.e[v]).count_ones(), v))?;
        let w = bits(self.p[v] & !self.e[v]).max_by_key(|&w| (self.e[w].count_ones(), std::cmp::Reverse(w)))?;
        Some((v, w))
    }
}

/// The eigenvalue test. A graph srg(n, k, 1, 2) has eigenvalues k, r and s
/// (the roots of x^2 + x - (k - 2)) with multiplicities 1, f and g, so
///   A - sI - ((k - s)/n) J  is positive semidefinite of rank f, and
///   rI - A + ((k - r)/n) J  is positive semidefinite of rank g
/// (J all ones). Every principal submatrix of these is then positive
/// semidefinite of at most that rank: this is checked on the vertices whose
/// pairs are all decided (the fixed part plus a greedy choice of the rest).
fn algebra_ok(s: &State) -> bool {
    let (n, k) = (s.n as f64, s.k as f64);
    let d = s.lambda as f64 - s.mu as f64;
    let root = (d * d + 4.0 * (k - s.mu as f64)).sqrt();
    let (r, t) = ((d + root) / 2.0, (d - root) / 2.0);
    let f = ((n - 1.0) - (2.0 * k + (n - 1.0) * d) / (r - t)) / 2.0;
    let g = n - 1.0 - f;
    let decided = |u: usize, v: usize| s.e[u] >> v & 1 == 1 || s.p[u] >> v & 1 == 0;
    let mut chosen: Vec<usize> = Vec::new();
    let mut rest: Vec<usize> = (0..s.n).collect();
    rest.sort_by_key(|&v| (s.p[v] & !s.e[v]).count_ones());
    for v in rest {
        if chosen.iter().all(|&u| decided(u, v)) {
            chosen.push(v);
        }
    }
    let adj = |u: usize, v: usize| if s.e[u] >> v & 1 == 1 { 1.0 } else { 0.0 };
    let first = |u: usize, v: usize| adj(u, v) - if u == v { t } else { 0.0 } - (k - t) / n;
    let second = |u: usize, v: usize| if u == v { r } else { 0.0 } - adj(u, v) + (k - r) / n;
    psd_rank(&chosen, &first).is_some_and(|rank| rank as f64 <= f + 1e-9) && psd_rank(&chosen, &second).is_some_and(|rank| rank as f64 <= g + 1e-9)
}

/// The rank of the matrix m on these vertices if it is positive
/// semidefinite (pivoted Cholesky), None if it is not.
fn psd_rank(vs: &[usize], m: &dyn Fn(usize, usize) -> f64) -> Option<usize> {
    let size = vs.len();
    let mut a: Vec<f64> = (0..size * size).map(|i| m(vs[i / size], vs[i % size])).collect();
    let mut order: Vec<usize> = (0..size).collect();
    let eps = 1e-8;
    for step in 0..size {
        let (best, value) = (step..size).map(|i| (i, a[order[i] * size + order[i]])).fold((step, f64::MIN), |b, x| if x.1 > b.1 { x } else { b });
        if value < eps {
            // what is left must be zero, else the matrix is not semidefinite
            let left = (step..size).all(|i| (step..size).all(|j| a[order[i] * size + order[j]].abs() < 1e-6));
            return left.then_some(step);
        }
        order.swap(step, best);
        let p = order[step];
        let pivot = a[p * size + p].sqrt();
        for i in step..size {
            a[order[i] * size + p] /= pivot;
        }
        for i in step + 1..size {
            for j in step + 1..size {
                let (oi, oj) = (order[i], order[j]);
                a[oi * size + oj] -= a[oi * size + p] * a[oj * size + p];
            }
        }
    }
    Some(size)
}

fn bits(mut s: Set) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if s == 0 {
            return None;
        }
        let i = s.trailing_zeros() as usize;
        s &= s - 1;
        Some(i)
    })
}

/// The fixed part: vertex 0, its k neighbours in k/2 triangles, and the
/// distance-2 vertices as pairs of neighbours from different triangles.
fn start(k: u32) -> Option<(State, Vec<(usize, usize)>)> {
    let k = k as usize;
    let first = 1..=k;
    let partner = |a: usize| if (a - 1) % 2 == 0 { a + 1 } else { a - 1 };
    let mut pairs = Vec::new();
    for a in first.clone() {
        for b in a + 1..=k {
            if (a - 1) / 2 != (b - 1) / 2 {
                pairs.push((a, b));
            }
        }
    }
    let n = 1 + k + pairs.len();
    if n > 128 {
        return None;
    }
    let all: Set = if n == 128 { !0 } else { (1u128 << n) - 1 };
    let mut s = State { n, k: k as u32, lambda: 1, mu: 2, block: 0, e: vec![0; n], p: (0..n).map(|v| all & !(1 << v)).collect(), trail: Vec::new() };
    let mut want = vec![vec![false; n]; n];
    for a in first.clone() {
        want[0][a] = true;
        want[a][partner(a)] = true;
    }
    for (i, &(a, b)) in pairs.iter().enumerate() {
        want[a][1 + k + i] = true;
        want[b][1 + k + i] = true;
    }
    for u in 0..=k {
        for v in u + 1..n {
            let ok = if want[u][v] || want[v][u] { s.edge(u, v) } else { s.non_edge(u, v) };
            if !ok {
                return None;
            }
        }
    }
    s.trail.clear();
    Some((s, pairs))
}

/// Every pair counted directly: an independent check of a found graph.
fn is_srg(adj: &[Set], k: u32) -> bool {
    let n = adj.len();
    (0..n).all(|u| {
        adj[u].count_ones() == k
            && (u + 1..n).all(|v| {
                let common = (adj[u] & adj[v]).count_ones();
                if adj[u] >> v & 1 == 1 {
                    common == 1
                } else {
                    common == 2
                }
            })
    })
}

pub struct Outcome {
    pub n: usize,
    pub k: u32,
    /// pairs the search decides
    pub open_pairs: usize,
    pub nodes: u64,
    /// the most pairs decided at once
    pub deepest: usize,
    /// graphs found (labelled, with vertex 0's neighbourhood fixed), each checked
    pub found: u64,
    /// the first graph found, as neighbour sets
    pub example: Option<Vec<Set>>,
    /// the whole tree was searched
    pub complete: bool,
}

/// Search srg(n, k, 1, 2) with at most `budget` branch nodes.
pub fn search(k: u32, budget: u64) -> Option<Outcome> {
    let (s, _) = start(k)?;
    Some(run(s, k, budget))
}

/// The search from a set-up state.
fn run(mut s: State, k: u32, budget: u64) -> Outcome {
    let open_pairs = (0..s.n).map(|v| (s.p[v] & !s.e[v]).count_ones() as usize).sum::<usize>() / 2;
    let mut out = Outcome { n: s.n, k, open_pairs, nodes: 0, deepest: 0, found: 0, example: None, complete: false };
    if !s.propagate() {
        out.complete = true;
        return out;
    }
    // explicit stack: (trail length before the choice, pair, the non-edge branch still to try)
    let mut stack: Vec<(usize, (usize, usize), bool)> = Vec::new();
    loop {
        out.deepest = out.deepest.max(s.trail.len());
        match s.choose() {
            None => {
                if s.e.iter().all(|x| x.count_ones() == k) && violations_with(&s.e, s.lambda, s.mu) == 0 {
                    out.found += 1;
                    out.example.get_or_insert_with(|| s.e.clone());
                }
            }
            Some((u, v)) if out.nodes < budget => {
                out.nodes += 1;
                let mark = s.trail.len();
                stack.push((mark, (u, v), true));
                if s.edge(u, v) && s.propagate() && algebra_ok(&s) {
                    continue;
                }
            }
            Some(_) => return out,
        }
        // backtrack to the latest choice with its other branch untried
        loop {
            let Some((mark, (u, v), other)) = stack.pop() else {
                out.complete = true;
                return out;
            };
            s.undo(mark);
            if other {
                stack.push((mark, (u, v), false));
                if s.non_edge(u, v) && s.propagate() && algebra_ok(&s) {
                    break;
                }
            }
        }
    }
}

/// Every perfect matching of the points 0..points, as partner arrays.
fn matchings(points: usize) -> Vec<Vec<usize>> {
    fn go(partner: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        let Some(a) = partner.iter().position(|&p| p == usize::MAX) else {
            out.push(partner.clone());
            return;
        };
        for b in a + 1..partner.len() {
            if partner[b] == usize::MAX {
                partner[a] = b;
                partner[b] = a;
                go(partner, out);
                partner[a] = usize::MAX;
                partner[b] = usize::MAX;
            }
        }
    }
    let mut out = Vec::new();
    go(&mut vec![usize::MAX; points], &mut out);
    out
}

/// All permutations of 0..m.
fn permutations(m: usize) -> Vec<Vec<usize>> {
    if m == 0 {
        return vec![vec![]];
    }
    let mut out = Vec::new();
    for p in permutations(m - 1) {
        for at in 0..=p.len() {
            let mut q = p.clone();
            q.insert(at, m - 1);
            out.push(q);
        }
    }
    out
}

/// Vertex 1's neighbours at distance 2 from vertex 0 are the pairs {1, b}
/// with b in the other k/2 - 1 triangles, and lambda = 1 pairs them up (the
/// edge from 1 to each lies in exactly one triangle). The symmetries of the
/// fixed part that keep vertex 1 (permute the other triangles, swap the two
/// sides of any of them) act on these matchings; one matching per orbit is
/// enough. Returns the representatives, as pairs of points 2t + side over
/// the other triangles t, and the number of matchings.
fn matching_orbits(k: u32) -> (Vec<Vec<(usize, usize)>>, usize) {
    let m = k as usize / 2 - 1;
    let all = matchings(2 * m);
    let key = |partner: &[usize]| partner.iter().fold(0u128, |acc, &p| acc * 32 + p as u128);
    let perms = permutations(m);
    let mut seen = std::collections::HashSet::new();
    let mut reps = Vec::new();
    for partner in &all {
        if seen.contains(&key(partner)) {
            continue;
        }
        reps.push((0..partner.len()).filter(|&a| a < partner[a]).map(|a| (a, partner[a])).collect());
        for perm in &perms {
            for flips in 0..1usize << m {
                let g = |p: usize| 2 * perm[p / 2] + ((p % 2) ^ (flips >> (p / 2) & 1));
                let mut image = vec![0; partner.len()];
                for a in 0..partner.len() {
                    image[g(a)] = g(partner[a]);
                }
                seen.insert(key(&image));
            }
        }
    }
    (reps, all.len())
}

/// The search split by vertex 1's matching, one case per orbit, the cases
/// run in parallel with at most `budget` branch points each. Returns the
/// number of matchings and each case's outcome.
pub fn search_split(k: u32, budget: u64) -> Option<(usize, Vec<Outcome>)> {
    use std::sync::{atomic::AtomicUsize, atomic::Ordering, Mutex};
    let (base, pairs) = start(k)?;
    let (reps, total) = matching_orbits(k);
    let index = |a: usize, b: usize| 1 + k as usize + pairs.iter().position(|&p| p == (a.min(b), a.max(b))).expect("a cross pair");
    // point 2t + side over the triangles after vertex 1's -> its vertex
    let vertex = |p: usize| 1 + 2 * (p / 2 + 1) + p % 2;
    let cases: Vec<Mutex<Option<State>>> = reps
        .iter()
        .map(|rep| {
            let mut s = State { n: base.n, k: base.k, lambda: 1, mu: 2, block: 0, e: base.e.clone(), p: base.p.clone(), trail: Vec::new() };
            for &(x, y) in rep {
                s.edge(index(1, vertex(x)), index(1, vertex(y)));
            }
            s.trail.clear();
            Mutex::new(Some(s))
        })
        .collect();
    let slots: Vec<Mutex<Option<Outcome>>> = (0..cases.len()).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= cases.len() {
                    break;
                }
                let s = cases[i].lock().expect("no panics").take().expect("each case once");
                let o = run(s, k, budget);
                *slots[i].lock().expect("no panics") = Some(o);
            });
        }
    });
    Some((total, slots.into_iter().map(|m| m.into_inner().expect("done").expect("ran")).collect()))
}

/// The fitness search, the same genetic search as the Goldbach formula
/// (src/evolve.rs): seeded, a population, the best tenth kept, parents by a
/// tournament of three. A member is a whole graph: the fixed part plus a
/// graph on the other points with every degree right. Its fitness is how
/// far the common-neighbour counts are off, summed over every pair (0 = the
/// graph asked for). A child is a parent with a few line swaps (a-b, c-d
/// become a-c, b-d: degrees stay), then `climb` swaps kept when not worse.
pub struct Evolve {
    pub population: usize,
    pub generations: usize,
    pub seed: u64,
    pub climb: usize,
}

/// Sum over all pairs of |common neighbours - target| (lambda = 1, mu = 2).
fn violations(g: &[Set]) -> u32 {
    violations_with(g, 1, 2)
}

/// The same for any lambda (joined pairs) and mu (the others).
fn violations_with(g: &[Set], lambda: u32, mu: u32) -> u32 {
    let n = g.len();
    let mut total = 0;
    for u in 0..n {
        for v in u + 1..n {
            let common = (g[u] & g[v]).count_ones() as i32;
            let target = if g[u] >> v & 1 == 1 { lambda } else { mu } as i32;
            total += (common - target).unsigned_abs();
        }
    }
    total
}

/// One swap among the free points (from `free` on); false if none was made.
fn swap(g: &mut [Set], free: usize, r: &mut crate::evolve::Rng) -> Option<[usize; 4]> {
    let n = g.len();
    let inside: Set = ((1u128 << n) - 1) & !((1u128 << free) - 1);
    let pick = |r: &mut crate::evolve::Rng, g: &[Set]| -> Option<(usize, usize)> {
        let a = free + r.below(n - free);
        let ns: Vec<usize> = bits(g[a] & inside).collect();
        (!ns.is_empty()).then(|| (a, ns[r.below(ns.len())]))
    };
    let (a, b) = pick(r, g)?;
    let (c, d) = pick(r, g)?;
    if a == c || a == d || b == c || b == d || g[a] >> c & 1 == 1 || g[b] >> d & 1 == 1 {
        return None;
    }
    for (x, y, on) in [(a, b, false), (c, d, false), (a, c, true), (b, d, true)] {
        if on {
            g[x] |= 1 << y;
            g[y] |= 1 << x;
        } else {
            g[x] &= !(1 << y);
            g[y] &= !(1 << x);
        }
    }
    Some([a, b, c, d])
}

fn unswap(g: &mut [Set], [a, b, c, d]: [usize; 4]) {
    for (x, y, on) in [(a, c, false), (b, d, false), (a, b, true), (c, d, true)] {
        if on {
            g[x] |= 1 << y;
            g[y] |= 1 << x;
        } else {
            g[x] &= !(1 << y);
            g[y] &= !(1 << x);
        }
    }
}

/// A child: a few swaps, then a climb that keeps swaps that are not worse.
fn child(parent: &[Set], free: usize, seed: u64, climb: usize) -> (u32, Vec<Set>) {
    let mut r = crate::evolve::Rng(seed.max(1));
    let mut g = parent.to_vec();
    for _ in 0..1 + r.below(3) {
        swap(&mut g, free, &mut r);
    }
    let mut score = violations(&g);
    for _ in 0..climb {
        if let Some(m) = swap(&mut g, free, &mut r) {
            let s = violations(&g);
            if s <= score {
                score = s;
            } else {
                unswap(&mut g, m);
            }
        }
    }
    (score, g)
}

pub struct Evolved {
    pub best: u32,
    /// the best fitness after each tenth of the generations
    pub history: Vec<u32>,
    pub graph: Option<Vec<Set>>,
    pub free_points: usize,
    /// the best graph of the last generation, perfect or not
    pub best_graph: Vec<Set>,
}

/// A picture of a graph as its adjacency matrix, one square per pair of
/// points, as a 24-bit BMP: black = a line, white = no line, where that
/// pair's common neighbours are right; red = too many, blue = too few (dark
/// for a line, light for no line); grey = a point with itself; thin grey
/// grid lines every `block` points. Returns the file and the counts of
/// pairs with too many and too few.
pub fn picture(g: &[Set], lambda: u32, mu: u32, block: usize, cell: usize) -> (Vec<u8>, usize, usize) {
    let n = g.len();
    let side = n * cell;
    let (mut many, mut few) = (0, 0);
    let mut px = vec![[255u8, 255, 255]; side * side]; // rgb, top row first
    for u in 0..n {
        for v in 0..n {
            let joined = g[u] >> v & 1 == 1;
            let colour = if u == v {
                [150, 150, 150]
            } else {
                let common = (g[u] & g[v]).count_ones();
                let target = if joined { lambda } else { mu };
                if u < v {
                    if common > target {
                        many += 1;
                    } else if common < target {
                        few += 1;
                    }
                }
                match (common.cmp(&target), joined) {
                    (std::cmp::Ordering::Equal, true) => [0, 0, 0],
                    (std::cmp::Ordering::Equal, false) => [255, 255, 255],
                    (std::cmp::Ordering::Greater, true) => [150, 0, 0],
                    (std::cmp::Ordering::Greater, false) => [255, 120, 120],
                    (std::cmp::Ordering::Less, true) => [0, 0, 150],
                    (std::cmp::Ordering::Less, false) => [130, 160, 255],
                }
            };
            for y in 0..cell {
                for x in 0..cell {
                    let edge = block > 0 && ((x == 0 && v % block == 0) || (y == 0 && u % block == 0));
                    px[(u * cell + y) * side + v * cell + x] = if edge && u != v { [190, 190, 190] } else { colour };
                }
            }
        }
    }
    // BMP: rows bottom-up, blue-green-red, each row padded to 4 bytes
    let row = (side * 3).div_ceil(4) * 4;
    let size = 54 + row * side;
    let mut out = Vec::with_capacity(size);
    out.extend(b"BM");
    out.extend((size as u32).to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(54u32.to_le_bytes());
    out.extend(40u32.to_le_bytes());
    out.extend((side as i32).to_le_bytes());
    out.extend((side as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(24u16.to_le_bytes());
    out.extend([0u8; 24]);
    for y in (0..side).rev() {
        for x in 0..side {
            let [r, gr, b] = px[y * side + x];
            out.extend([b, gr, r]);
        }
        out.extend(vec![0u8; row - side * 3]);
    }
    (out, many, few)
}

/// Evolve graphs srg(n, k, 1, 2) around the fixed part.
pub fn evolve(k: u32, s: &Evolve) -> Option<Evolved> {
    let (base, _) = start(k)?;
    let (n, free) = (base.n, k as usize + 1);
    let m = n - free;
    let d = k as usize - 2; // lines each free point still needs among the free points
    let mut r = crate::evolve::Rng(s.seed.max(1));
    // the first graph: a circulant (every degree d), then shuffled by swaps
    let mut first = base.e.clone();
    for i in 0..m {
        for j in 1..=d / 2 {
            let (x, y) = (free + i, free + (i + j) % m);
            first[x] |= 1 << y;
            first[y] |= 1 << x;
        }
    }
    let mut pop: Vec<(u32, Vec<Set>)> = (0..s.population)
        .map(|_| {
            let mut g = first.clone();
            for _ in 0..20 * m {
                swap(&mut g, free, &mut r);
            }
            (violations(&g), g)
        })
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut history = Vec::new();
    for generation in 0..s.generations {
        pop.sort_by_key(|p| p.0);
        if pop[0].0 == 0 {
            break;
        }
        if generation % (s.generations / 10).max(1) == 0 {
            history.push(pop[0].0);
        }
        let elite = (s.population / 10).max(1);
        let mut next: Vec<(u32, Vec<Set>)> = pop.iter().take(elite).cloned().collect();
        // parents and seeds drawn in a fixed order, children built in parallel
        let jobs: Vec<(usize, u64)> = (next.len()..s.population)
            .map(|_| {
                let mut best = r.below(pop.len());
                for _ in 0..2 {
                    let c = r.below(pop.len());
                    if pop[c].0 < pop[best].0 {
                        best = c;
                    }
                }
                (best, r.next())
            })
            .collect();
        let size = jobs.len().div_ceil(threads).max(1);
        let children: Vec<(u32, Vec<Set>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = jobs.chunks(size).map(|part| {
                let pop = &pop;
                sc.spawn(move || part.iter().map(|&(p, seed)| child(&pop[p].1, free, seed, s.climb)).collect::<Vec<_>>())
            }).collect();
            hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
        });
        next.extend(children);
        pop = next;
    }
    pop.sort_by_key(|p| p.0);
    let best = pop[0].0;
    history.push(best);
    Some(Evolved { best, history, graph: (best == 0 && is_srg(&pop[0].1, k)).then(|| pop[0].1.clone()), free_points: m, best_graph: pop[0].1.clone() })
}

/// The "evolve the small graph" rule: n = t copies of a block graph that
/// already meets the conditions inside itself (every pair of a block has its
/// common neighbours complete). Then a point outside a block may touch at
/// most one point of it, and when every point needs exactly t - 1 more
/// neighbours, it has exactly one in each other block: every two blocks are
/// joined by a perfect matching. A member is one permutation per pair of
/// blocks; a move swaps two entries of one permutation, so every member keeps
/// the block structure and every degree. Same genetic search as the Goldbach
/// formula; fitness = the common-neighbour counts off, summed over all pairs.
pub struct Blocks {
    /// the block graph, as neighbour sets on 0..size
    pub block: Vec<Set>,
    pub copies: usize,
    /// common neighbours of joined and of other pairs in the graph sought
    pub lambda: u32,
    pub mu: u32,
}

impl Blocks {
    fn size(&self) -> usize {
        self.block.len()
    }
    fn pairs(&self) -> Vec<(usize, usize)> {
        (0..self.copies).flat_map(|a| (a + 1..self.copies).map(move |b| (a, b))).collect()
    }
    /// The whole graph from one permutation per pair of blocks.
    fn graph(&self, perms: &[Vec<usize>]) -> Vec<Set> {
        let m = self.size();
        let mut g = vec![0 as Set; m * self.copies];
        for c in 0..self.copies {
            for v in 0..m {
                for w in bits(self.block[v]) {
                    g[c * m + v] |= 1 << (c * m + w);
                }
            }
        }
        for (p, &(a, b)) in self.pairs().iter().enumerate() {
            for (x, &y) in perms[p].iter().enumerate() {
                let (u, v) = (a * m + x, b * m + y);
                g[u] |= 1 << v;
                g[v] |= 1 << u;
            }
        }
        g
    }
}

/// The growing rule as a complete search: the blocks' own lines fixed, every
/// point with exactly one neighbour in each other block, and the choices
/// that symmetry allows made up front. The blocks are vertex-transitive, so
/// point 0's neighbour in each other block can be taken to be that block's
/// first point; with lambda = 1 those neighbours pair up into triangles
/// through point 0, and renumbering the other blocks makes the pairs
/// (1, 2), (3, 4), ... A complete search that finds nothing proves that no
/// such graph is built this way.
fn search_blocks_state(shape: &Blocks) -> Option<State> {
    let (m, t) = (shape.size(), shape.copies);
    let n = m * t;
    if n > 128 {
        return None;
    }
    let k = shape.block[0].count_ones() + t as u32 - 1;
    let all: Set = if n == 128 { !0 } else { (1u128 << n) - 1 };
    let mut s = State { n, k, lambda: shape.lambda, mu: shape.mu, block: m, e: vec![0; n], p: (0..n).map(|v| all & !(1 << v)).collect(), trail: Vec::new() };
    for c in 0..t {
        for x in 0..m {
            for y in x + 1..m {
                let ok = if shape.block[x] >> y & 1 == 1 { s.edge(c * m + x, c * m + y) } else { s.non_edge(c * m + x, c * m + y) };
                if !ok {
                    return None;
                }
            }
        }
    }
    for c in 1..t {
        s.edge(0, c * m);
    }
    if shape.lambda == 1 && (t - 1) % 2 == 0 {
        for c in (1..t).step_by(2) {
            s.edge(c * m, (c + 1) * m);
        }
    }
    s.trail.clear();
    let _ = k;
    Some(s)
}

pub fn search_blocks(shape: &Blocks, budget: u64) -> Option<Outcome> {
    let s = search_blocks_state(shape)?;
    let k = s.k;
    Some(run(s, k, budget))
}

/// Automorphisms of a block that fix its point 0.
fn stabiliser(block: &[Set]) -> Vec<Vec<usize>> {
    let m = block.len();
    permutations(m)
        .into_iter()
        .filter(|g| g[0] == 0 && (0..m).all(|x| (0..m).all(|y| (block[x] >> y & 1) == (block[g[x]] >> g[y] & 1))))
        .collect()
}

/// The growing rule's complete search split into cases by the matching
/// between blocks 0 and 1 (point 0 to point 0 already fixed): one matching
/// per orbit under the automorphisms of the two blocks that fix point 0.
/// Cases run in parallel, `budget` branch points each. Returns (number of
/// matchings, outcomes).
pub fn search_blocks_split(shape: &Blocks, budget: u64) -> Option<(usize, Vec<Outcome>)> {
    use std::sync::{atomic::AtomicUsize, atomic::Ordering, Mutex};
    let m = shape.size();
    let stab = stabiliser(&shape.block);
    let mut seen = std::collections::HashSet::new();
    let mut reps: Vec<Vec<usize>> = Vec::new();
    let mut total = 0;
    for rest in permutations(m - 1) {
        let pi: Vec<usize> = std::iter::once(0).chain(rest.iter().map(|&x| x + 1)).collect();
        total += 1;
        if seen.contains(&pi) {
            continue;
        }
        for a in &stab {
            for b in &stab {
                // pi' = b . pi . a^-1
                let mut image = vec![0; m];
                for x in 0..m {
                    image[a[x]] = b[pi[x]];
                }
                seen.insert(image);
            }
        }
        reps.push(pi);
    }
    let base = search_blocks_state(shape)?;
    let cases: Vec<Mutex<Option<State>>> = reps
        .iter()
        .map(|pi| {
            let mut s = State { n: base.n, k: base.k, lambda: base.lambda, mu: base.mu, block: base.block, e: base.e.clone(), p: base.p.clone(), trail: Vec::new() };
            let ok = (0..m).all(|x| s.edge(x, m + pi[x]));
            s.trail.clear();
            Mutex::new(ok.then_some(s))
        })
        .collect();
    let k = base.k;
    let slots: Vec<Mutex<Option<Outcome>>> = (0..cases.len()).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= cases.len() {
                    break;
                }
                let o = match cases[i].lock().expect("no panics").take() {
                    Some(s) => run(s, k, budget),
                    None => Outcome { n: base.n, k, open_pairs: 0, nodes: 0, deepest: 0, found: 0, example: None, complete: true },
                };
                *slots[i].lock().expect("no panics") = Some(o);
            });
        }
    });
    Some((total, slots.into_iter().map(|x| x.into_inner().expect("done").expect("ran")).collect()))
}

/// Complete graph on m points.
fn complete(m: usize) -> Vec<Set> {
    (0..m).map(|v| ((1 as Set) << m) - 1 & !(1 << v)).collect()
}

/// Known strongly regular graphs that the growing rule should rebuild:
/// (name, block, copies, lambda, mu).
pub fn known_block_graphs() -> Vec<(&'static str, Vec<Set>, usize, u32, u32)> {
    vec![
        ("3 triangles -> srg(9, 4, 1, 2), the 3 x 3 rook's graph", complete(3), 3, 1, 2),
        ("4 x K4 -> srg(16, 6, 2, 2), the 4 x 4 rook's graph", complete(4), 4, 2, 2),
        ("5 x K5 -> srg(25, 8, 3, 2), the 5 x 5 rook's graph", complete(5), 5, 3, 2),
        ("5 triangles -> srg(15, 6, 1, 3), the generalized quadrangle GQ(2, 2)", complete(3), 5, 1, 3),
        ("9 triangles -> srg(27, 10, 1, 5), the generalized quadrangle GQ(2, 4)", complete(3), 9, 1, 5),
        // a control: the rule applies, but no such graph exists (its multiplicities are not whole numbers)
        ("control, 7 triangles -> srg(21, 8, 1, 4), which does NOT exist", complete(3), 7, 1, 4),
    ]
}

/// The 3 x 3 rook's graph, srg(9, 4, 1, 2).
pub fn rook9() -> Vec<Set> {
    (0..9).map(|v| (0..9).filter(|&w| w != v && (w / 3 == v / 3 || w % 3 == v % 3)).fold(0 as Set, |s, w| s | 1 << w)).collect()
}

/// Evolve the joining permutations.
pub fn evolve_blocks(shape: &Blocks, s: &Evolve) -> Evolved {
    let m = shape.size();
    let links = shape.pairs().len();
    let mut r = crate::evolve::Rng(s.seed.max(1));
    let random_perms = |r: &mut crate::evolve::Rng| -> Vec<Vec<usize>> {
        (0..links)
            .map(|_| {
                let mut p: Vec<usize> = (0..m).collect();
                for i in (1..m).rev() {
                    p.swap(i, r.below(i + 1));
                }
                p
            })
            .collect()
    };
    let score = |perms: &[Vec<usize>]| violations_with(&shape.graph(perms), shape.lambda, shape.mu);
    let child = |parent: &[Vec<usize>], seed: u64| -> (u32, Vec<Vec<usize>>) {
        let mut r = crate::evolve::Rng(seed.max(1));
        let mut p = parent.to_vec();
        for _ in 0..1 + r.below(3) {
            let l = r.below(links);
            let (i, j) = (r.below(m), r.below(m));
            p[l].swap(i, j);
        }
        let mut best = score(&p);
        for _ in 0..s.climb {
            let l = r.below(links);
            let (i, j) = (r.below(m), r.below(m));
            p[l].swap(i, j);
            let now = score(&p);
            if now <= best {
                best = now;
            } else {
                p[l].swap(i, j);
            }
        }
        (best, p)
    };
    let mut pop: Vec<(u32, Vec<Vec<usize>>)> = (0..s.population).map(|_| {
        let p = random_perms(&mut r);
        (score(&p), p)
    }).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut history = Vec::new();
    for generation in 0..s.generations {
        pop.sort_by_key(|p| p.0);
        if pop[0].0 == 0 {
            break;
        }
        if generation % (s.generations / 10).max(1) == 0 {
            history.push(pop[0].0);
        }
        let elite = (s.population / 10).max(1);
        let mut next: Vec<(u32, Vec<Vec<usize>>)> = pop.iter().take(elite).cloned().collect();
        let jobs: Vec<(usize, u64)> = (next.len()..s.population)
            .map(|_| {
                let mut best = r.below(pop.len());
                for _ in 0..2 {
                    let c = r.below(pop.len());
                    if pop[c].0 < pop[best].0 {
                        best = c;
                    }
                }
                (best, r.next())
            })
            .collect();
        let size = jobs.len().div_ceil(threads).max(1);
        let children: Vec<(u32, Vec<Vec<usize>>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = jobs.chunks(size).map(|part| {
                let (pop, child) = (&pop, &child);
                sc.spawn(move || part.iter().map(|&(p, seed)| child(&pop[p].1, seed)).collect::<Vec<_>>())
            }).collect();
            hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
        });
        next.extend(children);
        pop = next;
    }
    pop.sort_by_key(|p| p.0);
    let best = pop[0].0;
    history.push(best);
    let g = shape.graph(&pop[0].1);
    let k = g[0].count_ones();
    let regular = g.iter().all(|x| x.count_ones() == k);
    Evolved { best, history, graph: (best == 0 && regular && violations_with(&g, shape.lambda, shape.mu) == 0).then(|| g.clone()), free_points: m * shape.copies, best_graph: g }
}

/// The fitness search started from a given graph: every member begins as
/// that graph, children swap line ends anywhere (a-b, c-d become a-c, b-d,
/// so every degree stays) and keep the swaps that are not worse. Same
/// genetic search as the Goldbach formula.
pub fn evolve_from(start: &[Set], s: &Evolve) -> Evolved {
    let mut r = crate::evolve::Rng(s.seed.max(1));
    let first = (violations(start), start.to_vec());
    let mut pop: Vec<(u32, Vec<Set>)> = vec![first; s.population.max(1)];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut history = vec![pop[0].0];
    for generation in 0..s.generations {
        pop.sort_by_key(|p| p.0);
        if pop[0].0 == 0 {
            break;
        }
        if generation > 0 && generation % (s.generations / 10).max(1) == 0 {
            history.push(pop[0].0);
        }
        let elite = (s.population / 10).max(1);
        let mut next: Vec<(u32, Vec<Set>)> = pop.iter().take(elite).cloned().collect();
        let jobs: Vec<(usize, u64)> = (next.len()..s.population)
            .map(|_| {
                let mut best = r.below(pop.len());
                for _ in 0..2 {
                    let c = r.below(pop.len());
                    if pop[c].0 < pop[best].0 {
                        best = c;
                    }
                }
                (best, r.next())
            })
            .collect();
        let size = jobs.len().div_ceil(threads).max(1);
        let children: Vec<(u32, Vec<Set>)> = std::thread::scope(|sc| {
            let hs: Vec<_> = jobs.chunks(size).map(|part| {
                let pop = &pop;
                sc.spawn(move || part.iter().map(|&(p, seed)| child(&pop[p].1, 0, seed, s.climb)).collect::<Vec<_>>())
            }).collect();
            hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
        });
        next.extend(children);
        pop = next;
    }
    pop.sort_by_key(|p| p.0);
    let best = pop[0].0;
    history.push(best);
    let g = pop[0].1.clone();
    let k = g.first().map_or(0, |x| x.count_ones());
    Evolved { best, history, graph: (best == 0 && is_srg(&g, k)).then(|| g.clone()), free_points: g.len(), best_graph: g }
}

/// A graph as text, one "a-b" per line.
pub fn edge_list(g: &[Set]) -> String {
    let mut out = String::new();
    for u in 0..g.len() {
        for v in bits(g[u]).filter(|&v| v > u) {
            out.push_str(&format!("{u}-{v}\n"));
        }
    }
    out
}

/// Read "a-b" lines into neighbour sets (at most 128 points).
pub fn parse_edges(text: &str) -> Option<Vec<Set>> {
    let mut edges = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (a, b) = line.split_once('-')?;
        edges.push((a.trim().parse::<usize>().ok()?, b.trim().parse::<usize>().ok()?));
    }
    let n = edges.iter().map(|&(a, b)| a.max(b) + 1).max()?;
    if n > 128 {
        return None;
    }
    let mut g = vec![0 as Set; n];
    for (a, b) in edges {
        if a != b {
            g[a] |= 1 << b;
            g[b] |= 1 << a;
        }
    }
    Some(g)
}

/// Check a graph given as text, one "a-b" per line, against the 99-graph
/// conditions: 99 points, 14 neighbours each, 1 common neighbour for joined
/// pairs, 2 for the others. Returns report lines (with pairs anyone can
/// check by hand) and whether it is a solution; draws it when `pictures`.
pub fn check_text(text: &str, pictures: Option<&str>) -> (Vec<String>, bool) {
    let mut out = Vec::new();
    let mut edges = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once('-').and_then(|(a, b)| Some((a.trim().parse::<usize>().ok()?, b.trim().parse::<usize>().ok()?))) {
            Some(e) => edges.push(e),
            None => out.push(format!("line {} is not \"a-b\": {line}", i + 1)),
        }
    }
    let n = edges.iter().map(|&(a, b)| a.max(b) + 1).max().unwrap_or(0);
    if n > 128 {
        out.push(format!("{n} points: this checker handles at most 128"));
        return (out, false);
    }
    let mut g = vec![0 as Set; n];
    let (mut repeated, mut loops) = (0, 0);
    for &(a, b) in &edges {
        if a == b {
            loops += 1;
        } else if g[a] >> b & 1 == 1 {
            repeated += 1;
        } else {
            g[a] |= 1 << b;
            g[b] |= 1 << a;
        }
    }
    let lines = edges.len() - repeated - loops;
    out.push(format!("points: {n} (needs 99); lines: {lines} (needs 693){}", if repeated + loops > 0 { format!("; {repeated} repeated and {loops} from a point to itself, ignored") } else { String::new() }));
    let wrong_degree: Vec<String> = (0..n).filter(|&v| g[v].count_ones() != 14).map(|v| format!("{v} has {}", g[v].count_ones())).collect();
    out.push(if wrong_degree.is_empty() { "neighbours: every point has 14: right".to_string() } else { format!("neighbours: {} points do not have 14: {}", wrong_degree.len(), wrong_degree.iter().take(10).cloned().collect::<Vec<_>>().join(", ")) });
    let list = |s: Set| bits(s).map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
    let (mut joined_ok, mut joined_bad, mut other_ok, mut other_bad) = (0, 0, 0, 0);
    let mut shown = Vec::new();
    for u in 0..n {
        for v in u + 1..n {
            let common = g[u] & g[v];
            let joined = g[u] >> v & 1 == 1;
            let right = common.count_ones() == if joined { 1 } else { 2 };
            match (joined, right) {
                (true, true) => joined_ok += 1,
                (true, false) => joined_bad += 1,
                (false, true) => other_ok += 1,
                (false, false) => other_bad += 1,
            }
            if !right && shown.len() < 8 {
                shown.push(format!(
                    "  {u} and {v} ({}): {u}'s neighbours are {}; {v}'s neighbours are {}; in both: {} = {} common, needs {}",
                    if joined { "joined" } else { "not joined" },
                    list(g[u]),
                    list(g[v]),
                    if common == 0 { "none".to_string() } else { list(common) },
                    common.count_ones(),
                    if joined { 1 } else { 2 }
                ));
            }
        }
    }
    out.push(format!("joined pairs: {joined_ok} right, {joined_bad} wrong (each needs exactly 1 common neighbour)"));
    out.push(format!("pairs not joined: {other_ok} right, {other_bad} wrong (each needs exactly 2 common neighbours)"));
    if !shown.is_empty() {
        out.push("the first wrong pairs, with both neighbour lists so they can be checked by hand:".into());
        out.extend(shown);
    }
    let solved = n == 99 && wrong_degree.is_empty() && joined_bad == 0 && other_bad == 0;
    out.push(if solved {
        "ALL 4851 PAIRS ARE RIGHT: this is a graph srg(99, 14, 1, 2), a solution to Conway's 99-graph problem".to_string()
    } else {
        "not a solution yet".to_string()
    });
    if let Some(dir) = pictures {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let (image, _, _) = picture(&g, 1, 2, 0, 6);
        let path = dir.join("your-graph.bmp");
        if std::fs::create_dir_all(&dir).is_ok() && std::fs::write(&path, image).is_ok() {
            out.push(format!("picture: {} (black = line, white = no line where right; red = too many common neighbours, blue = too few)", path.display()));
        }
    }
    (out, solved)
}

/// The known necessary conditions for a strongly regular graph
/// srg(n, k, lambda, mu), each as (name, what was computed, holds). A single
/// failure proves that no such graph exists.
pub fn conditions(n: i64, k: i64, l: i64, m: i64) -> Vec<(&'static str, String, bool)> {
    let mut out = Vec::new();
    let count = k * (k - l - 1) == (n - k - 1) * m;
    out.push(("counting", format!("k(k - lambda - 1) = {} and (n - k - 1) mu = {}", k * (k - l - 1), (n - k - 1) * m), count));
    // eigenvalues r > s: roots of x^2 - (lambda - mu) x - (k - mu)
    let disc = ((l - m) * (l - m) + 4 * (k - m)) as f64;
    let (r, s) = (((l - m) as f64 + disc.sqrt()) / 2.0, ((l - m) as f64 - disc.sqrt()) / 2.0);
    let f = ((n - 1) as f64 - (2.0 * k as f64 + (n - 1) as f64 * (l - m) as f64) / (r - s)) / 2.0;
    let g = (n - 1) as f64 - f;
    let whole = |x: f64| (x - x.round()).abs() < 1e-9;
    out.push(("multiplicities", format!("eigenvalues {r:.4} and {s:.4} with multiplicities {f:.4} and {g:.4}"), whole(f) && whole(g) && f > 0.0 && g > 0.0));
    let (kf, nf) = (k as f64, n as f64);
    let krein1 = (r + 1.0) * (kf + r + 2.0 * r * s) <= (kf + r) * (s + 1.0).powi(2) + 1e-9;
    let krein2 = (s + 1.0) * (kf + s + 2.0 * r * s) <= (kf + s) * (r + 1.0).powi(2) + 1e-9;
    out.push(("Krein conditions", format!("{:.2} <= {:.2} and {:.2} <= {:.2}", (r + 1.0) * (kf + r + 2.0 * r * s), (kf + r) * (s + 1.0).powi(2), (s + 1.0) * (kf + s + 2.0 * r * s), (kf + s) * (r + 1.0).powi(2)), krein1 && krein2));
    out.push(("absolute bound", format!("n = {n} <= f(f + 3)/2 = {:.1} and <= g(g + 3)/2 = {:.1}", f * (f + 3.0) / 2.0, g * (g + 3.0) / 2.0), nf <= f * (f + 3.0) / 2.0 + 1e-9 && nf <= g * (g + 3.0) / 2.0 + 1e-9));
    let clique = 1.0 - kf / s;
    out.push(("Hoffman clique bound", format!("a clique has at most 1 - k/s = {clique:.2} points; lambda = {l} needs cliques of {}", l + 2), (l + 2) as f64 <= clique + 1e-9));
    out.push(("Hoffman ratio bound", format!("an independent set has at most n(-s)/(k - s) = {:.2} points", nf * -s / (kf - s)), nf * -s / (kf - s) >= 1.0));
    let (mf, sf) = (m as f64, s);
    let claw = if (mf - sf * sf).abs() > 1e-9 && (mf - sf * (sf + 1.0)).abs() > 1e-9 { 2.0 * (r + 1.0) <= sf * (sf + 1.0) * (mf + 1.0) + 1e-9 } else { true };
    out.push(("claw bound (Brouwer)", format!("2(r + 1) = {:.2} <= s(s + 1)(mu + 1) = {:.2}", 2.0 * (r + 1.0), sf * (sf + 1.0) * (mf + 1.0)), claw));
    out
}

/// Report lines for the attempt at the problem; `budget` branch points in all.
pub fn report(budget: u64, fitness: &Evolve, pictures: Option<&str>, seed_graph: Option<&str>) -> Vec<String> {
    let (reps, total) = matching_orbits(14);
    let mut out = vec![
        "what is asked: 99 points, some joined by lines; every point has exactly 14 lines; two joined points have exactly 1 common neighbour; two points not joined have exactly 2".to_string(),
        "why a search can settle it: there are 99 * 98 / 2 = 4851 possible lines, so finitely many graphs (2^4851); a complete search answers yes (a graph, checked line by line) or no (every possibility ruled out)".into(),
        "step 1, triangles: two joined points share exactly 1 neighbour, so every line lies in exactly one triangle; the 14 neighbours of a point form 7 triangles through it, touching only at that point".into(),
        "step 2, name the points: take point 0 and its 14 neighbours (7 triangles). Each of the other 84 points is not joined to 0, so it shares exactly 2 neighbours with 0, from different triangles (from the same one, a line would lie in two triangles). Two neighbours of 0 from different triangles share exactly one neighbour besides 0. There are 14 * 12 / 2 = 84 such pairs, so the 84 points are exactly these pairs, one each".into(),
        "step 3, what is left: every line at point 0 and its neighbours is now fixed; open are the lines among the 84 points: 84 * 83 / 2 = 3486 yes/no choices, each of the 84 points needing 12 more lines".into(),
        format!("step 4, split into cases: point 1's 12 neighbours among the 84 must pair up into 6 lines (each line at point 1 lies in one triangle): {total} ways. Renaming the triangles and swapping points inside them turns many ways into each other (46080 renamings), which leaves {} truly different cases", reps.len()),
        "step 5, search each case: pick an open pair, try 'line', and if that fails 'no line'; after every choice apply everything it forces (a point with 14 lines gets no more; two points whose common neighbours are complete get no further shared ones; and the reverse when only just enough are possible); a contradiction rules the branch out".into(),
        "step 6, algebra: such a graph would have eigenvalues 14, 3 (54 times) and -4 (44 times), so A + 4I - (2/11)J and 3I - A + (1/9)J (J all ones) are positive semidefinite of rank 54 and 44; after every choice the part of the graph whose pairs are all decided is tested against this, and a failure rules the branch out. The search finishes one point at a time so that this part grows".into(),
        "how to read the result: a case is closed when every branch in it ends in a contradiction; all cases closed = no such graph; a completed graph = yes, checked pair by pair before it is reported".into(),
        "warm-ups first: the same search on the smaller members of the family, 1 common neighbour for joined and 2 for others, with 4, 6 and 8 neighbours (9, 19 and 33 points), where the answer is known: it must find the 9-point graph and rule out the other two".into(),
    ];
    // the mathematical route first: does a known necessary condition fail?
    out.push("known necessary conditions (one failure proves no such graph exists); tested first where the answer is known:".into());
    for (n, k, what) in [(9i64, 4i64, "exists"), (19, 6, "does not exist"), (33, 8, "does not exist"), (243, 22, "exists"), (99, 14, "OPEN")] {
        let c = conditions(n, k, 1, 2);
        let failed: Vec<&str> = c.iter().filter(|x| !x.2).map(|x| x.0).collect();
        out.push(format!(
            "  srg({n}, {k}, 1, 2), {what}: {}",
            if failed.is_empty() { format!("passes all {}", c.len()) } else { format!("fails {}", failed.join(", ")) }
        ));
    }
    for (name, computed, holds) in conditions(99, 14, 1, 2) {
        out.push(format!("  99: {name}: {computed}: {}", if holds { "holds" } else { "FAILS" }));
    }
    out.push("  so no known condition rules the 99-graph out: a proof that it does not exist needs a new one, and a construction needs a new idea; this is where the problem has stood since 1971".into());
    for k in [4u32, 6, 8] {
        if let Some(o) = search(k, u64::MAX) {
            let what = if o.found > 0 {
                format!("found {} (each checked pair by pair); it exists", o.found)
            } else {
                "none: it does not exist (the complete search proves it; the eigenvalue condition agrees)".to_string()
            };
            out.push(format!("warm-up srg({}, {k}, 1, 2): complete search of {} undecided pairs, {} branch points: {what}", o.n, o.open_pairs, o.nodes));
        }
    }
    let per_case = (budget / reps.len() as u64).max(1);
    let t0 = std::time::Instant::now();
    match search_split(14, per_case) {
        Some((_, cases)) => {
            let secs = t0.elapsed().as_secs_f64().max(1e-9);
            let nodes: u64 = cases.iter().map(|c| c.nodes).sum();
            let closed = cases.iter().filter(|c| c.complete && c.found == 0).count();
            let found: u64 = cases.iter().map(|c| c.found).sum();
            let deepest = cases.iter().map(|c| c.deepest).max().unwrap_or(0);
            let open: Vec<String> = cases.iter().enumerate().filter(|(_, c)| !c.complete).map(|(i, _)| (i + 1).to_string()).collect();
            out.push(format!(
                "the 99-point search: {} cases, each with {} open pairs after its 6 fixed lines",
                cases.len(),
                cases.first().map_or(0, |c| c.open_pairs)
            ));
            out.push(format!(
                "{nodes} branch points in all (at most {per_case} per case, {:.0} per second): {closed} of {} cases closed completely (no graph in them), {found} graphs found, cases still open: {}; at most {deepest} pairs decided at once",
                nodes as f64 / secs,
                cases.len(),
                if open.is_empty() { "none".to_string() } else { open.join(", ") }
            ));
            out.push(if found > 0 {
                "FOUND: such a graph, checked pair by pair (below)".to_string()
            } else if closed == cases.len() {
                "COMPLETE: every case is closed, so no such graph exists".to_string()
            } else {
                "the search is NOT complete: nothing is settled".to_string()
            });
            if let Some(g) = cases.iter().find_map(|c| c.example.clone()) {
                let edges: Vec<String> = (0..g.len()).flat_map(|u| bits(g[u]).filter(move |&v| v > u).map(move |v| format!("{u}-{v}"))).collect();
                out.push(format!("the graph: {}", edges.join(" ")));
            }
        }
        None => out.push("the search could not be set up".into()),
    }
    // the growing rule: 11 copies of the 9-point graph, joined pairwise by matchings
    let big = evolve_blocks(&Blocks { block: rook9(), copies: 11, lambda: 1, mu: 2 }, fitness);
    out.push("growing rule: build the 99-graph from 11 copies of the 9-point graph (4 neighbours each, conditions already met inside). A point outside a copy can then touch at most one point of it, and needing 10 more neighbours with 10 other copies it touches exactly one in each: every two copies are joined by a perfect matching of their 9 points. The search chooses these 55 matchings (a permutation of 9 each); a move swaps two entries, so the structure always holds".into());
    out.push("  the rule tested first on known graphs built the same way (copies of a block, every two joined by a matching):".into());
    for (name, block, copies, lambda, mu) in known_block_graphs() {
        let e = evolve_blocks(&Blocks { block, copies, lambda, mu }, fitness);
        out.push(format!(
            "    {name}: {}",
            if e.graph.is_some() { "built, checked pair by pair".to_string() } else { format!("NOT built (best {} off)", e.best) }
        ));
    }
    out.push("  the same rule as a complete search (it can prove, not only find): the blocks' own lines fixed, one neighbour in each other block, the matching between blocks 0 and 1 split into cases up to the blocks' symmetries:".into());
    for (name, block, copies, lambda, mu) in known_block_graphs() {
        if let Some((_, cases)) = search_blocks_split(&Blocks { block, copies, lambda, mu }, u64::MAX) {
            let found: u64 = cases.iter().map(|c| c.found).sum();
            out.push(format!("    {name}: {}", if found > 0 { format!("found ({found} labelled copies), search complete") } else { "none, search complete: proved that no such graph is built this way".to_string() }));
        }
    }
    let per_case = (budget / 662).max(1);
    if let Some((total, cases)) = search_blocks_split(&Blocks { block: rook9(), copies: 11, lambda: 1, mu: 2 }, per_case) {
        let closed = cases.iter().filter(|c| c.complete && c.found == 0).count();
        let found: u64 = cases.iter().map(|c| c.found).sum();
        out.push(format!(
            "    99 points (11 copies of the 9-point graph): {total} matchings, {} cases; {closed} closed, {found} graphs found, at most {per_case} branch points per case{}",
            cases.len(),
            if closed == cases.len() { ": ALL closed, so the 99-graph is not 11 copies of the 9-point graph joined this way" } else { ": not complete, nothing is settled" }
        ));
    }
    let steps: Vec<String> = big.history.iter().map(|h| h.to_string()).collect();
    out.push(format!("  99 points by the rule, fitness search: best fitness by tenths of the run: {}", steps.join(" -> ")));
    out.push(match &big.graph {
        Some(g) => {
            let edges: Vec<String> = (0..g.len()).flat_map(|u| bits(g[u]).filter(move |&v| v > u).map(move |v| format!("{u}-{v}"))).collect();
            format!("  FOUND by the growing rule, fitness 0 and checked pair by pair: {}", edges.join(" "))
        }
        None => format!("  best graph off by {} in total: not found. The rule assumes the 99-graph contains 11 separate copies of the 9-point graph, which nobody knows; if it does not, this route cannot succeed", big.best),
    });
    if let Some(dir) = pictures {
        // the best 99-point graph and the true 9-point graph, to compare by eye
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
        let (image, many, few) = picture(&big.best_graph, 1, 2, 9, 6);
        let (small, _, _) = picture(&rook9(), 1, 2, 3, 40);
        let saved = std::fs::create_dir_all(&dir).is_ok()
            && std::fs::write(dir.join("99-graph.bmp"), image).is_ok()
            && std::fs::write(dir.join("9-graph.bmp"), small).is_ok();
        out.push(if saved {
            format!("  pictures: {} and 9-graph.bmp beside it (the true 9-point graph). One square per pair of points: black = a line, white = no line, where the common neighbours are right; red = too many common neighbours ({many} pairs), blue = too few ({few} pairs), dark for a line and light for no line; grid lines between the 11 copies", dir.join("99-graph.bmp").display())
        } else {
            format!("  pictures could not be written to {}", dir.display())
        });
    }
    if let Some(path) = seed_graph {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        match std::fs::read_to_string(&file).ok().and_then(|t| parse_edges(&t)) {
            Some(g) if g.len() == 99 => {
                let wrong = |g: &[Set]| (0..99).map(|u| (u + 1..99).filter(|&v| (g[u] & g[v]).count_ones() != if g[u] >> v & 1 == 1 { 1 } else { 2 }).count()).sum::<usize>();
                // a fixed skeleton would be hopeless when two unjoined hubs share no neighbour
                let hubs_apart = (0..99).any(|u| (u + 1..99).any(|v| g[u] >> v & 1 == 0 && (g[u] & g[v]).count_ones() == 0));
                let e = evolve_from(&g, fitness);
                let steps: Vec<String> = e.history.iter().map(|h| h.to_string()).collect();
                out.push(format!("starting from Joose Hotari's hand-drawn 7-network graph ({path}): every point has {} neighbours; {} of the 4851 pairs are wrong", if g.iter().all(|x| x.count_ones() == 14) { "14" } else { "not always 14" }, wrong(&g)));
                if hubs_apart {
                    out.push("  it cannot be kept as a fixed skeleton: some unjoined pairs there share no neighbour at all, and with their lines fixed nothing could give them 2; so the search may move any line".into());
                }
                out.push(format!("  repaired by the fitness search (line swaps keeping every degree): fitness {}; {} pairs wrong in the best graph", steps.join(" -> "), wrong(&e.best_graph)));
                out.push(match &e.graph {
                    Some(g) => format!("  FOUND from Joose's graph, checked pair by pair: {}", edge_list(g).replace('\n', " ")),
                    None => "  not solved from it".into(),
                });
            }
            _ => out.push(format!("Joose's graph could not be read from {}", file.display())),
        }
    }
    if let Some(e) = evolve(14, fitness) {
        let steps: Vec<String> = e.history.iter().map(|h| h.to_string()).collect();
        out.push(format!(
            "fitness search beside it (the Goldbach formula's genetic search): {} graphs, {} generations, seed {}, the best tenth kept, parents by a tournament of three; fitness = how far the common-neighbour counts are off, summed over all 4851 pairs (0 = the graph); a child is a parent with a few swaps of line ends among the {} free points (degrees stay right) and then {} swaps kept when not worse",
            fitness.population, fitness.generations, fitness.seed, e.free_points, fitness.climb
        ));
        out.push(format!("best fitness by tenths of the run: {}", steps.join(" -> ")));
        out.push(match &e.graph {
            Some(g) => {
                let edges: Vec<String> = (0..g.len()).flat_map(|u| bits(g[u]).filter(move |&v| v > u).map(move |v| format!("{u}-{v}"))).collect();
                format!("FOUND by the fitness search, fitness 0 and checked pair by pair: {}", edges.join(" "))
            }
            None => format!("best graph found is off by {} in total: not the graph. A fitness search can find the graph but never prove it does not exist", e.best),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nine_vertex_graph_exists() {
        // srg(9, 4, 1, 2) is the 3 x 3 rook's graph (Paley graph of order 9)
        let o = search(4, u64::MAX).expect("set up");
        assert_eq!(o.n, 9);
        assert!(o.complete && o.found > 0);
        assert!(is_srg(o.example.as_ref().expect("found"), 4));
    }

    #[test]
    fn infeasible_parameters_have_no_graph() {
        // srg(19, 6, 1, 2) and srg(33, 8, 1, 2) fail the eigenvalue condition
        for (k, n) in [(6, 19), (8, 33)] {
            let o = search(k, u64::MAX).expect("set up");
            assert_eq!(o.n, n);
            assert!(o.complete && o.found == 0, "k = {k}");
        }
    }

    #[test]
    fn split_search_agrees() {
        // srg(33, 8, 1, 2) split by vertex 1's matching: every case closes
        let (total, cases) = search_split(8, u64::MAX).expect("set up");
        assert_eq!(total, 15); // matchings of 6 points
        assert!(cases.iter().all(|c| c.complete && c.found == 0));
        // srg(9, 4, 1, 2) is found again after the split
        let (_, cases) = search_split(4, u64::MAX).expect("set up");
        assert!(cases.iter().any(|c| c.found > 0));
        let (reps, total) = matching_orbits(14);
        assert_eq!(total, 10395);
        assert!(reps.len() < 100, "{}", reps.len());
    }

    #[test]
    fn fitness_search_finds_the_nine_point_graph() {
        let e = evolve(4, &Evolve { population: 10, generations: 20, seed: 1, climb: 20 }).expect("set up");
        assert_eq!(e.best, 0);
        assert!(is_srg(e.graph.as_ref().expect("found"), 4));
        let a = evolve(14, &Evolve { population: 6, generations: 2, seed: 5, climb: 10 }).expect("set up");
        let b = evolve(14, &Evolve { population: 6, generations: 2, seed: 5, climb: 10 }).expect("set up");
        assert_eq!(a.history, b.history); // seeded: the same run twice
    }

    #[test]
    fn the_block_rule_rebuilds_the_nine_point_graph() {
        // three triangles joined pairwise by matchings: the 3 x 3 rook's graph
        let triangle: Vec<Set> = vec![0b110, 0b101, 0b011];
        let e = evolve_blocks(&Blocks { block: triangle, copies: 3, lambda: 1, mu: 2 }, &Evolve { population: 10, generations: 20, seed: 3, climb: 20 });
        assert_eq!(e.best, 0);
        assert!(is_srg(e.graph.as_ref().expect("found"), 4));
        assert!(is_srg(&rook9(), 4));
    }

    #[test]
    fn the_complete_block_search_proves_both_ways() {
        let (_, cases) = search_blocks_split(&Blocks { block: complete(3), copies: 5, lambda: 1, mu: 3 }, u64::MAX).expect("set up");
        assert!(cases.iter().all(|c| c.complete) && cases.iter().any(|c| c.found > 0)); // GQ(2, 2) exists
        let (_, cases) = search_blocks_split(&Blocks { block: complete(3), copies: 7, lambda: 1, mu: 4 }, u64::MAX).expect("set up");
        assert!(cases.iter().all(|c| c.complete && c.found == 0)); // srg(21, 8, 1, 4) does not
    }

    #[test]
    fn the_99_graph_is_set_up() {
        let o = search(14, 10).expect("set up");
        assert_eq!((o.n, o.open_pairs), (99, 84 * 83 / 2));
    }
}
