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
                    if !is_edge && may_edge {
                        // undecided: an edge needs exactly 1, a non-edge exactly 2
                        let ok = if sure >= 2 || possible == 0 {
                            self.non_edge(u, v)
                        } else if possible < 2 {
                            self.edge(u, v)
                        } else {
                            true
                        };
                        if !ok || sure > 2 {
                            return false;
                        }
                        continue;
                    }
                    let target = if is_edge { 1 } else { 2 };
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
    let root = (4.0 * k - 7.0).sqrt();
    let (r, t) = ((-1.0 + root) / 2.0, (-1.0 - root) / 2.0);
    let f = ((n - 1.0) - (2.0 * k - (n - 1.0)) / (r - t)) / 2.0;
    let g = n - 1.0 - f;
    let fixed = s.k as usize + 1;
    let decided = |u: usize, v: usize| s.e[u] >> v & 1 == 1 || s.p[u] >> v & 1 == 0;
    let mut chosen: Vec<usize> = (0..fixed).collect();
    let mut rest: Vec<usize> = (fixed..s.n).collect();
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
    let mut s = State { n, k: k as u32, e: vec![0; n], p: (0..n).map(|v| all & !(1 << v)).collect(), trail: Vec::new() };
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
                if is_srg(&s.e, k) {
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
            let mut s = State { n: base.n, k: base.k, e: base.e.clone(), p: base.p.clone(), trail: Vec::new() };
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

/// Report lines for the attempt at the problem; `budget` branch points in all.
pub fn report(budget: u64) -> Vec<String> {
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
    fn the_99_graph_is_set_up() {
        let o = search(14, 10).expect("set up");
        assert_eq!((o.n, o.open_pairs), (99, 84 * 83 / 2));
    }
}
