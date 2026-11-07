//! Golden Tree: Golden grows paths out of randomness, and keeps the rule systems that stay quiet for a long time and
//! then turn complex.
//!
//! A path is a chain of small steps, each one simple rule drawn from a seed: add, multiply, shift, xor, mod, a step
//! taken only when x ≡ r (mod m), or, on a ring of 256 bits, the row combined with itself turned. A finished path is a
//! rule system, iterated from fixed plain starts (all ones, a single one bit, a single cell).
//!
//! Render distance: like a game world, the tree exists only a few steps ahead of where Golden stands (default 4).
//! Golden takes the step whose branch holds the best path, the other branches are dropped, and the frontier grows one
//! step further. Nothing deeper than the render distance is ever made.
//!
//! What it hunts: a long quiet run (the trace of the iteration exactly periodic, with period at most 8, or for a row
//! the same row turned), then a change, and what follows complex (compressed badly, high entropy). Shorter rules
//! score higher. Rejected: a rule that never changes (constant or a plain loop), one that falls into a loop or dies
//! right after the change, one that blows past 2^120.
//!
//! The iteration of a whole frontier runs on the GPU in one batch (one thread a rule and a start); the CPU does the
//! same iteration as the reference, and Golden Answer's logical kind checks every number it states against it.
//! Nothing about any rule is set by hand: no rule is written in, the engine finds them.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub const OUT: &str = "out/golden_tree";
/// iterations of a rule from each start
pub const T: usize = 8192;
/// the longest path
pub const MAX_LEN: usize = 8;
/// children drawn for a node
pub const BRANCH: usize = 4;
pub const RENDER_DISTANCE: usize = 4;
/// the fewest rules worth sending to the GPU (measured: below this the CPU is quicker)
pub const GPU_MIN: usize = 4096;
/// a path's numbers stop being numbers past this
const LIMIT_BITS: u32 = 120;
/// the longest period a quiet run may have
const MAX_PERIOD: usize = 8;
/// the furthest a quiet row may turn per iteration
const MAX_DRIFT: i32 = 8;
/// what must follow the change before it is the change
const AFTER_MIN: usize = 64;
const AFTER_MAX: usize = 1024;
const ROW_WORDS: usize = 4;
const ROW_BITS: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum World {
    /// x is a natural number
    Number,
    /// a is a ring of 256 cells, c the row before this iteration
    Row,
    /// two numbers x and y: sums, differences and products of each other, squares, swaps; a recurrence on two
    /// earlier values is a pair path (Fibonacci is "swap x, y ; x ← x + y")
    Pair,
}

/// One step of a path. In a number path the steps act on x in turn; in a row path on a, with c the row the
/// iteration started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Step {
    Add(u8),
    Mul(u8),
    Shr(u8),
    Shl(u8),
    Xor(u8),
    Mod(u8),
    /// the inner step only when x ≡ r (mod m)
    If { m: u8, r: u8, op: u8, arg: u8 },
    XorC(i8),
    OrC(i8),
    AndC(i8),
    Not,
    Rot(i8),
    /// x ← x + y
    PAdd,
    /// y ← y + x
    PAddY,
    /// x ← |x − y|
    PDiff,
    /// x ← c·x
    PMul(u8),
    /// x ← x·y
    PMulXY,
    /// x ← x²
    PSq,
    /// x, y ← y, x
    PSwap,
    /// x ← x >> k
    PShr(u8),
    /// x ← x mod m
    PMod(u8),
    /// the inner step on x only when x ≡ r (mod m)
    PIf { m: u8, r: u8, op: u8, arg: u8 },
}

const OP_ADD: u8 = 1;
const OP_MUL: u8 = 2;
const OP_SHR: u8 = 3;
const OP_SHL: u8 = 4;
const OP_XOR: u8 = 5;
const OP_MOD: u8 = 6;
const OP_IF: u8 = 7;
const OP_XORC: u8 = 10;
const OP_ORC: u8 = 11;
const OP_ANDC: u8 = 12;
const OP_NOT: u8 = 13;
const OP_ROT: u8 = 14;
const OP_PADD: u8 = 20;
const OP_PADDY: u8 = 21;
const OP_PDIFF: u8 = 22;
const OP_PMUL: u8 = 23;
const OP_PMULXY: u8 = 24;
const OP_PSQ: u8 = 25;
const OP_PSWAP: u8 = 26;
const OP_PSHR: u8 = 27;
const OP_PMOD: u8 = 28;
const OP_PIF: u8 = 29;

impl Step {
    pub fn world(&self) -> World {
        match self {
            Step::XorC(_) | Step::OrC(_) | Step::AndC(_) | Step::Not | Step::Rot(_) => World::Row,
            Step::PAdd | Step::PAddY | Step::PDiff | Step::PMul(_) | Step::PMulXY | Step::PSq | Step::PSwap | Step::PShr(_)
            | Step::PMod(_) | Step::PIf { .. } => World::Pair,
            _ => World::Number,
        }
    }

    /// The step as four bytes for the GPU: op, a, b, c.
    pub fn code(&self) -> u32 {
        let (op, a, b, c) = match *self {
            Step::Add(v) => (OP_ADD, v, 0, 0),
            Step::Mul(v) => (OP_MUL, v, 0, 0),
            Step::Shr(v) => (OP_SHR, v, 0, 0),
            Step::Shl(v) => (OP_SHL, v, 0, 0),
            Step::Xor(v) => (OP_XOR, v, 0, 0),
            Step::Mod(v) => (OP_MOD, v, 0, 0),
            Step::If { m, r, op, arg } => (OP_IF, m, r, (op & 15) | (arg << 4)),
            Step::XorC(k) => (OP_XORC, k as u8, 0, 0),
            Step::OrC(k) => (OP_ORC, k as u8, 0, 0),
            Step::AndC(k) => (OP_ANDC, k as u8, 0, 0),
            Step::Not => (OP_NOT, 0, 0, 0),
            Step::Rot(k) => (OP_ROT, k as u8, 0, 0),
            Step::PAdd => (OP_PADD, 0, 0, 0),
            Step::PAddY => (OP_PADDY, 0, 0, 0),
            Step::PDiff => (OP_PDIFF, 0, 0, 0),
            Step::PMul(c) => (OP_PMUL, c, 0, 0),
            Step::PMulXY => (OP_PMULXY, 0, 0, 0),
            Step::PSq => (OP_PSQ, 0, 0, 0),
            Step::PSwap => (OP_PSWAP, 0, 0, 0),
            Step::PShr(k) => (OP_PSHR, k, 0, 0),
            Step::PMod(m) => (OP_PMOD, m, 0, 0),
            Step::PIf { m, r, op, arg } => (OP_PIF, m, r, (op & 15) | (arg << 4)),
        };
        op as u32 | (a as u32) << 8 | (b as u32) << 16 | (c as u32) << 24
    }

    pub fn text(&self) -> String {
        let basic = |op: u8, v: u8| -> String {
            match op {
                OP_ADD => format!("x + {v}"),
                OP_MUL => format!("{v}x"),
                OP_SHR => format!("x >> {v}"),
                OP_SHL => format!("x << {v}"),
                OP_XOR => format!("x xor {v}"),
                OP_MOD => format!("x mod {v}"),
                _ => "?".into(),
            }
        };
        let turn = |k: i8| -> String {
            if k == 0 {
                "c".into()
            } else {
                format!("rot(c, {k})")
            }
        };
        match *self {
            Step::Add(v) => basic(OP_ADD, v),
            Step::Mul(v) => basic(OP_MUL, v),
            Step::Shr(v) => basic(OP_SHR, v),
            Step::Shl(v) => basic(OP_SHL, v),
            Step::Xor(v) => basic(OP_XOR, v),
            Step::Mod(v) => basic(OP_MOD, v),
            Step::If { m, r, op, arg } => format!("if x ≡ {r} (mod {m}): {}", basic(op, arg)),
            Step::XorC(k) => format!("a xor {}", turn(k)),
            Step::OrC(k) => format!("a or {}", turn(k)),
            Step::AndC(k) => format!("a and {}", turn(k)),
            Step::Not => "not a".into(),
            Step::Rot(k) => format!("rot(a, {k})"),
            Step::PAdd => "x ← x + y".into(),
            Step::PAddY => "y ← y + x".into(),
            Step::PDiff => "x ← |x − y|".into(),
            Step::PMul(c) => format!("x ← {c}x"),
            Step::PMulXY => "x ← x·y".into(),
            Step::PSq => "x ← x²".into(),
            Step::PSwap => "swap x, y".into(),
            Step::PShr(k) => format!("x ← x >> {k}"),
            Step::PMod(m) => format!("x ← x mod {m}"),
            Step::PIf { m, r, op, arg } => format!("if x ≡ {r} (mod {m}): x ← {}", basic(op, arg)),
        }
    }
}

/// A rule system: the steps of a path, in order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Rule {
    pub steps: Vec<Step>,
}

impl Rule {
    pub fn world(&self) -> World {
        self.steps.first().map(|s| s.world()).unwrap_or(World::Number)
    }

    pub fn text(&self) -> String {
        let body: Vec<String> = self.steps.iter().map(|s| s.text()).collect();
        match self.world() {
            World::Number => format!("x ↦ {}", body.join(" ; ")),
            World::Row => format!("c ← a ; {}", body.iter().map(|b| format!("a ← {b}")).collect::<Vec<_>>().join(" ; ")),
            World::Pair => format!("(x, y) ↦ {}", body.join(" ; ")),
        }
    }

    /// The rule written back from its text's codes, for the vault.
    pub fn codes(&self) -> Vec<u32> {
        self.steps.iter().map(|s| s.code()).collect()
    }

    pub fn from_codes(codes: &[u32]) -> Rule {
        Rule { steps: codes.iter().map(|&c| decode(c)).collect() }
    }
}

fn decode(c: u32) -> Step {
    let (op, a, b, cc) = ((c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8, ((c >> 24) & 0xff) as u8);
    match op {
        OP_ADD => Step::Add(a),
        OP_MUL => Step::Mul(a),
        OP_SHR => Step::Shr(a),
        OP_SHL => Step::Shl(a),
        OP_XOR => Step::Xor(a),
        OP_MOD => Step::Mod(a),
        OP_IF => Step::If { m: a, r: b, op: cc & 15, arg: cc >> 4 },
        OP_XORC => Step::XorC(a as i8),
        OP_ORC => Step::OrC(a as i8),
        OP_ANDC => Step::AndC(a as i8),
        OP_NOT => Step::Not,
        OP_PADD => Step::PAdd,
        OP_PADDY => Step::PAddY,
        OP_PDIFF => Step::PDiff,
        OP_PMUL => Step::PMul(a),
        OP_PMULXY => Step::PMulXY,
        OP_PSQ => Step::PSq,
        OP_PSWAP => Step::PSwap,
        OP_PSHR => Step::PShr(a),
        OP_PMOD => Step::PMod(a),
        OP_PIF => Step::PIf { m: a, r: b, op: cc & 15, arg: cc >> 4 },
        _ => Step::Rot(a as i8),
    }
}

// ------------------------------------------------------------------ randomness

/// splitmix64: every draw is fixed by the seed, so the same seed grows the same tree.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// One step drawn at random in a world.
pub fn draw_step(rng: &mut Rng, world: World) -> Step {
    match world {
        World::Pair => match rng.below(12) {
            0..=1 => Step::PAdd,
            2 => Step::PAddY,
            3 => Step::PDiff,
            4 => Step::PMul(2 + rng.below(8) as u8),
            5 => Step::PMulXY,
            6 => Step::PSq,
            7 => Step::PSwap,
            8 => Step::PShr(1 + rng.below(3) as u8),
            9 => Step::PMod(2 + rng.below(254) as u8),
            _ => {
                let m = 2 + rng.below(4) as u8;
                let r = rng.below(m as u64) as u8;
                let (op, arg) = match rng.below(5) {
                    0 => (OP_ADD, 1 + rng.below(9) as u8),
                    1 => (OP_MUL, 2 + rng.below(8) as u8),
                    2 => (OP_SHR, 1 + rng.below(3) as u8),
                    3 => (OP_SHL, 1 + rng.below(3) as u8),
                    _ => (OP_XOR, 1 + rng.below(15) as u8),
                };
                Step::PIf { m, r, op, arg }
            }
        },
        World::Number => {
            let basic = |rng: &mut Rng| -> (u8, u8) {
                match rng.below(5) {
                    0 => (OP_ADD, 1 + rng.below(9) as u8),
                    1 => (OP_MUL, 2 + rng.below(8) as u8),
                    2 => (OP_SHR, 1 + rng.below(3) as u8),
                    3 => (OP_SHL, 1 + rng.below(3) as u8),
                    _ => (OP_XOR, 1 + rng.below(15) as u8),
                }
            };
            match rng.below(8) {
                0..=3 => {
                    let m = 2 + rng.below(4) as u8;
                    let r = rng.below(m as u64) as u8;
                    let (op, arg) = basic(rng);
                    Step::If { m, r, op, arg }
                }
                4 => Step::Mod(2 + rng.below(30) as u8),
                _ => {
                    let (op, v) = basic(rng);
                    decode(op as u32 | (v as u32) << 8)
                }
            }
        }
        World::Row => {
            let k = loop {
                let k = rng.below(17) as i8 - 8;
                if k != 0 || rng.below(2) == 0 {
                    break k;
                }
            };
            match rng.below(9) {
                0..=2 => Step::XorC(k),
                3..=4 => Step::OrC(k),
                5..=6 => Step::AndC(k),
                7 => Step::Not,
                _ => Step::Rot(if k == 0 { 1 } else { k }),
            }
        }
    }
}

// ------------------------------------------------------------------ the iteration (CPU reference)

/// The plain starts every rule is iterated from.
pub fn number_starts() -> Vec<u128> {
    vec![(1u128 << 40) - 1, (1u128 << 60) - 1, 1u128 << 40, 1_000_003]
}

/// Pair paths start from (1, 1), (2^40 − 1, 1), (3, 2^20) and (1000003, 7).
pub fn pair_starts() -> Vec<(u128, u128)> {
    vec![(1, 1), ((1u128 << 40) - 1, 1), (3, 1u128 << 20), (1_000_003, 7)]
}

/// One iteration of a pair path; None when a number blows past 2^120.
pub fn step_pair(rule: &[u32], (mut x, mut y): (u128, u128)) -> Option<(u128, u128)> {
    let lim = 1u128 << LIMIT_BITS;
    for &c in rule {
        let (op, a, b, cc) = ((c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8, ((c >> 24) & 0xff) as u8);
        match op {
            OP_PADD => x = x.checked_add(y)?,
            OP_PADDY => y = y.checked_add(x)?,
            OP_PDIFF => x = x.abs_diff(y),
            OP_PMUL => x = x.checked_mul(a as u128)?,
            OP_PMULXY => x = x.checked_mul(y)?,
            OP_PSQ => x = x.checked_mul(x)?,
            OP_PSWAP => std::mem::swap(&mut x, &mut y),
            OP_PSHR => x >>= a,
            OP_PMOD => {
                if a > 0 {
                    x %= a as u128
                }
            }
            OP_PIF => {
                if a > 0 && x % a as u128 == b as u128 {
                    x = basic_num(cc & 15, cc >> 4, x)?;
                }
            }
            _ => {}
        }
        if x > lim || y > lim {
            return None;
        }
    }
    Some((x, y))
}

pub fn row_starts() -> Vec<[u64; ROW_WORDS]> {
    let mut single = [0u64; ROW_WORDS];
    single[2] = 1;
    let mut block = [0u64; ROW_WORDS];
    block[1] = 0xFFFF_0000_0000_0000;
    block[2] = 0x0000_0000_0000_FFFF;
    vec![single, block]
}

fn basic_num(op: u8, v: u8, x: u128) -> Option<u128> {
    let lim = 1u128 << LIMIT_BITS;
    let y = match op {
        OP_ADD => x.checked_add(v as u128)?,
        OP_MUL => x.checked_mul(v as u128)?,
        OP_SHR => x >> v,
        OP_SHL => {
            if x >> (127 - v as u32) != 0 {
                return None;
            }
            x << v
        }
        OP_XOR => x ^ v as u128,
        OP_MOD => {
            if v == 0 {
                x
            } else {
                x % v as u128
            }
        }
        _ => x,
    };
    if y > lim {
        None
    } else {
        Some(y)
    }
}

/// One iteration of a number path; None when the number blows past 2^120.
pub fn step_number(rule: &[u32], mut x: u128) -> Option<u128> {
    for &c in rule {
        let (op, a, b, cc) = ((c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8, ((c >> 24) & 0xff) as u8);
        x = if op == OP_IF {
            if a > 0 && x % a as u128 == b as u128 {
                basic_num(cc & 15, cc >> 4, x)?
            } else {
                x
            }
        } else {
            basic_num(op, a, x)?
        };
    }
    Some(x)
}

fn rot(row: &[u64; ROW_WORDS], k: i32) -> [u64; ROW_WORDS] {
    let k = k.rem_euclid(ROW_BITS) as usize;
    if k == 0 {
        return *row;
    }
    let (w, b) = (k / 64, k % 64);
    let mut out = [0u64; ROW_WORDS];
    for i in 0..ROW_WORDS {
        let src = (i + ROW_WORDS - w) % ROW_WORDS;
        let prev = (src + ROW_WORDS - 1) % ROW_WORDS;
        out[i] = if b == 0 { row[src] } else { (row[src] << b) | (row[prev] >> (64 - b)) };
    }
    out
}

/// One iteration of a row path.
pub fn step_row(rule: &[u32], row: &[u64; ROW_WORDS]) -> [u64; ROW_WORDS] {
    let c = *row;
    let mut a = *row;
    for &code in rule {
        let (op, k) = ((code & 0xff) as u8, ((code >> 8) & 0xff) as u8 as i8 as i32);
        let rc = rot(&c, k);
        for i in 0..ROW_WORDS {
            a[i] = match op {
                OP_XORC => a[i] ^ rc[i],
                OP_ORC => a[i] | rc[i],
                OP_ANDC => a[i] & rc[i],
                OP_NOT => !a[i],
                _ => a[i],
            };
        }
        if op == OP_ROT {
            a = rot(&a, k);
        }
    }
    a
}

/// What an iteration leaves to read: its symbols (a number path's direction and parity each iteration) or rows,
/// and where it ended (blew up).
#[derive(Clone)]
pub enum Trace {
    Number { xs: Vec<u128>, sym: Vec<u8>, blew_at: Option<usize> },
    Row { rows: Vec<[u64; ROW_WORDS]> },
}

pub fn trace_number(rule: &[u32], start: u128) -> Trace {
    let mut xs = Vec::with_capacity(T);
    let mut sym = Vec::with_capacity(T);
    let mut x = start;
    let mut blew_at = None;
    for t in 0..T {
        match step_number(rule, x) {
            Some(y) => {
                xs.push(x);
                let dir = if y > x { 2 } else if y < x { 1 } else { 0 };
                sym.push(dir * 2 + (x & 1) as u8);
                x = y;
            }
            None => {
                blew_at = Some(t);
                break;
            }
        }
    }
    Trace::Number { xs, sym, blew_at }
}

pub fn trace_row(rule: &[u32], start: &[u64; ROW_WORDS]) -> Trace {
    let mut rows = Vec::with_capacity(T);
    let mut a = *start;
    for _ in 0..T {
        rows.push(a);
        a = step_row(rule, &a);
    }
    Trace::Row { rows }
}

// ------------------------------------------------------------------ watching an iteration, never looking back

/// What a rule did from one start.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// iterations of the quiet run
    pub quiet: usize,
    /// how complex what follows the change is, 0 to 1
    pub after: f64,
    /// why it is no use, if it is none
    pub rejected: Option<&'static str>,
}



const OK: u32 = 0;
const NEVER: u32 = 1;
const EXPLODES: u32 = 2;
const LOOPS: u32 = 3;
const SHORT: u32 = 4;

/// What watching an iteration leaves: integers only, so the GPU's watch and the CPU's are the same to the bit.
///
/// The watch goes forward only, in fixed memory: the last MAX_PERIOD states; one flag a candidate quiet pattern
/// (period p, and for a row a turn d), put out at the first state that breaks it, the quiet run ending when the last
/// flag goes out; Brent's loop finder (one state kept, two counters); after the change, running counts. It stops as
/// soon as it knows enough: a loop, a blow-up, or AFTER_MAX states after the change.
#[derive(Clone, Debug, PartialEq)]
pub struct Watch {
    pub quiet: u32,
    pub post: u32,
    pub code: u32,
    /// how complex what followed the change was, read from the counts where they were kept
    pub after: f64,
}

fn verdict(quiet: Option<u32>, post: u32, looped: bool, blew: bool) -> u32 {
    match quiet {
        _ if blew => EXPLODES,
        None => NEVER,
        Some(_) if looped && post < AFTER_MIN as u32 => LOOPS,
        Some(_) if post < AFTER_MIN as u32 => SHORT,
        Some(_) => OK,
    }
}

/// The quiet run from when each candidate pattern broke (candidate i has period i / per + 1): the longest that
/// repeated at least three times before it broke; 0 when none did (a pattern seen once or twice is not a run).
fn quiet_of(death: &[u32], per: usize) -> u32 {
    let mut q = 0;
    for (i, &d) in death.iter().enumerate() {
        let p = (i / per + 1) as u32;
        if d >= 3 * p && d > q {
            q = d;
        }
    }
    q
}

/// The longest period a number path's symbols are predicted from after the change.
const PREDICT: usize = 64;

/// Where a loop of length `lambda` starts: two walkers from the start, one `lambda` ahead, forward until they meet.
fn loop_start<S: PartialEq + Copy>(start: S, lambda: u64, f: impl Fn(S) -> S) -> u32 {
    let mut hare = start;
    for _ in 0..lambda {
        hare = f(hare);
    }
    let mut tort = start;
    let mut mu = 0u32;
    while tort != hare && (mu as usize) < T {
        tort = f(tort);
        hare = f(hare);
        mu += 1;
    }
    mu
}

fn symbol(x: u128, y: u128) -> u8 {
    (if y > x { 2 } else if y < x { 1 } else { 0 }) * 2 + (x & 1) as u8
}

/// Watch a number path from one start. Three walks forward, none looking back: the quiet run and Brent's loop
/// finder; where the loop starts; then what lies between the change and the loop (or the end).
pub fn watch_number(rule: &[u32], start: u128) -> Watch {
    watch_seq(start, |x| step_number(rule, x), symbol, 6)
}

/// Watch a pair path (x, y) from one start: x's direction and parity, and y's parity.
pub fn watch_pair(rule: &[u32], start: (u128, u128)) -> Watch {
    watch_seq(start, |s| step_pair(rule, s), |a, b| symbol(a.0, b.0) * 2 + (a.1 & 1) as u8, 12)
}

/// Watch any path whose states can blow up, through its symbols (an alphabet of `alpha`). Three walks forward,
/// none looking back: the quiet run and Brent's loop finder; where the loop starts; then what lies between the
/// change and the loop (or the end).
fn watch_seq<S: Copy + PartialEq>(start: S, step: impl Fn(S) -> Option<S>, sym: impl Fn(S, S) -> u8, alpha: usize) -> Watch {
    let f = |x: S| step(x).unwrap_or(start);
    // 1. the quiet run, the loop, a blow-up
    let mut ring = [0u8; MAX_PERIOD];
    let mut alive: u32 = (1 << MAX_PERIOD) - 1;
    let mut death = [0u32; MAX_PERIOD + 1];
    let mut quiet = None::<u32>;
    let (mut tort, mut power, mut lam) = (start, 1u64, 1u64);
    let (mut lambda, mut blew) = (None::<u64>, false);
    let mut x = start;
    for t in 0..T as u32 {
        let Some(y) = step(x) else {
            blew = true;
            break;
        };
        let s = sym(x, y);
        if quiet.is_none() {
            for p in 1..=MAX_PERIOD as u32 {
                if t >= p && alive >> (p - 1) & 1 == 1 && ring[((t - p) % MAX_PERIOD as u32) as usize] != s {
                    alive &= !(1 << (p - 1));
                    death[p as usize] = t;
                }
            }
            if alive == 0 {
                quiet = Some(quiet_of(&death[1..], 1));
            }
        }
        ring[(t % MAX_PERIOD as u32) as usize] = s;
        if y == tort {
            lambda = Some(lam);
            break;
        }
        if power == lam {
            tort = y;
            power *= 2;
            lam = 0;
        }
        lam += 1;
        x = y;
    }
    let none = |code| Watch { quiet: quiet.unwrap_or(T as u32), post: 0, code, after: 0.0 };
    if blew {
        return none(EXPLODES);
    }
    let Some(q) = quiet else { return none(NEVER) };
    // 2. where the loop starts: the change must be followed by something before it
    let end = lambda.map(|l| loop_start(start, l, f)).unwrap_or(T as u32).min(T as u32);
    let post = end.saturating_sub(q);
    let code = verdict(quiet, post, lambda.is_some(), false);
    if code != OK {
        return Watch { quiet: q, post, code, after: 0.0 };
    }
    // 3. what follows the change: its symbols one and three at a time, and how well a period up to PREDICT foretells it
    let stop = end.min(q + AFTER_MAX as u32);
    let mut c1 = vec![0u32; alpha];
    let mut c3 = vec![0u32; alpha * alpha * alpha];
    let mut ring = [0u8; PREDICT];
    let (mut miss, mut tot) = ([0u32; PREDICT + 1], [0u32; PREDICT + 1]);
    let mut x = start;
    for t in 0..stop {
        let y = f(x);
        let s = sym(x, y);
        if t >= q {
            c1[s as usize] += 1;
            if t >= q + 2 {
                let a = ring[((t - 2) % PREDICT as u32) as usize] as usize;
                let b = ring[((t - 1) % PREDICT as u32) as usize] as usize;
                c3[a * alpha * alpha + b * alpha + s as usize] += 1;
            }
            for p in 1..=PREDICT as u32 {
                if t >= q + p {
                    tot[p as usize] += 1;
                    miss[p as usize] += (ring[((t - p) % PREDICT as u32) as usize] != s) as u32;
                }
            }
        }
        ring[(t % PREDICT as u32) as usize] = s;
        x = y;
    }
    Watch { quiet: q, post, code, after: after_seq(&c1, &c3, &miss, &tot) }
}

/// How complex what follows the change is: the entropy of its symbols, one and three at a time, times how badly
/// any period up to PREDICT foretells it (0 when one does).
fn after_seq(c1: &[u32], c3: &[u32], miss: &[u32], tot: &[u32]) -> f64 {
    let used = c1.iter().filter(|&&c| c > 0).count();
    if used < 2 {
        return 0.0;
    }
    let lu = (used as f64).log2();
    let n3: u64 = c3.iter().map(|&c| c as u64).sum();
    let h3_max = ((used * used * used) as f64).min(n3.max(2) as f64).log2();
    let h = (0.5 * entropy_of(c1) / lu + 0.5 * entropy_of(c3) / h3_max).min(1.0);
    let mut novelty = 1.0f64;
    for p in 1..tot.len() {
        if tot[p] >= 16 {
            novelty = novelty.min(miss[p] as f64 / tot[p] as f64);
        }
    }
    h * (2.0 * novelty).min(1.0)
}
/// Watch a row path from one start: the same three walks.
pub fn watch_row(rule: &[u32], start: &[u64; ROW_WORDS]) -> Watch {
    const DRIFTS: usize = (2 * MAX_DRIFT + 1) as usize;
    let f = |a: [u64; ROW_WORDS]| step_row(rule, &a);
    // 1. the quiet run (the same row turned, with a period up to MAX_PERIOD), the loop
    let mut ring = [[0u64; ROW_WORDS]; MAX_PERIOD];
    let mut alive = [true; MAX_PERIOD * DRIFTS];
    let mut death = [0u32; MAX_PERIOD * DRIFTS];
    let mut left = MAX_PERIOD * DRIFTS;
    let mut quiet = None::<u32>;
    let (mut tort, mut power, mut lam) = (*start, 1u64, 1u64);
    let mut lambda = None::<u64>;
    let mut a = *start;
    for t in 0..T as u32 {
        if quiet.is_none() {
            for p in 1..=MAX_PERIOD as u32 {
                if t < p {
                    continue;
                }
                let before = ring[((t - p) % MAX_PERIOD as u32) as usize];
                for di in 0..DRIFTS {
                    let i = (p as usize - 1) * DRIFTS + di;
                    if alive[i] && rot(&before, di as i32 - MAX_DRIFT) != a {
                        alive[i] = false;
                        death[i] = t;
                        left -= 1;
                    }
                }
            }
            if left == 0 {
                quiet = Some(quiet_of(&death, DRIFTS));
            }
        }
        ring[(t % MAX_PERIOD as u32) as usize] = a;
        let y = f(a);
        if y == tort {
            lambda = Some(lam);
            break;
        }
        if power == lam {
            tort = y;
            power *= 2;
            lam = 0;
        }
        lam += 1;
        a = y;
    }
    let Some(q) = quiet else { return Watch { quiet: T as u32, post: 0, code: NEVER, after: 0.0 } };
    // 2. where the loop starts
    let end = lambda.map(|l| loop_start(*start, l, f)).unwrap_or(T as u32).min(T as u32);
    let post = end.saturating_sub(q);
    let code = verdict(quiet, post, lambda.is_some(), false);
    if code != OK {
        return Watch { quiet: q, post, code, after: 0.0 };
    }
    // 3. what follows: its four-cell blocks, its density, and how far each row is from the nearest of the last
    //    MAX_PERIOD rows turned (a row that is only an old row moved is no news)
    let stop = end.min(q + (AFTER_MAX / 4) as u32);
    let mut blocks = [0u32; 16];
    let (mut ones, mut rows) = (0u64, 0u64);
    let mut news = 0.0f64;
    let mut ring = [[0u64; ROW_WORDS]; MAX_PERIOD];
    let mut a = *start;
    for t in 0..stop {
        if t >= q {
            for w in a.iter() {
                for k in 0..16 {
                    blocks[(w >> (4 * k) & 15) as usize] += 1;
                }
                ones += w.count_ones() as u64;
            }
            rows += 1;
            let mut near = ROW_BITS as u32;
            for p in 1..=(MAX_PERIOD as u32).min(t) {
                let before = ring[((t - p) % MAX_PERIOD as u32) as usize];
                for d in -MAX_DRIFT..=MAX_DRIFT {
                    let r = rot(&before, d);
                    let ham: u32 = (0..ROW_WORDS).map(|i| (r[i] ^ a[i]).count_ones()).sum();
                    near = near.min(ham);
                }
            }
            let pd = a.iter().map(|w| w.count_ones()).sum::<u32>() as f64 / ROW_BITS as f64;
            let expect = (ROW_BITS as f64 * 2.0 * pd * (1.0 - pd)).max(1.0);
            news += (near as f64 / expect).min(1.0);
        }
        ring[(t % MAX_PERIOD as u32) as usize] = a;
        a = f(a);
    }
    let rows_f = rows.max(1) as f64;
    let p = ones as f64 / (rows_f * ROW_BITS as f64);
    let h = if p <= 0.0 || p >= 1.0 { 0.0 } else { -p * p.log2() - (1.0 - p) * (1.0 - p).log2() };
    let after = (0.5 * entropy_of(&blocks) / 4.0 + 0.5 * h).min(1.0) * (news / rows_f).min(1.0);
    Watch { quiet: q, post, code, after }
}

fn entropy_of(counts: &[u32]) -> f64 {
    let n: u64 = counts.iter().map(|&c| c as u64).sum();
    if n == 0 {
        return 0.0;
    }
    counts.iter().filter(|&&c| c > 0).map(|&c| {
        let p = c as f64 / n as f64;
        -p * p.log2()
    }).sum()
}

/// A watch read: the quiet run, how complex what came after it is, and why it is no use if it is none.
pub fn reading(w: &Watch, world: World) -> Reading {
    let reject = |why| Reading { quiet: w.quiet as usize, after: 0.0, rejected: Some(why) };
    match w.code {
        NEVER => return reject("never changes"),
        EXPLODES => return reject("explodes"),
        LOOPS => return reject("falls into a loop right after the change"),
        SHORT => return reject("too little after the change"),
        _ => {}
    }
    let _ = world;
    let after = w.after;
    Reading { quiet: w.quiet as usize, after, rejected: if after < 0.35 { Some("what follows the change is simple") } else { None } }
}

/// A rule's score from a reading: the quiet run (in bits), times how complex the change is, times how short the rule is.
pub fn score(r: &Reading, len: usize) -> f64 {
    if r.rejected.is_some() || r.quiet < 2 {
        return 0.0;
    }
    let simple = 1.0 / (1.0 + 0.25 * (len.saturating_sub(1)) as f64);
    (1.0 + r.quiet as f64).log2() * r.after * simple
}

/// A rule weighed from every start; the best start's reading.
#[derive(Clone, Debug)]
pub struct Weighed {
    pub score: f64,
    pub start: usize,
    pub reading: Reading,
}

pub fn weigh_watches(rule: &Rule, watches: &[Watch]) -> Weighed {
    let mut best = Weighed { score: -1.0, start: 0, reading: Reading { quiet: 0, after: 0.0, rejected: Some("no start") } };
    for (i, w) in watches.iter().enumerate() {
        let r = reading(w, rule.world());
        let s = score(&r, rule.steps.len());
        if s > best.score || (s == best.score && best.reading.rejected.is_some() && r.rejected.is_none()) {
            best = Weighed { score: s, start: i, reading: r };
        }
    }
    best
}

/// The watches of a rule from every start, on the CPU: the reference the GPU is checked against.
pub fn watches(rule: &Rule) -> Vec<Watch> {
    let codes = rule.codes();
    match rule.world() {
        World::Pair => pair_starts().into_iter().map(|s| watch_pair(&codes, s)).collect(),
        World::Number => number_starts().into_iter().map(|s| watch_number(&codes, s)).collect(),
        World::Row => row_starts().iter().map(|s| watch_row(&codes, s)).collect(),
    }
}

pub fn weigh(rule: &Rule) -> Weighed {
    weigh_watches(rule, &watches(rule))
}

// ------------------------------------------------------------------ the GPU: the same watch, one thread a rule and a start

const KERNELS: &str = r#"
typedef unsigned long long u64;
// a 128-bit number as two words: NVRTC on Windows has no __int128
struct U { u64 hi, lo; };
#define LIM_HI (1ull << 56)
#define T_ITER 8192
#define MAXP 8
#define MAXD 8
#define DRIFTS 17
#define PREDICT 64
#define AFTER_MIN 64
#define AFTER_MAX 1024
#define OP_ADD 1
#define OP_MUL 2
#define OP_SHR 3
#define OP_SHL 4
#define OP_XOR 5
#define OP_MOD 6
#define OP_IF 7
#define OP_XORC 10
#define OP_ORC 11
#define OP_ANDC 12
#define OP_NOT 13
#define OP_ROT 14

__device__ int over(U x) { return x.hi > LIM_HI || (x.hi == LIM_HI && x.lo > 0); }
__device__ int gtu(U a, U b) { return a.hi > b.hi || (a.hi == b.hi && a.lo > b.lo); }
__device__ int equ(U a, U b) { return a.hi == b.hi && a.lo == b.lo; }
__device__ u64 umod(U x, u64 m) {
    u64 two64 = ((~0ull) % m + 1) % m;
    return ((x.hi % m) * two64 % m + x.lo % m) % m;
}

__device__ int basic(unsigned op, unsigned v, U* x) {
    U y = *x;
    if (op == OP_ADD) {
        u64 lo = y.lo + v; u64 c = lo < y.lo; if (c && y.hi == ~0ull) return 0; y.hi += c; y.lo = lo;
    } else if (op == OP_MUL) {
        u64 carry = __umul64hi(y.lo, (u64)v);
        if (__umul64hi(y.hi, (u64)v) != 0) return 0;
        u64 hi = y.hi * v; u64 hi2 = hi + carry; if (hi2 < hi) return 0;
        y.lo = y.lo * v; y.hi = hi2;
    } else if (op == OP_SHR) {
        if (v >= 64) { y.lo = y.hi >> (v - 64); y.hi = 0; }
        else if (v > 0) { y.lo = (y.lo >> v) | (y.hi << (64 - v)); y.hi >>= v; }
    } else if (op == OP_SHL) {
        if (v > 0) { if ((y.hi >> (63 - v)) != 0) return 0; y.hi = (y.hi << v) | (y.lo >> (64 - v)); y.lo <<= v; }
    } else if (op == OP_XOR) {
        y.lo ^= (u64)v;
    } else if (op == OP_MOD) {
        if (v) { y.lo = umod(y, v); y.hi = 0; }
    }
    if (over(y)) return 0;
    *x = y;
    return 1;
}

__device__ int step_number(const unsigned* code, int len, U* x) {
    for (int i = 0; i < len; i++) {
        unsigned c = code[i], op = c & 255, a = (c >> 8) & 255, b = (c >> 16) & 255, cc = (c >> 24) & 255;
        if (op == OP_IF) { if (a && umod(*x, a) == b) { if (!basic(cc & 15, cc >> 4, x)) return 0; } }
        else if (!basic(op, a, x)) return 0;
    }
    return 1;
}

__device__ U f_number(const unsigned* code, int len, U x) {
    U y = x;
    if (!step_number(code, len, &y)) { y.hi = 0; y.lo = 0; }
    return y;
}

__device__ unsigned char symbol(U x, U y) { return (gtu(y, x) ? 2 : (gtu(x, y) ? 1 : 0)) * 2 + (unsigned char)(x.lo & 1); }

__device__ double entropy(const unsigned* c, int n) {
    double tot = 0, h = 0;
    for (int i = 0; i < n; i++) tot += c[i];
    if (tot == 0) return 0;
    for (int i = 0; i < n; i++) if (c[i]) { double p = c[i] / tot; h -= p * log2(p); }
    return h;
}

__device__ unsigned verdict(int quiet_set, unsigned post, int looped, int blew) {
    if (blew) return 2;
    if (!quiet_set) return 1;
    if (looped && post < AFTER_MIN) return 3;
    if (post < AFTER_MIN) return 4;
    return 0;
}

// one thread: one number rule from one start; three walks forward, none looking back
extern "C" __global__ void numbers(const unsigned* codes, const int* lens, int maxs, int R, const u64* lo, const u64* hi,
                                   int S, unsigned* quiet_o, unsigned* post_o, unsigned* code_o, double* after_o) {
    int id = blockIdx.x * blockDim.x + threadIdx.x;
    if (id >= R * S) return;
    int r = id / S, s = id % S;
    const unsigned* code = codes + r * maxs;
    int len = lens[r];
    U start; start.hi = hi[s]; start.lo = lo[s];
    // 1. the quiet run, the loop, a blow-up
    unsigned char ring[MAXP];
    unsigned alive = (1u << MAXP) - 1, quiet = T_ITER, death[MAXP + 1];
    for (int i = 0; i <= MAXP; i++) death[i] = 0;
    int quiet_set = 0, blew = 0;
    u64 lambda = 0;
    U x = start, tort = start; u64 power = 1, lam = 1;
    for (unsigned t = 0; t < T_ITER; t++) {
        U y = x;
        if (!step_number(code, len, &y)) { blew = 1; break; }
        unsigned char sy = symbol(x, y);
        if (!quiet_set) {
            for (unsigned p = 1; p <= MAXP; p++)
                if (t >= p && ((alive >> (p - 1)) & 1) && ring[(t - p) % MAXP] != sy) { alive &= ~(1u << (p - 1)); death[p] = t; }
            if (alive == 0) {
                quiet = 0;
                for (unsigned p = 1; p <= MAXP; p++) if (death[p] >= 3 * p && death[p] > quiet) quiet = death[p];
                quiet_set = 1;
            }
        }
        ring[t % MAXP] = sy;
        if (equ(y, tort)) { lambda = lam; break; }
        if (power == lam) { tort = y; power *= 2; lam = 0; }
        lam++;
        x = y;
    }
    quiet_o[id] = quiet; post_o[id] = 0; after_o[id] = 0;
    if (blew) { code_o[id] = 2; return; }
    if (!quiet_set) { code_o[id] = 1; return; }
    // 2. where the loop starts
    unsigned end = T_ITER;
    if (lambda) {
        U hare = start, tt = start;
        for (u64 i = 0; i < lambda; i++) hare = f_number(code, len, hare);
        unsigned mu = 0;
        while (!equ(tt, hare) && mu < T_ITER) { tt = f_number(code, len, tt); hare = f_number(code, len, hare); mu++; }
        end = mu < T_ITER ? mu : T_ITER;
    }
    unsigned post = end > quiet ? end - quiet : 0;
    unsigned v = verdict(1, post, lambda != 0, 0);
    post_o[id] = post; code_o[id] = v;
    if (v != 0) return;
    // 3. what follows the change
    unsigned stop = end < quiet + AFTER_MAX ? end : quiet + AFTER_MAX;
    unsigned c1[6], c3[216], miss[PREDICT + 1], tot[PREDICT + 1];
    unsigned char pr[PREDICT];
    for (int i = 0; i < 6; i++) c1[i] = 0;
    for (int i = 0; i < 216; i++) c3[i] = 0;
    for (int i = 0; i <= PREDICT; i++) { miss[i] = 0; tot[i] = 0; }
    x = start;
    for (unsigned t = 0; t < stop; t++) {
        U y = f_number(code, len, x);
        unsigned char sy = symbol(x, y);
        if (t >= quiet) {
            c1[sy]++;
            if (t >= quiet + 2) c3[pr[(t - 2) % PREDICT] * 36 + pr[(t - 1) % PREDICT] * 6 + sy]++;
            for (unsigned p = 1; p <= PREDICT; p++)
                if (t >= quiet + p) { tot[p]++; miss[p] += pr[(t - p) % PREDICT] != sy; }
        }
        pr[t % PREDICT] = sy;
        x = y;
    }
    int used = 0;
    for (int i = 0; i < 6; i++) used += c1[i] > 0;
    if (used < 2) return;
    double lu = log2((double)used), n3 = 0;
    for (int i = 0; i < 216; i++) n3 += c3[i];
    double cap = (double)(used * used * used), m = n3 < 2 ? 2 : n3;
    double h3max = log2(cap < m ? cap : m);
    double h = 0.5 * entropy(c1, 6) / lu + 0.5 * entropy(c3, 216) / h3max;
    if (h > 1) h = 1;
    double nov = 1.0;
    for (int p = 1; p <= PREDICT; p++) if (tot[p] >= 16) { double q = (double)miss[p] / tot[p]; if (q < nov) nov = q; }
    double k = 2.0 * nov; if (k > 1) k = 1;
    after_o[id] = h * k;
}

__device__ void rot(const u64* row, int k, u64* out) {
    k = ((k % 256) + 256) % 256;
    int w = k / 64, b = k % 64;
    for (int i = 0; i < 4; i++) {
        int src = (i + 4 - w) % 4, prev = (src + 3) % 4;
        out[i] = b == 0 ? row[src] : ((row[src] << b) | (row[prev] >> (64 - b)));
    }
}

__device__ void step_row(const unsigned* code, int len, const u64* row, u64* a) {
    u64 c[4], rc[4];
    for (int i = 0; i < 4; i++) { c[i] = row[i]; a[i] = row[i]; }
    for (int j = 0; j < len; j++) {
        unsigned op = code[j] & 255;
        int k = (int)(signed char)((code[j] >> 8) & 255);
        rot(c, k, rc);
        for (int i = 0; i < 4; i++) {
            if (op == OP_XORC) a[i] ^= rc[i];
            else if (op == OP_ORC) a[i] |= rc[i];
            else if (op == OP_ANDC) a[i] &= rc[i];
            else if (op == OP_NOT) a[i] = ~a[i];
        }
        if (op == OP_ROT) { u64 t2[4]; rot(a, k, t2); for (int i = 0; i < 4; i++) a[i] = t2[i]; }
    }
}

__device__ int eq4(const u64* a, const u64* b) { return a[0] == b[0] && a[1] == b[1] && a[2] == b[2] && a[3] == b[3]; }

// one thread: one row rule from one start; the same three walks
extern "C" __global__ void rows(const unsigned* codes, const int* lens, int maxs, int R, const u64* starts, int S,
                                unsigned* quiet_o, unsigned* post_o, unsigned* code_o, double* after_o) {
    int id = blockIdx.x * blockDim.x + threadIdx.x;
    if (id >= R * S) return;
    int r = id / S, s = id % S;
    const unsigned* code = codes + r * maxs;
    int len = lens[r];
    u64 start[4], ring[MAXP][4], a[4], y[4], tort[4], tmp[4];
    for (int i = 0; i < 4; i++) { start[i] = starts[s * 4 + i]; a[i] = start[i]; tort[i] = start[i]; }
    // 1. the quiet run, the loop
    unsigned char alive[MAXP * DRIFTS];
    unsigned short death[MAXP * DRIFTS];
    int left = MAXP * DRIFTS;
    for (int i = 0; i < MAXP * DRIFTS; i++) { alive[i] = 1; death[i] = 0; }
    u64 power = 1, lam = 1, lambda = 0;
    unsigned quiet = T_ITER;
    int quiet_set = 0;
    for (unsigned t = 0; t < T_ITER; t++) {
        if (!quiet_set) {
            for (unsigned p = 1; p <= MAXP; p++) {
                if (t < p) continue;
                const u64* before = ring[(t - p) % MAXP];
                for (int di = 0; di < DRIFTS; di++) {
                    int i = (p - 1) * DRIFTS + di;
                    if (!alive[i]) continue;
                    rot(before, di - MAXD, tmp);
                    if (!eq4(tmp, a)) { alive[i] = 0; death[i] = (unsigned short)t; left--; }
                }
            }
            if (left == 0) {
                quiet = 0;
                for (int i = 0; i < MAXP * DRIFTS; i++) { unsigned p = i / DRIFTS + 1; if (death[i] >= 3 * p && death[i] > quiet) quiet = death[i]; }
                quiet_set = 1;
            }
        }
        for (int i = 0; i < 4; i++) ring[t % MAXP][i] = a[i];
        step_row(code, len, a, y);
        if (eq4(y, tort)) { lambda = lam; break; }
        if (power == lam) { for (int i = 0; i < 4; i++) tort[i] = y[i]; power *= 2; lam = 0; }
        lam++;
        for (int i = 0; i < 4; i++) a[i] = y[i];
    }
    quiet_o[id] = quiet; post_o[id] = 0; after_o[id] = 0;
    if (!quiet_set) { code_o[id] = 1; return; }
    // 2. where the loop starts
    unsigned end = T_ITER;
    if (lambda) {
        u64 hare[4], tt[4];
        for (int i = 0; i < 4; i++) { hare[i] = start[i]; tt[i] = start[i]; }
        for (u64 j = 0; j < lambda; j++) { step_row(code, len, hare, tmp); for (int i = 0; i < 4; i++) hare[i] = tmp[i]; }
        unsigned mu = 0;
        while (!eq4(tt, hare) && mu < T_ITER) {
            step_row(code, len, tt, tmp); for (int i = 0; i < 4; i++) tt[i] = tmp[i];
            step_row(code, len, hare, tmp); for (int i = 0; i < 4; i++) hare[i] = tmp[i];
            mu++;
        }
        end = mu < T_ITER ? mu : T_ITER;
    }
    unsigned post = end > quiet ? end - quiet : 0;
    unsigned v = verdict(1, post, lambda != 0, 0);
    post_o[id] = post; code_o[id] = v;
    if (v != 0) return;
    // 3. what follows: blocks, density, and how far each row is from the nearest recent row turned
    unsigned stop = end < quiet + AFTER_MAX / 4 ? end : quiet + AFTER_MAX / 4;
    unsigned blocks[16];
    for (int i = 0; i < 16; i++) blocks[i] = 0;
    u64 ones = 0, nrows = 0;
    double news = 0;
    for (int i = 0; i < 4; i++) a[i] = start[i];
    for (unsigned t = 0; t < stop; t++) {
        if (t >= quiet) {
            unsigned pc = 0;
            for (int w = 0; w < 4; w++) {
                for (int k = 0; k < 16; k++) blocks[(a[w] >> (4 * k)) & 15]++;
                pc += __popcll(a[w]);
            }
            ones += pc; nrows++;
            unsigned near = 256;
            unsigned pmax = t < MAXP ? t : MAXP;
            for (unsigned p = 1; p <= pmax; p++) {
                const u64* before = ring[(t - p) % MAXP];
                for (int d = -MAXD; d <= MAXD; d++) {
                    rot(before, d, tmp);
                    unsigned ham = 0;
                    for (int i = 0; i < 4; i++) ham += __popcll(tmp[i] ^ a[i]);
                    if (ham < near) near = ham;
                }
            }
            double pd = pc / 256.0;
            double expect = 256.0 * 2.0 * pd * (1.0 - pd); if (expect < 1) expect = 1;
            double q = near / expect; news += q < 1 ? q : 1;
        }
        for (int i = 0; i < 4; i++) ring[t % MAXP][i] = a[i];
        step_row(code, len, a, y);
        for (int i = 0; i < 4; i++) a[i] = y[i];
    }
    double rowsf = nrows > 0 ? (double)nrows : 1.0;
    double p = ones / (rowsf * 256.0);
    double h = (p <= 0 || p >= 1) ? 0 : -p * log2(p) - (1 - p) * log2(1 - p);
    double base = 0.5 * entropy(blocks, 16) / 4.0 + 0.5 * h; if (base > 1) base = 1;
    double nv = news / rowsf; if (nv > 1) nv = 1;
    after_o[id] = base * nv;
}
"#;

pub struct Gpu {
    dev: std::sync::Arc<cudarc::driver::CudaDevice>,
}

impl Gpu {
    pub fn new() -> Result<Gpu, String> {
        let dev = cudarc::driver::CudaDevice::new(0).map_err(|e| format!("no GPU: {e:?}"))?;
        let ptx = cudarc::nvrtc::compile_ptx(KERNELS).map_err(|e| format!("{e:?}"))?;
        dev.load_ptx(ptx, "tree", &["numbers", "rows"]).map_err(|e| format!("{e:?}"))?;
        Ok(Gpu { dev })
    }

    /// The watches of many rules of one world from every start, all at once; only the watches come back.
    pub fn watches(&self, rules: &[&Rule], world: World) -> Result<Vec<Vec<Watch>>, String> {
        use cudarc::driver::{LaunchAsync, LaunchConfig};
        let e = |x: cudarc::driver::DriverError| format!("{x:?}");
        let r = rules.len();
        if r == 0 {
            return Ok(vec![]);
        }
        let maxs = MAX_LEN;
        let mut codes = vec![0u32; r * maxs];
        let mut lens = vec![0i32; r];
        for (i, rule) in rules.iter().enumerate() {
            for (j, c) in rule.codes().iter().enumerate().take(maxs) {
                codes[i * maxs + j] = *c;
            }
            lens[i] = rule.steps.len().min(maxs) as i32;
        }
        let codes_d = self.dev.htod_copy(codes).map_err(e)?;
        let lens_d = self.dev.htod_copy(lens).map_err(e)?;
        let s = match world {
            World::Number => number_starts().len(),
            World::Row => row_starts().len(),
            World::Pair => return Err("pair paths are watched on the CPU".into()),
        };
        let n = r * s;
        let mut quiet = self.dev.alloc_zeros::<u32>(n).map_err(e)?;
        let mut post = self.dev.alloc_zeros::<u32>(n).map_err(e)?;
        let mut code = self.dev.alloc_zeros::<u32>(n).map_err(e)?;
        let mut after = self.dev.alloc_zeros::<f64>(n).map_err(e)?;
        let cfg = LaunchConfig { grid_dim: (n.div_ceil(64) as u32, 1, 1), block_dim: (64, 1, 1), shared_mem_bytes: 0 };
        match world {
            World::Number => {
                let starts = number_starts();
                let lo: Vec<u64> = starts.iter().map(|x| *x as u64).collect();
                let hi: Vec<u64> = starts.iter().map(|x| (*x >> 64) as u64).collect();
                let (lo_d, hi_d) = (self.dev.htod_copy(lo).map_err(e)?, self.dev.htod_copy(hi).map_err(e)?);
                let f = self.dev.get_func("tree", "numbers").ok_or("no kernel")?;
                unsafe { f.launch(cfg, (&codes_d, &lens_d, maxs as i32, r as i32, &lo_d, &hi_d, s as i32, &mut quiet, &mut post, &mut code, &mut after)) }.map_err(e)?;
            }
            World::Row => {
                let flat: Vec<u64> = row_starts().iter().flat_map(|r| r.iter().copied()).collect();
                let st_d = self.dev.htod_copy(flat).map_err(e)?;
                let f = self.dev.get_func("tree", "rows").ok_or("no kernel")?;
                unsafe { f.launch(cfg, (&codes_d, &lens_d, maxs as i32, r as i32, &st_d, s as i32, &mut quiet, &mut post, &mut code, &mut after)) }.map_err(e)?;
            }
            World::Pair => unreachable!(),
        }
        let (quiet, post, code, after) = (
            self.dev.dtoh_sync_copy(&quiet).map_err(e)?,
            self.dev.dtoh_sync_copy(&post).map_err(e)?,
            self.dev.dtoh_sync_copy(&code).map_err(e)?,
            self.dev.dtoh_sync_copy(&after).map_err(e)?,
        );
        Ok((0..r)
            .map(|i| {
                (0..s)
                    .map(|j| {
                        let k = i * s + j;
                        Watch { quiet: quiet[k], post: post[k], code: code[k], after: after[k] }
                    })
                    .collect()
            })
            .collect())
    }
}

/// Weigh many rules: watched on the GPU when there is one, else on the CPU in parallel; read on the CPU.
pub fn weigh_all(rules: &[Rule], gpu: Option<&Gpu>) -> Vec<Weighed> {
    let mut out: Vec<Option<Weighed>> = vec![None; rules.len()];
    for world in [World::Number, World::Row, World::Pair] {
        let idx: Vec<usize> = (0..rules.len()).filter(|&i| rules[i].world() == world && !rules[i].steps.is_empty()).collect();
        if idx.is_empty() {
            continue;
        }
        // a small batch is quicker on the CPU than the trip to the GPU and back
        // pair paths are watched on the CPU only
        let gpu = if idx.len() >= GPU_MIN && world != World::Pair { gpu } else { None };
        let on_gpu: Option<Vec<Vec<Watch>>> = gpu.and_then(|g| {
            let refs: Vec<&Rule> = idx.iter().map(|&i| &rules[i]).collect();
            g.watches(&refs, world).ok()
        });
        match on_gpu {
            Some(ws) => {
                for (k, &i) in idx.iter().enumerate() {
                    out[i] = Some(weigh_watches(&rules[i], &ws[k]));
                }
            }
            None => {
                let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
                let chunk = idx.len().div_ceil(threads).max(1);
                let done: Vec<(usize, Weighed)> = std::thread::scope(|s| {
                    let jobs: Vec<_> = idx.chunks(chunk).map(|part| s.spawn(move || part.iter().map(|&i| (i, weigh(&rules[i]))).collect::<Vec<_>>())).collect();
                    jobs.into_iter().flat_map(|j| j.join().unwrap()).collect()
                });
                for (i, w) in done {
                    out[i] = Some(w);
                }
            }
        }
    }
    out.into_iter()
        .map(|w| w.unwrap_or(Weighed { score: 0.0, start: 0, reading: Reading { quiet: 0, after: 0.0, rejected: Some("no steps") } }))
        .collect()
}

// ------------------------------------------------------------------ the tree

#[derive(Clone, Debug)]
pub struct Node {
    pub id: usize,
    pub parent: Option<usize>,
    pub step: Option<Step>,
    pub depth: usize,
    pub children: Vec<usize>,
    pub weighed: Option<Weighed>,
    pub alive: bool,
}

/// A grown tree: its nodes, the trunk of steps Golden took, and the deepest any node was made beyond where it stood.
pub struct Tree {
    /// the worlds the root's children are drawn from, in turn
    pub worlds: Vec<World>,
    pub seed: u64,
    pub render_distance: usize,
    pub nodes: Vec<Node>,
    pub trunk: Vec<usize>,
    /// (node, score) of every branch dropped, for the picture
    pub dropped: Vec<(usize, usize, f64)>,
    /// the furthest beyond the current position a node was ever made
    pub deepest_ahead: usize,
    pub weighed: usize,
}

impl Tree {
    fn rule_of(&self, mut n: usize) -> Rule {
        let mut steps = Vec::new();
        while let Some(s) = self.nodes[n].step {
            steps.push(s);
            n = self.nodes[n].parent.unwrap();
        }
        steps.reverse();
        Rule { steps }
    }

    fn best_in(&self, n: usize) -> f64 {
        let own = self.nodes[n].weighed.as_ref().map(|w| w.score).unwrap_or(0.0);
        self.nodes[n].children.iter().map(|&c| self.best_in(c)).fold(own, f64::max)
    }

    fn kill(&mut self, n: usize) {
        self.nodes[n].alive = false;
        for c in self.nodes[n].children.clone() {
            self.kill(c);
        }
    }

    /// Make the frontier: every live node above the render distance gets its children. Nothing deeper is made.
    fn grow_frontier(&mut self, at: usize, gpu: Option<&Gpu>) {
        let limit = self.nodes[at].depth + self.render_distance;
        let mut made = Vec::new();
        let mut stack = vec![at];
        while let Some(n) = stack.pop() {
            if self.nodes[n].depth >= limit || self.nodes[n].depth >= MAX_LEN {
                continue;
            }
            if self.nodes[n].children.is_empty() {
                let world = self.nodes[n].step.map(|s| s.world());
                let mut rng = Rng::new(self.seed.wrapping_mul(0x100_0000_01B3) ^ (n as u64).wrapping_mul(0x9E37_79B9));
                let mut seen = HashSet::new();
                for k in 0..BRANCH * 3 {
                    if self.nodes[n].children.len() == BRANCH {
                        break;
                    }
                    // the first step chooses the world: half the root's children each
                    let w = world.unwrap_or(self.worlds[k % self.worlds.len()]);
                    let step = draw_step(&mut rng, w);
                    if !seen.insert(step) {
                        continue;
                    }
                    let id = self.nodes.len();
                    let depth = self.nodes[n].depth + 1;
                    self.deepest_ahead = self.deepest_ahead.max(depth - self.nodes[at].depth);
                    self.nodes.push(Node { id, parent: Some(n), step: Some(step), depth, children: vec![], weighed: None, alive: true });
                    self.nodes[n].children.push(id);
                    made.push(id);
                }
            }
            for &c in &self.nodes[n].children {
                if self.nodes[c].alive {
                    stack.push(c);
                }
            }
        }
        made.sort();
        let rules: Vec<Rule> = made.iter().map(|&n| self.rule_of(n)).collect();
        let w = weigh_all(&rules, gpu);
        self.weighed += w.len();
        for (n, w) in made.into_iter().zip(w) {
            self.nodes[n].weighed = Some(w);
        }
    }
}

/// Grow a tree: `steps` moves, each one step ahead, with the tree only `render_distance` steps beyond where Golden
/// stands. A path that reaches the longest length starts again from the root, its old branches dropped.
pub fn grow(steps: usize, render_distance: usize, seed: u64, gpu: Option<&Gpu>) -> Tree {
    grow_in(&[World::Number, World::Row], steps, render_distance, seed, gpu)
}

/// Grow a tree whose paths are drawn from the given worlds.
pub fn grow_in(worlds: &[World], steps: usize, render_distance: usize, seed: u64, gpu: Option<&Gpu>) -> Tree {
    let mut tree = Tree {
        worlds: worlds.to_vec(),
        seed,
        render_distance: render_distance.max(1),
        nodes: vec![Node { id: 0, parent: None, step: None, depth: 0, children: vec![], weighed: None, alive: true }],
        trunk: vec![0],
        dropped: vec![],
        deepest_ahead: 0,
        weighed: 0,
    };
    let mut at = 0;
    tree.grow_frontier(at, gpu);
    for _ in 0..steps {
        let kids: Vec<usize> = tree.nodes[at].children.iter().copied().filter(|&c| tree.nodes[c].alive).collect();
        if kids.is_empty() || tree.nodes[at].depth >= MAX_LEN {
            // the path is as long as a path gets: back to the root, with fresh branches
            for c in tree.nodes[0].children.clone() {
                let best = tree.best_in(c);
                tree.dropped.push((0, c, best));
                tree.kill(c);
            }
            tree.nodes[0].children.clear();
            at = 0;
            tree.trunk.push(0);
            tree.grow_frontier(at, gpu);
            continue;
        }
        let next = *kids.iter().max_by(|&&a, &&b| tree.best_in(a).partial_cmp(&tree.best_in(b)).unwrap().then(b.cmp(&a))).unwrap();
        for &c in &kids {
            if c != next {
                let best = tree.best_in(c);
                tree.dropped.push((at, c, best));
                tree.kill(c);
            }
        }
        at = next;
        tree.trunk.push(at);
        tree.grow_frontier(at, gpu);
    }
    tree
}

// ------------------------------------------------------------------ the vault

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: String,
    pub rule: String,
    pub codes: Vec<u32>,
    pub seed: u64,
    pub score: f64,
    pub quiet: usize,
    pub after: f64,
    pub start: usize,
    pub world: World,
}

fn rule_id(codes: &[u32]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for c in codes {
        for b in c.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{:012x}", h & 0xffff_ffff_ffff)
}

/// The best paths of a tree, one entry each rule, best first.
pub fn best(tree: &Tree, how_many: usize) -> Vec<Entry> {
    let mut seen = HashSet::new();
    let mut all: Vec<(f64, usize)> = tree.nodes.iter().filter_map(|n| n.weighed.as_ref().map(|w| (w.score, n.id))).filter(|(s, _)| *s > 0.0).collect();
    all.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
    let mut out = Vec::new();
    for (_, n) in all {
        let rule = tree.rule_of(n);
        let codes = rule.codes();
        if !seen.insert(codes.clone()) {
            continue;
        }
        let w = tree.nodes[n].weighed.clone().unwrap();
        out.push(Entry {
            id: rule_id(&codes),
            rule: rule.text(),
            codes,
            seed: tree.seed,
            score: w.score,
            quiet: w.reading.quiet,
            after: w.reading.after,
            start: w.start,
            world: rule.world(),
        });
        if out.len() == how_many {
            break;
        }
    }
    out
}

fn vault_dir() -> PathBuf {
    Path::new(OUT).join("vault")
}

impl Entry {
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "id": self.id, "rule": self.rule, "codes": self.codes, "seed": self.seed, "score": self.score,
            "quiet": self.quiet, "after": self.after, "start": self.start,
            "world": match self.world { World::Number => "number", World::Row => "row", World::Pair => "pair" },
        })
        .to_string()
    }

    pub fn from_json(s: &str) -> Option<Entry> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        Some(Entry {
            id: v["id"].as_str()?.to_string(),
            rule: v["rule"].as_str()?.to_string(),
            codes: v["codes"].as_array()?.iter().filter_map(|c| c.as_u64().map(|c| c as u32)).collect(),
            seed: v["seed"].as_u64()?,
            score: v["score"].as_f64()?,
            quiet: v["quiet"].as_u64()? as usize,
            after: v["after"].as_f64()?,
            start: v["start"].as_u64()? as usize,
            world: match v["world"].as_str()? {
                "row" => World::Row,
                "pair" => World::Pair,
                _ => World::Number,
            },
        })
    }
}

pub fn save(entries: &[Entry]) -> Result<(), String> {
    let dir = vault_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for e in entries {
        std::fs::write(dir.join(format!("{}.json", e.id)), e.to_json()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn vault() -> Vec<Entry> {
    let mut out: Vec<Entry> = std::fs::read_dir(vault_dir())
        .map(|d| d.filter_map(|f| f.ok()).filter_map(|f| std::fs::read_to_string(f.path()).ok()).filter_map(|s| Entry::from_json(&s)).collect())
        .unwrap_or_default();
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap().then(a.id.cmp(&b.id)));
    out
}

pub fn find(id: &str) -> Option<Entry> {
    vault().into_iter().find(|e| e.id.starts_with(id))
}

/// An entry checked from scratch: its rule iterated again on the CPU, its quiet run and score read again.
pub fn check(e: &Entry) -> Result<Weighed, String> {
    let rule = Rule::from_codes(&e.codes);
    if rule.text() != e.rule {
        return Err("the rule's text is not its codes".into());
    }
    let w = weigh(&rule);
    if w.reading.quiet != e.quiet || (w.score - e.score).abs() > 1e-9 {
        return Err(format!("iterated again: quiet {} (says {}), score {:.4} (says {:.4})", w.reading.quiet, e.quiet, w.score, e.score));
    }
    Ok(w)
}

// ------------------------------------------------------------------ pictures, golden on black

struct Image {
    w: usize,
    h: usize,
    px: Vec<[f32; 3]>,
}

const GOLD: [f32; 3] = [1.0, 0.76, 0.22];

impl Image {
    fn new(w: usize, h: usize) -> Image {
        Image { w, h, px: vec![[0.0; 3]; w * h] }
    }
    fn add(&mut self, x: i64, y: i64, a: f32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let p = &mut self.px[y as usize * self.w + x as usize];
            for i in 0..3 {
                p[i] = (p[i] + GOLD[i] * a).min(1.0);
            }
        }
    }
    fn dot(&mut self, x: f64, y: f64, r: f64, a: f32) {
        let ri = r.ceil() as i64 + 1;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                if d <= r + 0.5 {
                    let f = (1.0 - (d / (r + 0.5)) as f32).max(0.0);
                    self.add(x as i64 + dx, y as i64 + dy, a * (0.35 + 0.65 * f));
                }
            }
        }
    }
    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, a: f32, width: f64) {
        let n = ((x1 - x0).abs().max((y1 - y0).abs()).ceil() as usize).max(1);
        for i in 0..=n {
            let t = i as f64 / n as f64;
            self.dot(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, width, a);
        }
    }
    fn png(&self, path: &Path) -> Result<(), String> {
        let mut raw = Vec::with_capacity(self.h * (1 + self.w * 3));
        for y in 0..self.h {
            raw.push(0u8);
            for x in 0..self.w {
                for c in self.px[y * self.w + x] {
                    raw.push((c.clamp(0.0, 1.0) * 255.0) as u8);
                }
            }
        }
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, png_bytes(self.w as u32, self.h as u32, &raw)).map_err(|e| e.to_string())
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// A PNG of RGB rows (each led by its filter byte), deflate in stored blocks: no library needed.
fn png_bytes(w: u32, h: u32, raw: &[u8]) -> Vec<u8> {
    let chunk = |out: &mut Vec<u8>, kind: &[u8], data: &[u8]| {
        out.extend((data.len() as u32).to_be_bytes());
        let mut c = kind.to_vec();
        c.extend_from_slice(data);
        out.extend(&c);
        out.extend(crc32(&c).to_be_bytes());
    };
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= raw.len();
        z.push(last as u8);
        z.extend((block.len() as u16).to_le_bytes());
        z.extend((!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in raw {
        a = (a + x as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend(((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([8, 2, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A path drawn: a number path as its orbit (iteration across, log2 x up), a row path as its spacetime (rows down).
/// The quiet run is drawn dim, the change on, bright.
pub fn render(e: &Entry) -> Result<PathBuf, String> {
    let codes = e.codes.clone();
    let path = Path::new(OUT).join(format!("{}.png", e.id));
    match e.world {
        World::Number | World::Pair => {
            // a pair path is drawn by its x
            let (xs, w0) = if e.world == World::Number {
                let start = number_starts()[e.start.min(3)];
                let Trace::Number { xs, .. } = trace_number(&codes, start) else { unreachable!() };
                (xs, watch_number(&codes, start))
            } else {
                let start = pair_starts()[e.start.min(3)];
                let (mut xs, mut s) = (Vec::with_capacity(T), start);
                for _ in 0..T {
                    xs.push(s.0);
                    match step_pair(&codes, s) {
                        Some(n) => s = n,
                        None => break,
                    }
                }
                (xs, watch_pair(&codes, start))
            };
            // what is worth seeing: the quiet run, the change, a little of the loop it settles into
            let shown = ((w0.quiet + w0.post) as usize + 64).min(xs.len());
            let xs = &xs[..shown];
            let (w, h) = (1024usize, 512usize);
            let mut img = Image::new(w, h);
            let len = xs.len().max(2);
            let top = xs.iter().map(|x| 128 - x.leading_zeros()).max().unwrap_or(1).max(1) as f64;
            let mut prev: Option<(f64, f64)> = None;
            for (t, x) in xs.iter().enumerate() {
                let px = 8.0 + (w as f64 - 16.0) * t as f64 / (len - 1) as f64;
                let lx = if *x == 0 { 0.0 } else { (*x as f64).log2() };
                let py = h as f64 - 8.0 - (h as f64 - 16.0) * lx / top;
                let a = if t < e.quiet { 0.25 } else { 0.9 };
                if let Some((qx, qy)) = prev {
                    img.line(qx, qy, px, py, a * 0.5, 0.4);
                }
                img.dot(px, py, 1.0, a);
                prev = Some((px, py));
            }
            img.png(&path)?;
        }
        World::Row => {
            let start = row_starts()[e.start.min(1)];
            let Trace::Row { rows } = trace_row(&codes, &start) else { unreachable!() };
            let shown = rows.len().min(e.quiet + 640).max(64);
            let mut img = Image::new(ROW_BITS as usize * 2, shown * 2);
            for (t, r) in rows.iter().take(shown).enumerate() {
                let a = if t < e.quiet { 0.35 } else { 1.0 };
                for bit in 0..ROW_BITS as usize {
                    if r[bit / 64] >> (bit % 64) & 1 == 1 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                img.add((bit * 2 + dx) as i64, (t * 2 + dy) as i64, a);
                            }
                        }
                    }
                }
            }
            img.png(&path)?;
        }
    }
    Ok(path)
}

/// The tree drawn: the trunk is the steps taken (upward), each dropped branch a short stub where it left, the frontier
/// still alive spread at the top; the brighter and bigger a node, the higher its path scored.
pub fn render_tree(tree: &Tree) -> Result<PathBuf, String> {
    let (w, h) = (1200usize, 1200usize);
    let mut img = Image::new(w, h);
    let top = tree.nodes.iter().filter_map(|n| n.weighed.as_ref().map(|w| w.score)).fold(1e-9, f64::max);
    let glow = |s: f64| (0.15 + 0.85 * (s / top).max(0.0)) as f32;
    let seg = (h as f64 * 0.62) / tree.trunk.len().max(1) as f64;
    let base_x = w as f64 / 2.0;
    let mut pos: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut y = h as f64 - 30.0;
    let mut x = base_x;
    // the trunk sways a little each step, by the step's own number
    for (i, &n) in tree.trunk.iter().enumerate() {
        let sway = if i == 0 { 0.0 } else { ((n as f64 * 0.618).fract() - 0.5) * seg * 0.6 };
        let (nx, ny) = (x + sway, y - if i == 0 { 0.0 } else { seg });
        if i > 0 {
            img.line(x, y, nx, ny, 0.9, (3.5 - 2.5 * i as f64 / tree.trunk.len() as f64).max(1.0));
        }
        x = nx;
        y = ny;
        pos.insert(i, (x, y));
    }
    // the branches Golden dropped, where it stood when it dropped them
    let trunk_at: HashMap<usize, usize> = tree.trunk.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    for (k, &(from, _, score)) in tree.dropped.iter().enumerate() {
        let i = trunk_at.get(&from).copied().unwrap_or(0);
        let Some(&(sx, sy)) = pos.get(&i) else { continue };
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let len = 18.0 + 40.0 * (score / top).max(0.0);
        let (ex, ey) = (sx + side * len, sy - len * 0.6);
        img.line(sx, sy, ex, ey, glow(score) * 0.5, 0.7);
        img.dot(ex, ey, 1.0 + 3.0 * (score / top).max(0.0), glow(score));
    }
    // the frontier: every live node beyond the tip, fanned out
    let tip = *tree.trunk.last().unwrap();
    let (tx, ty) = (x, y);
    let mut level: Vec<(usize, f64, f64, f64)> = vec![(tip, tx, ty, std::f64::consts::PI)];
    let mut depth = 0;
    while !level.is_empty() && depth < tree.render_distance + 1 {
        let mut next = Vec::new();
        for &(n, px, py, spread) in &level {
            let kids: Vec<usize> = tree.nodes[n].children.iter().copied().filter(|&c| tree.nodes[c].alive).collect();
            let k = kids.len().max(1) as f64;
            for (j, &c) in kids.iter().enumerate() {
                let ang = -std::f64::consts::FRAC_PI_2 + spread * ((j as f64 + 0.5) / k - 0.5);
                let len = 120.0 / (1.0 + depth as f64 * 0.6);
                let (cx, cy) = (px + len * ang.cos(), py + len * ang.sin());
                let s = tree.nodes[c].weighed.as_ref().map(|w| w.score).unwrap_or(0.0);
                img.line(px, py, cx, cy, glow(s) * 0.6, 0.8);
                img.dot(cx, cy, 1.0 + 4.0 * (s / top), glow(s));
                next.push((c, cx, cy, spread / k * 1.4));
            }
        }
        level = next;
        depth += 1;
    }
    let path = Path::new(OUT).join(format!("tree-{}.png", tree.seed));
    img.png(&path)?;
    Ok(path)
}

// ------------------------------------------------------------------ Golden Answer

/// The question, read whole.
pub const QUESTION: &str = "Grow golden tree paths.";

pub fn read_question(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// Golden Answer's kinds. Logical: the paths grown, every number iterated again on the CPU and checked. Abstract: the
/// rules alone, claiming nothing. Theoretical has none: a rule grown from a seed rests on no known result.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if !kind_logical && !kind_abstract {
        return Err("a rule grown from a seed rests on no known result to name; ask logical".into());
    }
    let gpu = Gpu::new().ok();
    let tree = grow(24, RENDER_DISTANCE, 0, gpu.as_ref());
    let found = best(&tree, 5);
    let mut lines = Vec::new();
    for e in &found {
        if kind_abstract {
            lines.push(e.rule.clone());
            continue;
        }
        match check(e) {
            Ok(w) => lines.push(format!(
                "{}: quiet for {} iterations from start {}, then complex {:.2}; score {:.3}; iterated again on the CPU: the same",
                e.rule, w.reading.quiet, e.start, w.reading.after, w.score
            )),
            Err(why) => lines.push(format!("{}: does not check ({why}); not said", e.rule)),
        }
    }
    if kind_abstract {
        lines.push("this is an answer that claims nothing".into());
    } else {
        let mut s = String::new();
        let _ = write!(s, "grown from seed 0: {} moves, render distance {}, {} paths weighed, none deeper than {} beyond where Golden stood", tree.trunk.len() - 1, tree.render_distance, tree.weighed, tree.deepest_ahead);
        lines.push(s);
    }
    Ok((format!("{} paths with a long quiet run and a complex change", found.len()), lines, vec![]))
}
