//! The God-function: a wholly new way to compute something, invented by
//! Golden in symbols of its own making.
//!
//! The question (QUESTION): invent a new function G, written in symbols of
//! Golden's own making, that computes a mathematical quantity Q — of
//! Golden's choosing — in a way not built from any formula already known
//! for Q; check G against Q's true values.
//!
//! G reads n as Golden sees it: its base-q digits d0..d5 and its own last
//! two values G(n-1), G(n-2); G(n) = E(d0, ..., d5, G(n-1), G(n-2)), E an
//! expression in Golden's invented symbols (tables over the q names it makes
//! up; the old +, -, x mod q only when it wants them; golden_symbols). No
//! formula for Q is given. The quantities to choose from are the ones Golden
//! Answer already computes exactly (primality, counting primes, factoring:
//! tau, sigma, phi; Fibonacci numbers; Collatz; sums of two squares), each mod q.
//!
//! Searched on n = 1..SEARCH_N, checked on SEARCH_N+1..CHECK_N (never
//! searched); frozen in out/golden/god_function.txt.

use crate::golden_invent::{Frame, Naming};
use crate::golden_symbols::{evolve, glossary, Invented, Lang, Sx};

pub const QUESTION: &str = "Invent a new function G, written in symbols of Golden's own making, that computes a mathematical quantity Q of Golden's choosing — and computes it in a way not built from any formula already known for Q. Check G against Q's true values.";
pub const Q: usize = 5;
pub const SEARCH_N: usize = 400;
pub const CHECK_N: usize = 2000;
pub const FILE: &str = "out/golden/god_function.txt";

/// The quantities Golden Answer already computes exactly, mod q: (name, values for n = 0..=CHECK_N).
fn quantities() -> Vec<(&'static str, Vec<u8>)> {
    let n = CHECK_N;
    let mut prime = vec![true; n + 1];
    prime[0] = false;
    prime[1] = false;
    for i in 2..=n {
        if prime[i] {
            let mut j = i * i;
            while j <= n {
                prime[j] = false;
                j += i;
            }
        }
    }
    let q = Q as u64;
    let m = |v: u64| (v % q) as u8;
    let divisors = |k: usize| (1..=k).filter(move |d| k % d == 0);
    let gcd = |mut a: usize, mut b: usize| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let mut pi = vec![0u8; n + 1];
    let mut c = 0u64;
    for k in 0..=n {
        if prime[k] {
            c += 1;
        }
        pi[k] = m(c);
    }
    let mut fib = vec![0u8; n + 1];
    let (mut a, mut b) = (0u64, 1u64);
    for k in 0..=n {
        fib[k] = m(a);
        (a, b) = (b, (a + b) % q);
    }
    let collatz = |k: usize| {
        let (mut x, mut s) = (k as u64, 0u64);
        while x > 1 {
            x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
            s += 1;
        }
        s
    };
    let two_sq = |k: usize| (0..).take_while(|a: &usize| a * a <= k).any(|a| {
        let r = k - a * a;
        let s = (r as f64).sqrt() as usize;
        (s.saturating_sub(1)..=s + 1).any(|t| t * t == r)
    });
    vec![
        ("is n prime (1 or 0)", (0..=n).map(|k| prime[k] as u8).collect()),
        ("π(n), the primes up to n, mod 5", pi),
        ("τ(n), the number of divisors, mod 5", (0..=n).map(|k| if k == 0 { 0 } else { m(divisors(k).count() as u64) }).collect()),
        ("σ(n), the sum of divisors, mod 5", (0..=n).map(|k| if k == 0 { 0 } else { m(divisors(k).sum::<usize>() as u64) }).collect()),
        ("φ(n), Euler's totient, mod 5", (0..=n).map(|k| if k == 0 { 0 } else { m((1..=k).filter(|&j| gcd(j, k) == 1).count() as u64) }).collect()),
        ("F(n), the Fibonacci numbers, mod 5", fib),
        ("the Collatz steps of n to 1, mod 5", (0..=n).map(|k| if k == 0 { 0 } else { m(collatz(k)) }).collect()),
        ("is n a sum of two squares (1 or 0)", (0..=n).map(|k| two_sq(k) as u8).collect()),
    ]
}

/// G(1), G(2), ..., G(upto): each from n's digits and the two values before it.
fn run(e: &Sx, l: &Lang, upto: usize) -> Vec<u8> {
    let mut g = vec![0u8; upto + 1];
    for n in 1..=upto {
        let mut v = [0u64; 8];
        let mut x = n as u64;
        for d in v.iter_mut().take(6) {
            *d = x % Q as u64;
            x /= Q as u64;
        }
        v[6] = g[n - 1] as u64;
        v[7] = if n >= 2 { g[n - 2] as u64 } else { 0 };
        g[n] = e.eval(&v, l);
    }
    g
}

pub struct Found {
    pub name: &'static str,
    pub expr: String,
    pub glossary: Vec<String>,
    pub search_ok: usize,
    pub check_ok: usize,
    pub first_wrong: Option<usize>,
}

/// Golden invents a G for every quantity it may choose, `seconds` each.
pub fn invent_all(seconds: f64, log: &mut dyn FnMut(String)) -> Vec<Found> {
    let fr = Frame::new(2 * Q);
    let mut out = Vec::new();
    for (i, (name, truth)) in quantities().into_iter().enumerate() {
        let fit = |e: &Sx, l: &Lang, _: &Naming| {
            let g = run(e, l, SEARCH_N);
            (1..=SEARCH_N).filter(|&n| g[n] != truth[n]).count() as u64 * 64 + e.size() as u64
        };
        let (best, _, _) = evolve(&fr, seconds, 0x60D0_0000 + i as u64, &fit, &[]);
        let Some((_, e, l, nm)) = best else { continue };
        let g = run(&e, &l, CHECK_N);
        let search_ok = (1..=SEARCH_N).filter(|&n| g[n] == truth[n]).count();
        let check_ok = (SEARCH_N + 1..=CHECK_N).filter(|&n| g[n] == truth[n]).count();
        let first_wrong = (1..=CHECK_N).find(|&n| g[n] != truth[n]);
        let inv = Invented { expr: e.clone(), lang: l, naming: nm, pairs_met_share: 0.0, verified: false, tried: 0, seconds, islands: 0 };
        let gl: Vec<String> = glossary(&inv).into_iter().filter(|x| !x.starts_with("adjacency relation")).collect();
        let text = e.text().replace("i1", "d0").replace("s1", "d1").replace("j1", "d2").replace("t1", "d3").replace("i2", "d4").replace("s2", "d5").replace("j2", "G(n-1)").replace("t2", "G(n-2)");
        log(format!("  {name}: search {search_ok}/{SEARCH_N}, check {check_ok}/{}{}", CHECK_N - SEARCH_N, first_wrong.map_or(String::new(), |n| format!(", first wrong at n = {n}"))));
        out.push(Found { name, expr: text, glossary: gl, search_ok, check_ok, first_wrong });
    }
    out
}

/// Golden Answer's answer to QUESTION in its kind.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let secs = std::env::var("GOLDEN_GOD_SECONDS").ok().and_then(|x| x.parse().ok()).unwrap_or(10.0);
    if !kind_logical && !kind_abstract {
        return Err("no known result names a function that is new by definition; ask logical (invented and checked) or abstract".into());
    }
    let mut lines = vec![
        format!("G(n) = E(d0..d5, G(n-1), G(n-2)): n's base-{Q} digits and G's own last two values, E in symbols Golden invents (tables over {Q} names it makes up); no formula for any Q given"),
        format!("Q of Golden's choosing among what Golden Answer computes exactly, each mod {Q}; searched on n = 1..{SEARCH_N}, checked on {}..{CHECK_N} (never searched); {secs:.0} s each", SEARCH_N + 1),
    ];
    let found = invent_all(secs, &mut |l| lines.push(l));
    let exact: Vec<&Found> = found.iter().filter(|f| f.first_wrong.is_none()).collect();
    let mut save = String::new();
    for f in &found {
        save.push_str(&format!("{}\tsearch {}/{SEARCH_N}\tcheck {}/{}\t{}\n", f.name, f.search_ok, f.check_ok, CHECK_N - SEARCH_N, f.expr));
    }
    let _ = std::fs::write(FILE, save);
    let short = match exact.first() {
        Some(f) => format!("G computes {}: G(n) = {}", f.name, f.expr),
        None => "no G found that computes any Q on every n checked".to_string(),
    };
    for f in &exact {
        lines.push(format!("built and checked to n = {CHECK_N}: {} — G(n) = {}", f.name, f.expr));
        lines.extend(f.glossary.iter().map(|g| format!("    {g}")));
    }
    if exact.is_empty() {
        // the closest: the one that stays right the longest from n = 1
        if let Some(b) = found.iter().max_by_key(|f| f.first_wrong.unwrap_or(usize::MAX)) {
            lines.push(format!("closest: {} — G(n) = {} (search {}/{SEARCH_N}, check {}/{}; first wrong at n = {})", b.name, b.expr, b.search_ok, b.check_ok, CHECK_N - SEARCH_N, b.first_wrong.unwrap_or(0)));
        }
    }
    if kind_abstract {
        lines.push("this is an answer that claims nothing".into());
    } else {
        lines.push(format!("verdict: {} of {} quantities computed exactly by an invented G on every n checked (to {CHECK_N}); not proved for all n", exact.len(), found.len()));
    }
    Ok((short, lines, vec![]))
}

/// The question, read whole.
pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

// ---------------------------------------------------------------- a problem of its own

/// The question: Golden invents the problem too, anywhere in mathematics.
pub const QUESTION_SELF: &str = "Invent a problem of your own anywhere in mathematics — objects that differ though everything Golden already knows calls them the same — and a new function G, in symbols of your own making, that tells them apart. Check G on objects it never saw.";

pub fn read_self(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION_SELF).trim_end_matches(['.', '?', '!'])
}

/// A finite structure: n elements and one relation, as rows of bits.
#[derive(Clone)]
struct Obj {
    n: usize,
    a: Vec<u16>,
}

impl Obj {
    fn at(&self, i: usize, j: usize) -> bool {
        self.a[i] >> j & 1 == 1
    }
    fn relabel(&self, p: &[usize]) -> Obj {
        let mut a = vec![0u16; self.n];
        for i in 0..self.n {
            for j in 0..self.n {
                if self.at(i, j) {
                    a[p[i]] |= 1 << p[j];
                }
            }
        }
        Obj { n: self.n, a }
    }
}

/// An area Golden can make objects in itself: name, tuple size for G, a random object.
struct Area {
    name: &'static str,
    k: usize,
    make: fn(&mut u64) -> Obj,
}

fn rnd(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

fn make_graph(s: &mut u64) -> Obj {
    let mut a = vec![0u16; 8];
    for i in 0..8 {
        for j in i + 1..8 {
            if rnd(s) & 1 == 1 {
                a[i] |= 1 << j;
                a[j] |= 1 << i;
            }
        }
    }
    Obj { n: 8, a }
}

fn make_map(s: &mut u64) -> Obj {
    let a = (0..7).map(|_| 1u16 << (rnd(s) % 7)).collect();
    Obj { n: 7, a }
}

fn make_perm(s: &mut u64) -> Obj {
    let mut p: Vec<usize> = (0..7).collect();
    for i in (1..7).rev() {
        p.swap(i, (rnd(s) % (i as u64 + 1)) as usize);
    }
    Obj { n: 7, a: p.iter().map(|&j| 1u16 << j).collect() }
}

fn areas() -> Vec<Area> {
    vec![
        Area { name: "graphs on 8 vertices", k: 4, make: make_graph },
        Area { name: "maps f: S -> S on 7 points", k: 3, make: make_map },
        Area { name: "permutations of 7 points", k: 3, make: make_perm },
    ]
}

/// Everything Golden already knows about such an object: its spectrum (the
/// closed walks tr A^k, k = 1..n, which fix the characteristic polynomial)
/// and the out- and in-degree sequences.
fn known(o: &Obj) -> Vec<i64> {
    let n = o.n;
    let m: Vec<Vec<i64>> = (0..n).map(|i| (0..n).map(|j| o.at(i, j) as i64).collect()).collect();
    let mut p = m.clone();
    let mut key = Vec::new();
    for _ in 0..n {
        key.push((0..n).map(|i| p[i][i]).sum());
        p = (0..n).map(|i| (0..n).map(|j| (0..n).map(|t| p[i][t] * m[t][j]).sum()).collect()).collect();
    }
    let mut out: Vec<i64> = (0..n).map(|i| o.a[i].count_ones() as i64).collect();
    let mut inn: Vec<i64> = (0..n).map(|j| (0..n).filter(|&i| o.at(i, j)).count() as i64).collect();
    out.sort();
    inn.sort();
    key.extend(out);
    key.push(-1);
    key.extend(inn);
    key
}

/// Whether two objects are the same up to renaming the elements (every renaming tried, with pruning).
fn isomorphic(x: &Obj, y: &Obj) -> bool {
    fn go(x: &Obj, y: &Obj, p: &mut Vec<usize>, used: u16) -> bool {
        let i = p.len();
        if i == x.n {
            return true;
        }
        for j in 0..x.n {
            if used >> j & 1 == 1 || x.a[i].count_ones() != y.a[j].count_ones() {
                continue;
            }
            if x.at(i, i) == y.at(j, j) && (0..i).all(|t| x.at(i, t) == y.at(j, p[t]) && x.at(t, i) == y.at(p[t], j)) {
                p.push(j);
                if go(x, y, p, used | 1 << j) {
                    return true;
                }
                p.pop();
            }
        }
        false
    }
    go(x, y, &mut Vec::new(), 0)
}

/// Golden's problem in an area: pairs that differ though everything known calls them the same.
fn invent_problem(area: &Area, tries: usize, want: usize, seed: u64) -> Vec<(Obj, Obj)> {
    let mut s = seed | 1;
    let mut seen: std::collections::HashMap<Vec<i64>, Vec<Obj>> = std::collections::HashMap::new();
    let mut pairs = Vec::new();
    for _ in 0..tries {
        let o = (area.make)(&mut s);
        let bucket = seen.entry(known(&o)).or_default();
        if bucket.iter().any(|b| isomorphic(b, &o)) {
            continue;
        }
        if let Some(b) = bucket.first() {
            pairs.push((b.clone(), o.clone()));
            if pairs.len() >= want {
                break;
            }
        }
        bucket.push(o);
    }
    pairs
}

/// The relations among a tuple of elements, as up to 8 bits for G's expression.
fn tuple_bits(o: &Obj, t: &[usize], k: usize) -> [u64; 8] {
    let mut v = [0u64; 8];
    let e: &[(usize, usize)] = if k == 4 { &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] } else { &[(0, 1), (1, 0), (0, 2), (2, 0), (1, 2), (2, 1), (0, 0), (1, 1)] };
    for (x, &(i, j)) in e.iter().enumerate() {
        v[x] = o.at(t[i], t[j]) as u64;
    }
    v
}

fn rec_patterns(o: &Obj, k: usize, t: &mut Vec<usize>, h: &mut std::collections::BTreeMap<[u64; 8], u64>) {
    if t.len() == k {
        *h.entry(tuple_bits(o, t, k)).or_default() += 1;
        return;
    }
    for x in 0..o.n {
        if t.contains(&x) {
            continue;
        }
        t.push(x);
        rec_patterns(o, k, t, h);
        t.pop();
    }
}

/// How often each relation pattern occurs over every ordered tuple of distinct elements.
fn patterns(o: &Obj, k: usize) -> Vec<([u64; 8], u64)> {
    let mut h = std::collections::BTreeMap::new();
    rec_patterns(o, k, &mut Vec::new(), &mut h);
    h.into_iter().collect()
}

/// G(object) = the sum over every ordered tuple of E(the tuple's relations), mod Q.
fn g_value(e: &Sx, l: &Lang, hist: &[([u64; 8], u64)]) -> u64 {
    hist.iter().map(|(v, c)| e.eval(v, l) as u64 * c).sum::<u64>() % Q as u64
}

type Hist = Vec<([u64; 8], u64)>;

pub fn answer_self(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let secs = std::env::var("GOLDEN_GOD_SECONDS").ok().and_then(|x| x.parse().ok()).unwrap_or(20.0);
    let mut lines = vec![
        "the areas Golden can make objects in itself: finite structures of one relation (graphs, maps f: S -> S, permutations); what it already knows of them: the spectrum (closed walks) and the degree sequences".to_string(),
        format!("the problem it invents: pairs that are different (no renaming makes them equal, every renaming tried) though everything known calls them the same; the answer it invents: G = Σ over every ordered tuple of E(the tuple's relations) mod {Q}, E in symbols of its own (so G cannot depend on names); {secs:.0} s an area"),
    ];
    let fr = Frame::new(2 * Q);
    let mut best: Option<(String, String, usize, usize, Vec<String>)> = None;
    for (ai, area) in areas().into_iter().enumerate() {
        let pairs = invent_problem(&area, 60_000, 24, 0xA5EA_0000 + ai as u64);
        if pairs.len() < 4 {
            lines.push(format!("{}: no problem to invent — among 60 000 objects, what the known invariants call the same is the same ({} pairs found)", area.name, pairs.len()));
            continue;
        }
        let half = pairs.len() / 2;
        let hist: Vec<(Hist, Hist)> = pairs.iter().map(|(x, y)| (patterns(x, area.k), patterns(y, area.k))).collect();
        let (search, check) = hist.split_at(half);
        let fit = |e: &Sx, l: &Lang, _: &Naming| search.iter().filter(|(a, b)| g_value(e, l, a) == g_value(e, l, b)).count() as u64 * 64 + e.size() as u64;
        let (found, _, _) = evolve(&fr, secs, 0x60D5_0000 + ai as u64, &fit, &[]);
        let Some((_, e, l, nm)) = found else { continue };
        let s_ok = search.iter().filter(|(a, b)| g_value(&e, &l, a) != g_value(&e, &l, b)).count();
        let c_ok = check.iter().filter(|(a, b)| g_value(&e, &l, a) != g_value(&e, &l, b)).count();
        // G is the same on a renaming of every object of the problem
        let mut sr = 7u64;
        let invariant = pairs.iter().flat_map(|(x, y)| [x, y]).all(|o| {
            let mut p: Vec<usize> = (0..o.n).collect();
            for i in (1..o.n).rev() {
                p.swap(i, (rnd(&mut sr) % (i as u64 + 1)) as usize);
            }
            g_value(&e, &l, &patterns(o, area.k)) == g_value(&e, &l, &patterns(&o.relabel(&p), area.k))
        });
        let names = if area.k == 4 { ["a01", "a02", "a03", "a12", "a13", "a23", "0", "0"] } else { ["a01", "a10", "a02", "a20", "a12", "a21", "a00", "a11"] };
        let mut text = e.text();
        for (v, nmv) in ["i1", "s1", "j1", "t1", "i2", "s2", "j2", "t2"].iter().zip(names) {
            text = text.replace(v, nmv);
        }
        let inv = Invented { expr: e.clone(), lang: l, naming: nm, pairs_met_share: 0.0, verified: false, tried: 0, seconds: secs, islands: 0 };
        let gl: Vec<String> = glossary(&inv).into_iter().filter(|x| !x.starts_with("adjacency relation")).collect();
        lines.push(format!(
            "{}: the problem = {} pairs ({} to search, {} never seen); E = {}; G tells apart {s_ok}/{} searched and {c_ok}/{} never seen; the same on a renaming of every object: {}",
            area.name,
            pairs.len(),
            search.len(),
            check.len(),
            text,
            search.len(),
            check.len(),
            if invariant { "yes" } else { "NO" }
        ));
        if best.as_ref().map_or(true, |b| c_ok + s_ok > b.2 + b.3) {
            best = Some((area.name.to_string(), text, s_ok, c_ok, gl));
        }
    }
    let Some((area, text, s_ok, c_ok, gl)) = best else {
        return Err("no area gave a problem to invent".into());
    };
    lines.push(format!("chosen: {area} — G tells apart {s_ok} searched and {c_ok} never-seen pairs that everything known calls the same"));
    lines.extend(gl.into_iter().map(|g| format!("    {g}")));
    if kind_abstract {
        lines.push("this is an answer that claims nothing".into());
    } else if !kind_logical {
        lines.push("what is known of its kind: a sum over tuples of a function of their relations is a combination of pattern (induced substructure) counts, a known kind of invariant; new here are the combination, its symbols, and the problem, found by Golden itself".into());
    } else {
        lines.push("checked: G is the same on a renaming of every object, and the pairs are different (every renaming tried); not proved beyond these objects".into());
    }
    Ok((format!("in {area}: G = Σ over tuples of {text} (mod {Q})"), lines, vec![]))
}
