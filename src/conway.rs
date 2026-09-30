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
        let v = (0..self.n).filter(|&v| self.p[v] & !self.e[v] != 0).max_by_key(|&v| (self.e[v].count_ones(), std::cmp::Reverse(v)))?;
        let w = bits(self.p[v] & !self.e[v]).max_by_key(|&w| (self.e[w].count_ones(), std::cmp::Reverse(w)))?;
        Some((v, w))
    }
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
fn start(k: u32) -> Option<State> {
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
    Some(s)
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
    let mut s = start(k)?;
    let open_pairs = (0..s.n).map(|v| (s.p[v] & !s.e[v]).count_ones() as usize).sum::<usize>() / 2;
    let mut out = Outcome { n: s.n, k, open_pairs, nodes: 0, deepest: 0, found: 0, example: None, complete: false };
    if !s.propagate() {
        out.complete = true;
        return Some(out);
    }
    // explicit stack: (trail length before the choice, pair, the non-edge branch still to try)
    let mut stack: Vec<(usize, (usize, usize), bool)> = Vec::new();
    let decided = |s: &State| s.trail.len();
    loop {
        out.deepest = out.deepest.max(decided(&s));
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
                if s.edge(u, v) && s.propagate() {
                    continue;
                }
            }
            Some(_) => return Some(out),
        }
        // backtrack to the latest choice with its other branch untried
        loop {
            let Some((mark, (u, v), other)) = stack.pop() else {
                out.complete = true;
                return Some(out);
            };
            s.undo(mark);
            if other {
                stack.push((mark, (u, v), false));
                if s.non_edge(u, v) && s.propagate() {
                    break;
                }
            }
        }
    }
}

/// Report lines for the attempt at the problem.
pub fn report(budget: u64) -> Vec<String> {
    let mut out = Vec::new();
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
    let t0 = std::time::Instant::now();
    match search(14, budget) {
        Some(o) => {
            let rate = o.nodes as f64 / t0.elapsed().as_secs_f64().max(1e-9);
            out.push(format!("srg(99, 14, 1, 2): the structure fixes every edge at vertex 0 and its 14 neighbours; the search decides the {} pairs among the other 84 vertices", o.open_pairs));
            out.push(if o.complete {
                if o.found > 0 {
                    format!("COMPLETE: found {} such graphs, each checked pair by pair", o.found)
                } else {
                    "COMPLETE: no such graph exists".to_string()
                }
            } else {
                format!(
                    "searched {} branch points (the budget), at most {} of the {} pairs decided at once, {} graphs found; the search is NOT complete: nothing is settled (about {rate:.0} branch points per second)",
                    o.nodes, o.deepest, o.open_pairs, o.found
                )
            });
            if let Some(g) = o.example {
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
    fn the_99_graph_is_set_up() {
        let o = search(14, 10).expect("set up");
        assert_eq!((o.n, o.open_pairs), (99, 84 * 83 / 2));
    }
}
