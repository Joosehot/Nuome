//! The graph function with a prescribed symmetry: a strongly regular graph
//! with an automorphism σ of prime order p that fixes one vertex ∞ and
//! moves the rest in m orbits of p (v = 1 + m p).
//!
//!   (i, a) ~ (j, b)  exactly when  b - a ∈ D_ij  (D_ji = -D_ij, 0 ∉ D_ii = -D_ii),
//!   ∞ ~ (i, a)       exactly when  i < k / p.
//!
//! Every such graph has σ: (i, a) ↦ (i, a + 1) as an automorphism, so the
//! search runs over the sets D_ij only (m² / 2 subsets of Z_p instead of
//! v² / 2 edges). Simulated annealing scores how far the graph is from
//! srg(v, k, λ, μ) (degrees and common neighbours, counted from the m + 1
//! orbit representatives, which is enough under σ); a graph at score 0 is
//! checked again from scratch on every pair before it is reported.

pub struct Model {
    pub p: usize,
    pub m: usize,
    pub k: usize,
    pub lambda: usize,
    pub mu: usize,
    /// d[i][j]: bitmask of D_ij ⊆ Z_p
    pub d: Vec<Vec<u32>>,
}

fn neg(mask: u32, p: usize) -> u32 {
    let mut out = 0;
    for c in 0..p {
        if mask >> c & 1 == 1 {
            out |= 1 << ((p - c) % p);
        }
    }
    out
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Model {
    pub fn v(&self) -> usize {
        1 + self.m * self.p
    }
    fn vertex(&self, i: usize, a: usize) -> usize {
        1 + i * self.p + a
    }
    /// The adjacency as bit rows (v <= 128).
    pub fn rows(&self) -> Vec<u128> {
        let (p, m) = (self.p, self.m);
        let mut r = vec![0u128; self.v()];
        let star = self.k / p;
        for i in 0..star {
            for a in 0..p {
                let x = self.vertex(i, a);
                r[0] |= 1 << x;
                r[x] |= 1;
            }
        }
        for i in 0..m {
            for j in 0..m {
                let mask = self.d[i][j];
                for a in 0..p {
                    for c in 0..p {
                        if mask >> c & 1 == 1 {
                            let (x, y) = (self.vertex(i, a), self.vertex(j, (a + c) % p));
                            r[x] |= 1 << y;
                        }
                    }
                }
            }
        }
        r
    }
    /// How far from srg(v, k, λ, μ): squared errors over pairs from the orbit representatives.
    pub fn cost(&self) -> i64 {
        let r = self.rows();
        let v = self.v();
        let reps: Vec<usize> = std::iter::once(0).chain((0..self.m).map(|i| self.vertex(i, 0))).collect();
        let mut c = 0i64;
        for &x in &reps {
            let deg = r[x].count_ones() as i64;
            c += 4 * (deg - self.k as i64).pow(2);
            for y in 0..v {
                if y == x {
                    continue;
                }
                let common = (r[x] & r[y]).count_ones() as i64;
                let want = if r[x] >> y & 1 == 1 { self.lambda } else { self.mu } as i64;
                c += (common - want).pow(2);
            }
        }
        c
    }
    fn toggle(&mut self, i: usize, j: usize, c: usize) {
        let p = self.p;
        self.d[i][j] ^= 1 << c;
        if i == j {
            if c != (p - c) % p {
                self.d[i][i] ^= 1 << ((p - c) % p);
            }
        } else {
            self.d[j][i] = neg(self.d[i][j], p);
        }
    }
}

pub struct Outcome {
    pub best: i64,
    pub pairs_wrong_share: f64,
    pub found: Option<Model>,
    pub moves: u64,
    pub seconds: f64,
}

/// Anneal for at most `seconds`, restarting; v = 1 + m p <= 128.
pub fn search(v: usize, k: usize, lambda: usize, mu: usize, p: usize, seconds: f64, seed: u64) -> Option<Outcome> {
    if (v - 1) % p != 0 || k % p != 0 || v > 128 {
        return None;
    }
    let m = (v - 1) / p;
    let t0 = std::time::Instant::now();
    let mut rng = Rng(seed | 1);
    let mut best = i64::MAX;
    let mut moves = 0u64;
    let pairs = ((m + 1) * (v - 1)) as f64;
    let mut best_wrong = 1.0;
    while t0.elapsed().as_secs_f64() < seconds {
        let mut md = Model { p, m, k, lambda, mu, d: vec![vec![0u32; m]; m] };
        // a random start near the right degrees
        for i in 0..m {
            for j in i..m {
                for c in 0..p {
                    if (i != j || c != 0) && rng.below(v) < k {
                        if i == j && md.d[i][i] >> c & 1 == 1 {
                            continue;
                        }
                        md.toggle(i, j, c);
                    }
                }
            }
        }
        let mut cost = md.cost();
        let sweeps = 30_000usize;
        for it in 0..sweeps {
            if it % 1024 == 0 && t0.elapsed().as_secs_f64() >= seconds {
                break;
            }
            let temp = 1.5 * (1.0 - it as f64 / sweeps as f64).powi(2) + 0.05;
            let (i, j) = (rng.below(m), rng.below(m));
            let c = rng.below(p);
            if i == j && c == 0 {
                continue;
            }
            md.toggle(i, j, c);
            moves += 1;
            let nc = md.cost();
            let delta = (nc - cost) as f64;
            if delta <= 0.0 || rng.unit() < (-delta / temp).exp() {
                cost = nc;
            } else {
                md.toggle(i, j, c);
            }
            if cost < best {
                best = cost;
                // the share of wrong pairs at the best
                let r = md.rows();
                let reps: Vec<usize> = std::iter::once(0).chain((0..m).map(|i| 1 + i * p)).collect();
                let wrong = reps.iter().map(|&x| (0..v).filter(|&y| y != x && (r[x] & r[y]).count_ones() as usize != if r[x] >> y & 1 == 1 { lambda } else { mu }).count()).sum::<usize>();
                best_wrong = wrong as f64 / pairs;
            }
            if cost == 0 {
                let adj: Vec<Vec<bool>> = {
                    let r = md.rows();
                    (0..v).map(|x| (0..v).map(|y| r[x] >> y & 1 == 1).collect()).collect()
                };
                if crate::golden_graph::check(&adj, k as u64, lambda as u64, mu as u64).is_ok() {
                    return Some(Outcome { best: 0, pairs_wrong_share: 0.0, found: Some(md), moves, seconds: t0.elapsed().as_secs_f64() });
                }
            }
        }
    }
    Some(Outcome { best, pairs_wrong_share: best_wrong, found: None, moves, seconds: t0.elapsed().as_secs_f64() })
}

pub fn report(v: usize, k: usize, lambda: usize, mu: usize, p: usize, seconds: f64) -> String {
    let Some(o) = search(v, k, lambda, mu, p, seconds, 0x9e37_79b9_7f4a_7c15) else {
        return format!("srg({v},{k},{lambda},{mu}) with an automorphism of order {p} fixing one vertex: the orders do not fit (v - 1 and k must be multiples of {p})\n");
    };
    let mut out = format!("srg({v},{k},{lambda},{mu}) with an automorphism of order {p} fixing one vertex ({} orbits of {p}): {} moves in {:.1} s\n", (v - 1) / p, o.moves, o.seconds);
    match o.found {
        Some(md) => {
            out.push_str("  FOUND, and checked on every pair from scratch:\n");
            for i in 0..md.m {
                let row: Vec<String> = (0..md.m).map(|j| format!("{:?}", (0..p).filter(|c| md.d[i][j] >> c & 1 == 1).collect::<Vec<_>>())).collect();
                out.push_str(&format!("  D_{i}: {}\n", row.join(" ")));
            }
        }
        None => out.push_str(&format!(
            "  not found; best score {} ({:.1}% of the pairs from the orbit representatives still wrong); a search that stops proves nothing\n",
            o.best,
            100.0 * o.pairs_wrong_share
        )),
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paley 29 written in the model: σ = multiplication by 16 (order 7), ∞ = 0,
    /// orbit i = the coset 2^r_i <16>, squares first.
    fn paley29() -> Model {
        let pw = |e: u64| -> u64 { (0..e).fold(1u64, |x, _| x * 2 % 29) };
        let is_sq = |x: u64| (1..29u64).any(|y| y * y % 29 == x);
        let reps = [0u64, 2, 1, 3];
        let mut d = vec![vec![0u32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for c in 0..7u64 {
                    let x = pw(reps[i]);
                    let y = pw(reps[j] + 4 * c);
                    if x != y && is_sq((y + 29 - x) % 29) {
                        d[i][j] |= 1 << c;
                    }
                }
            }
        }
        Model { p: 7, m: 4, k: 14, lambda: 6, mu: 7, d }
    }

    #[test]
    fn paley_29_scores_zero() {
        let md = paley29();
        assert_eq!(md.cost(), 0);
    }
}
