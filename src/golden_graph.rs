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
    /// asked which orders v have such graphs at all (v is then 0)
    pub list: bool,
    /// asked to invent a function (a formula) for it: golden_invent
    pub invent: bool,
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
        return Some(GraphQ { v: nums[0], k: Some(nums[1]), lambda: nums[2], mu: nums[3], make, list: false, invent: false });
    }
    // Conway's way: "make a 944-point closed network like Conway's", "tee 944 pisteinen suljettu verkko conwayn tapaan"
    if t.contains("conway") {
        let words: Vec<String> = t.replace('-', " ").split_whitespace().map(|w| w.trim_matches(|c: char| c == ',' || c == '\'').to_string()).collect();
        const OK: &[&str] = &[
            "make", "build", "construct", "find", "a", "an", "the", "graph", "network", "closed", "vertex", "vertices", "point", "points", "on", "with", "like", "conway", "conways", "conway's", "s", "style", "in", "way",
            "tee", "rakenna", "pisteinen", "pisteen", "suljettu", "suljettua", "verkko", "verkkoa", "conwayn", "tapaan", "is", "there", "onko", "olemassa", "exist", "does",
            "keksi", "funktio", "verkolle", "suljetulle", "invent", "function", "formula", "for", "kaava",
        ];
        let nums: Vec<u64> = words.iter().filter_map(|w| w.parse().ok()).collect();
        // which orders: "minkälaisia verkkoja on olemassa conwayn tapaan", "which graphs exist in conway's way"
        const LIST: &[&str] = &["minkälaisia", "mitkä", "mitä", "millaisia", "which", "what", "list", "all"];
        const LIST_OK: &[&str] = &[
            "minkälaisia", "mitkä", "mitä", "millaisia", "verkkoja", "verkot", "on", "ovat", "olemassa", "conwayn", "tapaan", "which", "what", "list", "all", "graphs", "networks", "exist", "are", "there", "in", "conway's", "conways", "way", "style", "like", "conway", "s", "the",
        ];
        if nums.is_empty() && words.iter().any(|w| LIST.contains(&w.as_str())) && words.iter().all(|w| LIST_OK.contains(&w.as_str())) {
            return Some(GraphQ { v: 0, k: None, lambda: 1, mu: 2, make: false, list: true, invent: false });
        }
        if nums.len() == 1 && words.iter().all(|w| w.parse::<u64>().is_ok() || OK.contains(&w.as_str())) {
            let make = !["is there", "onko", "does"].iter().any(|w| t.starts_with(w));
            let invent = words.iter().any(|w| ["keksi", "invent", "funktio", "function", "formula", "kaava"].contains(&w.as_str()));
            return Some(GraphQ { v: nums[0], k: None, lambda: 1, mu: 2, make, list: false, invent });
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
    Some(GraphQ { v, k: None, lambda, mu, make, list: false, invent: false })
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

/// Golden invents a function for the graph, from nothing but the forced frame.
fn invent_answer(kind: Kind3, g: &GraphQ) -> Option<(String, Vec<String>, Vec<String>)> {
    if !matches!(kind, Kind3::Logical) || g.lambda != 1 || g.mu != 2 {
        return None;
    }
    let ks = degrees(g.v, 1, 2);
    let mut lines = Vec::new();
    let Some(&k) = ks.first() else {
        lines.push(format!("no whole k solves k(k - 2) = 2({} - k): there is no graph to invent a function for", g.v - 1));
        return Some(("no such graph exists".into(), lines, vec![]));
    };
    let secs = if cfg!(test) { 2.0 } else { std::env::var("GOLDEN_INVENT_SECONDS").ok().and_then(|x| x.parse().ok()).unwrap_or(60.0) };
    let q = k / 2;
    lines.push(format!("the frame the conditions force (nothing else is given): x, its {k} neighbours (i, s) with triangle i among {q} names and corner s in {{0,1}}, and the far vertices, the pairs {{(i1,s1),(j1,t1)}}"));
    lines.push("Golden may invent new symbols without limit (one-, two- and three-place, meanings as tables it makes up, new ones from scratch or grown from old ones) and use the old + - × when it wants; it invents an expression E, the names of the triangles and corners, and the relation ≈ that decides adjacency: f ~ g when E(f, g) ≈ E(g, f)".into());
    // the 243-graph's formula is given, not learned again: its parts are old tools in Golden's language (x⁻¹, x², x³, χ)
    let seeds: Vec<(crate::golden_symbols::Sx, crate::golden_symbols::Lang, crate::golden_invent::Naming)> = Vec::new();
    let rest = secs;
    let Some(inv) = crate::golden_symbols::invent_seeded(k as usize, rest, 0x2545_f491_4f6c_dd1d, &seeds) else {
        return None;
    };
    lines.push(format!("{} languages and expressions tried in {:.0} s on {} islands", inv.tried, inv.seconds, inv.islands));
    lines.push(format!("E = {}", inv.expr.text()));
    lines.extend(crate::golden_symbols::glossary(&inv));
    if inv.verified {
        lines.push(format!("checked from scratch: the graph it gives is srg({},{k},1,2) on every pair", g.v));
        return Some((format!("invented: f ~ g when E(f,g) ≈ E(g,f), E = {}", inv.expr.text()), lines, vec![]));
    }
    lines.push(format!("its graph meets {:.1}% of the pair conditions; not found", 100.0 * inv.pairs_met_share));
    UNDECIDED.with(|u| *u.borrow_mut() = lines.clone());
    None
}

/// How long the logical answer may search the forced structure's last choice (GOLDEN_FORCED_SECONDS overrides).
pub const FORCED_SECONDS: f64 = 20.0;

thread_local! {
    /// The working of the last undecided forced search, for the "no answer" text.
    pub static UNDECIDED: std::cell::RefCell<Vec<String>> = std::cell::RefCell::new(Vec::new());
}

pub enum Forced {
    Built(Vec<Vec<bool>>),
    None,
    Undecided,
}

/// Conway's kind (λ = 1, μ = 2) with degree k, assuming nothing else: the
/// structure every vertex x forces, then the one choice left, searched
/// exactly for at most `seconds`.
///
/// x's k neighbours pair into k/2 triangles through x (λ = 1). Each vertex at
/// distance 2 has exactly two neighbours among them (μ = 2), not adjacent to
/// each other (an edge in two triangles would break λ = 1), and each of the
/// k(k-2)/2 non-adjacent pairs of neighbours has exactly one common neighbour
/// besides x: so the far vertices ARE those pairs. What is left: which far
/// vertices are adjacent. For a far vertex f = {a, b} and a neighbour u of
/// x, f has exactly t(f, u) far neighbours that contain u, t = 1 when u is in
/// f or is the partner of an end of f, else 2; and two far vertices f, g
/// have 1 - |f ∩ g| far common neighbours when adjacent, 2 - |f ∩ g| when not.
pub fn forced(v: u64, k: u64, seconds: f64) -> (Forced, Vec<String>) {
    let mut lines = Vec::new();
    let k = k as usize;
    let far: Vec<(usize, usize)> = (0..k).flat_map(|a| (a + 1..k).map(move |b| (a, b))).filter(|&(a, b)| b != (a ^ 1)).collect();
    let n = far.len();
    lines.push(format!("fix a vertex x: its {k} neighbours pair into {} triangles through x (each neighbour has exactly λ = 1 common neighbour with x)", k / 2));
    lines.push(format!("a vertex at distance 2 has exactly μ = 2 neighbours among them, and they are not adjacent (an edge in two triangles would break λ = 1)"));
    lines.push(format!("the {k}·{}/2 = {n} non-adjacent pairs of x's neighbours each need exactly one common neighbour besides x, so the {} vertices at distance 2 are exactly these pairs", k - 2, v as usize - 1 - k));
    if n != v as usize - 1 - k {
        lines.push("the counts do not match".into());
        return (Forced::None, lines);
    }
    lines.push(format!("left to choose: the edges among the {n} far vertices, each far vertex having {} of them, with the star and common-neighbour counts these conditions fix", k - 2));
    if n > 128 * 8 {
        return (Forced::Undecided, lines);
    }
    let words = n.div_ceil(64);
    let has = |f: usize, u: usize| far[f].0 == u || far[f].1 == u;
    // t(f, u)
    let target: Vec<Vec<u8>> = (0..n)
        .map(|f| (0..k).map(|u| if has(f, u) || has(f, u ^ 1) { 1 } else { 2 }).collect())
        .collect();
    let shared: Vec<Vec<u8>> = (0..n).map(|f| (0..n).map(|g| (has(g, far[f].0) as u8) + (has(g, far[f].1) as u8)).collect()).collect();
    let need = k - 2;
    let t0 = std::time::Instant::now();
    let mut adj = vec![vec![0u64; words]; n];
    let mut deg = vec![0usize; n];
    let mut star = vec![vec![0u8; k]; n];
    let mut best_rows = 0usize;
    let mut nodes = 0u64;
    let mut timed_out = false;

    // depth-first over the pairs (f, g), f < g, row by row
    fn bit(r: &[u64], i: usize) -> bool {
        r[i / 64] >> (i % 64) & 1 == 1
    }
    #[allow(clippy::too_many_arguments)]
    fn go(
        f: usize, g: usize, n: usize, k: usize, need: usize, far: &[(usize, usize)], target: &[Vec<u8>], shared: &[Vec<u8>], adj: &mut Vec<Vec<u64>>, deg: &mut Vec<usize>, star: &mut Vec<Vec<u8>>,
        best_rows: &mut usize, nodes: &mut u64, t0: &std::time::Instant, seconds: f64, timed_out: &mut bool,
    ) -> bool {
        *nodes += 1;
        if *nodes % 4096 == 0 {
            let el = t0.elapsed().as_secs_f64();
            if el > seconds {
                *timed_out = true;
                return false;
            }
            // progress once a minute on long searches
            static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let minute = (el / 60.0) as u64;
            if seconds > 120.0 && minute > LAST.load(std::sync::atomic::Ordering::Relaxed) {
                LAST.store(minute, std::sync::atomic::Ordering::Relaxed);
                eprintln!("[{minute} min] {nodes} steps, best {best_rows} of {n} far vertices fixed, now at row {f}", nodes = *nodes, best_rows = *best_rows);
            }
        }
        if f == n {
            return true;
        }
        if g == n {
            // row f is complete: its degree, its stars, and every earlier pair's common count
            if deg[f] != need || (0..k).any(|u| star[f][u] != target[f][u]) {
                return false;
            }
            for h in 0..f {
                let common: u32 = adj[f].iter().zip(&adj[h]).map(|(a, b)| (a & b).count_ones()).sum();
                let want = if bit(&adj[f], h) { 1 } else { 2 } as i32 - shared[f][h] as i32;
                if common as i32 != want {
                    return false;
                }
            }
            if f + 1 > *best_rows {
                *best_rows = f + 1;
            }
            return go(f + 1, f + 2, n, k, need, far, target, shared, adj, deg, star, best_rows, nodes, t0, seconds, timed_out);
        }
        // f and g adjacent? Not when they share an end (λ = 1 would break: the shared end is a common neighbour already... allowed only if no other)
        let (a, b) = far[g];
        let (c, d) = far[f];
        let can = deg[f] < need && deg[g] < need && star[f][a] < target[f][a] && star[f][b] < target[f][b] && star[g][c] < target[g][c] && star[g][d] < target[g][d] && shared[f][g] <= 1;
        if can {
            adj[f][g / 64] |= 1 << (g % 64);
            adj[g][f / 64] |= 1 << (f % 64);
            deg[f] += 1;
            deg[g] += 1;
            star[f][a] += 1;
            star[f][b] += 1;
            star[g][c] += 1;
            star[g][d] += 1;
            if go(f, g + 1, n, k, need, far, target, shared, adj, deg, star, best_rows, nodes, t0, seconds, timed_out) {
                return true;
            }
            adj[f][g / 64] &= !(1 << (g % 64));
            adj[g][f / 64] &= !(1 << (f % 64));
            deg[f] -= 1;
            deg[g] -= 1;
            star[f][a] -= 1;
            star[f][b] -= 1;
            star[g][c] -= 1;
            star[g][d] -= 1;
        }
        if *timed_out {
            return false;
        }
        // not adjacent: row f must still be able to reach its degree
        if deg[f] + (n - g - 1) < need {
            return false;
        }
        go(f, g + 1, n, k, need, far, target, shared, adj, deg, star, best_rows, nodes, t0, seconds, timed_out)
    }

    let found = go(0, 1, n, k, need, &far, &target, &shared, &mut adj, &mut deg, &mut star, &mut best_rows, &mut nodes, &t0, seconds, &mut timed_out);
    if found {
        // the whole graph: x = 0, neighbours 1..=k, far vertices after
        let vv = 1 + k + n;
        let mut m = vec![vec![false; vv]; vv];
        let mut link = |p: usize, q: usize| {
            m[p][q] = true;
            m[q][p] = true;
        };
        for u in 0..k {
            link(0, 1 + u);
        }
        for u in (0..k).step_by(2) {
            link(1 + u, 2 + u);
        }
        for (f, &(a, b)) in far.iter().enumerate() {
            link(1 + k + f, 1 + a);
            link(1 + k + f, 1 + b);
            for g in f + 1..n {
                if bit(&adj[f], g) {
                    link(1 + k + f, 1 + k + g);
                }
            }
        }
        lines.push(format!("the exact search over that last choice found one ({nodes} steps)"));
        return (Forced::Built(m), lines);
    }
    if timed_out {
        lines.push(format!("the exact search over that last choice was stopped after {seconds:.0} s ({nodes} steps): at most {best_rows} of the {n} far vertices had all their edges fixed consistently; undecided"));
        return (Forced::Undecided, lines);
    }
    lines.push(format!("the exact search over that last choice ran to its end ({nodes} steps) and found no way to complete it"));
    (Forced::None, lines)
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
/// Which orders v have a graph of Conway's kind (λ = 1, μ = 2).
fn family(kind: Kind3) -> Option<(String, Vec<String>, Vec<String>)> {
    let mut lines = Vec::new();
    match kind {
        Kind3::Logical => {
            lines.push("such a graph is regular (A·A² = A²·A with μ = 2), and counting the edges around one vertex gives k(k - 2) = 2(v - 1 - k), i.e. k² = 2(v - 1)".into());
            lines.push("so k is even and v = k²/2 + 1: v = 3, 9, 19, 33, 51, 73, 99, 129, 163, 201, ... are the only orders the count allows".into());
            let triangle = vec![vec![false, true, true], vec![true, false, true], vec![true, true, false]];
            let tri = check(&triangle, 2, 1, 2).ok()?;
            lines.push(format!("built and checked: v = 3, the triangle (every pair adjacent with 1 common neighbour; {tri} pairs)"));
            let (name, adj) = construct(9, 4, 1, 2)?;
            let n9 = check(&adj, 4, 1, 2).ok()?;
            lines.push(format!("built and checked: v = 9, {name} ({n9} pairs)"));
            lines.push("the count alone does not decide the other orders".into());
            Some(("only v = k²/2 + 1 with k even can have one; built: 3 and 9".into(), lines, vec![]))
        }
        Kind3::Theoretical => {
            let mut admissible = Vec::new();
            for k in (2..=2_000u64).step_by(2) {
                let v = k * k / 2 + 1;
                if integral(v, k, 1, 2).0 {
                    admissible.push((v, k));
                }
            }
            lines.push(format!("of the orders v = k²/2 + 1 with even k up to 2000, the eigenvalue multiplicities are whole only for {}", admissible.iter().map(|(v, k)| format!("v = {v} (k = {k})")).collect::<Vec<_>>().join(", ")));
            lines.push("and none beyond: √(4k - 7) must divide 63, so k is one of 2, 4, 14, 22, 112, 994".into());
            for (v, _) in &admissible {
                match conway_state(*v) {
                    Some((state, why)) => lines.push(format!("v = {v}: {state} ({why})")),
                    None if *v == 3 => lines.push("v = 3: yes (the triangle; no two vertices are non-adjacent)".into()),
                    None => {}
                }
            }
            Some((
                "v = 3, 9, 99, 243, 6273, 494019 only: 3, 9 and 243 exist; 99, 6273 and 494019 are open".into(),
                lines,
                vec![
                    "the eigenvalue multiplicities of a strongly regular graph are whole numbers (Bose & Mesner 1959; Brouwer & Haemers, Spectra of Graphs, ch. 9)".into(),
                    "Conway, Five $1,000 Problems (2014): the orders 9, 99, 243, 6273, 494019; Berlekamp, van Lint & Seidel (1973) for 243".into(),
                ],
            ))
        }
        Kind3::Abstract => Some(("probably only a few: 9 and 243 are known, the rest unknown".into(), vec!["the known ones are rare and come from special structures (a grid, the ternary Golay code)".into()], vec![])),
    }
}

pub fn answer(kind: Kind3, g: &GraphQ) -> Option<(String, Vec<String>, Vec<String>)> {
    if g.list {
        return family(kind);
    }
    if g.invent {
        return invent_answer(kind, g);
    }
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
            // Conway's kind (λ = 1, μ = 2): no relatives assumed, only what the conditions force
            if lambda == 1 && mu == 2 && ks.len() == 1 && v <= 1000 {
                let k = ks[0];
                let secs = if cfg!(test) { 1.0 } else { std::env::var("GOLDEN_FORCED_SECONDS").ok().and_then(|x| x.parse().ok()).unwrap_or(FORCED_SECONDS) };
                let (verdict, more) = forced(v, k, secs);
                lines.extend(more);
                return match verdict {
                    Forced::Built(adj) => {
                        let pairs = check(&adj, k, 1, 2).ok()?;
                        lines.push(format!("checked: every vertex has degree {k}; every one of the {pairs} pairs has 1 common neighbour when adjacent and 2 when not"));
                        if make && v <= 30 {
                            for (i, row) in adj.iter().enumerate() {
                                let ns: Vec<String> = (0..adj.len()).filter(|&j| row[j]).map(|j| j.to_string()).collect();
                                lines.push(format!("vertex {i}: {}", ns.join(" ")));
                            }
                        }
                        Some((if make { format!("constructed: srg({v},{k},1,2)") } else { "yes".into() }, lines, vec![]))
                    }
                    Forced::None => {
                        lines.push(format!("so no graph on {v} vertices has these properties"));
                        Some((no(), lines, vec![]))
                    }
                    Forced::Undecided => None.or_else(|| {
                        UNDECIDED.with(|u| *u.borrow_mut() = lines.clone());
                        None
                    }),
                };
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
        assert!(read("minkälaisia verkkoja on olemassa conwayn tapaan").unwrap().list);
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
