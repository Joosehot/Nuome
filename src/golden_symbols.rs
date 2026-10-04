//! Golden invents its own mathematics for a graph of Conway's kind: new
//! symbols with meanings it makes up, and expressions written in them.
//! Nothing learned is given — no +, -, ×, no known graph, no formula:
//!
//! - three binary symbols ◇ ★ ⊛ and two unary ones † ‡ on the q = k/2
//!   names of the triangles; each symbol's meaning is a table Golden
//!   invents (random at first, evolved);
//! - an expression E in those symbols and the names of a far vertex pair
//!   (i1 s1 j1 t1 i2 s2 j2 t2);
//! - the rule: far vertices f ~ g when E(f, g) = E(g, f) (symmetric with no
//!   arithmetic at all);
//! - the names of the triangles and corners, invented too (the forced frame
//!   gives none).
//!
//! A language and expression are kept when their graph meets more of the
//! conditions (degree k everywhere; 1 common neighbour for adjacent pairs,
//! 2 for the rest). A graph meeting all is checked from scratch. Each
//! invented symbol is then described: commutative, associative, identity,
//! a Latin square, and whether it is only a renamed + or × of Z_q.

use crate::golden_invent::{Frame, Naming};

/// Names for the symbols Golden invents (as many as it wants up to these).
const BIN: [&str; 16] = ["◇", "★", "⊛", "⊕", "⊗", "△", "▽", "◆", "♦", "♣", "♠", "♥", "☉", "☽", "⊞", "⊠"];
const UN: [&str; 8] = ["†", "‡", "§", "¶", "∂", "∇", "♮", "♯"];
const TER: [(&str, &str); 4] = [("⟨", "⟩"), ("⟦", "⟧"), ("⦃", "⦄"), ("⦉", "⦊")];
/// The old ones it may use when it wants: +, -, × mod q.
const KNOWN: [&str; 3] = ["+", "-", "×"];
const VARS: [&str; 8] = ["i1", "s1", "j1", "t1", "i2", "s2", "j2", "t2"];

#[derive(Clone, Debug)]
pub enum Sx {
    Var(u8),
    Const(u8),
    Known(u8, Box<Sx>, Box<Sx>),
    Un(u8, Box<Sx>),
    Bin(u8, Box<Sx>, Box<Sx>),
    Ter(u8, Box<Sx>, Box<Sx>, Box<Sx>),
}

/// The invented symbols' meanings: tables over the q names.
#[derive(Clone, Debug)]
pub struct Lang {
    pub q: usize,
    pub bin: Vec<Vec<u8>>,
    pub un: Vec<Vec<u8>>,
    pub ter: Vec<Vec<u8>>,
    /// the invented relation ≈ that decides adjacency: f ~ g when E(f,g) ≈ E(g,f) (a symmetric q x q table of 0/1)
    pub rel: Vec<u8>,
}

impl Sx {
    fn eval(&self, v: &[u64; 8], l: &Lang) -> u8 {
        let q = l.q;
        match self {
            Sx::Var(i) => (v[*i as usize] % q as u64) as u8,
            Sx::Const(c) => (*c as usize % q) as u8,
            Sx::Known(o, a, b) => {
                let (x, y) = (a.eval(v, l) as usize, b.eval(v, l) as usize);
                (match o {
                    0 => (x + y) % q,
                    1 => (x + q - y) % q,
                    _ => x * y % q,
                }) as u8
            }
            Sx::Un(o, a) => l.un[*o as usize][a.eval(v, l) as usize],
            Sx::Bin(o, a, b) => l.bin[*o as usize][a.eval(v, l) as usize * q + b.eval(v, l) as usize],
            Sx::Ter(o, a, b, c) => l.ter[*o as usize][(a.eval(v, l) as usize * q + b.eval(v, l) as usize) * q + c.eval(v, l) as usize],
        }
    }
    fn size(&self) -> usize {
        match self {
            Sx::Var(_) | Sx::Const(_) => 1,
            Sx::Un(_, a) => 1 + a.size(),
            Sx::Known(_, a, b) | Sx::Bin(_, a, b) => 1 + a.size() + b.size(),
            Sx::Ter(_, a, b, c) => 1 + a.size() + b.size() + c.size(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Sx::Var(i) => VARS[*i as usize].into(),
            Sx::Const(c) => c.to_string(),
            Sx::Known(o, a, b) => format!("({} {} {})", a.text(), KNOWN[*o as usize], b.text()),
            Sx::Un(o, a) => format!("{}{}", UN[*o as usize], a.text()),
            Sx::Bin(o, a, b) => format!("({} {} {})", a.text(), BIN[*o as usize], b.text()),
            Sx::Ter(o, a, b, c) => format!("{}{}, {}, {}{}", TER[*o as usize].0, a.text(), b.text(), c.text(), TER[*o as usize].1),
        }
    }
    /// The symbols used: (arity, index), and whether old ones are used.
    fn uses(&self, out: &mut Vec<(u8, u8)>, known: &mut [bool; 3]) {
        match self {
            Sx::Var(_) | Sx::Const(_) => {}
            Sx::Known(o, a, b) => {
                known[*o as usize] = true;
                a.uses(out, known);
                b.uses(out, known);
            }
            Sx::Un(o, a) => {
                out.push((1, *o));
                a.uses(out, known);
            }
            Sx::Bin(o, a, b) => {
                out.push((2, *o));
                a.uses(out, known);
                b.uses(out, known);
            }
            Sx::Ter(o, a, b, c) => {
                out.push((3, *o));
                a.uses(out, known);
                b.uses(out, known);
                c.uses(out, known);
            }
        }
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
}

impl Lang {
    fn empty(q: usize) -> Lang {
        Lang { q, bin: Vec::new(), un: Vec::new(), ter: Vec::new(), rel: (0..q * q).map(|c| (c / q == c % q) as u8).collect() }
    }
    fn start(q: usize, r: &mut Rng) -> Lang {
        let mut l = Lang::empty(q);
        if r.below(2) == 0 {
            // grown from an old relation: a + b ≡ 0
            l.rel = (0..q * q).map(|c| ((c / q + c % q) % q == 0) as u8).collect();
        }
        l
    }
    pub fn related(&self, a: u8, b: u8) -> bool {
        self.rel[a as usize * self.q + b as usize] == 1
    }
    /// A new table: random, or copied from an old operation to grow from (Golden's choice).
    fn new_table(&self, arity: u32, r: &mut Rng) -> Vec<u8> {
        let q = self.q;
        let n = q.pow(arity);
        match (arity, r.below(3)) {
            (2, 0) => (0..n).map(|c| ((c / q + c % q) % q) as u8).collect(),
            (2, 1) => (0..n).map(|c| ((c / q) * (c % q) % q) as u8).collect(),
            _ => (0..n).map(|_| r.below(q as u64) as u8).collect(),
        }
    }
    /// Invent a new symbol of this arity, if there is room; its index.
    fn invent(&mut self, arity: u32, r: &mut Rng) -> Option<u8> {
        let t = self.new_table(arity, r);
        let (list, cap) = match arity {
            1 => (&mut self.un, UN.len()),
            2 => (&mut self.bin, BIN.len()),
            _ => (&mut self.ter, TER.len()),
        };
        if list.len() >= cap {
            return None;
        }
        list.push(t);
        Some((list.len() - 1) as u8)
    }
    fn mutate(&self, r: &mut Rng) -> Lang {
        let mut l = self.clone();
        let q = l.q as u64;
        let total = l.un.len() + l.bin.len() + l.ter.len();
        if total == 0 || r.below(4) == 0 {
            // the relation: flip one symmetric pair
            let (a, b) = (r.below(q) as usize, r.below(q) as usize);
            let x = 1 - l.rel[a * l.q + b];
            l.rel[a * l.q + b] = x;
            l.rel[b * l.q + a] = x;
            return l;
        }
        for _ in 0..1 + r.below(3) {
            let pick = r.below(total as u64) as usize;
            let table = if pick < l.un.len() {
                &mut l.un[pick]
            } else if pick < l.un.len() + l.bin.len() {
                &mut l.bin[pick - l.un.len()]
            } else {
                &mut l.ter[pick - l.un.len() - l.bin.len()]
            };
            let c = r.below(table.len() as u64) as usize;
            table[c] = r.below(q) as u8;
        }
        l
    }
}

/// A random expression; it may invent new symbols on the way (into `l`) or use old ones.
fn random_sx(r: &mut Rng, depth: u32, l: &mut Lang) -> Sx {
    if depth == 0 || r.below(10) < 3 {
        return if r.below(10) < 8 { Sx::Var(r.below(8) as u8) } else { Sx::Const(r.below(l.q as u64) as u8) };
    }
    let sub = |r: &mut Rng, l: &mut Lang| Box::new(random_sx(r, depth - 1, l));
    // which kind of node: an old operation, or an invented symbol (an existing one, or a new one)
    match r.below(10) {
        0..=2 => Sx::Known(r.below(3) as u8, sub(r, l), sub(r, l)),
        3 | 4 => {
            let o = if l.un.is_empty() || r.below(5) == 0 { l.invent(1, r) } else { None }.unwrap_or_else(|| r.below(l.un.len().max(1) as u64) as u8);
            if l.un.is_empty() {
                return Sx::Var(r.below(8) as u8);
            }
            Sx::Un(o, sub(r, l))
        }
        5..=8 => {
            let o = if l.bin.is_empty() || r.below(5) == 0 { l.invent(2, r) } else { None }.unwrap_or_else(|| r.below(l.bin.len().max(1) as u64) as u8);
            if l.bin.is_empty() {
                return Sx::Var(r.below(8) as u8);
            }
            Sx::Bin(o, sub(r, l), sub(r, l))
        }
        _ => {
            let o = if l.ter.is_empty() || r.below(5) == 0 { l.invent(3, r) } else { None }.unwrap_or_else(|| r.below(l.ter.len().max(1) as u64) as u8);
            if l.ter.is_empty() {
                return Sx::Var(r.below(8) as u8);
            }
            Sx::Ter(o, sub(r, l), sub(r, l), sub(r, l))
        }
    }
}

fn mutate_sx(e: &Sx, r: &mut Rng, l: &mut Lang) -> Sx {
    if r.below(4) == 0 {
        return random_sx(r, 3, l);
    }
    match e {
        Sx::Var(_) => Sx::Var(r.below(8) as u8),
        Sx::Const(_) => Sx::Const(r.below(l.q as u64) as u8),
        Sx::Known(o, a, b) => match r.below(4) {
            0 => Sx::Known(r.below(3) as u8, a.clone(), b.clone()),
            1 => Sx::Known(*o, b.clone(), a.clone()),
            2 => Sx::Known(*o, Box::new(mutate_sx(a, r, l)), b.clone()),
            _ => Sx::Known(*o, a.clone(), Box::new(mutate_sx(b, r, l))),
        },
        Sx::Un(o, a) => match r.below(3) {
            0 => Sx::Un(r.below(l.un.len().max(1) as u64) as u8, a.clone()),
            _ => Sx::Un(*o, Box::new(mutate_sx(a, r, l))),
        },
        Sx::Bin(o, a, b) => match r.below(4) {
            0 => Sx::Bin(r.below(l.bin.len().max(1) as u64) as u8, a.clone(), b.clone()),
            1 => Sx::Bin(*o, b.clone(), a.clone()),
            2 => Sx::Bin(*o, Box::new(mutate_sx(a, r, l)), b.clone()),
            _ => Sx::Bin(*o, a.clone(), Box::new(mutate_sx(b, r, l))),
        },
        Sx::Ter(o, a, b, c) => match r.below(4) {
            0 => Sx::Ter(r.below(l.ter.len().max(1) as u64) as u8, a.clone(), b.clone(), c.clone()),
            1 => Sx::Ter(*o, Box::new(mutate_sx(a, r, l)), b.clone(), c.clone()),
            2 => Sx::Ter(*o, a.clone(), Box::new(mutate_sx(b, r, l)), c.clone()),
            _ => Sx::Ter(*o, a.clone(), b.clone(), Box::new(mutate_sx(c, r, l))),
        },
    }
}

/// What an invented binary symbol is like.
pub fn describe_bin(t: &[u8], q: usize) -> String {
    let at = |a: usize, b: usize| t[a * q + b] as usize;
    let comm = (0..q).all(|a| (0..q).all(|b| at(a, b) == at(b, a)));
    let assoc = (0..q).all(|a| (0..q).all(|b| (0..q).all(|c| at(at(a, b), c) == at(a, at(b, c)))));
    let ident = (0..q).find(|&e| (0..q).all(|a| at(e, a) == a && at(a, e) == a));
    let latin = (0..q).all(|a| {
        let mut row = vec![false; q];
        let mut col = vec![false; q];
        (0..q).all(|b| !std::mem::replace(&mut row[at(a, b)], true)) && (0..q).all(|b| !std::mem::replace(&mut col[at(b, a)], true))
    });
    let mut words = vec![
        if comm { "commutative" } else { "not commutative" }.to_string(),
        if assoc { "associative" } else { "not associative" }.to_string(),
        match ident {
            Some(e) => format!("identity {e}"),
            None => "no identity".into(),
        },
        if latin { "a Latin square (every equation a ◇ x = b solvable uniquely)" } else { "not a Latin square" }.to_string(),
    ];
    // what it is when it is something known: a group of order q is addition mod q renamed (q prime);
    // the table of x·y mod q or of x + y mod q as written is that operation
    let mul = (0..q).all(|a| (0..q).all(|b| at(a, b) == a * b % q));
    let add = (0..q).all(|a| (0..q).all(|b| at(a, b) == (a + b) % q));
    if latin && assoc && comm && ident.is_some() {
        words.push(format!("so it is a group of order {q}: addition mod {q}, renamed{}", if add { " (here exactly x + y mod q)" } else { "" }));
    } else if mul {
        words.push(format!("it is exactly multiplication mod {q}: known, not new"));
    } else {
        words.push("not a group and not multiplication as written: an operation of its own".into());
    }
    words.join(", ")
}

pub struct Invented {
    pub expr: Sx,
    pub lang: Lang,
    pub naming: Naming,
    pub pairs_met_share: f64,
    pub verified: bool,
    pub tried: u64,
    pub seconds: f64,
    pub islands: usize,
}

/// Invent symbols, their meanings and an expression, for at most `seconds` on every core.
pub fn invent(k: usize, seconds: f64, seed: u64) -> Option<Invented> {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;
    if k % 2 != 0 || k < 4 {
        return None;
    }
    let fr = Frame::new(k);
    if fr.v > 512 {
        return None;
    }
    let q = fr.q as usize;
    let t0 = std::time::Instant::now();
    let islands = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16);
    let done = AtomicBool::new(false);
    let tried = AtomicU64::new(0);
    type Ind = (u64, Sx, Lang, Naming);
    let best: Mutex<Option<Ind>> = Mutex::new(None);
    std::thread::scope(|sc| {
        for isl in 0..islands {
            let (fr, done, tried, best) = (&fr, &done, &tried, &best);
            sc.spawn(move || {
                let mut r = Rng((seed ^ (isl as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)) | 1);
                let pop_n = 200;
                let fit = |e: &Sx, l: &Lang, nm: &Naming| fr.score_with(nm, |f, g| l.related(e.eval(f, l), e.eval(g, l))).0 * 64 + e.size() as u64;
                let mut pop: Vec<Ind> = (0..pop_n)
                    .map(|_| {
                        let mut l = Lang::start(q, &mut r);
                        let e = random_sx(&mut r, 4, &mut l);
                        let nm = Naming::shuffled(fr.q, r.next());
                        (fit(&e, &l, &nm), e, l, nm)
                    })
                    .collect();
                pop.sort_by_key(|p| p.0);
                let mut my = pop_n as u64;
                while !done.load(Ordering::Relaxed) && t0.elapsed().as_secs_f64() < seconds {
                    let mut next: Vec<Ind> = pop[..8].to_vec();
                    while next.len() < pop_n {
                        let a = (r.below(pop_n as u64) as usize).min(r.below(pop_n as u64) as usize);
                        let (_, e, l, nm) = &pop[a];
                        let (e, l, nm) = match r.below(10) {
                            0..=3 => {
                                let mut l2 = l.clone();
                                (mutate_sx(e, &mut r, &mut l2), l2, nm.clone())
                            }
                            4..=7 => (e.clone(), l.mutate(&mut r), nm.clone()),
                            _ => (e.clone(), l.clone(), nm.mutate_seeded(r.next())),
                        };
                        if e.size() > 30 {
                            continue;
                        }
                        my += 1;
                        next.push((fit(&e, &l, &nm), e, l, nm));
                    }
                    next.sort_by_key(|p| p.0);
                    pop = next;
                    let mut b = best.lock().unwrap();
                    if b.as_ref().map_or(true, |x| pop[0].0 < x.0) {
                        *b = Some(pop[0].clone());
                    }
                    if isl == 0 && seconds > 600.0 {
                        static LAST: AtomicU64 = AtomicU64::new(0);
                        let slot = (t0.elapsed().as_secs_f64() / 300.0) as u64;
                        if slot > LAST.load(Ordering::Relaxed) {
                            LAST.store(slot, Ordering::Relaxed);
                            if let Some((f, e, _, _)) = b.as_ref() {
                                eprintln!("[{} min] best error {}: {}", slot * 5, f / 64, e.text());
                            }
                        }
                    }
                    if pop[0].0 / 64 == 0 {
                        done.store(true, Ordering::Relaxed);
                    }
                }
                tried.fetch_add(my, Ordering::Relaxed);
            });
        }
    });
    let (_, expr, lang, naming) = best.into_inner().unwrap()?;
    let adj = fr.adjacency_with(&naming, |f, g| lang.related(expr.eval(f, &lang), expr.eval(g, &lang)));
    let (mut met, pairs) = (0usize, fr.v * (fr.v - 1) / 2);
    for x in 0..fr.v {
        for y in x + 1..fr.v {
            let c = (0..fr.v).filter(|&z| adj[x][z] && adj[y][z]).count();
            met += (c == if adj[x][y] { 1 } else { 2 }) as usize;
        }
    }
    let verified = crate::golden_graph::check(&adj, k as u64, 1, 2).is_ok();
    Some(Invented { expr, lang, naming, pairs_met_share: met as f64 / pairs as f64, verified, tried: tried.load(Ordering::Relaxed), seconds: t0.elapsed().as_secs_f64(), islands })
}

/// The symbols the expression uses, each invented one with its table and what it is like.
pub fn glossary(inv: &Invented) -> Vec<String> {
    let mut used = Vec::new();
    let mut known = [false; 3];
    inv.expr.uses(&mut used, &mut known);
    used.sort();
    used.dedup();
    let q = inv.lang.q;
    let mut out = Vec::new();
    let old: Vec<&str> = (0..3).filter(|&o| known[o]).map(|o| KNOWN[o]).collect();
    out.push(if old.is_empty() { "old operations used: none".to_string() } else { format!("old operations used: {} (mod {q})", old.join(" ")) });
    let rel = &inv.lang.rel;
    let eq = (0..q * q).all(|c| rel[c] == (c / q == c % q) as u8);
    let sum0 = (0..q * q).all(|c| rel[c] == ((c / q + c % q) % q == 0) as u8);
    let pairs: Vec<String> = (0..q).flat_map(|a| (a..q).map(move |b| (a, b))).filter(|&(a, b)| rel[a * q + b] == 1).map(|(a, b)| format!("{a}≈{b}")).collect();
    out.push(format!(
        "adjacency relation ≈: {}",
        if eq { "equality (old)".to_string() } else if sum0 { "a + b ≡ 0 (old)".to_string() } else { format!("invented, {} related pairs: {}", pairs.len(), pairs.join(" ")) }
    ));
    for (arity, o) in used {
        let o = o as usize;
        match arity {
            1 => {
                let t = &inv.lang.un[o];
                let mut seen = vec![false; q];
                let perm = t.iter().all(|&x| !std::mem::replace(&mut seen[x as usize], true));
                out.push(format!("invented {}: {:?} — {}", UN[o], t, if perm { "a permutation of the names" } else { "not a permutation" }));
            }
            2 => {
                let t = &inv.lang.bin[o];
                let rows: Vec<String> = (0..q).map(|a| t[a * q..(a + 1) * q].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")).collect();
                out.push(format!("invented {}: table rows [{}] — {}", BIN[o], rows.join(" | "), describe_bin(t, q)));
            }
            _ => {
                let t = &inv.lang.ter[o];
                let sym = (0..q).all(|a| (0..q).all(|b| (0..q).all(|c| t[(a * q + b) * q + c] == t[(b * q + a) * q + c] && t[(a * q + b) * q + c] == t[(a * q + c) * q + b])));
                out.push(format!("invented {}a, b, c{}: a three-place symbol, table of {} values — {}", TER[o].0, TER[o].1, t.len(), if sym { "symmetric in its three places" } else { "not symmetric" }));
            }
        }
    }
    out
}
