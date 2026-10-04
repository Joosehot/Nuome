//! Nuome looks for new terms that describe chess positions, the Goldbach
//! way: it does not know whether such terms exist. The board is a closed
//! network of 64 nodes; from the rules come its basic node sets (each
//! side's pieces by kind, the nodes it strikes, its pawns' paths and
//! queening nodes, its king's zone, the empty nodes). Nuome builds terms
//! from them - the size of a set, the overlap or difference of two sets,
//! the nodes next to a set, the distance between two sets in the network -
//! always as mine minus theirs, so a term means the same for both sides.
//!
//! The truth is exact: the endgame tables say which moves keep the result
//! (tools/golden_truth.py). The golden function is the supergenius's
//! formula for the position plus the terms Nuome finds; a term is kept only
//! if, with its best weight, the function plays a right move in more
//! positions of the first half of the data. Every kept term is then tested
//! on the unseen half and on 5-piece classes the search never saw.

use crate::golden::{attack_edges, value_of, Board};
use crate::supergenius_golden::function_for;

const NAMES: [&str; 35] = [
    "my pawns", "my knights", "my bishops", "my rooks", "my queens", "my king",
    "their pawns", "their knights", "their bishops", "their rooks", "their queens", "their king",
    "my pieces", "their pieces", "empty nodes", "nodes I strike", "nodes they strike",
    "my pawn paths", "their pawn paths", "my queening nodes", "their queening nodes", "my king zone", "their king zone",
    "nodes my pawns strike", "nodes my knights strike", "nodes my bishops strike", "nodes my rooks strike", "nodes my queens strike", "nodes my king strikes",
    "nodes their pawns strike", "nodes their knights strike", "nodes their bishops strike", "nodes their rooks strike", "nodes their queens strike", "nodes their king strikes",
];
const SHORT: [&str; 35] = [
    "myP", "myN", "myB", "myR", "myQ", "myK", "thP", "thN", "thB", "thR", "thQ", "thK", "my", "th", "empty", "myHit", "thHit",
    "myPath", "thPath", "myPromo", "thPromo", "myKzone", "thKzone",
    "myPhit", "myNhit", "myBhit", "myRhit", "myQhit", "myKhit", "thPhit", "thNhit", "thBhit", "thRhit", "thQhit", "thKhit",
];
const N_BASE: usize = NAMES.len();

/// The same set with the sides swapped.
fn mirror(i: usize) -> usize {
    match i {
        0..=5 => i + 6,
        6..=11 => i - 6,
        12 => 13,
        13 => 12,
        14 => 14,
        15 => 16,
        16 => 15,
        17 => 18,
        18 => 17,
        19 => 20,
        20 => 19,
        21 => 22,
        22 => 21,
        23..=28 => i + 6,
        _ => i - 6,
    }
}

const NOT_A: u64 = 0xfefe_fefe_fefe_fefe;
const NOT_H: u64 = 0x7f7f_7f7f_7f7f_7f7f;

/// The set and every node next to it (one king step).
fn grow(s: u64) -> u64 {
    let h = s | ((s << 1) & NOT_A) | ((s >> 1) & NOT_H);
    h | (h << 8) | (h >> 8)
}

/// The nodes the one piece on `n` strikes (its edges under the rules).
fn piece_strikes(b: &Board, n: usize) -> u64 {
    let p = b.sq[n];
    let (f, r) = ((n % 8) as i32, (n / 8) as i32);
    let at = |x: i32, y: i32| ((0..8).contains(&x) && (0..8).contains(&y)).then_some((y * 8 + x) as usize);
    let mut out = 0u64;
    match p.abs() {
        1 => {
            let dr = if p > 0 { 1 } else { -1 };
            for df in [-1, 1] {
                if let Some(t) = at(f + df, r + dr) {
                    out |= 1 << t;
                }
            }
        }
        2 | 6 => {
            let knight: &[(i32, i32)] = &[(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)];
            let king: &[(i32, i32)] = &[(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
            for &(df, dr) in if p.abs() == 2 { knight } else { king } {
                if let Some(t) = at(f + df, r + dr) {
                    out |= 1 << t;
                }
            }
        }
        k => {
            let rook: &[(i32, i32)] = &[(1, 0), (-1, 0), (0, 1), (0, -1)];
            let bishop: &[(i32, i32)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
            let dirs: Vec<(i32, i32)> = match k {
                3 => bishop.to_vec(),
                4 => rook.to_vec(),
                _ => rook.iter().chain(bishop.iter()).copied().collect(),
            };
            for (df, dr) in dirs {
                let (mut x, mut y) = (f + df, r + dr);
                while let Some(t) = at(x, y) {
                    out |= 1 << t;
                    if b.sq[t] != 0 {
                        break;
                    }
                    x += df;
                    y += dr;
                }
            }
        }
    }
    out
}

/// The basic node sets of `b`, "my" = the side to move.
fn base_sets(b: &Board) -> [u64; N_BASE] {
    let mut s = [0u64; N_BASE];
    let w = b.white;
    let mine = |p: i8| if w { p > 0 } else { p < 0 };
    for (n, &p) in b.sq.iter().enumerate() {
        if p == 0 {
            s[14] |= 1 << n;
            continue;
        }
        let t = p.unsigned_abs() as usize - 1;
        let my = mine(p);
        s[if my { t } else { t + 6 }] |= 1 << n;
        s[if my { 12 } else { 13 }] |= 1 << n;
        if t == 0 {
            let (f, r) = (n % 8, n / 8);
            let white = p > 0;
            let ahead = if white { (r + 1..8).collect::<Vec<_>>() } else { (0..r).collect() };
            for y in ahead {
                s[if my { 17 } else { 18 }] |= 1 << (y * 8 + f);
            }
            s[if my { 19 } else { 20 }] |= 1 << (if white { 56 + f } else { f });
        }
    }
    let e = attack_edges(b);
    let (me, them) = if w { (0, 1) } else { (1, 0) };
    for n in 0..64 {
        if e[me][n] > 0 {
            s[15] |= 1 << n;
        }
        if e[them][n] > 0 {
            s[16] |= 1 << n;
        }
    }
    s[21] = grow(s[5]);
    s[22] = grow(s[11]);
    for (n, &p) in b.sq.iter().enumerate() {
        if p != 0 {
            let t = p.unsigned_abs() as usize - 1;
            s[if mine(p) { 23 + t } else { 29 + t }] |= piece_strikes(b, n);
        }
    }
    s
}

/// Total edges (reach) of each kind of piece: [my P..K, their P..K].
fn reaches(b: &Board) -> [i64; 12] {
    let mut r = [0i64; 12];
    for (n, &p) in b.sq.iter().enumerate() {
        if p != 0 {
            let t = p.unsigned_abs() as usize - 1;
            let my = if b.white { p > 0 } else { p < 0 };
            r[if my { t } else { t + 6 }] += piece_strikes(b, n).count_ones() as i64;
        }
    }
    r
}

/// A set built from the basic ones.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Set {
    Base(usize),
    And(usize, usize),
    Minus(usize, usize),
    Near(usize),
}

impl Set {
    fn eval(self, s: &[u64; N_BASE], swap: bool) -> u64 {
        let m = |i: usize| if swap { mirror(i) } else { i };
        match self {
            Set::Base(a) => s[m(a)],
            Set::And(a, b) => s[m(a)] & s[m(b)],
            Set::Minus(a, b) => s[m(a)] & !s[m(b)],
            Set::Near(a) => grow(s[m(a)]),
        }
    }
    fn short(self) -> String {
        match self {
            Set::Base(a) => SHORT[a].into(),
            Set::And(a, b) => format!("{}&{}", SHORT[a], SHORT[b]),
            Set::Minus(a, b) => format!("{}-{}", SHORT[a], SHORT[b]),
            Set::Near(a) => format!("near({})", SHORT[a]),
        }
    }
    fn words(self) -> String {
        match self {
            Set::Base(a) => NAMES[a].into(),
            Set::And(a, b) => format!("{} that are {}", NAMES[a], NAMES[b]),
            Set::Minus(a, b) => format!("{} not {}", NAMES[a], NAMES[b]),
            Set::Near(a) => format!("nodes at or next to {}", NAMES[a]),
        }
    }
}

/// A term: a number of a position, mine minus theirs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Term {
    Count(Set),
    Dist(usize, usize),
    /// total edges of a kind of piece (0..5 = pawn..king)
    Reach(usize),
}

/// Distance in king steps between two sets (8 if either is empty).
fn dist(a: u64, b: u64) -> i64 {
    if a == 0 || b == 0 {
        return 8;
    }
    let mut g = b;
    for k in 0..8 {
        if g & a != 0 {
            return k;
        }
        g = grow(g);
    }
    8
}

impl Term {
    /// The same term with the sides swapped: a term seen from the position
    /// after the move is its mirror seen from the side that moved.
    fn mirrored(self) -> Term {
        match self {
            Term::Count(Set::Base(a)) => Term::Count(Set::Base(mirror(a))),
            Term::Count(Set::And(a, b)) => Term::Count(Set::And(mirror(a), mirror(b))),
            Term::Count(Set::Minus(a, b)) => Term::Count(Set::Minus(mirror(a), mirror(b))),
            Term::Count(Set::Near(a)) => Term::Count(Set::Near(mirror(a))),
            Term::Dist(a, b) => Term::Dist(mirror(a), mirror(b)),
            Term::Reach(t) => Term::Reach(t),
        }
    }

    /// The term in short notation, for the written function.
    fn short(self) -> String {
        match self {
            Term::Count(set) => format!("#({})", set.short()),
            Term::Dist(a, b) => format!("d({},{})", SHORT[a], SHORT[b]),
            Term::Reach(t) => format!("reach({})", ["myP", "myN", "myB", "myR", "myQ", "myK"][t]),
        }
    }

    fn one(self, s: &[u64; N_BASE], r: &[i64; 12], swap: bool) -> i64 {
        match self {
            Term::Reach(t) => r[if swap { t + 6 } else { t }],
            Term::Count(set) => set.eval(s, swap).count_ones() as i64,
            Term::Dist(a, b) => {
                let m = |i: usize| if swap { mirror(i) } else { i };
                dist(s[m(a)], s[m(b)])
            }
        }
    }
    /// mine minus theirs
    fn value(self, s: &[u64; N_BASE], r: &[i64; 12]) -> i64 {
        self.one(s, r, false) - self.one(s, r, true)
    }
    pub fn words(self) -> String {
        match self {
            Term::Count(set) => format!("number of {}", set.words()),
            Term::Dist(a, b) => format!("distance from {} to {}", NAMES[a], NAMES[b]),
            Term::Reach(t) => format!("total reach of my {}", ["pawns", "knights", "bishops", "rooks", "queens", "king"][t]),
        }
    }
}

/// Every candidate term (sets up to one operation, distances between basic sets).
fn candidates() -> Vec<Term> {
    let mut out: Vec<Term> = (0..6).map(Term::Reach).collect();
    for a in 0..N_BASE {
        out.push(Term::Count(Set::Base(a)));
        out.push(Term::Count(Set::Near(a)));
        for b in 0..N_BASE {
            if a != b {
                out.push(Term::Count(Set::And(a, b)));
                out.push(Term::Count(Set::Minus(a, b)));
                out.push(Term::Dist(a, b));
            }
        }
    }
    out
}

struct Child {
    /// the supergenius's value for the mover, or the rules' value (mate, draw)
    base: i64,
    fixed: bool,
    sets: [u64; N_BASE],
    reach: [i64; 12],
}

pub struct Case {
    pub class: String,
    pub fen: String,
    good: Vec<bool>,
    kids: Vec<Child>,
}

/// Read a truth file (class TAB fen TAB good moves).
pub fn read(path: &str) -> Result<Vec<Case>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let b = Board::from_fen(parts[1])?;
        let good: Vec<&str> = parts[2].split_whitespace().collect();
        let f = function_for(&b).0;
        let mut kids = Vec::new();
        let mut g = Vec::new();
        for m in b.moves() {
            let a = b.play(m);
            let fixed = a.moves().is_empty() || a.insufficient() || a.half >= 100;
            kids.push(Child { base: value_of(&b, m, &[], &f), fixed, sets: base_sets(&a), reach: reaches(&a) });
            g.push(good.contains(&m.uci().as_str()));
        }
        out.push(Case { class: parts[0].into(), fen: parts[1].into(), good: g, kids });
    }
    Ok(out)
}

/// How many cases the function plays right in: the supergenius value plus
/// the chosen terms (seen from the child, so subtracted for the mover).
fn right(cases: &[&Case], terms: &[(Term, i64)]) -> usize {
    cases
        .iter()
        .filter(|c| {
            let mut best = (i64::MIN, 0usize);
            for (i, k) in c.kids.iter().enumerate() {
                let v = if k.fixed { k.base } else { k.base - terms.iter().map(|(t, w)| w * t.value(&k.sets, &k.reach)).sum::<i64>() };
                if v > best.0 {
                    best = (v, i);
                }
            }
            c.good[best.1]
        })
        .count()
}

pub fn report(train: &str, test: &str, max_terms: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let all = read(train)?;
    let five = read(test)?;
    // the halves: alternate cases, so every class is in both
    let first: Vec<&Case> = all.iter().step_by(2).collect();
    let unseen: Vec<&Case> = all.iter().skip(1).step_by(2).collect();
    let fives: Vec<&Case> = five.iter().collect();
    let pct = |n: usize, d: usize| format!("{n}/{d} ({:.1} %)", 100.0 * n as f64 / d.max(1) as f64);
    let mut out = format!(
        "Nuome looks for new terms for the golden function, the Goldbach way\n\
         truth: the endgame tables; {} positions of 3-4 pieces (searched on the first half, {}; tested on the unseen half, {}), {} positions of 5 pieces never seen\n\n",
        all.len(),
        first.len(),
        unseen.len(),
        fives.len()
    );
    let mut chosen: Vec<(Term, i64)> = Vec::new();
    let line = |chosen: &[(Term, i64)]| (right(&first, chosen), right(&unseen, chosen), right(&fives, chosen));
    let (a, b, c) = line(&chosen);
    out.push_str(&format!("the supergenius's function alone: first half {}, unseen half {}, 5 pieces {}\n\n", pct(a, first.len()), pct(b, unseen.len()), pct(c, fives.len())));
    let cands = candidates();
    let weights: Vec<i64> = [-4096, -2048, -1024, -512, -256, -128, -64, -32, 32, 64, 128, 256, 512, 1024, 2048, 4096].to_vec();
    let mut score = a;
    for round in 1..=max_terms {
        // the best term with its best weight on the first half, in parallel
        let threads = 12;
        let chunk = cands.len().div_ceil(threads);
        let best: Option<(usize, Term, i64)> = std::thread::scope(|sc| {
            let hs: Vec<_> = cands
                .chunks(chunk)
                .map(|part| {
                    let (chosen, first, weights) = (&chosen, &first, &weights);
                    sc.spawn(move || {
                        let mut best: Option<(usize, Term, i64)> = None;
                        for &t in part {
                            if chosen.iter().any(|x| x.0 == t) {
                                continue;
                            }
                            for &w in weights {
                                let mut c = chosen.clone();
                                c.push((t, w));
                                let r = right(first, &c);
                                if best.as_ref().map_or(true, |b| r > b.0) {
                                    best = Some((r, t, w));
                                }
                            }
                        }
                        best
                    })
                })
                .collect();
            hs.into_iter().filter_map(|h| h.join().expect("search")).max_by_key(|b| b.0)
        });
        let Some((r, t, w)) = best else { break };
        if r <= score {
            out.push_str(&format!("round {round}: no term makes the first half better; the search stops\n"));
            break;
        }
        chosen.push((t, w));
        score = r;
        let (a, b, c) = line(&chosen);
        out.push_str(&format!(
            "term {round}: {:+} x ({}, mine minus theirs, for the side that moves)\n   first half {}, unseen half {}, 5 pieces {}\n",
            -w,
            t.mirrored().words(),
            pct(a, first.len()),
            pct(b, unseen.len()),
            pct(c, fives.len())
        ));
    }
    let written: String = chosen.iter().map(|(t, w)| format!(" {:+}*{}", -w, t.mirrored().short())).collect();
    out.push_str("\nTHE GOLDEN FUNCTION (for the side that moves; each term mine minus theirs) = the supergenius's formula for the position +");
    out.push_str(&written);
    out.push_str(&format!("\n({} characters in the terms Nuome found)", written.len()));
    out.push_str(&format!("\n(values from the side that moves; {} candidate terms, {:.1} s)\n", cands.len(), t0.elapsed().as_secs_f64()));
    let saved: String = chosen.iter().map(|(t, w)| format!("{w}\t{t:?}\n")).collect();
    let _ = std::fs::write("out/golden/terms.txt", saved);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_term_is_zero_in_a_symmetric_position() {
        let s = base_sets(&Board::start());
        for t in candidates() {
            assert_eq!(t.value(&s, &reaches(&Board::start())), 0, "{}", t.words());
        }
    }

    #[test]
    fn distance_in_king_steps() {
        assert_eq!(dist(1, 1 << 63), 7);
        assert_eq!(dist(1, 1 << 9), 1);
    }
}
