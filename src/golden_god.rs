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
