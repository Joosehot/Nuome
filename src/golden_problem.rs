//! Golden invents new open problems (the owner's: invent them, not solve those already invented).
//!
//! A problem is a function Golden grows. F reads what two points of a finite structure see of each other (a tree of
//! + - * min max if abs over the readings; mod is left out, what it wrote were parity tricks), and the question it writes is: for which n does a structure on n points
//! exist with F(x, y) = 0 for every two distinct points? F reads no names, so its answer cannot depend on them.
//!
//! The structures are graphs and tournaments. Permutations and maps f: S -> S can be asked (`Map` stays for the tests)
//! and are not: their pair conditions come down to cycle types and in-degrees, a count by hand (golden_god found no
//! problem among permutations either; of the maps kept in a first run, those worked by hand were all parity).
//!
//! Deciding is only the sieve. A short exact search (forcing by intervals over F, one structure of each renaming class
//! at most kept apart by swapping neighbours) gives each n one of three: built and checked, impossible (the search ran
//! to its end), undecided; what it leaves undecided a climb then tries to build (flip a cell, keep what breaks fewer
//! pairs). What Golden keeps has real structures, none of them a plain one (empty, cycle, ...), at least one impossible
//! n, and an n neither could decide; questions with the same answers on every n are one question, written shortest.
//! Open means open here: undecided by both within their fixed number of steps. Whether anyone has asked or settled the
//! same question is not checked.
//!
//! Then Nuome solves what Golden wrote, with nothing from outside: the parts a structure can hold are grown point by
//! point (`parts`). A part with a pair F can never bring to 0 is in no structure, so when no part of d points is left no
//! structure has d points or more, and below that every structure is among the parts, tried whole. A problem whose parts
//! run out is solved for every n; one whose parts grow too many says how far it got.
//!
//! No clock in it: steps are counted and seeds fixed, so the same run writes the same problems.
use std::collections::BTreeMap;

const MAX_N: usize = 32;
const LIMIT: i64 = 1_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Graph,
    Tournament,
    Map,
}
use Class::*;

pub const CLASSES: [Class; 3] = [Graph, Tournament, Map];
/// the classes Golden writes problems in
const ASKED: [Class; 2] = [Graph, Tournament];

const READS: usize = 10;
const A: u8 = 0;
const B: u8 = 1;
const OUT_X: u8 = 2;
const OUT_Y: u8 = 3;
const IN_X: u8 = 4;
const IN_Y: u8 = 5;
const COMMON_OUT: u8 = 6;
const COMMON_IN: u8 = 7;
const PATHS: u8 = 8;
const SIZE: u8 = 9;

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Graph => "graph",
            Tournament => "tournament",
            Map => "map f: S -> S",
        }
    }
    /// the readings that say something in this class, with their names
    fn reads(self) -> &'static [(u8, &'static str)] {
        match self {
            Graph => &[(A, "adj(x,y)"), (OUT_X, "deg(x)"), (OUT_Y, "deg(y)"), (COMMON_OUT, "common(x,y)"), (SIZE, "n")],
            Tournament => &[(A, "beats(x,y)"), (OUT_X, "out(x)"), (OUT_Y, "out(y)"), (COMMON_OUT, "both_beat(x,y)"), (COMMON_IN, "beat_both(x,y)"), (PATHS, "paths(x,y)"), (SIZE, "n")],
            Map => &[(A, "[f(x)=y]"), (B, "[f(y)=x]"), (IN_X, "in(x)"), (IN_Y, "in(y)"), (COMMON_OUT, "[f(x)=f(y)]"), (PATHS, "[f(f(x))=y]"), (SIZE, "n")],
        }
    }
    /// how far Golden looks while growing
    fn grow_to(self) -> usize {
        match self {
            Graph => 11,
            Tournament => 9,
            Map => 9,
        }
    }
    fn explain(self) -> &'static str {
        match self {
            Graph => "adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both",
            Tournament => "beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y",
            Map => "[...] is 1 when it holds and 0 when not, in(x) counts the z with f(z) = x",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum E {
    Num(i64),
    Read(u8),
    Add(Box<E>, Box<E>),
    Sub(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
    Min(Box<E>, Box<E>),
    Max(Box<E>, Box<E>),
    If(Box<E>, Box<E>, Box<E>),
    Abs(Box<E>),
}
use E::*;

fn clamp(x: i64) -> i64 {
    x.clamp(-LIMIT, LIMIT)
}

impl E {
    /// the value on readings all known
    fn val(&self, r: &[i64; READS]) -> i64 {
        match self {
            Num(n) => *n,
            Read(i) => r[*i as usize],
            Add(a, b) => clamp(a.val(r) + b.val(r)),
            Sub(a, b) => clamp(a.val(r) - b.val(r)),
            Mul(a, b) => clamp(a.val(r) * b.val(r)),
            Min(a, b) => a.val(r).min(b.val(r)),
            Max(a, b) => a.val(r).max(b.val(r)),
            If(c, a, b) => {
                if c.val(r) != 0 {
                    a.val(r)
                } else {
                    b.val(r)
                }
            }
            Abs(a) => a.val(r).abs(),
        }
    }

    /// every value it can take while each reading stays in its range
    fn range(&self, r: &[(i64, i64); READS]) -> (i64, i64) {
        match self {
            Num(n) => (*n, *n),
            Read(i) => r[*i as usize],
            Add(a, b) => {
                let (x, y) = (a.range(r), b.range(r));
                (clamp(x.0 + y.0), clamp(x.1 + y.1))
            }
            Sub(a, b) => {
                let (x, y) = (a.range(r), b.range(r));
                (clamp(x.0 - y.1), clamp(x.1 - y.0))
            }
            Mul(a, b) => {
                let (x, y) = (a.range(r), b.range(r));
                let c = [x.0 * y.0, x.0 * y.1, x.1 * y.0, x.1 * y.1];
                (clamp(*c.iter().min().unwrap()), clamp(*c.iter().max().unwrap()))
            }
            Min(a, b) => {
                let (x, y) = (a.range(r), b.range(r));
                (x.0.min(y.0), x.1.min(y.1))
            }
            Max(a, b) => {
                let (x, y) = (a.range(r), b.range(r));
                (x.0.max(y.0), x.1.max(y.1))
            }
            If(c, a, b) => {
                let t = c.range(r);
                if t == (0, 0) {
                    b.range(r)
                } else if t.0 > 0 || t.1 < 0 {
                    a.range(r)
                } else {
                    let (x, y) = (a.range(r), b.range(r));
                    (x.0.min(y.0), x.1.max(y.1))
                }
            }
            Abs(a) => {
                let x = a.range(r);
                if x.0 >= 0 {
                    x
                } else if x.1 <= 0 {
                    (-x.1, -x.0)
                } else {
                    (0, x.1.max(-x.0))
                }
            }
        }
    }

    fn kids(&self) -> Vec<&E> {
        match self {
            Num(_) | Read(_) => vec![],
            Add(a, b) | Sub(a, b) | Mul(a, b) | Min(a, b) | Max(a, b) => vec![a, b],
            If(c, a, b) => vec![c, a, b],
            Abs(a) => vec![a],
        }
    }

    /// the same node over other children
    fn over(&self, k: Vec<E>) -> E {
        let mut k = k.into_iter().map(Box::new);
        let mut n = || k.next().unwrap();
        match self {
            Num(_) | Read(_) => self.clone(),
            Add(..) => { let a = n(); Add(a, n()) }
            Sub(..) => { let a = n(); Sub(a, n()) }
            Mul(..) => { let a = n(); Mul(a, n()) }
            Min(..) => { let a = n(); Min(a, n()) }
            Max(..) => { let a = n(); Max(a, n()) }
            If(..) => { let c = n(); let a = n(); If(c, a, n()) }
            Abs(_) => Abs(n()),
        }
    }

    pub fn size(&self) -> usize {
        1 + self.kids().iter().map(|k| k.size()).sum::<usize>()
    }

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
        let kids = self
            .kids()
            .into_iter()
            .map(|k| {
                let s = k.size() as i64;
                let out = if n >= 0 && n < s { k.with(n as usize, new) } else { k.clone() };
                n -= s;
                out
            })
            .collect();
        self.over(kids)
    }

    /// the same function, written shorter: what reads nothing is computed, the plain identities removed
    pub fn tidy(&self) -> E {
        let e = self.over(self.kids().into_iter().map(|k| k.tidy()).collect());
        let kids = e.kids();
        if !kids.is_empty() && kids.iter().all(|k| matches!(k, Num(_))) {
            return Num(e.val(&[0; READS]));
        }
        match &e {
            Add(a, b) if **a == Num(0) => (**b).clone(),
            Add(a, b) | Sub(a, b) if **b == Num(0) => (**a).clone(),
            Mul(a, b) if **a == Num(1) => (**b).clone(),
            Mul(a, b) if **b == Num(1) => (**a).clone(),
            Mul(a, b) if **a == Num(0) || **b == Num(0) => Num(0),
            If(c, a, b) if matches!(**c, Num(_)) => if **c == Num(0) { (**b).clone() } else { (**a).clone() },
            If(_, a, b) | Min(a, b) | Max(a, b) if a == b => (**a).clone(),
            Abs(a) if matches!(**a, Abs(_)) => (**a).clone(),
            _ => e,
        }
    }

    pub fn show(&self, class: Class) -> String {
        let two = |a: &E, o: &str, b: &E| format!("({} {o} {})", a.show(class), b.show(class));
        match self {
            Num(n) => n.to_string(),
            Read(i) => class.reads().iter().find(|r| r.0 == *i).map_or("?", |r| r.1).to_string(),
            Add(a, b) => two(a, "+", b),
            Sub(a, b) => two(a, "-", b),
            Mul(a, b) => two(a, "*", b),
            Min(a, b) => format!("min({}, {})", a.show(class), b.show(class)),
            Max(a, b) => format!("max({}, {})", a.show(class), b.show(class)),
            If(c, a, b) => format!("(if {} != 0 then {} else {})", c.show(class), a.show(class), b.show(class)),
            Abs(a) => format!("|{}|", a.show(class)),
        }
    }
}

// ---------------------------------------------------------------- one structure, whole

/// the readings of two points of a whole structure, counted one by one (the search's masks are not used here)
fn readings(rows: &[u32], x: usize, y: usize) -> [i64; READS] {
    let n = rows.len();
    let at = |i: usize, j: usize| (rows[i] >> j & 1) as i64;
    let mut r = [0i64; READS];
    r[A as usize] = at(x, y);
    r[B as usize] = at(y, x);
    r[SIZE as usize] = n as i64;
    for z in 0..n {
        r[OUT_X as usize] += at(x, z);
        r[OUT_Y as usize] += at(y, z);
        r[IN_X as usize] += at(z, x);
        r[IN_Y as usize] += at(z, y);
        r[COMMON_OUT as usize] += at(x, z) * at(y, z);
        r[COMMON_IN as usize] += at(z, x) * at(z, y);
        r[PATHS as usize] += at(x, z) * at(z, y);
    }
    r
}

/// is this a structure of the class at all?
fn of_class(class: Class, rows: &[u32]) -> bool {
    let n = rows.len();
    let at = |i: usize, j: usize| rows[i] >> j & 1 == 1;
    match class {
        Graph => (0..n).all(|i| !at(i, i) && (0..n).all(|j| at(i, j) == at(j, i))),
        Tournament => (0..n).all(|i| !at(i, i) && (0..n).all(|j| i == j || at(i, j) != at(j, i))),
        Map => rows.iter().all(|r| r.count_ones() == 1),
    }
}

/// checked from scratch: the class's own shape, and F = 0 on every two distinct points
pub fn holds(f: &E, class: Class, rows: &[u32]) -> bool {
    let n = rows.len();
    of_class(class, rows) && (0..n).all(|x| (0..n).all(|y| x == y || f.val(&readings(rows, x, y)) == 0))
}

/// the plain structures of a class: one of these answering makes that n no problem
fn plain(class: Class, n: usize) -> Vec<(&'static str, Vec<u32>)> {
    let matrix = |g: &dyn Fn(usize, usize) -> bool| -> Vec<u32> { (0..n).map(|i| (0..n).fold(0u32, |r, j| r | (g(i, j) as u32) << j)).collect() };
    let mut out: Vec<(&'static str, Vec<u32>)> = Vec::new();
    match class {
        Graph => {
            out.push(("empty", matrix(&|_, _| false)));
            out.push(("complete", matrix(&|i, j| i != j)));
            out.push(("cycle", matrix(&|i, j| i != j && ((i + 1) % n == j || (j + 1) % n == i))));
            out.push(("the complement of a cycle", matrix(&|i, j| i != j && (i + 1) % n != j && (j + 1) % n != i)));
            out.push(("path", matrix(&|i, j| i + 1 == j || j + 1 == i)));
            out.push(("star", matrix(&|i, j| i != j && (i == 0 || j == 0))));
            out.push(("matching", matrix(&|i, j| i != j && i / 2 == j / 2)));
            out.push(("complete bipartite", matrix(&|i, j| i % 2 != j % 2)));
            out.push(("triangles", matrix(&|i, j| i != j && i / 3 == j / 3)));
        }
        Tournament => {
            out.push(("transitive", matrix(&|i, j| i < j)));
            if n % 2 == 1 {
                out.push(("rotational", matrix(&|i, j| i != j && (j + n - i) % n <= n / 2)));
            }
        }
        Map => {
            out.push(("identity", matrix(&|i, j| i == j)));
            out.push(("constant", matrix(&|_, j| j == 0)));
            out.push(("one cycle", matrix(&|i, j| (i + 1) % n == j)));
            out.push(("swaps", matrix(&|i, j| if n % 2 == 1 && i == n - 1 { j == i } else { (i ^ 1) == j })));
            out.push(("a path into a fixed point", matrix(&|i, j| j == i.saturating_sub(1))));
        }
    }
    out.retain(|(_, rows)| of_class(class, rows));
    out
}

// ---------------------------------------------------------------- the search that sieves

/// a partly known structure: `one` holds the cells known 1, `may` the cells not known 0, by rows and by columns
#[derive(Clone, Copy)]
struct St {
    one: [u32; MAX_N],
    may: [u32; MAX_N],
    ione: [u32; MAX_N],
    imay: [u32; MAX_N],
}

struct Sieve<'a> {
    f: &'a E,
    class: Class,
    n: usize,
}

enum Found {
    Built(Vec<u32>),
    Empty,
    OutOfSteps,
}

impl Sieve<'_> {
    fn start(&self) -> Option<St> {
        let full = ((1u64 << self.n) - 1) as u32;
        let mut s = St { one: [0; MAX_N], may: [0; MAX_N], ione: [0; MAX_N], imay: [0; MAX_N] };
        for i in 0..self.n {
            s.may[i] = full;
            s.imay[i] = full;
        }
        if self.class != Map {
            for i in 0..self.n {
                s.may[i] &= !(1 << i);
                s.imay[i] &= !(1 << i);
            }
        }
        self.pairs_ok(&s, full).then_some(s)
    }

    fn ranges(&self, s: &St, x: usize, y: usize) -> [(i64, i64); READS] {
        let cell = |r: u32, i: usize, m: u32| ((r >> i & 1) as i64, (m >> i & 1) as i64);
        let c = |a: u32, b: u32| (a & b).count_ones() as i64;
        let mut r = [(0, 0); READS];
        r[A as usize] = cell(s.one[x], y, s.may[x]);
        r[B as usize] = cell(s.one[y], x, s.may[y]);
        r[OUT_X as usize] = (s.one[x].count_ones() as i64, s.may[x].count_ones() as i64);
        r[OUT_Y as usize] = (s.one[y].count_ones() as i64, s.may[y].count_ones() as i64);
        r[IN_X as usize] = (s.ione[x].count_ones() as i64, s.imay[x].count_ones() as i64);
        r[IN_Y as usize] = (s.ione[y].count_ones() as i64, s.imay[y].count_ones() as i64);
        r[COMMON_OUT as usize] = (c(s.one[x], s.one[y]), c(s.may[x], s.may[y]));
        r[COMMON_IN as usize] = (c(s.ione[x], s.ione[y]), c(s.imay[x], s.imay[y]));
        r[PATHS as usize] = (c(s.one[x], s.ione[y]), c(s.may[x], s.imay[y]));
        r[SIZE as usize] = (self.n as i64, self.n as i64);
        r
    }

    /// F can still be 0 on every pair a touched point is in
    fn pairs_ok(&self, s: &St, touched: u32) -> bool {
        for x in 0..self.n {
            for y in 0..self.n {
                if x != y && (touched >> x & 1 == 1 || touched >> y & 1 == 1) {
                    let (l, h) = self.f.range(&self.ranges(s, x, y));
                    if l > 0 || h < 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// a cell known, with what the class's shape then forces
    fn set(&self, s: &mut St, u: usize, v: usize, b: bool, touched: &mut u32) -> bool {
        let bit = 1u32 << v;
        if b {
            if s.may[u] & bit == 0 {
                return false;
            }
            if s.one[u] & bit != 0 {
                return true;
            }
            s.one[u] |= bit;
            s.ione[v] |= 1 << u;
        } else {
            if s.one[u] & bit != 0 {
                return false;
            }
            if s.may[u] & bit == 0 {
                return true;
            }
            s.may[u] &= !bit;
            s.imay[v] &= !(1 << u);
        }
        *touched |= 1 << u | 1 << v;
        match self.class {
            Graph => self.set(s, v, u, b, touched),
            Tournament => self.set(s, v, u, !b, touched),
            Map => {
                if b {
                    (0..self.n).all(|w| w == v || self.set(s, u, w, false, touched))
                } else if s.may[u] == 0 {
                    false
                } else if s.may[u].count_ones() == 1 {
                    self.set(s, u, s.may[u].trailing_zeros() as usize, true, touched)
                } else {
                    true
                }
            }
        }
    }

    /// no swap of two neighbouring points may give a larger reading of the cells: of each renaming class the largest
    /// survives, so the cut never loses a class (test: a_cut_never_changes_a_verdict)
    fn largest(&self, s: &St) -> bool {
        let cell = |a: usize, b: usize| -> Option<bool> {
            if s.one[a] >> b & 1 == 1 {
                Some(true)
            } else if s.may[a] >> b & 1 == 0 {
                Some(false)
            } else {
                None
            }
        };
        for i in 0..self.n - 1 {
            let swap = |p: usize| if p == i { i + 1 } else if p == i + 1 { i } else { p };
            'cells: for a in 0..self.n {
                for b in 0..self.n {
                    let (c, d) = (swap(a), swap(b));
                    if (a, b) == (c, d) {
                        continue;
                    }
                    match (cell(a, b), cell(c, d)) {
                        (Some(x), Some(y)) if x == y => {}
                        (Some(false), Some(true)) => return false,
                        _ => break 'cells,
                    }
                }
            }
        }
        true
    }

    fn free_cell(&self, s: &St) -> Option<(usize, usize)> {
        (0..self.n).find_map(|u| {
            let free = s.may[u] & !s.one[u];
            (free != 0).then(|| (u, free.trailing_zeros() as usize))
        })
    }

    fn dfs(&self, s: &St, steps: &mut u64, budget: u64) -> Found {
        if *steps >= budget {
            return Found::OutOfSteps;
        }
        *steps += 1;
        let Some((u, v)) = self.free_cell(s) else {
            return Found::Built(s.one[..self.n].to_vec());
        };
        let mut out = false;
        for b in [true, false] {
            let mut t = *s;
            let mut touched = 0;
            if self.set(&mut t, u, v, b, &mut touched) && self.pairs_ok(&t, touched) && self.largest(&t) {
                match self.dfs(&t, steps, budget) {
                    Found::Empty => {}
                    Found::OutOfSteps => out = true,
                    built => return built,
                }
            }
        }
        if out { Found::OutOfSteps } else { Found::Empty }
    }

    /// every free cell tried both ways: a value that fails at once is out, until nothing moves. False when a cell has no
    /// value left. Whatever it removes holds no structure at all, so it may stand beside the swaps' cut.
    fn force(&self, s: &mut St) -> bool {
        loop {
            let mut moved = false;
            for u in 0..self.n {
                let mut free = s.may[u] & !s.one[u];
                if self.class != Map {
                    free &= !((1u32 << u) - 1);
                }
                while free != 0 {
                    let v = free.trailing_zeros() as usize;
                    free &= free - 1;
                    if s.may[u] >> v & 1 == 0 || s.one[u] >> v & 1 == 1 {
                        continue;
                    }
                    let can = |b: bool| {
                        let (mut t, mut touched) = (*s, 0);
                        self.set(&mut t, u, v, b, &mut touched) && self.pairs_ok(&t, touched)
                    };
                    let (yes, no) = (can(true), can(false));
                    if !yes && !no {
                        return false;
                    }
                    if yes != no {
                        let mut touched = 0;
                        if !self.set(s, u, v, yes, &mut touched) || !self.pairs_ok(s, touched) {
                            return false;
                        }
                        moved = true;
                    }
                }
            }
            if !moved {
                return true;
            }
        }
    }

    /// the search Nuome solves with: forcing at every step, then the cell of the point with the fewest left
    fn deep(&self, s: &St, steps: &mut u64, budget: u64) -> Found {
        if *steps >= budget {
            return Found::OutOfSteps;
        }
        *steps += 1;
        let mut t = *s;
        if !self.force(&mut t) || !self.largest(&t) {
            return Found::Empty;
        }
        let point = (0..self.n).filter(|&u| t.may[u] & !t.one[u] != 0).min_by_key(|&u| (t.may[u] & !t.one[u]).count_ones());
        let Some(u) = point else {
            return Found::Built(t.one[..self.n].to_vec());
        };
        let v = (t.may[u] & !t.one[u]).trailing_zeros() as usize;
        let mut out = false;
        for b in [true, false] {
            let mut c = t;
            let mut touched = 0;
            if self.set(&mut c, u, v, b, &mut touched) && self.pairs_ok(&c, touched) {
                match self.deep(&c, steps, budget) {
                    Found::Empty => {}
                    Found::OutOfSteps => out = true,
                    built => return built,
                }
            }
        }
        if out { Found::OutOfSteps } else { Found::Empty }
    }

    /// what forcing alone gives from nothing: a cell is known when its other value fails at once, until nothing moves;
    /// (cells known, cells in all), None when a cell has no value left
    fn forced(&self) -> Option<(usize, usize)> {
        let mut s = self.start()?;
        let all = |s: &St| (0..self.n).map(|u| (s.may[u] & !s.one[u]).count_ones() as usize).sum::<usize>();
        let before = all(&s);
        loop {
            let mut moved = false;
            for u in 0..self.n {
                for v in 0..self.n {
                    if s.may[u] >> v & 1 == 0 || s.one[u] >> v & 1 == 1 {
                        continue;
                    }
                    let can = |b: bool| {
                        let (mut t, mut touched) = (s, 0);
                        self.set(&mut t, u, v, b, &mut touched) && self.pairs_ok(&t, touched)
                    };
                    let (yes, no) = (can(true), can(false));
                    if !yes && !no {
                        return None;
                    }
                    if yes != no {
                        let mut touched = 0;
                        self.set(&mut s, u, v, yes, &mut touched);
                        moved = true;
                    }
                }
            }
            if !moved {
                return Some((before - all(&s), before));
            }
        }
    }
}

/// the degrees F leaves a point: d is out when F cannot be 0 on a pair where the point has degree d, whatever the rest is
fn degrees_left(f: &E, class: Class, n: usize) -> Vec<usize> {
    let top = n as i64;
    let can = |d: usize, role_x: bool| {
        let mut r = [(0, top); READS];
        r[A as usize] = (0, 1);
        r[B as usize] = (0, 1);
        r[SIZE as usize] = (top, top);
        let d = d as i64;
        let (out_at, in_at) = if role_x { (OUT_X, IN_X) } else { (OUT_Y, IN_Y) };
        match class {
            Graph => {
                r[out_at as usize] = (d, d);
                r[in_at as usize] = (d, d);
            }
            Tournament => {
                r[out_at as usize] = (d, d);
                r[in_at as usize] = (top - 1 - d, top - 1 - d);
            }
            Map => r[in_at as usize] = (d, d),
        }
        let (l, h) = f.range(&r);
        l <= 0 && h >= 0
    };
    let most = if class == Map { n } else { n - 1 };
    (0..=most).filter(|&d| can(d, true) && can(d, false)).collect()
}

/// the sum of n degrees the class demands, against the degrees F leaves: Graph an even sum, Tournament n(n-1)/2, Map n.
/// True when no n allowed degrees add up to it.
fn sum_forbids(f: &E, class: Class, n: usize) -> bool {
    let left = degrees_left(f, class, n);
    let top = n * n;
    let mut can = vec![false; top + 1];
    can[0] = true;
    for _ in 0..n {
        let mut next = vec![false; top + 1];
        for (s, _) in can.iter().enumerate().filter(|x| *x.1) {
            for d in &left {
                if s + d <= top {
                    next[s + d] = true;
                }
            }
        }
        can = next;
    }
    match class {
        Graph => !can.iter().step_by(2).any(|x| *x),
        Tournament => !can[n * (n - 1) / 2],
        Map => !can[n],
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    /// a plain structure answers this n: no problem here
    Plain(&'static str),
    /// built, and checked from scratch
    Built(Vec<u32>),
    /// the search ran to its end (0 steps: forced, the degrees F leaves cannot make the sum the class demands)
    Impossible(u64),
    /// out of steps
    Undecided(u64),
}

pub fn decide(f: &E, class: Class, n: usize, budget: u64) -> Verdict {
    for (name, rows) in plain(class, n) {
        if holds(f, class, &rows) {
            return Verdict::Plain(name);
        }
    }
    if sum_forbids(f, class, n) {
        return Verdict::Impossible(0);
    }
    let sieve = Sieve { f, class, n };
    let Some(s) = sieve.start() else {
        return Verdict::Impossible(0);
    };
    let mut steps = 0;
    match sieve.dfs(&s, &mut steps, budget) {
        Found::Built(rows) => {
            assert!(holds(f, class, &rows), "a structure the search built fails the check: a bug in the search");
            Verdict::Built(rows)
        }
        Found::Empty => Verdict::Impossible(steps),
        Found::OutOfSteps => Verdict::Undecided(steps),
    }
}

/// Nuome solves one n of a problem: the same verdicts, by the search that forces at every step
pub fn solve_n(f: &E, class: Class, n: usize, budget: u64) -> Verdict {
    for (name, rows) in plain(class, n) {
        if holds(f, class, &rows) {
            return Verdict::Plain(name);
        }
    }
    if sum_forbids(f, class, n) {
        return Verdict::Impossible(0);
    }
    let sieve = Sieve { f, class, n };
    let Some(s) = sieve.start() else {
        return Verdict::Impossible(0);
    };
    let mut steps = 0;
    match sieve.deep(&s, &mut steps, budget) {
        Found::Built(rows) => {
            assert!(holds(f, class, &rows), "a structure the search built fails the check: a bug in the search");
            Verdict::Built(rows)
        }
        Found::Empty => Verdict::Impossible(steps),
        Found::OutOfSteps => Verdict::Undecided(steps),
    }
}

/// every structure of the class on n points, tried one by one with no search; None when too many to try
fn enumerate(f: &E, class: Class, n: usize) -> Option<usize> {
    let rows_of = |code: u64| -> Vec<u32> {
        let mut r = vec![0u32; n];
        match class {
            Map => {
                let mut c = code;
                for x in r.iter_mut() {
                    *x = 1 << (c % n as u64);
                    c /= n as u64;
                }
            }
            _ => {
                let mut k = 0;
                for i in 0..n {
                    for j in i + 1..n {
                        let b = code >> k & 1 == 1;
                        k += 1;
                        if b {
                            r[i] |= 1 << j;
                        }
                        if (b && class == Graph) || (!b && class == Tournament) {
                            r[j] |= 1 << i;
                        }
                    }
                }
            }
        }
        r
    };
    let total = match class {
        Map => (n as u64).checked_pow(n as u32)?,
        _ => 1u64.checked_shl((n * (n - 1) / 2) as u32)?,
    };
    (total <= 1 << 17).then(|| (0..total).filter(|&c| holds(f, class, &rows_of(c))).count())
}

// ---------------------------------------------------------------- Nuome solves: the parts a structure can hold

/// points outside a part: as many as you like
const OUTSIDE: i64 = 1_000_000;

/// can F still be 0 on this pair of a part, whatever points come later? What is counted can only grow, the part's edges
/// between its own points stay; a range only widens when points are taken away, so a part of a good structure is good
fn part_pair_ok(f: &E, rows: &[u32], x: usize, y: usize) -> bool {
    let r = readings(rows, x, y);
    let mut ranges = [(0, 0); READS];
    for i in 0..READS {
        ranges[i] = (r[i], r[i] + OUTSIDE);
    }
    ranges[A as usize] = (r[A as usize], r[A as usize]);
    ranges[B as usize] = (r[B as usize], r[B as usize]);
    let (l, h) = f.range(&ranges);
    l <= 0 && h >= 0
}

/// the new point's edges to the old ones, one at a time, dropping a part as soon as a pair fails
fn extend(f: &E, class: Class, rows: &mut Vec<u32>, at: usize, out: &mut Vec<Vec<u32>>) {
    let new = rows.len() - 1;
    if at == new {
        out.push(rows.clone());
        return;
    }
    for choice in [false, true] {
        match (class, choice) {
            (Tournament, true) => rows[new] |= 1 << at,
            (Tournament, false) => rows[at] |= 1 << new,
            (_, true) => {
                rows[new] |= 1 << at;
                rows[at] |= 1 << new;
            }
            _ => {}
        }
        // the pairs this edge can move, among the points whose edge to the new one is known
        let known = |p: usize| p <= at || p == new;
        let ok = (0..=new).filter(|&p| known(p)).all(|p| (p == at || part_pair_ok(f, rows, at, p) && part_pair_ok(f, rows, p, at)) && (p == new || (part_pair_ok(f, rows, new, p) && part_pair_ok(f, rows, p, new))));
        if ok {
            extend(f, class, rows, at + 1, out);
        }
        rows[new] &= !(1 << at);
        rows[at] &= !(1 << new);
    }
}

/// a part written in an order that depends little on its names: the same writing means the same structure, so dropping
/// the same writing never loses one (two writings of one structure only cost time)
fn canon(rows: &[u32]) -> Vec<u32> {
    let n = rows.len();
    let at = |i: usize, j: usize| (rows[i] >> j & 1) as u64;
    let mut key: Vec<u64> = (0..n).map(|i| (0..n).map(|j| at(i, j) * 1_000 + at(j, i)).sum()).collect();
    for _ in 0..3 {
        key = (0..n)
            .map(|i| {
                let mut seen: Vec<u64> = (0..n).filter(|&j| j != i).map(|j| key[j].wrapping_mul(4).wrapping_add(at(i, j) * 2 + at(j, i))).collect();
                seen.sort_unstable();
                seen.iter().fold(key[i].wrapping_mul(0x100000001B3), |h, s| (h ^ s).wrapping_mul(0x9E3779B97F4A7C15))
            })
            .collect();
    }
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (key[i], i));
    order.iter().map(|&i| order.iter().enumerate().fold(0u32, |r, (b, &j)| r | ((rows[i] >> j & 1) << b))).collect()
}

/// what the parts say of a problem
pub struct Parts {
    /// how many parts of each size, from 1
    pub sizes: Vec<usize>,
    /// the n decided, each with a structure or none: exact, every structure on n points is among the parts of size n
    pub decided: Vec<(usize, Option<Vec<u32>>)>,
    /// no part of this size exists, so no structure on this many points or more
    pub dies_at: Option<usize>,
}

const PARTS_MAX: usize = 100_000;

/// every part a structure can hold, size by size, until none is left or there are too many to hold
pub fn parts(f: &E, class: Class, most: usize) -> Parts {
    let mut level: Vec<Vec<u32>> = vec![vec![0]];
    let mut out = Parts { sizes: vec![1], decided: Vec::new(), dies_at: None };
    for size in 2..MAX_N {
        let chunk = level.len().div_ceil(12);
        let found: Vec<Vec<Vec<u32>>> = std::thread::scope(|sc| {
            let hs: Vec<_> = level
                .chunks(chunk)
                .map(|c| {
                    sc.spawn(move || {
                        let mut got = Vec::new();
                        for part in c {
                            let mut rows = part.clone();
                            rows.push(0);
                            extend(f, class, &mut rows, 0, &mut got);
                        }
                        got.iter().map(|r| canon(r)).collect::<Vec<_>>()
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let next: std::collections::BTreeSet<Vec<u32>> = found.into_iter().flatten().collect();
        out.sizes.push(next.len());
        if next.is_empty() {
            out.dies_at = Some(size);
            return out;
        }
        level = next.into_iter().collect();
        if size >= 3 {
            out.decided.push((size, level.iter().find(|r| holds(f, class, r)).cloned()));
        }
        if level.len() > most {
            return out;
        }
    }
    out
}

// ---------------------------------------------------------------- Golden grows the questions

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

fn leaf(class: Class, r: &mut Rng) -> E {
    if r.below(3) == 0 {
        Num(r.below(7) as i64 - 2)
    } else {
        let reads = class.reads();
        Read(reads[r.below(reads.len())].0)
    }
}

fn grow(class: Class, r: &mut Rng, depth: usize) -> E {
    if depth == 0 || r.below(5) == 0 {
        return leaf(class, r);
    }
    let d = depth - 1;
    let mut sub = |r: &mut Rng| Box::new(grow(class, r, d));
    match r.below(7) {
        0 => { let a = sub(r); Add(a, sub(r)) }
        1 => { let a = sub(r); Sub(a, sub(r)) }
        2 => { let a = sub(r); Mul(a, sub(r)) }
        3 => { let a = sub(r); Min(a, sub(r)) }
        4 => { let a = sub(r); Max(a, sub(r)) }
        5 => { let c = sub(r); let a = sub(r); If(c, a, sub(r)) }
        _ => Abs(sub(r)),
    }
}

const MAX_NODES: usize = 24;
const GROW_STEPS: u64 = 600;
const GROW_MOVES: usize = 500;
const ISLANDS: usize = 8;
const POP: usize = 150;
const GENERATIONS: usize = 40;

/// how many ordered pairs with a point of `touched` break F
fn broken(f: &E, rows: &[u32], touched: u32) -> i64 {
    let n = rows.len();
    let mut c = 0;
    for x in 0..n {
        for y in 0..n {
            if x != y && (touched >> x & 1 == 1 || touched >> y & 1 == 1) && f.val(&readings(rows, x, y)) != 0 {
                c += 1;
            }
        }
    }
    c
}

/// tries to build what the search left undecided: start anywhere, change one cell, keep what breaks no more pairs (now
/// and then what breaks more), start again when stuck. Fixed seed; what it finds is checked from scratch by `holds`
fn climb(f: &E, class: Class, n: usize, moves: usize) -> Option<Vec<u32>> {
    let mut r = Rng(0x2545F4914F6CDD1D ^ (n as u64 * 1000 + class as u64));
    let all = ((1u64 << n) - 1) as u32;
    let random = |r: &mut Rng| -> Vec<u32> {
        let mut rows = vec![0u32; n];
        for i in 0..n {
            if class == Map {
                rows[i] = 1 << r.below(n);
                continue;
            }
            for j in i + 1..n {
                let on = r.below(2) == 1;
                if on {
                    rows[i] |= 1 << j;
                }
                if (on && class == Graph) || (!on && class == Tournament) {
                    rows[j] |= 1 << i;
                }
            }
        }
        rows
    };
    let mut rows = random(&mut r);
    let mut cost = broken(f, &rows, all);
    let mut stale = 0;
    for _ in 0..moves {
        if cost == 0 {
            break;
        }
        let u = r.below(n);
        let v = (u + 1 + r.below(n - 1)) % n;
        let mut next = rows.clone();
        let touched = match class {
            Graph => {
                next[u] ^= 1 << v;
                next[v] ^= 1 << u;
                1u32 << u | 1 << v
            }
            Tournament => {
                next[u] ^= 1 << v;
                next[v] ^= 1 << u;
                1u32 << u | 1 << v
            }
            Map => {
                let old = rows[u].trailing_zeros();
                next[u] = 1 << v;
                1u32 << u | 1 << v | 1 << old
            }
        };
        let change = broken(f, &next, touched) - broken(f, &rows, touched);
        if change <= 0 || r.below(100) == 0 {
            rows = next;
            cost += change;
            stale = if change < 0 { 0 } else { stale + 1 };
        } else {
            stale += 1;
        }
        if stale > 400 {
            rows = random(&mut r);
            cost = broken(f, &rows, all);
            stale = 0;
        }
    }
    (cost == 0 && holds(f, class, &rows)).then_some(rows)
}

/// how good a question this could be, as far as a few short searches see: built and impossible n's that alternate, n's
/// undecided, and no plain structure answering
fn score(f: &E, class: Class) -> f64 {
    let (mut built, mut undecided, mut plain_n, mut turns) = (0, 0, 0, 0);
    let mut last = None;
    for n in 3..=class.grow_to() {
        let now = match decide(f, class, n, GROW_STEPS) {
            Verdict::Plain(_) => {
                plain_n += 1;
                continue;
            }
            Verdict::Built(_) => {
                built += 1;
                true
            }
            Verdict::Impossible(_) => false,
            Verdict::Undecided(_) => {
                if climb(f, class, n, GROW_MOVES).is_some() {
                    built += 1;
                    true
                } else {
                    undecided += 1;
                    continue;
                }
            }
        };
        if last.is_some_and(|l| l != now) {
            turns += 1;
        }
        last = Some(now);
    }
    if built == 0 {
        return -1.0 - f.size() as f64 * 0.05;
    }
    3.0 * turns as f64 + built.min(3) as f64 + 4.0 * undecided.min(2) as f64 - 5.0 * plain_n as f64 - f.size() as f64 * 0.05
}

fn child(class: Class, pop: &[(f64, E)], r: &mut Rng) -> E {
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
        6 | 7 => grow(class, r, 2),
        _ => leaf(class, r),
    };
    let c = a.with(at, &new);
    if c.size() > MAX_NODES { a.clone() } else { c }
}

/// the questions Golden grew for a class, best first
fn grown(class: Class) -> Vec<(f64, E)> {
    let halls: Vec<BTreeMap<String, (f64, E)>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..ISLANDS)
            .map(|i| {
                sc.spawn(move || {
                    let mut r = Rng(0x9E3779B97F4A7C15 ^ ((class as u64 * 97 + i as u64 + 1) << 32));
                    let mut pop: Vec<(f64, E)> = (0..POP).map(|k| { let e = grow(class, &mut r, 1 + k % 5); (score(&e, class), e) }).collect();
                    let mut hall = BTreeMap::new();
                    for _ in 0..GENERATIONS {
                        for (s, e) in &pop {
                            if *s > 0.0 {
                                hall.insert(e.show(class), (*s, e.clone()));
                            }
                        }
                        let top = pop.iter().cloned().fold((f64::MIN, Num(0)), |x, y| if y.0 > x.0 { y } else { x });
                        let mut next = vec![top];
                        while next.len() < POP {
                            let e = child(class, &pop, &mut r);
                            next.push((score(&e, class), e));
                        }
                        pop = next;
                    }
                    hall
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut all = BTreeMap::new();
    for h in halls {
        all.extend(h);
    }
    let mut out: Vec<(f64, E)> = all.into_values().collect();
    out.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.size().cmp(&b.1.size())));
    out
}

const LOOK_TO: usize = 24;
const LOOK_STEPS: u64 = 200_000;
const LOOK_MOVES: usize = 100_000;
const FINALISTS: usize = 80;

/// a question Golden keeps
pub struct Problem {
    pub class: Class,
    pub f: E,
    pub spectrum: Vec<(usize, Verdict)>,
    /// no structure on this many points or more
    pub none_from: Option<usize>,
    /// how many parts of each size Nuome found, from 1
    pub part_sizes: Vec<usize>,
}

impl Problem {
    /// Nuome solves it as far as the parts go; a verdict of the sieve that the parts contradict is a bug
    fn solve(&mut self) {
        let p = parts(&self.f, self.class, PARTS_MAX);
        for (n, rows) in p.decided {
            let v = rows.map_or(Verdict::Impossible(0), Verdict::Built);
            match self.spectrum.iter_mut().find(|x| x.0 == n) {
                Some(old) => {
                    let same = match &old.1 {
                        Verdict::Built(_) => matches!(v, Verdict::Built(_)),
                        Verdict::Impossible(_) => matches!(v, Verdict::Impossible(_)),
                        _ => true,
                    };
                    assert!(same, "the search and the parts disagree at n = {n}: a bug");
                    if matches!(old.1, Verdict::Undecided(_)) {
                        old.1 = v;
                    }
                }
                None => self.spectrum.push((n, v)),
            }
        }
        if let Some(d) = p.dies_at {
            assert!(!self.spectrum.iter().any(|(n, v)| *n >= d && matches!(v, Verdict::Built(_))), "a structure the parts say cannot exist: a bug");
            self.spectrum.retain(|(n, _)| *n < d);
        }
        self.spectrum.sort_by_key(|x| x.0);
        self.none_from = p.dies_at;
        self.part_sizes = p.sizes;
    }

    pub fn solved(&self) -> bool {
        self.none_from.is_some() && self.first_open() == usize::MAX
    }

    pub fn first_open(&self) -> usize {
        self.spectrum.iter().find(|(_, v)| matches!(v, Verdict::Undecided(_))).map_or(usize::MAX, |x| x.0)
    }
    fn built(&self) -> usize {
        self.spectrum.iter().filter(|(_, v)| matches!(v, Verdict::Built(_))).count()
    }
    fn answers(&self) -> Vec<usize> {
        self.spectrum.iter().filter(|(_, v)| matches!(v, Verdict::Built(_))).map(|x| x.0).collect()
    }
    fn none(&self) -> Vec<usize> {
        self.spectrum.iter().filter(|(_, v)| matches!(v, Verdict::Impossible(_))).map(|x| x.0).collect()
    }
    fn open(&self) -> Vec<usize> {
        self.spectrum.iter().filter(|(_, v)| matches!(v, Verdict::Undecided(_))).map(|x| x.0).collect()
    }

    pub fn statement(&self) -> String {
        format!("For which n is there a {} on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = {}?", self.class.name(), self.f.show(self.class))
    }

    /// the problem and what was found of it, every line checkable
    pub fn lines(&self) -> Vec<String> {
        let list = |v: Vec<usize>| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
        let mut out = vec![
            self.statement(),
            format!("  reading: {}", self.class.explain()),
            format!("  built and checked: n = {}", list(self.answers())),
            format!("  impossible, every structure ruled out: n = {}", list(self.none())),
        ];
        if !self.open().is_empty() {
            out.push(format!("  open: n = {} (the sieve's search, its climb and the parts did not decide)", list(self.open())));
        }
        let last = self.part_sizes.len();
        out.push(match self.none_from {
            Some(d) => format!("  {}: no part of {d} points can be in such a structure, so none on {d} points or more; parts of each size from 1: {:?}", if self.solved() { "SOLVED for every n" } else { "solved from there on" }, self.part_sizes),
            None => format!("  not solved: the parts grew too many at {last} points; parts of each size from 1: {:?}", self.part_sizes),
        });
        if let Some((n, Verdict::Built(rows))) = self.spectrum.iter().find(|(_, v)| matches!(v, Verdict::Built(_))) {
            let mut edges = Vec::new();
            for x in 0..*n {
                for y in 0..*n {
                    if rows[x] >> y & 1 == 1 && (self.class != Graph || x < y) {
                        edges.push(format!("{x}{}{y}", if self.class == Graph { "-" } else { ">" }));
                    }
                }
            }
            out.push(format!("  the smallest built, n = {n}: {}", edges.join(" ")));
        }
        let n = self.first_open();
        if n != usize::MAX {
            if let Some((known, all)) = (Sieve { f: &self.f, class: self.class, n }).forced() {
                out.push(format!("  forced at n = {n}: {known} of {all} free cells; the choice left is the other {}", all - known));
            }
        }
        let mut again = 0;
        for (n, v) in &self.spectrum {
            if let Some(count) = enumerate(&self.f, self.class, *n) {
                again += 1;
                assert_eq!(count > 0, matches!(v, Verdict::Built(_) | Verdict::Plain(_)), "the search and the enumeration disagree at n = {n}: a bug in the search");
            }
        }
        out.push(format!("  the search's verdicts tried again one structure at a time, with no search: {again} of the n's above agree (the larger have too many structures)"));
        out
    }
}

/// F's zeros on a fixed handful of structures: two questions the same here are taken to be the same question. A
/// tournament with every game turned round is a tournament, so a question and its mirror are one
fn fingerprint(f: &E, class: Class) -> u64 {
    let mut r = Rng(0xABCDEF0123456789);
    let (mut h, mut mirror) = (0xCBF29CE484222325u64, 0xCBF29CE484222325u64);
    for k in 0..150 {
        let n = 4 + k % 6;
        let mut rows = vec![0u32; n];
        for i in 0..n {
            for j in 0..n {
                let on = r.below(2) == 1;
                match class {
                    Graph if i < j && on => {
                        rows[i] |= 1 << j;
                        rows[j] |= 1 << i;
                    }
                    Tournament if i < j => {
                        if on {
                            rows[i] |= 1 << j;
                        } else {
                            rows[j] |= 1 << i;
                        }
                    }
                    _ => {}
                }
            }
            if class == Map {
                rows[i] = 1 << r.below(n);
            }
        }
        let turned: Vec<u32> = (0..n).map(|i| !rows[i] & ((1u32 << n) - 1) & !(1 << i)).collect();
        for x in 0..n {
            for y in 0..n {
                if x != y {
                    h = (h ^ (f.val(&readings(&rows, x, y)) == 0) as u64).wrapping_mul(0x100000001B3);
                    mirror = (mirror ^ (f.val(&readings(&turned, x, y)) == 0) as u64).wrapping_mul(0x100000001B3);
                }
            }
        }
    }
    if class == Tournament { h.min(mirror) } else { h }
}

/// every open n has one parity and every decided n of that parity is impossible: most likely a count the sieve does not
/// see, not a problem (worked by hand, the ones looked at were)
fn looks_like_parity(spectrum: &[(usize, Verdict)]) -> bool {
    let of = |p: &dyn Fn(&Verdict) -> bool| spectrum.iter().filter(|(_, v)| p(v)).map(|x| x.0).collect::<Vec<_>>();
    let open = of(&|v| matches!(v, Verdict::Undecided(_)));
    let built = of(&|v| matches!(v, Verdict::Built(_)));
    !open.is_empty() && open.iter().all(|n| n % 2 == open[0] % 2) && built.iter().all(|n| n % 2 != open[0] % 2)
}

/// looks further at a question, n by n, until two n's in a row are undecided
fn look(f: &E, class: Class) -> Vec<(usize, Verdict)> {
    let mut out = Vec::new();
    let mut open = 0;
    for n in 3..=LOOK_TO {
        let v = match decide(f, class, n, LOOK_STEPS) {
            Verdict::Undecided(s) => climb(f, class, n, LOOK_MOVES).map_or(Verdict::Undecided(s), Verdict::Built),
            v => v,
        };
        open = if matches!(v, Verdict::Undecided(_)) { open + 1 } else { 0 };
        out.push((n, v));
        if open == 2 {
            break;
        }
    }
    out
}

/// what the sieve needs to be trusted: known cases, written as F, must come out as known
pub fn srg(k: i64, lambda: i64, mu: i64) -> E {
    let minus = |a: E, b: i64| Box::new(Abs(Box::new(Sub(Box::new(a), Box::new(Num(b))))));
    Add(minus(Read(OUT_X), k), Box::new(If(Box::new(Read(A)), minus(Read(COMMON_OUT), lambda), minus(Read(COMMON_OUT), mu))))
}

fn validated() -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for (name, f, n, built) in [("srg(5,2,0,1)", srg(2, 0, 1), 5, true), ("srg(9,4,1,2)", srg(4, 1, 2), 9, true), ("srg(10,3,0,1), Petersen's", srg(3, 0, 1), 10, true), ("srg(13,6,2,3), Paley's", srg(6, 2, 3), 13, true), ("srg(7,3,1,1): 7 x 3 is odd", srg(3, 1, 1), 7, false), ("srg(11,4,1,2): the counting forbids it", srg(4, 1, 2), 11, false)] {
        let sieve = Sieve { f: &f, class: Graph, n };
        let Some(s) = sieve.start() else { return Err(format!("{name}: no start")) };
        let mut steps = 0;
        let got = match sieve.dfs(&s, &mut steps, LOOK_STEPS) {
            Found::Built(rows) if holds(&f, Graph, &rows) => true,
            Found::Empty => false,
            _ => return Err(format!("the sieve did not decide the known case {name} in {steps} steps")),
        };
        if got != built {
            return Err(format!("the sieve is wrong on the known case {name}"));
        }
        lines.push(format!("known case {name}: {} as known, {steps} steps", if built { "built" } else { "impossible" }));
    }
    Ok(lines)
}

/// Golden writes its problems: the validation, then the kept ones, the shortest statement first
pub fn invent(log: &mut dyn FnMut(String)) -> Result<Vec<Problem>, String> {
    for l in validated()? {
        log(l);
    }
    let mut kept = Vec::new();
    for class in ASKED {
        let t0 = std::time::Instant::now();
        let hall = grown(class);
        let mut seen = std::collections::BTreeSet::new();
        let mut finalists: Vec<E> = hall.iter().map(|(_, e)| e.tidy()).filter(|e| seen.insert(fingerprint(e, class))).take(FINALISTS).collect();
        finalists.sort_by_key(|e| e.size());
        let (mut decided, mut plain_answer, mut empty) = (0, 0, 0);
        let looked: Vec<Vec<(usize, Verdict)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = finalists.chunks(FINALISTS.div_ceil(ISLANDS)).map(|c| sc.spawn(move || c.iter().map(|f| look(f, class)).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut here = 0;
        let mut twins = 0;
        let mut parity = 0;
        let mut answers = std::collections::BTreeSet::new();
        for (f, spectrum) in finalists.into_iter().zip(looked) {
            let has = |p: &dyn Fn(&Verdict) -> bool| spectrum.iter().any(|(_, v)| p(v));
            if has(&|v| matches!(v, Verdict::Plain(_))) {
                plain_answer += 1;
            } else if !has(&|v| matches!(v, Verdict::Built(_))) || !has(&|v| matches!(v, Verdict::Impossible(_))) {
                empty += 1;
            } else if !has(&|v| matches!(v, Verdict::Undecided(_))) {
                decided += 1;
            } else if looks_like_parity(&spectrum) {
                parity += 1;
            } else if !answers.insert(spectrum.iter().map(|(_, v)| match v {
                Verdict::Built(_) => 'y',
                Verdict::Impossible(_) => 'n',
                _ => '?',
            }).collect::<String>()) {
                twins += 1;
            } else {
                here += 1;
                kept.push(Problem { class, f, spectrum, none_from: None, part_sizes: Vec::new() });
            }
        }
        log(format!("{}: {} questions grown, {} looked at further: {here} kept; dropped: {plain_answer} a plain structure answers, {empty} never built or never impossible, {decided} decided on every n to {LOOK_TO}, {parity} looking like parity, {twins} with the same answers as a shorter one ({:.0} s)", class.name(), hall.len(), here + plain_answer + empty + decided + parity + twins, t0.elapsed().as_secs_f64()));
    }
    let t0 = std::time::Instant::now();
    for p in kept.iter_mut() {
        p.solve();
    }
    log(format!("Nuome solved {} of the {} for every n ({:.0} s)", kept.iter().filter(|p| p.solved()).count(), kept.len(), t0.elapsed().as_secs_f64()));
    kept.sort_by_key(|p| (p.f.size(), p.first_open(), std::cmp::Reverse(p.built())));
    Ok(kept)
}

/// `nuome --ideas golden-problems [HOW_MANY]`: the first problems shown, all of them written to bench/golden_problem/invented.md
pub fn run(how_many: usize) -> Result<(), String> {
    let kept = invent(&mut |l| println!("{l}"))?;
    let mut text = String::from("# Problems Golden invented, solved by Nuome\n\nEach was undecided by the sieve's short search when Golden kept it; then Nuome worked on it, with nothing from outside. Open means Nuome did not decide it. Whether anyone has asked or settled the same question is not checked.\n");
    println!("\n{} problems kept; the first {}:", kept.len(), how_many.min(kept.len()));
    for (i, p) in kept.iter().enumerate() {
        let lines = p.lines();
        if i < how_many {
            println!("\n{}. {}", i + 1, lines.join("\n"));
        }
        text.push_str(&format!("\n## {}\n\n{}\n", i + 1, lines.join("\n\n")));
    }
    std::fs::create_dir_all("bench/golden_problem").map_err(|e| e.to_string())?;
    std::fs::write("bench/golden_problem/invented.md", text).map_err(|e| e.to_string())
}

/// The question, read whole.
pub const QUESTION: &str = "Invent new open problems about finite structures.";

pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// Golden Answer's kinds. Logical: the problems with what is checked of them. Abstract: the same questions, claiming
/// nothing. Theoretical has none: a problem invented here rests on no known result.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if !kind_logical && !kind_abstract {
        return Err("a problem invented here rests on no known result to name; ask logical".into());
    }
    let mut lines = Vec::new();
    let kept = invent(&mut |l| lines.push(l))?;
    if kind_abstract {
        lines.clear();
    }
    for p in kept.iter().take(5) {
        if kind_abstract {
            lines.push(p.statement());
        } else {
            lines.extend(p.lines());
        }
    }
    if kind_abstract {
        lines.push("this is an answer that claims nothing".into());
    } else {
        lines.push("open means Nuome did not decide it; whether anyone has asked or settled the same question is not checked".into());
    }
    Ok((format!("{} problems invented", kept.len()), lines, vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_tree(class: Class, r: &mut Rng) -> E {
        grow(class, r, 4)
    }

    #[test]
    fn the_known_cases_come_out_as_known() {
        validated().unwrap();
    }

    #[test]
    fn counts_of_known_families() {
        // no condition: every graph; regular graphs by degree (OEIS A001205, A002829)
        assert_eq!(enumerate(&Num(0), Graph, 5), Some(1024));
        let regular = |k: i64| Abs(Box::new(Sub(Box::new(Read(OUT_X)), Box::new(Num(k)))));
        assert_eq!(enumerate(&regular(2), Graph, 5), Some(12));
        assert_eq!(enumerate(&regular(3), Graph, 6), Some(70));
        assert_eq!(enumerate(&Num(0), Map, 5), Some(3125));
        assert_eq!(enumerate(&Num(0), Tournament, 5), Some(1024));
    }

    #[test]
    fn a_range_holds_every_value_its_completions_take() {
        let mut r = Rng(77);
        for class in CLASSES {
            for _ in 0..300 {
                let f = random_tree(class, &mut r);
                let n = 3 + r.below(4);
                let sieve = Sieve { f: &f, class, n };
                let mut s = St { one: [0; MAX_N], may: [0; MAX_N], ione: [0; MAX_N], imay: [0; MAX_N] };
                let full = ((1u64 << n) - 1) as u32;
                for i in 0..n {
                    s.may[i] = if class == Map { full } else { full & !(1 << i) };
                    s.imay[i] = if class == Map { full } else { full & !(1 << i) };
                }
                // a whole structure, and a part of it known
                let mut whole = s;
                let mut touched = 0;
                while let Some((u, v)) = sieve.free_cell(&whole) {
                    let b = r.below(2) == 1;
                    assert!(sieve.set(&mut whole, u, v, b, &mut touched));
                    if r.below(3) == 0 {
                        let mut t = 0;
                        assert!(sieve.set(&mut s, u, v, b, &mut t));
                    }
                }
                let rows = whole.one[..n].to_vec();
                assert!(of_class(class, &rows));
                for x in 0..n {
                    for y in 0..n {
                        if x != y {
                            let v = f.val(&readings(&rows, x, y));
                            let (l, h) = f.range(&sieve.ranges(&whole, x, y));
                            assert_eq!((l, h), (v, v), "whole: {}", f.show(class));
                            let (l, h) = f.range(&sieve.ranges(&s, x, y));
                            assert!(l <= v && v <= h, "part: {}", f.show(class));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_cut_never_changes_a_verdict() {
        let mut r = Rng(4242);
        for class in CLASSES {
            for _ in 0..300 {
                let f = random_tree(class, &mut r);
                for n in 3..=5 {
                    let count = enumerate(&f, class, n).unwrap();
                    let sieve = Sieve { f: &f, class, n };
                    let found = match sieve.start() {
                        None => false,
                        Some(s) => {
                            let mut steps = 0;
                            matches!(sieve.dfs(&s, &mut steps, u64::MAX), Found::Built(_))
                        }
                    };
                    assert_eq!(found, count > 0, "{} n = {n}: {}", class.name(), f.show(class));
                }
            }
        }
    }

    #[test]
    fn a_verdict_agrees_with_every_structure_tried() {
        let mut r = Rng(31337);
        for class in CLASSES {
            for _ in 0..400 {
                let f = random_tree(class, &mut r);
                for n in 3..=5 {
                    let any = enumerate(&f, class, n).unwrap() > 0;
                    let v = decide(&f, class, n, u64::MAX);
                    assert_eq!(any, matches!(v, Verdict::Built(_) | Verdict::Plain(_)), "{} n = {n}: {}", class.name(), f.show(class));
                }
            }
        }
    }

    #[test]
    fn the_sum_of_the_degrees_forbids_what_it_should() {
        // every degree 3 on 7 points
        assert!(sum_forbids(&srg(3, 1, 1), Graph, 7));
        assert!(!sum_forbids(&srg(4, 1, 2), Graph, 9));
    }

    #[test]
    fn parity_is_seen_only_when_every_open_n_shares_it() {
        let y = || Verdict::Built(vec![]);
        let (no, open) = (Verdict::Impossible(1), Verdict::Undecided(1));
        assert!(looks_like_parity(&[(4, y()), (5, no.clone()), (6, y()), (7, open.clone()), (9, open.clone())]));
        assert!(!looks_like_parity(&[(4, y()), (5, y()), (7, open.clone()), (9, open.clone())]));
        assert!(!looks_like_parity(&[(4, y()), (7, open.clone()), (8, open)]));
    }

    #[test]
    fn a_question_and_its_mirror_are_one() {
        let minus_one = |r: u8| Box::new(Sub(Box::new(Read(r)), Box::new(Num(1))));
        let a = Mul(minus_one(COMMON_IN), Box::new(Sub(Box::new(Read(COMMON_OUT)), Box::new(Num(4)))));
        let b = Mul(minus_one(COMMON_OUT), Box::new(Sub(Box::new(Read(COMMON_IN)), Box::new(Num(4)))));
        assert_eq!(fingerprint(&a, Tournament), fingerprint(&b, Tournament));
        assert_ne!(fingerprint(&a, Tournament), fingerprint(&minus_one(COMMON_IN), Tournament));
    }

    #[test]
    fn solving_agrees_with_every_structure_tried() {
        let mut r = Rng(2718);
        for class in CLASSES {
            for _ in 0..300 {
                let f = random_tree(class, &mut r);
                for n in 3..=5 {
                    let any = enumerate(&f, class, n).unwrap() > 0;
                    let v = solve_n(&f, class, n, u64::MAX);
                    assert_eq!(any, matches!(v, Verdict::Built(_) | Verdict::Plain(_)), "{} n = {n}: {}", class.name(), f.show(class));
                }
            }
        }
    }

    #[test]
    fn the_parts_agree_with_every_structure_tried() {
        let mut r = Rng(1618);
        for class in ASKED {
            for _ in 0..80 {
                let f = random_tree(class, &mut r);
                let p = parts(&f, class, 300);
                for (n, rows) in &p.decided {
                    if let Some(count) = enumerate(&f, class, *n) {
                        assert_eq!(count > 0, rows.is_some(), "{} n = {n}: {}", class.name(), f.show(class));
                    }
                }
                if let Some(d) = p.dies_at {
                    for n in d..=6 {
                        assert_eq!(enumerate(&f, class, n), Some(0), "{} dies at {d}, n = {n}: {}", class.name(), f.show(class));
                    }
                }
            }
        }
    }

    #[test]
    fn with_no_condition_every_part_is_there() {
        // at least the structures up to renaming (OEIS A000088, A000568); a few more only if two writings of one remain
        let sizes = |class| parts(&Num(0), class, 200).sizes;
        let at_least = |got: Vec<usize>, known: &[usize]| known.iter().zip(&got).all(|(k, g)| g >= k);
        assert!(at_least(sizes(Graph), &[1, 2, 4, 11, 34, 156]));
        assert!(at_least(sizes(Tournament), &[1, 1, 2, 4, 12, 56]));
    }

    #[test]
    fn the_first_problem_comes_out_the_same_by_the_other_search() {
        let m = |r: u8| Box::new(Sub(Box::new(Read(r)), Box::new(Num(1))));
        let p = parts(&Mul(m(COMMON_OUT), m(COMMON_IN)), Tournament, PARTS_MAX);
        assert_eq!(p.dies_at, Some(11));
        assert_eq!(p.decided.iter().filter(|x| x.1.is_some()).map(|x| x.0).collect::<Vec<_>>(), [4, 6, 7]);
        // what the parts die of, asked whole of the search with no parts in it: no pair has two above it and two below
        let over = |r: u8| Box::new(Max(m(r), Box::new(Num(0))));
        let no_wings = Min(over(COMMON_OUT), over(COMMON_IN));
        assert!(matches!(decide(&no_wings, Tournament, 10, 5_000_000), Verdict::Built(_)));
        assert!(matches!(decide(&no_wings, Tournament, 11, 5_000_000), Verdict::Impossible(_)));
    }

    #[test]
    fn a_climb_builds_only_what_holds() {
        let rows = climb(&srg(2, 0, 1), Graph, 5, 100_000).unwrap();
        assert!(holds(&srg(2, 0, 1), Graph, &rows));
        assert!(climb(&srg(3, 1, 1), Graph, 7, 2_000).is_none());
    }

    #[test]
    fn a_tidy_function_is_the_same_function() {
        let mut r = Rng(9);
        for class in CLASSES {
            for _ in 0..500 {
                let f = random_tree(class, &mut r);
                let t = f.tidy();
                for _ in 0..20 {
                    let mut x = [0i64; READS];
                    for v in x.iter_mut() {
                        *v = r.below(9) as i64;
                    }
                    assert_eq!(f.val(&x), t.val(&x), "{}", f.show(class));
                }
            }
        }
    }

    #[test]
    fn the_question_is_read_whole() {
        assert!(read("invent new open problems about finite structures"));
        assert!(!read("invent new open problems about finite structures and solve them"));
    }

    #[test]
    fn a_theoretical_answer_does_not_slip() {
        assert!(answer(false, false).is_err());
    }
}
