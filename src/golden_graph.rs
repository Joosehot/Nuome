//! Golden Answer's graphs: "make a graph on v vertices in which every two
//! adjacent vertices have exactly λ common neighbours and every two
//! non-adjacent vertices exactly μ" (Conway's 99-graph is v = 99, λ = 1,
//! μ = 2), and srg(v, k, λ, μ).
//!
//! - logical: from the question itself. For μ >= 2 the graph must be
//!   regular (the matrix identity below), so its degree k satisfies the
//!   count k(k - 1 - λ) = μ(v - 1 - k); when no whole k does, no graph
//!   exists. When one does and a family of known constructions has these
//!   parameters, the graph is built and every pair checked.
//! - theoretical: the integrality of the eigenvalue multiplicities (a
//!   theorem of linear algebra), and the known state of Conway's family.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphQ {
    pub v: u64,
    pub k: Option<u64>,
    pub lambda: u64,
    pub mu: u64,
    /// asked to build it, not only whether it exists
    pub make: bool,
}

fn word_number(w: &str) -> Option<u64> {
    let words = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"];
    words.iter().position(|x| *x == w).map(|i| i as u64).or_else(|| w.parse().ok())
}

fn normal(q: &str) -> String {
    let t = q.to_lowercase().replace(['$', '?', '!'], " ").replace(" .", " ");
    let t = t.trim().trim_end_matches('.').to_string();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip<'a>(t: &'a str, options: &[&str]) -> Option<&'a str> {
    options.iter().find_map(|o| t.strip_prefix(o))
}

/// Read the whole question, or None.
pub fn read(q: &str) -> Option<GraphQ> {
    let t = normal(q);
    // srg(v,k,l,m) with only the words around it that ask for it
    if let Some(i) = t.find("srg(") {
        let close = t[i..].find(')')? + i;
        let nums: Vec<u64> = t[i + 4..close].split(',').map(|x| x.trim().parse().ok()).collect::<Option<_>>()?;
        if nums.len() != 4 {
            return None;
        }
        let rest = format!("{} {}", &t[..i], &t[close + 1..]);
        const OK: &[&str] = &["is", "there", "a", "an", "strongly", "regular", "graph", "make", "construct", "build", "find", "does", "exist", "with", "parameters", "give"];
        if !rest.split_whitespace().all(|w| OK.contains(&w)) {
            return None;
        }
        let make = ["make", "construct", "build", "find", "give"].iter().any(|w| rest.split_whitespace().any(|x| x == *w));
        return Some(GraphQ { v: nums[0], k: Some(nums[1]), lambda: nums[2], mu: nums[3], make });
    }
    // Conway's way: "make a 944-point closed network like Conway's", "tee 944 pisteinen suljettu verkko conwayn tapaan"
    if t.contains("conway") {
        let words: Vec<String> = t.replace('-', " ").split_whitespace().map(|w| w.trim_matches(|c: char| c == ',' || c == '\'').to_string()).collect();
        const OK: &[&str] = &[
            "make", "build", "construct", "find", "a", "an", "the", "graph", "network", "closed", "vertex", "vertices", "point", "points", "on", "with", "like", "conway", "conways", "conway's", "s", "style", "in", "way",
            "tee", "rakenna", "pisteinen", "pisteen", "suljettu", "suljettua", "verkko", "verkkoa", "conwayn", "tapaan", "is", "there", "onko", "olemassa", "exist", "does",
        ];
        let nums: Vec<u64> = words.iter().filter_map(|w| w.parse().ok()).collect();
        if nums.len() == 1 && words.iter().all(|w| w.parse::<u64>().is_ok() || OK.contains(&w.as_str())) {
            let make = !["is there", "onko", "does"].iter().any(|w| t.starts_with(w));
            return Some(GraphQ { v: nums[0], k: None, lambda: 1, mu: 2, make });
        }
        return None;
    }
    // the sentence: (make|is there) a graph on V vertices in which every two adjacent vertices have exactly A common neighbour(s) and every two non-adjacent vertices (have) exactly B (common neighbours)
    let (make, t2) = match strip(&t, &["make ", "build ", "construct ", "find ", "give "]) {
        Some(r) => (true, r),
        None => (false, strip(&t, &["is there ", "does there exist "])?),
    };
    let t2 = strip(t2, &["a graph on ", "a graph with "])?;
    let (v, t2) = t2.split_once(' ')?;
    let v = word_number(v)?;
    let t2 = strip(t2, &["vertices "])?;
    let t2 = strip(t2, &["in which ", "where ", "such that "])?;
    let t2 = strip(t2, &["every two adjacent vertices have exactly ", "any two adjacent vertices have exactly "])?;
    let (a, t2) = t2.split_once(' ')?;
    let lambda = word_number(a)?;
    let t2 = strip(t2, &["common neighbours", "common neighbour", "common neighbors", "common neighbor"])?;
    let t2 = strip(t2, &[" and ", ", and ", ", "])?;
    let t2 = strip(t2, &["every two non-adjacent vertices have exactly ", "every two non-adjacent vertices exactly ", "any two non-adjacent vertices have exactly ", "any two non-adjacent vertices exactly "])?;
    let (b, tail) = t2.split_once(' ').unwrap_or((t2, ""));
    let mu = word_number(b)?;
    if !["", "common neighbours", "common neighbors"].contains(&tail.trim()) {
        return None;
    }
    Some(GraphQ { v, k: None, lambda, mu, make })
}

/// The whole degrees k with k(k - 1 - λ) = μ(v - 1 - k), 0 < k <= v - 1.
fn degrees(v: u64, lambda: u64, mu: u64) -> Vec<u64> {
    // k = v - 1 is the complete graph, which fits only when λ = v - 2
    (1..v).filter(|&k| k as i128 * (k as i128 - 1 - lambda as i128) == mu as i128 * (v as i128 - 1 - k as i128)).collect()
}

fn isqrt(n: u128) -> u128 {
    let mut x = (n as f64).sqrt() as u128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// The eigenvalue multiplicities f, g are whole numbers (as twice-values, exact).
fn integral(v: u64, k: u64, lambda: u64, mu: u64) -> (bool, String) {
    let (v, k, l, m) = (v as i128, k as i128, lambda as i128, mu as i128);
    let disc = (l - m) * (l - m) + 4 * (k - m);
    if disc < 0 {
        return (false, "the eigenvalue discriminant is negative".into());
    }
    let num = 2 * k + (v - 1) * (l - m);
    let r = isqrt(disc as u128) as i128;
    if r * r == disc {
        // f = ((v-1) - num/r)/2 must be a whole number >= 0
        if num % r != 0 || ((v - 1) - num / r) % 2 != 0 || (v - 1) - num / r < 0 || (v - 1) + num / r < 0 {
            return (false, format!("the multiplicities ½((v-1) ∓ (2k + (v-1)(λ-μ))/√Δ) with Δ = {disc} are not whole numbers"));
        }
        let f = ((v - 1) - num / r) / 2;
        return (true, format!("the eigenvalues ({} ± {r})/2 have whole multiplicities {f} and {}", l - m, v - 1 - f));
    }
    // √Δ irrational: only the conference case num = 0 works, with f = g = (v-1)/2
    if num == 0 && (v - 1) % 2 == 0 {
        (true, format!("Δ = {disc} is not a square, and 2k + (v-1)(λ-μ) = 0 (the conference case), so f = g = {}", (v - 1) / 2))
    } else {
        (false, format!("Δ = {disc} is not a square, so the multiplicities are whole only when 2k + (v-1)(λ-μ) = 0, but it is {num}"))
    }
}

/// A graph from a known family with these parameters, if one has them.
fn construct(v: u64, k: u64, lambda: u64, mu: u64) -> Option<(String, Vec<Vec<bool>>)> {
    let fam = |name: String, adj: Vec<Vec<bool>>| Some((name, adj));
    let cands: Vec<(String, Vec<Vec<bool>>)> = {
        let mut c = Vec::new();
        // the n x n rook's graph: srg(n², 2(n-1), n-2, 2)
        for n in 2..=12u64 {
            if n * n == v {
                let adj = (0..v).map(|a| (0..v).map(|b| a != b && (a / n == b / n || a % n == b % n)).collect()).collect();
                c.push((format!("the {n} x {n} rook's graph (cells adjacent in one row or one column)"), adj));
            }
        }
        // the triangular graph T(n): pairs of {1..n} sharing one element, srg(n(n-1)/2, 2(n-2), n-2, 4)
        for n in 4..=16u64 {
            if n * (n - 1) / 2 == v {
                let pairs: Vec<(u64, u64)> = (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))).collect();
                let adj = pairs.iter().map(|p| pairs.iter().map(|q| p != q && (p.0 == q.0 || p.0 == q.1 || p.1 == q.0 || p.1 == q.1)).collect()).collect();
                c.push((format!("the triangular graph T({n}) (pairs from {n} things, adjacent when they share one)"), adj));
            }
        }
        // the Paley graph on a prime q ≡ 1 (mod 4): adjacent when the difference is a square
        if v % 4 == 1 && v < 400 && (2..v).take_while(|d| d * d <= v).all(|d| v % d != 0) {
            let squares: Vec<bool> = {
                let mut s = vec![false; v as usize];
                for x in 1..v {
                    s[(x * x % v) as usize] = true;
                }
                s
            };
            let adj = (0..v).map(|a| (0..v).map(|b| a != b && squares[((a + v - b) % v) as usize]).collect()).collect();
            c.push((format!("the Paley graph on {v} (adjacent when the difference is a nonzero square mod {v})"), adj));
        }
        c
    };
    for (name, adj) in cands {
        if check(&adj, k, lambda, mu).is_ok() {
            return fam(name, adj);
        }
        // its complement
        let comp: Vec<Vec<bool>> = (0..adj.len()).map(|a| (0..adj.len()).map(|b| a != b && !adj[a][b]).collect()).collect();
        if check(&comp, k, lambda, mu).is_ok() {
            return fam(format!("the complement of {name}"), comp);
        }
    }
    None
}

/// Every vertex has degree k, every adjacent pair λ common neighbours,
/// every non-adjacent pair μ: checked over every pair. Err names the first failure.
pub fn check(adj: &[Vec<bool>], k: u64, lambda: u64, mu: u64) -> Result<usize, String> {
    let n = adj.len();
    for i in 0..n {
        let d = adj[i].iter().filter(|&&e| e).count() as u64;
        if d != k {
            return Err(format!("vertex {i} has degree {d}"));
        }
    }
    let mut pairs = 0;
    for i in 0..n {
        for j in i + 1..n {
            let c = (0..n).filter(|&x| adj[i][x] && adj[j][x]).count() as u64;
            let want = if adj[i][j] { lambda } else { mu };
            if c != want || adj[i][j] != adj[j][i] {
                return Err(format!("vertices {i} and {j} have {c} common neighbours"));
            }
            pairs += 1;
        }
    }
    Ok(pairs)
}

/// Conway's family λ = 1, μ = 2: the only v the integrality condition allows, and what is known.
fn conway_state(v: u64) -> Option<(&'static str, &'static str)> {
    match v {
        9 => Some(("yes", "the 3 x 3 rook's graph (Paley graph on 9) is one")),
        99 => Some(("open", "Conway's 99-graph problem ($1000 prize, 2014): unsolved")),
        243 => Some(("yes", "the Berlekamp-van Lint-Seidel graph (1973), from the ternary Golay code")),
        6273 | 494019 => Some(("open", "one of Conway's five admissible orders (9, 99, 243, 6273, 494019); existence unknown")),
        _ => None,
    }
}

pub enum Kind3 {
    Logical,
    Theoretical,
    Abstract,
}

/// The answer: short, lines, sources (sources only for theoretical).
pub fn answer(kind: Kind3, g: &GraphQ) -> Option<(String, Vec<String>, Vec<String>)> {
    let GraphQ { v, lambda, mu, make, .. } = g.clone();
    let mut lines = Vec::new();
    let ks: Vec<u64> = match g.k {
        Some(k) => vec![k],
        None => degrees(v, lambda, mu),
    };
    let no = || if make { "no such graph exists".to_string() } else { "no".to_string() };
    match kind {
        Kind3::Logical => {
            if g.k.is_none() {
                if mu < 2 {
                    return None; // regularity is not forced; this derivation does not apply
                }
                lines.push("let A be the adjacency matrix and D the diagonal of the degrees; the question says A² = D + λA + μ(J - I - A)".into());
                lines.push("then A·A² = A²·A gives, entry by entry, (d_x - d_y)(μ - A_xy) = 0 for every two vertices x, y".into());
                lines.push(format!("with μ = {mu} >= 2 neither μ - 0 nor μ - 1 is 0, so d_x = d_y for every pair: the graph is regular, of some degree k"));
                lines.push("fix a vertex x: each of its k neighbours has λ common neighbours with x, so k - 1 - λ edges leave it away from x and its neighbours".into());
                lines.push(format!("each of the v - 1 - k other vertices has μ neighbours among x's neighbours, so k(k - 1 - λ) = μ(v - 1 - k), i.e. k(k - {}) = {mu}({} - k)", 1 + lambda, v - 1));
                if ks.is_empty() {
                    lines.push(format!("no whole k from 1 to {} solves it (each one tried)", v.saturating_sub(2)));
                    lines.push(format!("so no graph on {v} vertices has these properties"));
                    return Some((no(), lines, vec![]));
                }
                lines.push(format!("the only whole solution is k = {}", ks.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(" or ")));
            } else {
                let k = ks[0];
                if k as i128 * (k as i128 - 1 - lambda as i128) != mu as i128 * (v as i128 - 1 - k as i128) {
                    lines.push(format!("fix a vertex x: counting the edges between its {k} neighbours and the {} other vertices gives k(k - 1 - λ) = {} on one side and μ(v - 1 - k) = {} on the other", v - 1 - k, k as i128 * (k as i128 - 1 - lambda as i128), mu as i128 * (v as i128 - 1 - k as i128)));
                    lines.push("they differ, so no such graph exists".into());
                    return Some((no(), lines, vec![]));
                }
            }
            if v > 400 {
                return None;
            }
            for &k in &ks {
                if let Some((name, adj)) = construct(v, k, lambda, mu) {
                    let pairs = check(&adj, k, lambda, mu).ok()?;
                    lines.push(format!("built: {name}"));
                    lines.push(format!("checked: every vertex has degree {k}; every one of the {pairs} pairs has {lambda} common neighbours when adjacent and {mu} when not"));
                    if make && v <= 30 {
                        for (i, row) in adj.iter().enumerate() {
                            let ns: Vec<String> = (0..adj.len()).filter(|&j| row[j]).map(|j| j.to_string()).collect();
                            lines.push(format!("vertex {i}: {}", ns.join(" ")));
                        }
                    }
                    return Some((if make { format!("constructed: srg({v},{k},{lambda},{mu})") } else { "yes".into() }, lines, vec![]));
                }
            }
            None
        }
        Kind3::Theoretical => {
            let src_int = "the eigenvalue multiplicities of a strongly regular graph are whole numbers (Bose & Mesner 1959; Brouwer & Haemers, Spectra of Graphs, ch. 9)".to_string();
            let src_reg = "a graph in which adjacent pairs have λ and non-adjacent pairs μ >= 2 common neighbours is regular, hence strongly regular".to_string();
            if g.k.is_none() && mu < 2 {
                return None;
            }
            if ks.is_empty() {
                lines.push(format!("strongly regular with v = {v} needs k(k - 1 - λ) = μ(v - 1 - k), and no whole k solves it"));
                return Some((no(), lines, vec![src_reg]));
            }
            for &k in &ks {
                let (ok, why) = integral(v, k, lambda, mu);
                lines.push(format!("srg({v},{k},{lambda},{mu}): {why}"));
                if !ok {
                    return Some((no(), lines, vec![src_reg, src_int]));
                }
            }
            if lambda == 1 && mu == 2 {
                if let Some((state, why)) = conway_state(v) {
                    lines.push(why.to_string());
                    let src = match state {
                        "open" => "Conway, Five $1,000 Problems (2014); Brouwer's table of strongly regular graphs",
                        _ => "Brouwer's table of strongly regular graphs (Berlekamp, van Lint & Seidel 1973 for 243)",
                    };
                    let short = if make && state == "yes" { "such a graph exists".to_string() } else { state.to_string() };
                    return Some((short, lines, vec![src_int, src.to_string()]));
                }
            }
            None
        }
        Kind3::Abstract => {
            if ks.is_empty() && (mu >= 2 || g.k.is_some()) {
                return Some(("probably none".into(), vec!["no whole degree fits the count k(k - 1 - λ) = μ(v - 1 - k)".into()], vec![]));
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_question_three_ways() {
        let g = read("make a graph on 944 vertices in which every two adjacent vertices have exactly one common neighbour and every two non-adjacent vertices have exactly two").unwrap();
        assert_eq!((g.v, g.lambda, g.mu, g.make), (944, 1, 2, true));
        let g = read("tee 944 pisteinen suljettu verkko conwayn tapaan").unwrap();
        assert_eq!((g.v, g.lambda, g.mu), (944, 1, 2));
        let g = read("is there a strongly regular graph srg(10,3,0,1)").unwrap();
        assert_eq!((g.v, g.k, g.lambda, g.mu, g.make), (10, Some(3), 0, 1, false));
        assert!(read("make a graph on 944 vertices that is planar").is_none());
        let g = read("onko 999 pisteen verkkoa olemassa conwayn tapaan").unwrap();
        assert_eq!((g.v, g.make), (999, false));
    }

    #[test]
    fn conway_944_does_not_exist_and_9_is_built() {
        let g = read("tee 944 pisteinen suljettu verkko conwayn tapaan").unwrap();
        assert_eq!(answer(Kind3::Logical, &g).unwrap().0, "no such graph exists");
        let g = read("make a graph on 9 vertices in which every two adjacent vertices have exactly one common neighbour and every two non-adjacent vertices have exactly two").unwrap();
        assert_eq!(answer(Kind3::Logical, &g).unwrap().0, "constructed: srg(9,4,1,2)");
        let g = read("is there a graph on 99 vertices in which every two adjacent vertices have exactly one common neighbour and every two non-adjacent vertices have exactly two").unwrap();
        assert!(answer(Kind3::Logical, &g).is_none());
        assert_eq!(answer(Kind3::Theoretical, &g).unwrap().0, "open");
    }

    #[test]
    fn petersen_and_infeasible() {
        let g = read("is there a strongly regular graph srg(10,3,0,1)").unwrap();
        assert_eq!(answer(Kind3::Logical, &g).unwrap().0, "yes");
        let g = read("is there a strongly regular graph srg(19,6,1,2)").unwrap();
        assert_eq!(answer(Kind3::Theoretical, &g).unwrap().0, "no");
    }
}
