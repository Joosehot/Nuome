//! The golden function looks at the patterns (the owner's): the answer comes from a function Golden grows, and what it
//! sees is the board's patterns, not numbers set by hand.
//!
//! A pattern is 64 squares. Every piece's place, movement and value make them: for each side and kind of piece the ply
//! it can reach every square (crack_wave::times_to, the side to move on plies 1, 3, 5, ..., the other on 2, 4, 6, ...),
//! and a side's wave (the pieces' worth fading per move, added). The places where a pattern is read are the kings and
//! their squares, the queening squares, the pawns' paths, the pieces of each kind, the edge, the corners, the board.
//!
//! The relation is two patterns meeting: Meet(a, b, place) reads a - b square by square (how many squares a reaches
//! first, b first, both at once, the largest and smallest gap). The function over the readings is a tree of + - * min
//! max if, so the parts may interfere (a sum could not say that two knights do not mate). > 0 a win for the side to
//! move, < 0 a loss, 0 a draw; the rules' verdict comes first.
//!
//! Golden grows the tree on bench/crack/train.tsv only (islands, tournament, crossover, mutation, a small price per
//! node, fixed seeds: the same run gives the same function); it is judged on bench/crack/smoke.tsv, never seen.
use crate::crack::{rules, searched_with, V};
use crate::crack_wave::{load, times_to, UNREACHED};
use crate::golden::Board;

const PLAN: u8 = 8;
const FAR: i16 = 99;
const GRIDS: usize = 18;
const MASKS: usize = 23;
const GRID_NAMES: [&str; GRIDS] = ["myP", "myN", "myB", "myR", "myQ", "myK", "thP", "thN", "thB", "thR", "thQ", "thK", "myForce", "thForce", "myAll", "thAll", "myWave", "thWave"];
const MASK_NAMES: [&str; MASKS] = ["@myK", "@thK", "@myKring", "@thKring", "@myQueening", "@thQueening", "@myPaths", "@thPaths", "@mine", "@theirs", "@edge", "@corners", "@board", "@myP", "@myN", "@myB", "@myR", "@myQ", "@thP", "@thN", "@thB", "@thR", "@thQ"];
const READ: [&str; 3] = ["min", "max", "sum"];
const MEET: [&str; 6] = ["minGap", "maxGap", "aFirst", "bFirst", "together", "sumGap"];

/// a position's patterns, seen by the side to move
pub struct Pat {
    g: [[i16; 64]; GRIDS],
    m: [u64; MASKS],
    v0: i8,
}

fn worth(t: usize) -> f64 {
    [1.0, 3.0, 3.0, 5.0, 9.0, 1.0][t]
}

pub fn patterns(b: &Board) -> Pat {
    let mut g = [[FAR; 64]; GRIDS];
    let mut wave = [[0.0f64; 64]; 2];
    let mut m = [0u64; MASKS];
    for s in 0..64 {
        let p = b.sq[s];
        if p == 0 {
            continue;
        }
        let mine = (p > 0) == b.white;
        let side = if mine { 0 } else { 1 };
        let t = p.unsigned_abs() as usize - 1;
        let bit = 1u64 << s;
        m[8 + side] |= bit;
        if t == 5 {
            m[side] |= bit;
            let (f, r) = ((s % 8) as i32, (s / 8) as i32);
            for df in -1..=1 {
                for dr in -1..=1 {
                    let (x, y) = (f + df, r + dr);
                    if (df, dr) != (0, 0) && (0..8).contains(&x) && (0..8).contains(&y) {
                        m[2 + side] |= 1u64 << (y * 8 + x);
                    }
                }
            }
        } else {
            m[13 + side * 5 + t] |= bit;
        }
        if t == 0 {
            let (f, mut r) = ((s % 8) as i32, (s / 8) as i32);
            let dir = if p > 0 { 1 } else { -1 };
            r += dir;
            while (0..8).contains(&r) {
                m[6 + side] |= 1u64 << (r * 8 + f);
                if r == 0 || r == 7 {
                    m[4 + side] |= 1u64 << (r * 8 + f);
                }
                r += dir;
            }
        }
        let w = times_to(b, s, PLAN);
        for q in 0..64 {
            if w[q] == UNREACHED {
                continue;
            }
            let moves = w[q] as i16;
            let ply = if moves == 0 { 0 } else if mine { 2 * moves - 1 } else { 2 * moves };
            let kind = t + side * 6;
            g[kind][q] = g[kind][q].min(ply);
            if t != 5 {
                g[12 + side][q] = g[12 + side][q].min(ply);
            }
            g[14 + side][q] = g[14 + side][q].min(ply);
            wave[side][q] += worth(t) * 0.8f64.powi(moves as i32);
        }
    }
    for q in 0..64 {
        g[16][q] = (wave[0][q] * 10.0) as i16;
        g[17][q] = (wave[1][q] * 10.0) as i16;
        let (f, r) = (q % 8, q / 8);
        if f == 0 || f == 7 || r == 0 || r == 7 {
            m[10] |= 1u64 << q;
        }
        if (f == 0 || f == 7) && (r == 0 || r == 7) {
            m[11] |= 1u64 << q;
        }
    }
    m[12] = u64::MAX;
    Pat { g, m, v0: crate::crack::intuition(b) }
}

/// the function Golden grows
#[derive(Clone, Debug)]
pub enum E {
    Num(i32),
    Intuition,
    Size(u8),
    Read(u8, u8, u8),
    Meet(u8, u8, u8, u8),
    Add(Box<E>, Box<E>),
    Sub(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
    Min(Box<E>, Box<E>),
    Max(Box<E>, Box<E>),
    If(Box<E>, Box<E>, Box<E>),
}
use E::*;

const LIMIT: i64 = 1_000_000;

impl E {
    pub fn eval(&self, p: &Pat) -> i64 {
        let v = match self {
            Num(n) => *n as i64,
            Intuition => p.v0 as i64,
            Size(m) => p.m[*m as usize].count_ones() as i64,
            Read(g, m, how) => {
                let (mut lo, mut hi, mut sum, mut any) = (i64::MAX, i64::MIN, 0i64, false);
                let mut bits = p.m[*m as usize];
                while bits != 0 {
                    let q = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    let x = p.g[*g as usize][q] as i64;
                    lo = lo.min(x);
                    hi = hi.max(x);
                    sum += x;
                    any = true;
                }
                match (*how, any) {
                    (2, _) => sum,
                    (_, false) => FAR as i64,
                    (0, _) => lo,
                    _ => hi,
                }
            }
            Meet(a, b, m, how) => {
                let (mut lo, mut hi, mut sum, mut first, mut second, mut even, mut any) = (i64::MAX, i64::MIN, 0i64, 0i64, 0i64, 0i64, false);
                let mut bits = p.m[*m as usize];
                while bits != 0 {
                    let q = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    let d = p.g[*a as usize][q] as i64 - p.g[*b as usize][q] as i64;
                    lo = lo.min(d);
                    hi = hi.max(d);
                    sum += d;
                    first += (d < 0) as i64;
                    second += (d > 0) as i64;
                    even += (d == 0) as i64;
                    any = true;
                }
                match *how {
                    0 => if any { lo } else { 0 },
                    1 => if any { hi } else { 0 },
                    2 => first,
                    3 => second,
                    4 => even,
                    _ => sum,
                }
            }
            Add(a, b) => a.eval(p) + b.eval(p),
            Sub(a, b) => a.eval(p) - b.eval(p),
            Mul(a, b) => a.eval(p) * b.eval(p),
            Min(a, b) => a.eval(p).min(b.eval(p)),
            Max(a, b) => a.eval(p).max(b.eval(p)),
            If(c, a, b) => if c.eval(p) > 0 { a.eval(p) } else { b.eval(p) },
        };
        v.clamp(-LIMIT, LIMIT)
    }

    pub fn size(&self) -> usize {
        match self {
            Num(_) | Intuition | Size(_) | Read(..) | Meet(..) => 1,
            Add(a, b) | Sub(a, b) | Mul(a, b) | Min(a, b) | Max(a, b) => 1 + a.size() + b.size(),
            If(c, a, b) => 1 + c.size() + a.size() + b.size(),
        }
    }

    pub fn show(&self) -> String {
        match self {
            Num(n) => n.to_string(),
            Intuition => "v0".to_string(),
            Size(m) => format!("count{}", MASK_NAMES[*m as usize]),
            Read(g, m, h) => format!("{}({}{})", READ[*h as usize], GRID_NAMES[*g as usize], MASK_NAMES[*m as usize]),
            Meet(a, b, m, h) => format!("{}({}|{}{})", MEET[*h as usize], GRID_NAMES[*a as usize], GRID_NAMES[*b as usize], MASK_NAMES[*m as usize]),
            Add(a, b) => format!("({} + {})", a.show(), b.show()),
            Sub(a, b) => format!("({} - {})", a.show(), b.show()),
            Mul(a, b) => format!("({} * {})", a.show(), b.show()),
            Min(a, b) => format!("min({}, {})", a.show(), b.show()),
            Max(a, b) => format!("max({}, {})", a.show(), b.show()),
            If(c, a, b) => format!("(if {} > 0 then {} else {})", c.show(), a.show(), b.show()),
        }
    }

    fn kids(&self) -> Vec<&E> {
        match self {
            Num(_) | Intuition | Size(_) | Read(..) | Meet(..) => vec![],
            Add(a, b) | Sub(a, b) | Mul(a, b) | Min(a, b) | Max(a, b) => vec![&**a, &**b],
            If(c, a, b) => vec![&**c, &**a, &**b],
        }
    }

    /// the n-th node (depth first)
    fn node(&self, n: usize) -> &E {
        if n == 0 {
            return self;
        }
        let mut n = n - 1;
        for k in self.kids() {
            let s = k.size();
            if n < s {
                return k.node(n);
            }
            n -= s;
        }
        unreachable!()
    }

    /// this tree with the n-th node replaced
    fn with(&self, n: usize, new: &E) -> E {
        if n == 0 {
            return new.clone();
        }
        let mut n = n as i64 - 1;
        let mut step = |k: &E| -> Box<E> {
            let s = k.size() as i64;
            let out = if n >= 0 && n < s { k.with(n as usize, new) } else { k.clone() };
            n -= s;
            Box::new(out)
        };
        match self {
            Num(_) | Intuition | Size(_) | Read(..) | Meet(..) => unreachable!(),
            Add(a, b) => { let x = step(a); Add(x, step(b)) }
            Sub(a, b) => { let x = step(a); Sub(x, step(b)) }
            Mul(a, b) => { let x = step(a); Mul(x, step(b)) }
            Min(a, b) => { let x = step(a); Min(x, step(b)) }
            Max(a, b) => { let x = step(a); Max(x, step(b)) }
            If(c, a, b) => { let x = step(c); let y = step(a); If(x, y, step(b)) }
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
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// may the function Golden grows read v0's answer?
static WITH_V0: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn leaf(r: &mut Rng) -> E {
    let kinds = if WITH_V0.load(std::sync::atomic::Ordering::Relaxed) { 9 } else { 8 };
    match r.below(kinds) {
        8 => Intuition,
        0 | 1 => Num([-50, -10, -3, -2, -1, 0, 1, 2, 3, 10, 50][r.below(11)]),
        2 => Size(r.below(MASKS) as u8),
        3 | 4 => Read(r.below(GRIDS) as u8, r.below(MASKS) as u8, r.below(3) as u8),
        _ => Meet(r.below(GRIDS) as u8, r.below(GRIDS) as u8, r.below(MASKS) as u8, r.below(6) as u8),
    }
}

fn grow(r: &mut Rng, depth: usize) -> E {
    if depth == 0 || r.below(5) == 0 {
        return leaf(r);
    }
    let d = depth - 1;
    let sub = |r: &mut Rng| Box::new(grow(r, d));
    match r.below(6) {
        0 => { let a = sub(r); Add(a, sub(r)) }
        1 => { let a = sub(r); Sub(a, sub(r)) }
        2 => { let a = sub(r); Mul(a, sub(r)) }
        3 => { let a = sub(r); Min(a, sub(r)) }
        4 => { let a = sub(r); Max(a, sub(r)) }
        _ => { let c = sub(r); let a = sub(r); If(c, a, sub(r)) }
    }
}

fn verdict(x: i64) -> V {
    x.signum() as V
}

/// the grown function's value of a position
pub fn value(b: &Board, f: &E) -> V {
    rules(b).unwrap_or_else(|| verdict(f.eval(&patterns(b))))
}

/// the training set seen through its patterns: (the rules' verdict, the patterns, the truth)
type Seen = Vec<(Option<V>, Pat, V)>;

const PRICE: f64 = 0.0002;
const MAX_NODES: usize = 80;

fn score(f: &E, seen: &Seen) -> f64 {
    let right = seen.iter().filter(|(ruled, p, truth)| ruled.unwrap_or_else(|| verdict(f.eval(p))) == *truth).count();
    right as f64 / seen.len() as f64 - PRICE * f.size() as f64
}

fn child(pop: &[(f64, E)], r: &mut Rng) -> E {
    let pick = |r: &mut Rng| {
        let mut best = r.below(pop.len());
        for _ in 0..3 {
            let c = r.below(pop.len());
            if pop[c].0 > pop[best].0 {
                best = c;
            }
        }
        &pop[best].1
    };
    let a = pick(r);
    let at = r.below(a.size());
    let new = match r.below(10) {
        0..=5 => {
            let b = pick(r);
            b.node(r.below(b.size())).clone()
        }
        6 | 7 => grow(r, 2),
        _ => leaf(r),
    };
    let c = a.with(at, &new);
    if c.size() > MAX_NODES { a.clone() } else { c }
}

/// Golden grows the function on train
fn grown(rounds: usize) -> Result<E, String> {
    const ISLANDS: usize = 10;
    const POP: usize = 150;
    const GENERATIONS: usize = 40;
    let train = load("bench/crack/train.tsv")?;
    let seen: Seen = train.iter().map(|(_, b, v)| (rules(b), patterns(b), *v)).collect();
    let t0 = std::time::Instant::now();
    let mut pops: Vec<Vec<(f64, E)>> = (0..ISLANDS)
        .map(|i| {
            let mut r = Rng(0x9E3779B97F4A7C15 ^ ((i as u64 + 1) << 32));
            (0..POP).map(|k| { let e = grow(&mut r, 1 + k % 4); (score(&e, &seen), e) }).collect()
        })
        .collect();
    let mut best: (f64, E) = (f64::MIN, Num(0));
    for round in 0..rounds {
        let seen = &seen;
        let done: Vec<Vec<(f64, E)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = pops.drain(..).enumerate().map(|(i, mut pop)| {
                sc.spawn(move || {
                    let mut r = Rng(0xD1B54A32D192ED03 ^ (((round * ISLANDS + i) as u64 + 1) << 24));
                    for _ in 0..GENERATIONS {
                        let top = pop.iter().cloned().fold((f64::MIN, Num(0)), |x, y| if y.0 > x.0 { y } else { x });
                        let mut next = vec![top];
                        while next.len() < POP {
                            let e = child(&pop, &mut r);
                            next.push((score(&e, seen), e));
                        }
                        pop = next;
                    }
                    pop
                })
            }).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        pops = done;
        for p in &pops {
            for x in p {
                if x.0 > best.0 {
                    best = x.clone();
                }
            }
        }
        for p in pops.iter_mut() {
            p[POP - 1] = best.clone();
        }
        println!("  round {}: training score {:.3}, {} nodes ({:.0} s)", round + 1, best.0, best.1.size(), t0.elapsed().as_secs_f64());
    }
    Ok(best.1)
}

/// `nuome --ideas crack-rest [FROM] [TO] [ROUNDS]`: the smoke positions a cracker still has wrong searching FROM plies,
/// searched TO plies, each cracker on its own misses
pub fn rest(from: usize, to: usize, rounds: usize) -> Result<(), String> {
    let smoke = load("bench/crack/smoke.tsv")?;
    let f = grown(rounds)?;
    let f = &f;
    let at = |g: &(dyn Fn(&Board) -> V + Sync), d: usize, rows: &[&(String, Board, V)]| -> Vec<V> {
        let (next, out) = (std::sync::atomic::AtomicUsize::new(0), std::sync::Mutex::new(vec![0 as V; rows.len()]));
        std::thread::scope(|sc| {
            for _ in 0..10 {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= rows.len() {
                        break;
                    }
                    let v = searched_with(&rows[i].1, d, -1, 1, g);
                    out.lock().unwrap()[i] = v;
                });
            }
        });
        out.into_inner().unwrap()
    };
    let crackers: [(&str, &(dyn Fn(&Board) -> V + Sync)); 2] = [("intuition v0", &crate::crack::intuition), ("Golden's function", &|b: &Board| value(b, f))];
    let all: Vec<&(String, Board, V)> = smoke.iter().collect();
    for (name, g) in crackers {
        let t0 = std::time::Instant::now();
        let before = at(g, from, &all);
        let missed: Vec<&(String, Board, V)> = all.iter().zip(&before).filter(|(r, v)| r.2 != **v).map(|(r, _)| *r).collect();
        let after = at(g, to, &missed);
        let solved = missed.iter().zip(&after).filter(|(r, v)| r.2 == **v).count();
        println!("\n{name}: {} of {} wrong at search {from}; at search {to} {solved} of them right, {} still wrong ({:.0} s)", missed.len(), all.len(), missed.len() - solved, t0.elapsed().as_secs_f64());
        for (r, v) in missed.iter().zip(&after) {
            println!("  {} {} {} truth {:+} answer {:+}", if r.2 == *v { "right" } else { "WRONG" }, r.0, r.1.fen(), r.2, v);
        }
    }
    Ok(())
}

/// v0 and Golden's function joined by a table of nine answers, chosen on train alone: the answer for each pair
#[derive(Clone, Copy)]
struct Joint([V; 9]);
impl Joint {
    fn at(&self, a: V, b: V) -> V {
        self.0[((a + 1) * 3 + (b + 1)) as usize]
    }
}

/// `nuome --ideas crack-both [DEPTH] [ROUNDS]`: can v0 and Golden's function be joined? Three ways, judged on smoke:
/// a table of their answers at the search's leaves; each searching alone with the disagreements drawn; a function grown
/// again that may read v0's answer as one of its patterns
pub fn both(depth: usize, rounds: usize) -> Result<(), String> {
    let train = load("bench/crack/train.tsv")?;
    let smoke = load("bench/crack/smoke.tsv")?;
    let f = grown(rounds)?;
    WITH_V0.store(true, std::sync::atomic::Ordering::Relaxed);
    let g = grown(rounds)?;
    WITH_V0.store(false, std::sync::atomic::Ordering::Relaxed);
    println!("\nGrown with v0 ({} nodes): {}", g.size(), g.show());
    let (f, g) = (&f, &g);
    // every table of nine answers, the best on train (ties: the one closest to v0's own answers)
    let pairs: Vec<(V, V, V)> = train.iter().map(|(_, b, v)| (crate::crack::intuition(b), value(b, f), *v)).collect();
    let mut counts = [[0usize; 3]; 9];
    for (a, b, v) in &pairs {
        counts[((a + 1) * 3 + (b + 1)) as usize][(v + 1) as usize] += 1;
    }
    let mut joint = Joint([0; 9]);
    for k in 0..9 {
        let v0 = (k / 3) as V - 1;
        let top = *counts[k].iter().max().unwrap();
        joint.0[k] = if counts[k][(v0 + 1) as usize] == top { v0 } else { (0..3).find(|&o| counts[k][o] == top).unwrap() as V - 1 };
    }
    println!("\nThe table Golden chose on train (v0, function -> answer), with the truth's counts [loss draw win]:");
    for k in 0..9 {
        println!("  {:+} {:+} -> {:+}   {:?}", (k / 3) as i32 - 1, (k % 3) as i32 - 1, joint.0[k], counts[k]);
    }
    let joined = |b: &Board| joint.at(crate::crack::intuition(b), value(b, f));
    let joined = &joined;
    let judge = |name: &str, h: &(dyn Fn(&Board) -> V + Sync)| {
        let right: Vec<bool> = std::thread::scope(|sc| {
            let hs: Vec<_> = smoke.chunks(smoke.len().div_ceil(10)).map(|c| sc.spawn(move || c.iter().map(|r| h(&r.1) == r.2).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut by: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for (r, ok) in smoke.iter().zip(&right) {
            let e = by.entry(r.0.as_str()).or_default();
            e.0 += *ok as usize;
            e.1 += 1;
        }
        let total = right.iter().filter(|x| **x).count();
        let mut classes: Vec<&str> = smoke.iter().map(|r| r.0.as_str()).collect();
        classes.dedup();
        println!("{name}: {total}/{} ({:.1}%)\n    {}", smoke.len(), 100.0 * total as f64 / smoke.len() as f64, classes.iter().filter(|c| by[**c].0 < by[**c].1).map(|c| format!("{c} {}", by[c].0)).collect::<Vec<_>>().join("  "));
    };
    let v0 = |b: &Board| crate::crack::intuition(b);
    let v0 = &v0;
    let one = |b: &Board| value(b, f);
    let one = &one;
    let two = |b: &Board| value(b, g);
    let two = &two;
    for d in 0..=depth {
        println!("\nsearch {d} (the classes not full, n = 25 each):");
        judge("  v0", &|b: &Board| searched_with(b, d, -1, 1, v0));
        judge("  the function", &|b: &Board| searched_with(b, d, -1, 1, one));
        judge("  joined by the table at the leaves", &|b: &Board| searched_with(b, d, -1, 1, joined));
        judge("  each alone, disagreement a draw", &|b: &Board| {
            let (x, y) = (searched_with(b, d, -1, 1, v0), searched_with(b, d, -1, 1, one));
            if x == y { x } else { 0 }
        });
        judge("  grown with v0", &|b: &Board| searched_with(b, d, -1, 1, two));
    }
    Ok(())
}

/// `nuome --ideas crack-golden [DEPTH] [ROUNDS]`: Golden grows the function on train; judged on smoke
pub fn run(depth: usize, rounds: usize) -> Result<(), String> {
    let smoke = load("bench/crack/smoke.tsv")?;
    let f = grown(rounds)?;
    println!("\nGolden's function ({} nodes; > 0 a win, < 0 a loss, 0 a draw for the side to move):\n{}", f.size(), f.show());
    let f = &f;
    let judge = |name: &str, g: &(dyn Fn(&Board) -> V + Sync)| {
        let right: Vec<bool> = std::thread::scope(|sc| {
            let hs: Vec<_> = smoke.chunks(smoke.len().div_ceil(10)).map(|c| sc.spawn(move || c.iter().map(|r| g(&r.1) == r.2).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut by: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for (r, ok) in smoke.iter().zip(&right) {
            let e = by.entry(r.0.as_str()).or_default();
            e.0 += *ok as usize;
            e.1 += 1;
        }
        let total = right.iter().filter(|x| **x).count();
        let mut classes: Vec<&str> = smoke.iter().map(|r| r.0.as_str()).collect();
        classes.dedup();
        println!("\n{name}: {total}/{} ({:.1}%)\n  {}", smoke.len(), 100.0 * total as f64 / smoke.len() as f64, classes.iter().map(|c| format!("{c} {}/{}", by[c].0, by[c].1)).collect::<Vec<_>>().join("  "));
    };
    judge("intuition v0 (yardstick)", &crate::crack::intuition);
    judge("Golden's function on the patterns", &|b: &Board| value(b, f));
    for d in 1..=depth {
        judge(&format!("intuition v0 + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &crate::crack::intuition));
        judge(&format!("Golden's function + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &|x: &Board| value(x, f)));
    }
    Ok(())
}
