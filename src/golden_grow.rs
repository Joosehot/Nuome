//! Golden Boy writes the Golden Function longer as it plays Stockfish, and
//! may invent new concepts for it.
//!
//! The net (as designed): 64 points on the grid; each point holds its own
//! numbers — the piece's value (100 / 325 / 325 / 500 / 975, the king and
//! an empty square 0), the side (+1 the mover's, -1 the opponent's, 0
//! empty), how many of the mover's and of the opponent's pieces attack it,
//! and what stands on it. How they combine is the function's to decide.
//!
//! The language of terms, each summed over the 64 points:
//!   product   A · B · C            up to three factors of the point
//!   concept   A · B · #(R ∩ C)     factors of the point times how many
//!                                  points in relation R to it have the
//!                                  property C — relations: the king's ring,
//!                                  a knight's jump, the same file, the same
//!                                  rank, the diagonals, the squares ahead
//!                                  (the mover's way), the squares the point's
//!                                  piece attacks, the squares it is attacked
//!                                  from
//! Golden invents concepts by drawing them from this grammar (about 45 000);
//! a round tries a fresh draw of them with every product term.
//!
//! Every position of the games is a lesson: Stockfish's own moves, and for
//! Golden Boy's positions the move Stockfish would play there (a point when
//! Golden Boy played it), plus PRETRAIN positions of the Hugging Face data.
//! A round removes terms the function is not worse without, re-tunes every
//! weight, and adds the best new term; every change is kept only if it also
//! raises a separate validate set. The fixed check set is only reported.

use crate::fast::movegen::legal_moves;
use crate::fast::position::{color_of, move_to_uci, type_of, Position};

pub const FUNCTION_FILE: &str = "out/golden/grow_function.txt";
pub const LESSONS: &str = "out/golden/grow_lessons.tsv";
pub const PRETRAIN: usize = 3_000;
const CHECK_N: usize = 2_000;
const WORTH: [f32; 7] = [0.0, 100.0, 325.0, 325.0, 500.0, 975.0, 0.0];
const DRAW: usize = 1_000;

/// The factors of a point. The first four are numbers, the rest 0 or 1.
pub const FACTORS: [&str; 23] = [
    "value", "side", "my_attackers", "their_attackers", "mine", "theirs", "empty", "more_mine", "more_theirs", "mine_attack", "they_attack", "none_mine", "none_theirs",
    "my_pawn", "my_minor", "my_rook", "my_queen", "my_king", "their_pawn", "their_minor", "their_rook", "their_queen", "their_king",
];
const NF: usize = FACTORS.len();
const ONE: u8 = NF as u8;
pub const RELATIONS: [&str; 8] = ["ring", "knight", "file", "rank", "diagonals", "ahead", "it_attacks", "attacked_from"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Term {
    /// up to three factors of the point
    Prod([u8; 3], u8),
    /// a · b · #(r ∩ c); a or b may be ONE
    Rel { a: u8, b: u8, r: u8, c: u8 },
}

fn fname(i: u8) -> &'static str {
    if i == ONE {
        "1"
    } else {
        FACTORS[i as usize]
    }
}

impl Term {
    pub fn text(&self) -> String {
        match self {
            Term::Prod(f, n) => format!("Σ {}", f[..*n as usize].iter().map(|&i| fname(i)).collect::<Vec<_>>().join("·")),
            Term::Rel { a, b, r, c } => {
                let ab: Vec<&str> = [*a, *b].iter().filter(|&&x| x != ONE).map(|&x| fname(x)).collect();
                format!("Σ {}{}#({} ∩ {})", ab.join("·"), if ab.is_empty() { "" } else { "·" }, RELATIONS[*r as usize], fname(*c))
            }
        }
    }
    fn code(&self) -> String {
        match self {
            Term::Prod(f, n) => f[..*n as usize].iter().map(|&i| fname(i)).collect::<Vec<_>>().join(" "),
            Term::Rel { a, b, r, c } => format!("REL {} {} {} {}", fname(*a), fname(*b), RELATIONS[*r as usize], fname(*c)),
        }
    }
    fn from_code(w: &[&str]) -> Option<Term> {
        let idx = |x: &str| if x == "1" { Some(ONE) } else { FACTORS.iter().position(|n| *n == x).map(|i| i as u8) };
        if w.first() == Some(&"REL") {
            if w.len() != 5 {
                return None;
            }
            return Some(Term::Rel { a: idx(w[1])?, b: idx(w[2])?, r: RELATIONS.iter().position(|n| *n == w[3])? as u8, c: idx(w[4])? });
        }
        let f: Vec<u8> = w.iter().map(|x| idx(x)).collect::<Option<Vec<_>>>()?;
        if f.is_empty() || f.len() > 3 {
            return None;
        }
        let mut a = [0u8; 3];
        a[..f.len()].copy_from_slice(&f);
        Some(Term::Prod(a, f.len() as u8))
    }
}

/// What the function sees of a position from `me`: each point's content and attack counts, and the attack relations.
#[derive(Clone)]
pub struct Feat {
    me: u8,
    /// 0 empty, 1..6 the mover's pawn..king, 7..12 the opponent's
    content: [u8; 64],
    am: [u8; 64],
    at: [u8; 64],
    att_from: [u64; 64],
    att_by: [u64; 64],
}

fn feat(p: &Position, me: usize) -> Feat {
    use crate::fast::bitboard::SQ64_TO_120;
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    let mut f = Feat { me: me as u8, content: [0; 64], am: [0; 64], at: [0; 64], att_from: [0; 64], att_by: [0; 64] };
    for c in 0..2 {
        for ty in 0..6 {
            let mut b = p.bb[c * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let at = match ty {
                    0 => if c == 0 { t.att.pawn_w[s] } else { t.att.pawn_b[s] },
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, occ),
                    3 => t.rook_attacks(s, occ),
                    4 => t.queen_attacks(s, occ),
                    _ => t.att.king[s],
                };
                f.att_from[s] = at;
                let mut x = at;
                while x != 0 {
                    let q = x.trailing_zeros() as usize;
                    x &= x - 1;
                    f.att_by[q] |= 1 << s;
                    if c == me {
                        f.am[q] += 1;
                    } else {
                        f.at[q] += 1;
                    }
                }
            }
        }
    }
    for s in 0..64 {
        let pc = p.board[SQ64_TO_120[s]];
        if pc != 0 {
            f.content[s] = type_of(pc) + if color_of(pc) as usize == me { 0 } else { 6 };
        }
    }
    f
}

#[inline]
fn factor(f: &Feat, i: u8, s: usize) -> f32 {
    let c = f.content[s];
    let (am, at) = (f.am[s], f.at[s]);
    let b = |x: bool| x as u8 as f32;
    match i {
        0 => if c == 0 { 0.0 } else { WORTH[((c - 1) % 6 + 1) as usize] },
        1 => if c == 0 { 0.0 } else if c <= 6 { 1.0 } else { -1.0 },
        2 => am as f32,
        3 => at as f32,
        4 => b(c >= 1 && c <= 6),
        5 => b(c >= 7),
        6 => b(c == 0),
        7 => b(am > at),
        8 => b(at > am),
        9 => b(am > 0),
        10 => b(at > 0),
        11 => b(am == 0),
        12 => b(at == 0),
        13 => b(c == 1),
        14 => b(c == 2 || c == 3),
        15 => b(c == 4),
        16 => b(c == 5),
        17 => b(c == 6),
        18 => b(c == 7),
        19 => b(c == 8 || c == 9),
        20 => b(c == 10),
        21 => b(c == 11),
        22 => b(c == 12),
        _ => 1.0,
    }
}

fn bool_bb(f: &Feat, c: u8) -> u64 {
    let mut b = 0u64;
    for s in 0..64 {
        if factor(f, c, s) != 0.0 {
            b |= 1 << s;
        }
    }
    b
}

/// The static relations' masks: ring, knight, file, rank, diagonals; ahead for white and for black.
fn masks() -> &'static [[u64; 64]; 7] {
    static M: std::sync::OnceLock<[[u64; 64]; 7]> = std::sync::OnceLock::new();
    M.get_or_init(|| {
        let mut m = [[0u64; 64]; 7];
        for s in 0..64i32 {
            let (f, r) = (s % 8, s / 8);
            for t in 0..64i32 {
                if t == s {
                    continue;
                }
                let (g, q) = (t % 8, t / 8);
                let (df, dr) = ((g - f).abs(), (q - r).abs());
                let bit = 1u64 << t;
                if df <= 1 && dr <= 1 {
                    m[0][s as usize] |= bit;
                }
                if (df == 1 && dr == 2) || (df == 2 && dr == 1) {
                    m[1][s as usize] |= bit;
                }
                if g == f {
                    m[2][s as usize] |= bit;
                }
                if q == r {
                    m[3][s as usize] |= bit;
                }
                if df == dr {
                    m[4][s as usize] |= bit;
                }
                if g == f && q > r {
                    m[5][s as usize] |= bit;
                }
                if g == f && q < r {
                    m[6][s as usize] |= bit;
                }
            }
        }
        m
    })
}

#[inline]
fn rel_mask(f: &Feat, r: u8, s: usize) -> u64 {
    let m = masks();
    match r {
        0..=4 => m[r as usize][s],
        5 => if f.me == 0 { m[5][s] } else { m[6][s] },
        6 => f.att_from[s],
        _ => f.att_by[s],
    }
}

fn term_value(t: &Term, f: &Feat) -> f32 {
    match t {
        Term::Prod(fs, n) => {
            let fs = &fs[..*n as usize];
            (0..64).map(|s| fs.iter().map(|&i| factor(f, i, s)).product::<f32>()).sum()
        }
        Term::Rel { a, b, r, c } => {
            let bc = bool_bb(f, *c);
            let mut v = 0f32;
            for s in 0..64 {
                let ab = factor(f, *a, s) * factor(f, *b, s);
                if ab != 0.0 {
                    v += ab * (rel_mask(f, *r, s) & bc).count_ones() as f32;
                }
            }
            v
        }
    }
}

/// The function: terms with their weights.
#[derive(Clone, Default)]
pub struct Function {
    pub terms: Vec<(Term, f32)>,
}

impl Function {
    pub fn load() -> Function {
        let text = std::fs::read_to_string(FUNCTION_FILE).unwrap_or_default();
        let terms = text
            .lines()
            .filter_map(|l| {
                let w: Vec<&str> = l.split_whitespace().collect();
                Some((Term::from_code(w.get(1..)?)?, w.first()?.parse().ok()?))
            })
            .collect();
        Function { terms }
    }
    pub fn save(&self) -> Result<(), String> {
        let s: String = self.terms.iter().map(|(t, w)| format!("{w} {}\n", t.code())).collect();
        std::fs::write(FUNCTION_FILE, s).map_err(|e| e.to_string())
    }
    pub fn written(&self) -> String {
        if self.terms.is_empty() {
            return "0 (empty)".into();
        }
        self.terms.iter().map(|(t, w)| format!("{w:+.5}·{}", t.text())).collect::<Vec<_>>().join(" ")
    }
    fn eval(&self, f: &Feat) -> f32 {
        self.terms.iter().map(|(t, w)| w * term_value(t, f)).sum()
    }
}

struct Kid {
    uci: String,
    feat: Feat,
    /// mate (+) or a draw by the rules (0): the rules' value instead of the function's
    fixed: Option<f32>,
}

struct Lesson {
    kids: Vec<Kid>,
    sf: String,
}

fn lesson(fen: &str, sf: &str) -> Option<Lesson> {
    let mut p = Position::from_fen(fen).ok()?;
    let me = p.turn as usize;
    let mut kids = Vec::new();
    for m in legal_moves(&mut p) {
        p.make(m);
        let fixed = if legal_moves(&mut p).is_empty() { Some(if p.in_check() { 1.0e9 } else { 0.0 }) } else { None };
        kids.push(Kid { uci: move_to_uci(m), feat: feat(&p, me), fixed });
        p.unmake();
    }
    Some(Lesson { kids, sf: sf.to_string() })
}

fn value(f: &Function, k: &Kid) -> f32 {
    k.fixed.unwrap_or_else(|| f.eval(&k.feat))
}

fn pick<'a>(f: &Function, l: &'a Lesson) -> &'a str {
    let mut best = (f32::NEG_INFINITY, "");
    for k in &l.kids {
        let v = value(f, k);
        if v > best.0 {
            best = (v, k.uci.as_str());
        }
    }
    best.1
}

fn share(f: &Function, set: &[Lesson]) -> f64 {
    set.iter().filter(|l| pick(f, l) == l.sf).count() as f64 / set.len().max(1) as f64
}

fn base_values(f: &Function, set: &[Lesson]) -> Vec<Vec<f32>> {
    set.iter().map(|l| l.kids.iter().map(|k| value(f, k)).collect()).collect()
}

fn column(t: &Term, set: &[Lesson]) -> Vec<Vec<f32>> {
    set.iter().map(|l| l.kids.iter().map(|k| if k.fixed.is_some() { 0.0 } else { term_value(t, &k.feat) }).collect()).collect()
}

fn share_with(set: &[Lesson], base: &[Vec<f32>], extra: Option<(&[Vec<f32>], f32)>) -> f64 {
    let mut hit = 0;
    for (li, l) in set.iter().enumerate() {
        let mut best = (f32::NEG_INFINITY, 0usize);
        for (ki, k) in l.kids.iter().enumerate() {
            let v = match k.fixed {
                Some(v) => v,
                None => base[li][ki] + extra.map_or(0.0, |(col, w)| w * col[li][ki]),
            };
            if v > best.0 {
                best = (v, ki);
            }
        }
        hit += (l.kids.get(best.1).map(|k| k.uci == l.sf).unwrap_or(false)) as usize;
    }
    hit as f64 / set.len().max(1) as f64
}

fn read_lessons(path: &str, limit: usize) -> Vec<Lesson> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let rows: Vec<(String, String)> = text.lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.trim().to_string())).take(limit).collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = rows.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = rows.chunks(chunk).map(|c| s.spawn(move || c.iter().filter_map(|(f, m)| lesson(f, m)).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

/// A piece of the Hugging Face data: `n` positions whose key falls in `bucket` (of 20), written once to `path`.
fn hf_set(path: &str, bucket: std::ops::Range<u64>, n: usize) -> Vec<Lesson> {
    if std::fs::metadata(path).is_err() {
        let text = std::fs::read_to_string(crate::golden_table::HF_FILE).unwrap_or_default();
        let mut out = String::new();
        let mut k = 0;
        for line in text.lines() {
            let mut it = line.splitn(3, '\t');
            let (Some(_), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
            let Ok(p) = Position::from_fen(fen) else { continue };
            if bucket.contains(&(p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20)) {
                out.push_str(&format!("{fen}\t{mv}\n"));
                k += 1;
                if k >= n {
                    break;
                }
            }
        }
        let _ = std::fs::write(path, out);
    }
    read_lessons(path, n)
}

fn all_lessons() -> Vec<Lesson> {
    let mut l = hf_set("out/golden/grow_pretrain.tsv", 0..18, PRETRAIN);
    l.extend(read_lessons(LESSONS, usize::MAX));
    l
}

fn check_set() -> Vec<Lesson> {
    hf_set("out/golden/grow_check.tsv", 19..20, CHECK_N)
}

/// The candidates of a round: every product of one or two factors, and a fresh draw of products of three and of concepts.
fn candidates(seed: u64) -> Vec<Term> {
    let mut out = Vec::new();
    for a in 0..NF as u8 {
        out.push(Term::Prod([a, 0, 0], 1));
        for b in a + 1..NF as u8 {
            out.push(Term::Prod([a, b, 0], 2));
        }
    }
    let mut r = seed | 1;
    let mut next = || {
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        r
    };
    let bools: Vec<u8> = (4..NF as u8).collect();
    for _ in 0..DRAW / 4 {
        let (a, b, c) = ((next() % NF as u64) as u8, (next() % NF as u64) as u8, (next() % NF as u64) as u8);
        if a < b && b < c {
            out.push(Term::Prod([a, b, c], 3));
        }
    }
    for _ in 0..DRAW {
        let a = (next() % (NF as u64 + 1)) as u8;
        let b = (next() % (NF as u64 + 1)) as u8;
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        let r = (next() % RELATIONS.len() as u64) as u8;
        let c = bools[(next() % bools.len() as u64) as usize];
        out.push(Term::Rel { a, b, r, c });
    }
    out.sort_by_key(|t| format!("{t:?}"));
    out.dedup();
    out
}

/// One round: remove, re-tune, add (each also raising validate). Returns (removed, added, re-tuned).
fn round(f: &mut Function, lessons: &[Lesson], validate: &[Lesson], seed: u64) -> (Vec<String>, Option<String>, usize) {
    let mut removed = Vec::new();
    let mut i = 0;
    while i < f.terms.len() {
        let mut g = f.clone();
        g.terms.remove(i);
        if share(&g, lessons) >= share(f, lessons) && share(&g, validate) >= share(f, validate) {
            removed.push(f.terms[i].0.text());
            *f = g;
        } else {
            i += 1;
        }
    }
    let mut tuned = 0;
    for i in 0..f.terms.len() {
        let (cur_l, cur_v) = (share(f, lessons), share(f, validate));
        let mut best: Option<(f64, f32)> = None;
        for k in [0.5f32, 0.8, 1.25, 2.0, -1.0] {
            let mut g = f.clone();
            g.terms[i].1 *= k;
            let (l, v) = (share(&g, lessons), share(&g, validate));
            if l > cur_l && v >= cur_v && best.map_or(true, |b| l > b.0) {
                best = Some((l, g.terms[i].1));
            }
        }
        if let Some((_, w)) = best {
            f.terms[i].1 = w;
            tuned += 1;
        }
    }
    // addition: candidates scored on all cores; the best that also raises validate is added
    let cands: Vec<Term> = candidates(seed).into_iter().filter(|t| !f.terms.iter().any(|x| x.0 == *t)).collect();
    let base = base_values(f, lessons);
    let vbase = base_values(f, validate);
    let cur_l = share_with(lessons, &base, None);
    let cur_v = share_with(validate, &vbase, None);
    let scale = if f.terms.is_empty() { 1.0 } else { base.iter().flatten().filter(|x| x.abs() < 1e8).fold(0f32, |m, x| m.max(x.abs())).max(1.0) };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = cands.len().div_ceil(threads).max(1);
    let mut ranked: Vec<(f64, Term, f32)> = std::thread::scope(|s| {
        let hs: Vec<_> = cands
            .chunks(chunk)
            .map(|cs| {
                let (base, lessons) = (&base, lessons);
                s.spawn(move || {
                    let mut out = Vec::new();
                    for t in cs {
                        let col = column(t, lessons);
                        let spread = col.iter().flatten().fold(0f32, |m, x| m.max(x.abs()));
                        if spread < 1e-6 {
                            continue;
                        }
                        for k in [-1.0f32, -0.3, -0.1, -0.03, 0.03, 0.1, 0.3, 1.0] {
                            let w = k * scale / spread;
                            let sh = share_with(lessons, base, Some((&col, w)));
                            if sh > cur_l {
                                out.push((sh, *t, w));
                            }
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut added = None;
    for &(_, t, w) in ranked.iter().take(40) {
        let vcol = column(&t, validate);
        if share_with(validate, &vbase, Some((&vcol, w))) > cur_v {
            f.terms.push((t, w));
            added = Some(format!("{w:+.5}·{}{}", t.text(), if matches!(t, Term::Rel { .. }) { " (a new concept)" } else { "" }));
            break;
        }
    }
    (removed, added, tuned)
}

/// `rounds` rounds of growth on every lesson so far.
pub fn grow(rounds: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let lessons = all_lessons();
    if lessons.is_empty() {
        return Err("no lessons yet".into());
    }
    let validate = hf_set("out/golden/grow_validate.tsv", 18..19, 2_000);
    let check = check_set();
    let mut f = Function::load();
    let before = (share(&f, &lessons), share(&f, &validate), share(&f, &check), f.terms.len());
    let mut out = String::new();
    let seed0 = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7);
    for r in 1..=rounds {
        let (removed, added, tuned) = round(&mut f, &lessons, &validate, seed0 ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        out.push_str(&format!(
            "  round {r}: removed {}; re-tuned {tuned}; added {}; {} terms; lessons {:.1}%, validate {:.1}%, check {:.1}%\n",
            if removed.is_empty() { "nothing".to_string() } else { removed.join(", ") },
            added.clone().unwrap_or_else(|| "nothing".into()),
            f.terms.len(),
            100.0 * share(&f, &lessons),
            100.0 * share(&f, &validate),
            100.0 * share(&f, &check)
        ));
    }
    f.save()?;
    Ok(format!(
        "the Golden Function: {} lessons ({PRETRAIN} from the data, the rest from the games); {} -> {} terms; Stockfish's move: lessons {:.1}% -> {:.1}%, validate {:.1}% -> {:.1}%, fixed check {:.1}% -> {:.1}% ({:.0} s)\n{out}  F = {}\n",
        lessons.len(),
        before.3,
        f.terms.len(),
        100.0 * before.0,
        100.0 * share(&f, &lessons),
        100.0 * before.1,
        100.0 * share(&f, &validate),
        100.0 * before.2,
        100.0 * share(&f, &check),
        t0.elapsed().as_secs_f64(),
        f.written()
    ))
}

// ---------------------------------------------------------------- playing: the opening table, else the search

fn table_move(fen: &str) -> Option<String> {
    let p = Position::from_fen(fen).ok()?;
    let text = std::fs::read_to_string(crate::golden_table::TABLE_FILE).ok()?;
    let key = p.key64().to_string();
    for l in text.lines() {
        let mut it = l.splitn(3, '\t');
        if it.next() == Some(key.as_str()) {
            return it.next().map(|m| m.to_string());
        }
    }
    None
}

const MATE: f32 = 1.0e9;

/// Every line to `depth`, nothing cut; at a leaf the function's value for the side that has just moved, so minus it.
fn search(f: &Function, p: &mut Position, depth: usize) -> f32 {
    if depth == 0 {
        if !crate::golden_table::has_legal(p) {
            return if p.in_check() { -MATE } else { 0.0 };
        }
        return -f.eval(&feat(p, 1 - p.turn as usize));
    }
    use crate::fast::movegen::{gen_moves, MAX_MOVES};
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, false);
    let mut best = f32::NEG_INFINITY;
    let mut any = false;
    for &m in &list[..n] {
        p.make(m);
        if p.in_check_c(me) {
            p.unmake();
            continue;
        }
        any = true;
        let v = -search(f, p, depth - 1);
        p.unmake();
        best = best.max(v);
    }
    if !any {
        return if p.in_check() { -MATE } else { 0.0 };
    }
    best
}

fn pick_deep(f: &Function, fen: &str, depth: usize) -> Option<String> {
    let mut p = Position::from_fen(fen).ok()?;
    let ms = legal_moves(&mut p);
    let vals: Vec<(f32, String)> = std::thread::scope(|s| {
        let hs: Vec<_> = ms
            .iter()
            .map(|&m| {
                s.spawn(move || {
                    let mut q = Position::from_fen(fen).unwrap();
                    q.make(m);
                    (-search(f, &mut q, depth - 1), move_to_uci(m))
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    vals.into_iter().fold(None::<(f32, String)>, |b, x| match b { Some(b) if b.0 >= x.0 => Some(b), _ => Some(x) }).map(|x| x.1)
}

/// Golden Boy's move: the opening table when familiar, else every line with the function at the leaves, deepening within `secs`.
pub fn goldenboy_move_deep(fen: &str, secs: f64) -> Result<(String, String), String> {
    if let Some(m) = table_move(fen) {
        return Ok((m, "the opening table".into()));
    }
    let f = Function::load();
    let t0 = std::time::Instant::now();
    let (mut best, mut depth, mut last, mut factor) = (String::new(), 0, 0.0f64, 35.0f64);
    for d in 1..=12 {
        let spent = t0.elapsed().as_secs_f64();
        if d > 2 && spent + last * factor > secs {
            break;
        }
        let t1 = std::time::Instant::now();
        match pick_deep(&f, fen, d) {
            Some(x) => {
                best = x;
                depth = d;
            }
            None => break,
        }
        let s = t1.elapsed().as_secs_f64();
        if last > 0.0005 {
            factor = (s / last).clamp(5.0, 60.0);
        }
        last = s;
        if t0.elapsed().as_secs_f64() > secs {
            break;
        }
    }
    if best.is_empty() {
        return Err("no legal move".into());
    }
    Ok((best, format!("the Golden Function ({} terms), every line to depth {depth}", f.terms.len())))
}
