//! Evolving a formula: symbolic regression by a genetic search. Formulas
//! are small trees over ln n, ln ln n, constants, +, * and powers; each one
//! gets the best multiplying constant in closed form, and its fitness is the
//! training error on ln p plus a price per node, so a longer formula has to
//! earn its length. The search is seeded, so the same settings always evolve
//! the same formulas.

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Ln,
    LnLn,
    Const(f64),
    Add(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    /// A power with a constant exponent.
    Pow(Box<Node>, f64),
}

impl Node {
    pub fn eval(&self, n: f64) -> f64 {
        match self {
            Node::Ln => n.ln(),
            Node::LnLn => n.ln().ln(),
            Node::Const(c) => *c,
            Node::Add(a, b) => a.eval(n) + b.eval(n),
            Node::Mul(a, b) => a.eval(n) * b.eval(n),
            Node::Pow(a, e) => a.eval(n).powf(*e),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Node::Ln | Node::LnLn | Node::Const(_) => 1,
            Node::Add(a, b) | Node::Mul(a, b) => 1 + a.size() + b.size(),
            Node::Pow(a, _) => 1 + a.size(),
        }
    }
    pub fn show(&self) -> String {
        match self {
            Node::Ln => "ln n".into(),
            Node::LnLn => "ln ln n".into(),
            Node::Const(c) => format!("{c:.3}"),
            Node::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            Node::Mul(a, b) => format!("{} * {}", a.show(), b.show()),
            Node::Pow(a, e) => format!("({})^{e:.3}", a.show()),
        }
    }
    fn nodes(&self) -> usize {
        self.size()
    }
    /// The k-th node in preorder, mutably.
    fn at(&mut self, k: usize) -> &mut Node {
        if k == 0 {
            return self;
        }
        match self {
            Node::Add(a, b) | Node::Mul(a, b) => {
                let s = a.size();
                if k <= s {
                    a.at(k - 1)
                } else {
                    b.at(k - 1 - s)
                }
            }
            Node::Pow(a, _) => a.at(k - 1),
            _ => self,
        }
    }
    fn get(&self, k: usize) -> &Node {
        if k == 0 {
            return self;
        }
        match self {
            Node::Add(a, b) | Node::Mul(a, b) => {
                let s = a.size();
                if k <= s {
                    a.get(k - 1)
                } else {
                    b.get(k - 1 - s)
                }
            }
            Node::Pow(a, _) => a.get(k - 1),
            _ => self,
        }
    }
}

/// xorshift64*: small, fast and reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn random_tree(r: &mut Rng, depth: usize) -> Node {
    if depth == 0 || r.below(3) == 0 {
        return match r.below(3) {
            0 => Node::Ln,
            1 => Node::LnLn,
            _ => Node::Const((r.unit() * 4.0 * 100.0).round() / 100.0),
        };
    }
    match r.below(3) {
        0 => Node::Add(Box::new(random_tree(r, depth - 1)), Box::new(random_tree(r, depth - 1))),
        1 => Node::Mul(Box::new(random_tree(r, depth - 1)), Box::new(random_tree(r, depth - 1))),
        _ => Node::Pow(Box::new(random_tree(r, depth - 1)), (0.5 + r.unit() * 3.0 * 100.0).round() / 100.0),
    }
}

/// The best multiplying constant k for p = k g(n), and the error in ln p.
pub fn scale_and_error(g: &Node, data: &[(f64, f64)]) -> Option<(f64, f64)> {
    let mut logs = Vec::with_capacity(data.len());
    for &(n, p) in data {
        let v = g.eval(n);
        if !(v.is_finite() && v > 0.0) {
            return None;
        }
        logs.push(p.ln() - v.ln());
    }
    let lk = logs.iter().sum::<f64>() / logs.len() as f64;
    let rmse = (logs.iter().map(|d| (d - lk).powi(2)).sum::<f64>() / logs.len() as f64).sqrt();
    Some((lk.exp(), rmse))
}

pub struct Settings {
    /// Fitness by forward prediction: fit on the smaller two thirds of the
    /// training cases, score on the largest third.
    pub forward: bool,
    pub population: usize,
    pub generations: usize,
    pub seed: u64,
    pub price_per_node: f64,
}

pub struct Found {
    pub tree: Node,
    pub k: f64,
    pub train: f64,
    pub fitness: f64,
}

fn fitness(t: &Node, data: &[(f64, f64)], price: f64, forward: bool) -> Option<(f64, f64, f64)> {
    let (k, e) = scale_and_error(t, data)?;
    if !forward {
        return Some((k, e, e + price * t.size() as f64));
    }
    // how well does it predict ahead? constant from the first two thirds,
    // error on the last third (the largest n)
    let cut = data.len() * 2 / 3;
    let (k_early, _) = scale_and_error(t, &data[..cut])?;
    let ahead = (data[cut..].iter().map(|&(n, p)| (k_early * t.eval(n)).ln() - p.ln()).map(|d| d * d).sum::<f64>() / (data.len() - cut) as f64).sqrt();
    Some((k, e, ahead + price * t.size() as f64))
}

/// Evolve formulas for the training data; the best few, best fitness first.
pub fn evolve(train: &[(f64, f64)], s: &Settings) -> Vec<Found> {
    let mut r = Rng(s.seed.max(1));
    // seed the population with the simple families too, so evolution starts
    // from what the family fit already knows
    let mut pop: Vec<Node> = vec![Node::Pow(Box::new(Node::Ln), 2.0), Node::Mul(Box::new(Node::Pow(Box::new(Node::Ln), 2.0)), Box::new(Node::LnLn)), Node::Pow(Box::new(Node::Ln), 2.8)];
    while pop.len() < s.population {
        pop.push(random_tree(&mut r, 3));
    }
    let score = |t: &Node| fitness(t, train, s.price_per_node, s.forward).map_or(f64::INFINITY, |f| f.2);
    for _ in 0..s.generations {
        let mut scored: Vec<(f64, Node)> = pop.drain(..).map(|t| (score(&t), t)).collect();
        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let elite = s.population / 10;
        let mut next: Vec<Node> = scored.iter().take(elite).map(|(_, t)| t.clone()).collect();
        let pick = |r: &mut Rng| -> Node {
            // tournament of three
            let mut best: Option<&(f64, Node)> = None;
            for _ in 0..3 {
                let c = &scored[r.below(scored.len())];
                if best.is_none_or(|b| c.0 < b.0) {
                    best = Some(c);
                }
            }
            best.expect("population").1.clone()
        };
        while next.len() < s.population {
            let mut child = pick(&mut r);
            match r.below(4) {
                0 => {
                    // crossover: a subtree of another parent replaces one of ours
                    let donor = pick(&mut r);
                    let piece = donor.get(r.below(donor.nodes())).clone();
                    let k = r.below(child.nodes());
                    *child.at(k) = piece;
                }
                1 => {
                    let k = r.below(child.nodes());
                    *child.at(k) = random_tree(&mut r, 2);
                }
                _ => {
                    // nudge a constant or an exponent
                    let k = r.below(child.nodes());
                    let factor = 1.0 + (r.unit() - 0.5) * 0.2;
                    match child.at(k) {
                        Node::Const(c) => *c *= factor,
                        Node::Pow(_, e) => *e *= factor,
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
    let mut out: Vec<Found> = pop
        .into_iter()
        .filter_map(|t| fitness(&t, train, s.price_per_node, s.forward).map(|(k, e, f)| Found { tree: t, k, train: e, fitness: f }))
        .collect();
    out.sort_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(std::cmp::Ordering::Equal));
    out.dedup_by(|a, b| a.tree.show() == b.tree.show());
    out.truncate(5);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evolves_reproducibly() {
        let data: Vec<(f64, f64)> = (3..20).map(|k| {
            let n = 10f64.powi(k);
            (n, 2.0 * n.ln().powf(2.5))
        }).collect();
        let s = Settings { forward: false, population: 60, generations: 30, seed: 7, price_per_node: 0.001 };
        let a = evolve(&data, &s);
        let b = evolve(&data, &s);
        assert_eq!(a[0].tree, b[0].tree);
        assert!(a[0].train < 0.05, "{}", a[0].train);
    }
}
