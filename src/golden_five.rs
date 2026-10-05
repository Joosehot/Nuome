//! The five checks as one function, found the way the Goldbach bound was:
//! the data first, then a formula evolved on one part and checked on a part
//! it never saw, then frozen.
//!
//!   the question: one function f of a chess position that checks legality,
//!   safety, force, material and tempo at the same time, in the apple
//!
//!   the apple: after a move, the forcing lines (captures, promotions, the
//!   mover's checks, every reply in check) followed to where they stop, at
//!   most 20 plies, each side free to stop
//!
//!   what the rules force (logical): legality decides first and alone —
//!   an apple where the side to move is mated is won or lost outright, a
//!   stalemate is a draw (f of the empty position, every variable 0)
//!
//!   what is left: how the other four combine. Measured in the apple, from
//!   the mover's side, with no exchange played out:
//!     M   material, mine minus theirs (pawns: 1, 3.25, 3.25, 5, 9.75)
//!     S   safety: my pieces they attack that are unguarded or attacked by
//!         something cheaper (their worth), and their attacks on my king's
//!         squares
//!     F   force: the same for their pieces and king, by me
//!     T   tempo: my moves to squares not my own minus theirs, and whose
//!         move it is in the apple (+1 mine, -1 theirs)
//!
//!   the choice: Golden evolves g over these seven numbers (+, -, *, max,
//!   min, constants), nothing given to start from; a formula is worth the
//!   share of positions where its best move is the one Stockfish played
//!   (Stockfish's moves in CCRL games; no Stockfish evaluation), less a
//!   price per node. Evolution starts only after every position of the
//!   heard games is in. Checked on games it never heard, then frozen in
//!   out/golden/five_function.txt.

use crate::fast::movegen::{gen_moves, legal_moves, uci_to_move, MAX_MOVES};
use crate::fast::position::{m_flag, m_promo, m_to, move_to_uci, Position};

pub const FILE: &str = "out/golden/five_function.txt";
pub const PLIES: usize = 20;
const NODES: u64 = 5_000;
const WORTH: [i64; 6] = [100, 325, 325, 500, 975, 0];
pub const NAMES: [&str; 7] = ["M", "S_hang", "S_king", "F_hang", "F_king", "T_moves", "T_turn"];
pub const NV: usize = 7;
const WIN: f64 = 1e9;

// ---------------------------------------------------------------- the apple

fn legal_one(p: &mut Position, m: u32) -> bool {
    p.make(m);
    let ok = !p.in_check_c(1 - p.turn);
    p.unmake();
    ok
}

fn material(p: &Position, side: usize) -> i64 {
    let mut v = [0i64; 2];
    for c in 0..2 {
        for t in 0..6 {
            v[c] += WORTH[t] * p.bb[c * 6 + t].count_ones() as i64;
        }
    }
    v[side] - v[1 - side]
}

/// The forcing lines from `p`: the material for the side to move where
/// they stop, and the line to there. Mate is -WIN for the mated side.
fn forcing(p: &mut Position, plies: usize, attacker: bool, nodes: &mut u64, line: &mut Vec<u32>) -> i64 {
    *nodes += 1;
    let check = p.in_check();
    let side = p.turn as usize;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, !(check || attacker));
    if plies == 0 || *nodes > NODES {
        if check && !list[..n].iter().any(|&m| legal_one(p, m)) {
            return -WIN as i64;
        }
        return material(p, side);
    }
    let mut best = if check { -WIN as i64 } else { material(p, side) };
    let mut best_line: Vec<u32> = Vec::new();
    let mut any = false;
    for &m in &list[..n] {
        let capture = p.board[m_to(m) as usize] != 0 || m_flag(m) == 2;
        let promo = m_promo(m) != 0;
        p.make(m);
        if p.in_check_c(1 - p.turn) {
            p.unmake();
            continue;
        }
        any = true;
        if check || capture || promo || (attacker && p.in_check()) {
            let mut sub = Vec::new();
            let v = -forcing(p, plies - 1, !attacker, nodes, &mut sub);
            if v > best {
                best = v;
                best_line.clear();
                best_line.push(m);
                best_line.extend(sub);
            }
        }
        p.unmake();
    }
    if check && !any {
        return -WIN as i64;
    }
    *line = best_line;
    best
}

/// What the apple is: the rules decide it (mate, stalemate) or the seven numbers.
#[derive(Clone, Copy, Debug)]
pub enum Apple {
    Won,
    Lost,
    Stalemate,
    Vars([f32; NV]),
}

/// The seven numbers of position `p` seen from `me`.
pub fn vars(p: &Position, me: usize) -> [f32; NV] {
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    // attacks per colour and piece type, and each piece's moves to squares not its own
    let mut a = [[0u64; 6]; 2];
    let mut moves = [0i64; 2];
    for c in 0..2 {
        let own = p.by_color[c];
        for ty in 0..6 {
            let mut b = p.bb[c * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let at = match ty {
                    0 => {
                        if c == 0 {
                            t.att.pawn_w[s]
                        } else {
                            t.att.pawn_b[s]
                        }
                    }
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, occ),
                    3 => t.rook_attacks(s, occ),
                    4 => t.queen_attacks(s, occ),
                    _ => t.att.king[s],
                };
                a[c][ty] |= at;
                if ty != 0 {
                    moves[c] += (at & !own).count_ones() as i64;
                }
            }
        }
    }
    let all = |c: usize| a[c].iter().fold(0u64, |x, y| x | y);
    // the worth of side c's pieces the other side strikes: unguarded, or attacked by something cheaper
    let hanging = |c: usize| -> i64 {
        let o = 1 - c;
        let (mine, theirs) = (all(c), all(o));
        let mut sum = 0;
        for ty in 0..5 {
            let mut b = p.bb[c * 6 + ty] & theirs;
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let bit = 1u64 << s;
                let cheapest = (0..6).find(|&k| a[o][k] & bit != 0).unwrap_or(5);
                if mine & bit == 0 {
                    sum += WORTH[ty];
                } else if WORTH[cheapest] < WORTH[ty] {
                    sum += WORTH[ty] - WORTH[cheapest];
                }
            }
        }
        sum
    };
    let king_zone = |c: usize| -> i64 {
        let k = p.bb[c * 6 + 5];
        if k == 0 {
            return 0;
        }
        let s = k.trailing_zeros() as usize;
        ((t.att.king[s] | k) & all(1 - c)).count_ones() as i64
    };
    let them = 1 - me;
    [
        material(p, me) as f32 / 100.0,
        hanging(me) as f32 / 100.0,
        king_zone(me) as f32,
        hanging(them) as f32 / 100.0,
        king_zone(them) as f32,
        (moves[me] - moves[them]) as f32,
        if p.turn as usize == me { 1.0 } else { -1.0 },
    ]
}

/// The apple of move `m` in `p`: the forcing lines after it to their end, and what is there.
pub fn apple(p: &mut Position, m: u32) -> Apple {
    let me = p.turn as usize;
    p.make(m);
    let mut nodes = 0;
    let mut line = Vec::new();
    forcing(p, PLIES - 1, false, &mut nodes, &mut line);
    for &x in &line {
        p.make(x);
    }
    let r = if legal_moves(p).is_empty() {
        if p.in_check() {
            if p.turn as usize == me {
                Apple::Lost
            } else {
                Apple::Won
            }
        } else {
            Apple::Stalemate
        }
    } else {
        Apple::Vars(vars(p, me))
    };
    for _ in &line {
        p.unmake();
    }
    p.unmake();
    r
}

// ---------------------------------------------------------------- formulas

#[derive(Clone, Debug)]
pub enum G {
    V(usize),
    C(f64),
    Add(Box<G>, Box<G>),
    Sub(Box<G>, Box<G>),
    Mul(Box<G>, Box<G>),
    Max(Box<G>, Box<G>),
    Min(Box<G>, Box<G>),
}

impl G {
    pub fn eval(&self, x: &[f32; NV]) -> f64 {
        match self {
            G::V(i) => x[*i] as f64,
            G::C(c) => *c,
            G::Add(a, b) => a.eval(x) + b.eval(x),
            G::Sub(a, b) => a.eval(x) - b.eval(x),
            G::Mul(a, b) => a.eval(x) * b.eval(x),
            G::Max(a, b) => a.eval(x).max(b.eval(x)),
            G::Min(a, b) => a.eval(x).min(b.eval(x)),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            G::V(_) | G::C(_) => 1,
            G::Add(a, b) | G::Sub(a, b) | G::Mul(a, b) | G::Max(a, b) | G::Min(a, b) => 1 + a.size() + b.size(),
        }
    }
    fn kids(&self) -> Option<(&G, &G)> {
        match self {
            G::Add(a, b) | G::Sub(a, b) | G::Mul(a, b) | G::Max(a, b) | G::Min(a, b) => Some((a, b)),
            _ => None,
        }
    }
    /// Written for reading: infix.
    pub fn show(&self) -> String {
        match self {
            G::V(i) => NAMES[*i].to_string(),
            G::C(c) => format!("{c:.3}"),
            G::Add(a, b) => format!("({} + {})", a.show(), b.show()),
            G::Sub(a, b) => format!("({} - {})", a.show(), b.show()),
            G::Mul(a, b) => format!("{} * {}", a.show(), b.show()),
            G::Max(a, b) => format!("max({}, {})", a.show(), b.show()),
            G::Min(a, b) => format!("min({}, {})", a.show(), b.show()),
        }
    }
    /// Written for keeping: prefix, one token per node.
    pub fn save(&self) -> String {
        match self {
            G::V(i) => NAMES[*i].to_string(),
            G::C(c) => format!("{c}"),
            G::Add(a, b) => format!("+ {} {}", a.save(), b.save()),
            G::Sub(a, b) => format!("- {} {}", a.save(), b.save()),
            G::Mul(a, b) => format!("* {} {}", a.save(), b.save()),
            G::Max(a, b) => format!("max {} {}", a.save(), b.save()),
            G::Min(a, b) => format!("min {} {}", a.save(), b.save()),
        }
    }
    pub fn parse(text: &str) -> Option<G> {
        fn go<'a>(it: &mut impl Iterator<Item = &'a str>) -> Option<G> {
            let t = it.next()?;
            let two = |it: &mut _| -> Option<(Box<G>, Box<G>)> { Some((Box::new(go(it)?), Box::new(go(it)?))) };
            Some(match t {
                "+" => two(it).map(|(a, b)| G::Add(a, b))?,
                "-" => two(it).map(|(a, b)| G::Sub(a, b))?,
                "*" => two(it).map(|(a, b)| G::Mul(a, b))?,
                "max" => two(it).map(|(a, b)| G::Max(a, b))?,
                "min" => two(it).map(|(a, b)| G::Min(a, b))?,
                _ => match NAMES.iter().position(|n| *n == t) {
                    Some(i) => G::V(i),
                    None => G::C(t.parse().ok()?),
                },
            })
        }
        let mut it = text.split_whitespace();
        let g = go(&mut it)?;
        it.next().is_none().then_some(g)
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
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn random_g(r: &mut Rng, depth: u32) -> G {
    if depth == 0 || r.below(3) == 0 {
        return if r.below(3) == 0 { G::C(((r.unit() * 8.0 - 4.0) * 100.0).round() / 100.0) } else { G::V(r.below(NV)) };
    }
    let (a, b) = (Box::new(random_g(r, depth - 1)), Box::new(random_g(r, depth - 1)));
    match r.below(5) {
        0 => G::Add(a, b),
        1 => G::Sub(a, b),
        2 => G::Mul(a, b),
        3 => G::Max(a, b),
        _ => G::Min(a, b),
    }
}

fn nodes_of(g: &G) -> Vec<G> {
    let mut out = vec![g.clone()];
    if let Some((a, b)) = g.kids() {
        out.extend(nodes_of(a));
        out.extend(nodes_of(b));
    }
    out
}

/// `g` with its k-th node (preorder) replaced by `new`.
fn replace(g: &G, k: &mut isize, new: &G) -> G {
    if *k == 0 {
        *k -= 1;
        return new.clone();
    }
    *k -= 1;
    let rebuild = |a: &G, b: &G, k: &mut isize| (Box::new(replace(a, k, new)), Box::new(replace(b, k, new)));
    match g {
        G::Add(a, b) => {
            let (a, b) = rebuild(a, b, k);
            G::Add(a, b)
        }
        G::Sub(a, b) => {
            let (a, b) = rebuild(a, b, k);
            G::Sub(a, b)
        }
        G::Mul(a, b) => {
            let (a, b) = rebuild(a, b, k);
            G::Mul(a, b)
        }
        G::Max(a, b) => {
            let (a, b) = rebuild(a, b, k);
            G::Max(a, b)
        }
        G::Min(a, b) => {
            let (a, b) = rebuild(a, b, k);
            G::Min(a, b)
        }
        _ => g.clone(),
    }
}

fn mutate(g: &G, r: &mut Rng) -> G {
    let n = g.size();
    let mut k = r.below(n) as isize;
    let target = nodes_of(g)[k as usize].clone();
    let new = match target {
        G::C(c) if r.below(2) == 0 => G::C(((c * (0.5 + r.unit())) * 1000.0).round() / 1000.0),
        _ => random_g(r, 2),
    };
    replace(g, &mut k, &new)
}

fn crossover(a: &G, b: &G, r: &mut Rng) -> G {
    let from_b = nodes_of(b);
    let piece = &from_b[r.below(from_b.len())];
    let mut k = r.below(a.size()) as isize;
    replace(a, &mut k, piece)
}

// ---------------------------------------------------------------- hearing

/// One heard position: every legal move's apple and the move Stockfish played.
pub struct Heard {
    pub apples: Vec<Apple>,
    pub chosen: usize,
}

/// Stockfish's moves from the games in `lines` (sides TAB result TAB moves),
/// one position per game (a ply chosen by the game's number, past the
/// opening, more than five pieces on the board).
fn positions(lines: &[&str], first: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let (w, b) = (parts[0].contains('W'), parts[0].contains('B'));
        let moves: Vec<&str> = parts[2].split_whitespace().collect();
        if moves.len() < 20 {
            continue;
        }
        let mut h = ((first + i) as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15);
        h ^= h >> 29;
        let ply = 12 + (h as usize) % (moves.len() - 12);
        let white_to_move = ply % 2 == 0;
        if (white_to_move && !w) || (!white_to_move && !b) {
            continue;
        }
        let Ok(mut p) = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1") else { continue };
        let mut ok = true;
        for u in &moves[..ply] {
            match uci_to_move(&mut p, u) {
                Some(m) => p.make(m),
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && p.occ.count_ones() > 5 {
            out.push((p.to_fen(), moves[ply].to_string()));
        }
    }
    out
}

fn hear_one(fen: &str, sf: &str) -> Option<Heard> {
    let mut p = Position::from_fen(fen).ok()?;
    let ms = legal_moves(&mut p);
    if ms.len() < 2 {
        return None;
    }
    let chosen = ms.iter().position(|&m| move_to_uci(m) == sf)?;
    let apples = ms.iter().map(|&m| apple(&mut p, m)).collect();
    Some(Heard { apples, chosen })
}

/// Every position heard, on all cores.
fn hear(pos: &[(String, String)]) -> Vec<Heard> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = pos.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = pos.chunks(chunk).map(|c| s.spawn(move || c.iter().filter_map(|(f, m)| hear_one(f, m)).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

// ---------------------------------------------------------------- choosing

fn value(g: &G, a: &Apple) -> f64 {
    match a {
        Apple::Won => WIN,
        Apple::Lost => -WIN,
        Apple::Stalemate => g.eval(&[0.0; NV]),
        Apple::Vars(x) => g.eval(x),
    }
}

/// The share of positions where g's best move is Stockfish's (a tie of k moves counts 1/k).
pub fn agreement(g: &G, set: &[Heard]) -> f64 {
    let mut sum = 0.0;
    for h in set {
        let v: Vec<f64> = h.apples.iter().map(|a| value(g, a)).collect();
        let best = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if !best.is_finite() {
            continue;
        }
        if v[h.chosen] == best {
            sum += 1.0 / v.iter().filter(|&&x| x == best).count() as f64;
        }
    }
    sum / set.len().max(1) as f64
}

const PRICE: f64 = 0.0005;

fn fitness(g: &G, set: &[Heard]) -> f64 {
    agreement(g, set) - PRICE * g.size() as f64
}

fn score_all(pop: &[G], set: &[Heard]) -> Vec<f64> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = pop.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = pop.chunks(chunk).map(|c| s.spawn(move || c.iter().map(|g| fitness(g, set)).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

pub fn evolve(train: &[Heard], seconds: f64, seed: u64, log: &mut dyn FnMut(String)) -> G {
    let mut r = Rng(seed | 1);
    let size = 240;
    let mut pop: Vec<G> = (0..size).map(|_| random_g(&mut r, 3)).collect();
    let mut fit = score_all(&pop, train);
    let t0 = std::time::Instant::now();
    let mut generation = 0;
    let mut last_best = f64::NEG_INFINITY;
    while t0.elapsed().as_secs_f64() < seconds {
        generation += 1;
        let mut order: Vec<usize> = (0..size).collect();
        order.sort_by(|&a, &b| fit[b].total_cmp(&fit[a]));
        let elite: Vec<G> = order.iter().take(size / 10).map(|&i| pop[i].clone()).collect();
        let pick = |r: &mut Rng| -> usize {
            let (a, b, c) = (r.below(size), r.below(size), r.below(size));
            [a, b, c].into_iter().max_by(|&x, &y| fit[x].total_cmp(&fit[y])).unwrap()
        };
        let mut next = elite.clone();
        while next.len() < size {
            let child = match r.below(10) {
                0..=4 => mutate(&pop[pick(&mut r)], &mut r),
                5..=8 => crossover(&pop[pick(&mut r)], &pop[pick(&mut r)], &mut r),
                _ => random_g(&mut r, 3),
            };
            if child.size() <= 41 {
                next.push(child);
            }
        }
        pop = next;
        fit = score_all(&pop, train);
        let b = fit.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if b > last_best + 1e-9 {
            last_best = b;
            let i = fit.iter().position(|&x| x == b).unwrap();
            log(format!("  generation {generation}: {:.1}% heard, size {}: {}", 100.0 * agreement(&pop[i], train), pop[i].size(), pop[i].show()));
        }
    }
    let i = (0..size).max_by(|&a, &b| fit[a].total_cmp(&fit[b])).unwrap();
    pop[i].clone()
}

// ---------------------------------------------------------------- the whole run

pub fn load() -> Option<G> {
    G::parse(std::fs::read_to_string(FILE).ok()?.lines().next()?)
}

/// Hear `n_heard` games from the front of the file and `n_check` from its end,
/// evolve for `seconds` on the heard ones only, check on the others, freeze.
pub fn run(path: &str, n_heard: usize, n_check: usize, seconds: f64, log: &mut dyn FnMut(String)) -> Result<G, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let lines: Vec<&str> = text.lines().collect();
    let n_heard = n_heard.min(lines.len() / 2);
    let n_check = n_check.min(lines.len() - n_heard);
    let t0 = std::time::Instant::now();
    let heard = hear(&positions(&lines[..n_heard], 0));
    let checks = hear(&positions(&lines[lines.len() - n_check..], lines.len() - n_check));
    log(format!("heard {} positions of Stockfish's from {n_heard} games, and {} from {n_check} other games kept for the check ({:.0} s)", heard.len(), checks.len(), t0.elapsed().as_secs_f64()));
    // the rules' part alone: material where the forcing lines stop
    let material = G::V(0);
    log(format!("material alone in the apple: {:.1}% heard, {:.1}% check", 100.0 * agreement(&material, &heard), 100.0 * agreement(&material, &checks)));
    log("evolving g (only now, every heard position is in):".into());
    let g = evolve(&heard, seconds, 0x5EED_F17E, log);
    log(format!("found: f = {}", g.show()));
    log(format!("Stockfish's move: {:.1}% of the heard positions, {:.1}% of the {} never heard", 100.0 * agreement(&g, &heard), 100.0 * agreement(&g, &checks), checks.len()));
    std::fs::write(FILE, format!("{}\n# f = {}\n# heard {:.4}, check {:.4} ({} / {} positions)\n", g.save(), g.show(), agreement(&g, &heard), agreement(&g, &checks), heard.len(), checks.len())).map_err(|e| e.to_string())?;
    log(format!("frozen in {FILE}"));
    Ok(g)
}

/// Every legal move with f in its apple, best first.
pub fn rank(fen: &str, g: &G) -> Option<Vec<(String, f64, Apple)>> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut rows: Vec<(String, f64, Apple)> = legal_moves(&mut p).into_iter().map(|m| {
        let a = apple(&mut p, m);
        (move_to_uci(m), value(g, &a), a)
    }).collect();
    rows.sort_by(|a, b| b.1.total_cmp(&a.1));
    Some(rows)
}

/// Golden Answer's answer to the question, in its kind.
/// logical: what the rules force, and that they leave the combination open;
/// theoretical: the frozen f, with what it was measured to do;
/// abstract: the frozen f, claiming nothing.
pub fn answer(logical: bool, abstract_: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let forced = vec![
        "legality decides first and alone: an apple where the side to move is mated is won or lost outright, whatever the rest says".to_string(),
        "a stalemate in the apple is a draw: f of the empty position, every variable 0".to_string(),
        "f is read from the mover's side, so the same apple seen by the other side is judged by the other side's numbers".to_string(),
    ];
    if logical {
        let mut l = forced;
        l.push("the rules do not force how safety, force, material and tempo combine; that choice is left (undecided by the rules alone)".into());
        return Err(format!("not decided; what the rules force:\n  - {}", l.join("\n  - ")));
    }
    let text = std::fs::read_to_string(FILE).map_err(|_| "the function is not found yet: run golden-five".to_string())?;
    let g = G::parse(text.lines().next().unwrap_or("")).ok_or("the frozen function cannot be read")?;
    let measured = text.lines().find(|l| l.starts_with("# heard")).unwrap_or("").trim_start_matches("# ").to_string();
    let mut lines = vec![format!("f = {}", g.show()), format!("in the apple (where the forcing lines stop, up to {PLIES} plies); variables: M material, S_hang / S_king my danger, F_hang / F_king my threats, T_moves / T_turn the tempo")];
    lines.extend(forced);
    if abstract_ {
        lines.push("this is an answer that claims nothing".into());
        return Ok((g.show(), lines, vec![]));
    }
    lines.push(format!("measured: Stockfish's move chosen by f, share of positions — {measured} (the check games were never heard)"));
    Ok((g.show(), lines, vec![format!("Stockfish's moves in CCRL 40/15 games (computerchess.org.uk), frozen in {FILE}")]))
}

/// The question, read whole.
pub fn read(q: &str) -> bool {
    let t: String = q.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_end_matches(['.', '?', '!']);
    [
        "give one function of a chess position that checks legality, safety, force, material and tempo at the same time",
        "the function of the five checks",
        "find the function of legality, safety, force, material and tempo",
    ]
    .contains(&t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_reads_back() {
        let g = G::Add(Box::new(G::V(0)), Box::new(G::Mul(Box::new(G::C(-0.5)), Box::new(G::Max(Box::new(G::V(1)), Box::new(G::V(6)))))));
        let back = G::parse(&g.save()).unwrap();
        assert_eq!(back.show(), g.show());
    }

    #[test]
    fn the_apple_sees_mate_and_material() {
        // back-rank mate: Rd8#
        let mut p = Position::from_fen("6k1/5ppp/8/8/8/8/8/3R2K1 w - - 0 1").unwrap();
        let m = uci_to_move(&mut p, "d1d8").unwrap();
        assert!(matches!(apple(&mut p, m), Apple::Won));
        // a free queen: after Rxd5 the apple has a queen more
        let mut p = Position::from_fen("6k1/5ppp/8/3q4/8/8/8/3R2K1 w - - 0 1").unwrap();
        let m = uci_to_move(&mut p, "d1d5").unwrap();
        match apple(&mut p, m) {
            Apple::Vars(x) => assert!(x[0] > 0.0),
            a => panic!("{a:?}"),
        }
    }
}
