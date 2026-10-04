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

const BIN: [&str; 3] = ["◇", "★", "⊛"];
const UN: [&str; 2] = ["†", "‡"];
const VARS: [&str; 8] = ["i1", "s1", "j1", "t1", "i2", "s2", "j2", "t2"];

#[derive(Clone, Debug)]
pub enum Sx {
    Var(u8),
    Un(u8, Box<Sx>),
    Bin(u8, Box<Sx>, Box<Sx>),
}

/// The invented symbols' meanings: tables over Z_q.
#[derive(Clone, Debug)]
pub struct Lang {
    pub q: usize,
    pub bin: Vec<Vec<u8>>,
    pub un: Vec<Vec<u8>>,
}

impl Sx {
    fn eval(&self, v: &[u64; 8], l: &Lang) -> u8 {
        match self {
            Sx::Var(i) => (v[*i as usize] % l.q as u64) as u8,
            Sx::Un(o, a) => l.un[*o as usize][a.eval(v, l) as usize],
            Sx::Bin(o, a, b) => l.bin[*o as usize][a.eval(v, l) as usize * l.q + b.eval(v, l) as usize],
        }
    }
    fn size(&self) -> usize {
        match self {
            Sx::Var(_) => 1,
            Sx::Un(_, a) => 1 + a.size(),
            Sx::Bin(_, a, b) => 1 + a.size() + b.size(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Sx::Var(i) => VARS[*i as usize].into(),
            Sx::Un(o, a) => format!("{}{}", UN[*o as usize], a.text()),
            Sx::Bin(o, a, b) => format!("({} {} {})", a.text(), BIN[*o as usize], b.text()),
        }
    }
    fn uses(&self, out: &mut [bool; 5]) {
        match self {
            Sx::Var(_) => {}
            Sx::Un(o, a) => {
                out[3 + *o as usize] = true;
                a.uses(out);
            }
            Sx::Bin(o, a, b) => {
                out[*o as usize] = true;
                a.uses(out);
                b.uses(out);
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

fn random_sx(r: &mut Rng, depth: u32) -> Sx {
    if depth == 0 || r.below(10) < 3 {
        return Sx::Var(r.below(8) as u8);
    }
    if r.below(4) == 0 {
        Sx::Un(r.below(2) as u8, Box::new(random_sx(r, depth - 1)))
    } else {
        Sx::Bin(r.below(3) as u8, Box::new(random_sx(r, depth - 1)), Box::new(random_sx(r, depth - 1)))
    }
}

fn mutate_sx(e: &Sx, r: &mut Rng) -> Sx {
    if r.below(4) == 0 {
        return random_sx(r, 3);
    }
    match e {
        Sx::Var(_) => Sx::Var(r.below(8) as u8),
        Sx::Un(o, a) => {
            if r.below(3) == 0 {
                Sx::Un(1 - *o, a.clone())
            } else {
                Sx::Un(*o, Box::new(mutate_sx(a, r)))
            }
        }
        Sx::Bin(o, a, b) => match r.below(4) {
            0 => Sx::Bin(r.below(3) as u8, a.clone(), b.clone()),
            1 => Sx::Bin(*o, b.clone(), a.clone()),
            2 => Sx::Bin(*o, Box::new(mutate_sx(a, r)), b.clone()),
            _ => Sx::Bin(*o, a.clone(), Box::new(mutate_sx(b, r))),
        },
    }
}

impl Lang {
    fn random(q: usize, r: &mut Rng) -> Lang {
        Lang { q, bin: (0..3).map(|_| (0..q * q).map(|_| r.below(q as u64) as u8).collect()).collect(), un: (0..2).map(|_| (0..q).map(|_| r.below(q as u64) as u8).collect()).collect() }
    }
    fn mutate(&self, r: &mut Rng) -> Lang {
        let mut l = self.clone();
        let q = l.q as u64;
        for _ in 0..1 + r.below(3) {
            if r.below(4) == 0 {
                let o = r.below(2) as usize;
                let c = r.below(q) as usize;
                l.un[o][c] = r.below(q) as u8;
            } else {
                let o = r.below(3) as usize;
                let c = r.below(q * q) as usize;
                l.bin[o][c] = r.below(q) as u8;
            }
        }
        l
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
                let fit = |e: &Sx, l: &Lang, nm: &Naming| fr.score_with(nm, |f, g| e.eval(f, l) == e.eval(g, l)).0 * 64 + e.size() as u64;
                let mut pop: Vec<Ind> = (0..pop_n)
                    .map(|_| {
                        let e = random_sx(&mut r, 4);
                        let l = Lang::random(q, &mut r);
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
                            0..=3 => (mutate_sx(e, &mut r), l.clone(), nm.clone()),
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
    let adj = fr.adjacency_with(&naming, |f, g| expr.eval(f, &lang) == expr.eval(g, &lang));
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

/// The invented symbols the expression uses, each with its table and what it is like.
pub fn glossary(inv: &Invented) -> Vec<String> {
    let mut used = [false; 5];
    inv.expr.uses(&mut used);
    let q = inv.lang.q;
    let mut out = Vec::new();
    for o in 0..3 {
        if used[o] {
            let rows: Vec<String> = (0..q).map(|a| inv.lang.bin[o][a * q..(a + 1) * q].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")).collect();
            out.push(format!("{}: table rows [{}] — {}", BIN[o], rows.join(" | "), describe_bin(&inv.lang.bin[o], q)));
        }
    }
    for o in 0..2 {
        if used[3 + o] {
            let perm = {
                let mut seen = vec![false; q];
                inv.lang.un[o].iter().all(|&x| !std::mem::replace(&mut seen[x as usize], true))
            };
            out.push(format!("{}: {:?} — {}", UN[o], inv.lang.un[o], if perm { "a permutation of the names" } else { "not a permutation" }));
        }
    }
    out
}
