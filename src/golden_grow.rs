//! Golden Boy writes the Golden Function longer as it plays Stockfish.
//!
//! The net (as designed): 64 points on the grid; each point holds its own
//! numbers only — the piece's value (100 / 325 / 325 / 500 / 975, the king
//! and an empty square 0), the side (+1 the mover's, -1 the opponent's, 0
//! empty), and how many of the mover's and of the opponent's pieces attack
//! it. How they combine is the Golden Function's to decide: a term is a sum
//! over the 64 points of a product of the point's numbers and comparisons
//! of them; the function is a weighted sum of terms, for the position after
//! a move, seen from the mover. Golden Boy plays the legal move the function
//! values most. It starts empty.
//!
//! Every position of the games is a lesson: Stockfish's own moves, and for
//! Golden Boy's positions the move Stockfish would play there (a point when
//! Golden Boy played it). After every game the function grows: each term is
//! tried away and removed when the function is better without it; then
//! candidate terms with a range of weights, and the best is added only when
//! it raises the share of Stockfish's moves over every lesson so far. A
//! fixed check set (Stockfish's moves in middlegame and endgame positions,
//! never learned from) shows whether it really gets better.

use crate::fast::movegen::legal_moves;
use crate::fast::position::{color_of, move_to_uci, type_of, Position};

pub const FUNCTION_FILE: &str = "out/golden/grow_function.txt";
pub const LESSONS: &str = "out/golden/grow_lessons.tsv";
const CHECK_N: usize = 2_000;
const WORTH: [f32; 7] = [0.0, 100.0, 325.0, 325.0, 500.0, 975.0, 0.0];

/// The factors a term may multiply, read from one point's numbers.
pub const FACTORS: [&str; 13] = ["value", "side", "my_attackers", "their_attackers", "mine", "theirs", "empty", "more_mine", "more_theirs", "mine_attack", "they_attack", "none_mine", "none_theirs"];
const NF: usize = FACTORS.len();

/// A term: the sum over the 64 points of the product of up to three factors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Term(pub [u8; 3], pub u8);

impl Term {
    fn text(&self) -> String {
        let f: Vec<&str> = self.0[..self.1 as usize].iter().map(|&i| FACTORS[i as usize]).collect();
        format!("Σ {}", f.join("·"))
    }
}

/// Every candidate term: products of one to three different factors.
fn candidates() -> Vec<Term> {
    let mut out = Vec::new();
    for a in 0..NF as u8 {
        out.push(Term([a, 0, 0], 1));
        for b in a + 1..NF as u8 {
            out.push(Term([a, b, 0], 2));
            for c in b + 1..NF as u8 {
                out.push(Term([a, b, c], 3));
            }
        }
    }
    out
}

/// The points' factors of a position seen from `me`: [point][factor].
fn factors(p: &Position, me: usize) -> Vec<[f32; NF]> {
    use crate::fast::bitboard::SQ64_TO_120;
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    let mut cnt = [[0u8; 64]; 2];
    for c in 0..2 {
        for ty in 0..6 {
            let mut b = p.bb[c * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let mut at = match ty {
                    0 => if c == 0 { t.att.pawn_w[s] } else { t.att.pawn_b[s] },
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, occ),
                    3 => t.rook_attacks(s, occ),
                    4 => t.queen_attacks(s, occ),
                    _ => t.att.king[s],
                };
                while at != 0 {
                    let q = at.trailing_zeros() as usize;
                    at &= at - 1;
                    cnt[c][q] += 1;
                }
            }
        }
    }
    (0..64)
        .map(|s| {
            let pc = p.board[SQ64_TO_120[s]];
            let (value, side) = if pc == 0 { (0.0, 0.0) } else { (WORTH[type_of(pc) as usize], if color_of(pc) as usize == me { 1.0 } else { -1.0 }) };
            let (am, at) = (cnt[me][s] as f32, cnt[1 - me][s] as f32);
            let b = |x: bool| x as u8 as f32;
            [value, side, am, at, b(side > 0.0), b(side < 0.0), b(pc == 0), b(am > at), b(at > am), b(am > 0.0), b(at > 0.0), b(am == 0.0), b(at == 0.0)]
        })
        .collect()
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
                let wt: f32 = w.first()?.parse().ok()?;
                let f: Vec<u8> = w[1..].iter().filter_map(|x| FACTORS.iter().position(|n| n == x).map(|i| i as u8)).collect();
                if f.is_empty() || f.len() > 3 {
                    return None;
                }
                let mut a = [0u8; 3];
                a[..f.len()].copy_from_slice(&f);
                Some((Term(a, f.len() as u8), wt))
            })
            .collect();
        Function { terms }
    }
    pub fn save(&self) -> Result<(), String> {
        let s: String = self.terms.iter().map(|(t, w)| format!("{w} {}\n", t.0[..t.1 as usize].iter().map(|&i| FACTORS[i as usize]).collect::<Vec<_>>().join(" "))).collect();
        std::fs::write(FUNCTION_FILE, s).map_err(|e| e.to_string())
    }
    pub fn written(&self) -> String {
        if self.terms.is_empty() {
            return "0 (empty)".into();
        }
        self.terms.iter().map(|(t, w)| format!("{w:+.4}·{}", t.text())).collect::<Vec<_>>().join(" ")
    }
}

/// One child: every term's value is computed on demand from its points.
struct Kid {
    uci: String,
    pts: Vec<[f32; NF]>,
    /// mate (+) or a draw by the rules (0): the rules' value instead of the function's
    fixed: Option<f32>,
}

fn term_value(t: &Term, pts: &[[f32; NF]]) -> f32 {
    let f = &t.0[..t.1 as usize];
    pts.iter().map(|p| f.iter().map(|&i| p[i as usize]).product::<f32>()).sum()
}

struct Lesson {
    kids: Vec<Kid>,
    sf: String,
    /// every kid's value of every term in the function and of the candidates, filled as needed
    cache: std::collections::HashMap<(usize, Term), f32>,
}

fn lesson(fen: &str, sf: &str) -> Option<Lesson> {
    let mut p = Position::from_fen(fen).ok()?;
    let me = p.turn as usize;
    let mut kids = Vec::new();
    for m in legal_moves(&mut p) {
        p.make(m);
        let fixed = if legal_moves(&mut p).is_empty() { Some(if p.in_check() { 1.0e9 } else { 0.0 }) } else { None };
        kids.push(Kid { uci: move_to_uci(m), pts: factors(&p, me), fixed });
        p.unmake();
    }
    Some(Lesson { kids, sf: sf.to_string(), cache: Default::default() })
}

fn value(f: &Function, k: &Kid) -> f32 {
    if let Some(v) = k.fixed {
        return v;
    }
    f.terms.iter().map(|(t, w)| w * term_value(t, &k.pts)).sum()
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

/// Every kid's value of each term (so trying a weight is a sum, not a recount).
fn table(set: &[Lesson], terms: &[Term]) -> Vec<Vec<Vec<f32>>> {
    set.iter().map(|l| l.kids.iter().map(|k| terms.iter().map(|t| if k.fixed.is_some() { 0.0 } else { term_value(t, &k.pts) }).collect()).collect()).collect()
}

/// The share of Stockfish's moves, from the per-term tables: base values per kid plus `w` x the extra term column (if any).
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

fn base_values(f: &Function, set: &[Lesson]) -> Vec<Vec<f32>> {
    set.iter().map(|l| l.kids.iter().map(|k| value(f, k)).collect()).collect()
}

fn share(f: &Function, set: &[Lesson]) -> f64 {
    set.iter().filter(|l| pick(f, l) == l.sf).count() as f64 / set.len().max(1) as f64
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

/// The fixed check set: held-out middlegame and endgame positions with Stockfish's move (Hugging Face data).
fn check_set() -> Vec<Lesson> {
    let path = "out/golden/grow_check.tsv";
    if std::fs::metadata(path).is_err() {
        let text = std::fs::read_to_string(crate::golden_table::HF_FILE).unwrap_or_default();
        let mut out = String::new();
        let mut n = 0;
        for line in text.lines() {
            let mut it = line.splitn(3, '\t');
            let (Some(_), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
            let Ok(p) = Position::from_fen(fen) else { continue };
            if p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20 == 19 {
                out.push_str(&format!("{fen}\t{mv}\n"));
                n += 1;
                if n >= CHECK_N {
                    break;
                }
            }
        }
        let _ = std::fs::write(path, out);
    }
    read_lessons(path, CHECK_N)
}

/// A held-out piece of the Hugging Face data: `n` positions whose key falls in `bucket` (of 20), written once to `path`.
fn hf_set(path: &str, bucket: std::ops::Range<u64>, n: usize, skip: usize) -> Vec<Lesson> {
    if std::fs::metadata(path).is_err() {
        let text = std::fs::read_to_string(crate::golden_table::HF_FILE).unwrap_or_default();
        let mut out = String::new();
        let (mut k, mut seen) = (0, 0);
        for line in text.lines() {
            let mut it = line.splitn(3, '\t');
            let (Some(_), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
            let Ok(p) = Position::from_fen(fen) else { continue };
            if bucket.contains(&(p.key64().wrapping_mul(0x9E37_79B9_7F4A_7C15) % 20)) {
                seen += 1;
                if seen <= skip {
                    continue;
                }
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

/// The lessons: Stockfish's moves in PRETRAIN Hugging Face positions (train part of the data), then every position of the games.
pub const PRETRAIN: usize = 3_000;

fn all_lessons() -> Vec<Lesson> {
    let mut l = hf_set("out/golden/grow_pretrain.tsv", 0..18, PRETRAIN, 0);
    l.extend(read_lessons(LESSONS, usize::MAX));
    l
}

/// One round of growth: remove what does not help, re-tune every weight, add
/// the best new term. Every change must raise (or for a removal, not lower)
/// Stockfish's share on the validate set as well as on the lessons, so a
/// change that only fits the lessons is not taken. The fixed check set is
/// only reported.
fn round(f: &mut Function, lessons: &[Lesson], validate: &[Lesson], cols: &[Vec<Vec<f32>>], vcols: &[Vec<Vec<f32>>], cands: &[Term]) -> (Vec<String>, Option<String>, usize) {
    // removal: shorter and not worse on either set
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
    // re-tune every weight: a few multipliers, kept when both sets gain (the lessons strictly, validate not lower)
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
    // addition: candidates ranked on the lessons; the best that also raises validate is added
    let base = base_values(f, lessons);
    let vbase = base_values(f, validate);
    let (cur_l, cur_v) = (share_with(lessons, &base, None), share_with(validate, &vbase, None));
    let scale = if f.terms.is_empty() { 1.0 } else { base.iter().flatten().filter(|x| x.abs() < 1e8).fold(0f32, |m, x| m.max(x.abs())).max(1.0) };
    let mut ranked: Vec<(f64, usize, f32)> = Vec::new();
    for (ci, t) in cands.iter().enumerate() {
        if f.terms.iter().any(|x| x.0 == *t) {
            continue;
        }
        let col: Vec<Vec<f32>> = cols.iter().map(|l| l.iter().map(|k| k[ci]).collect()).collect();
        let spread = col.iter().flatten().fold(0f32, |m, x| m.max(x.abs())).max(1e-6);
        for k in [-1.0f32, -0.3, -0.1, -0.03, 0.03, 0.1, 0.3, 1.0] {
            let w = k * scale / spread;
            let s = share_with(lessons, &base, Some((&col, w)));
            if s > cur_l {
                ranked.push((s, ci, w));
            }
        }
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut added = None;
    for &(_, ci, w) in ranked.iter().take(40) {
        let vcol: Vec<Vec<f32>> = vcols.iter().map(|l| l.iter().map(|k| k[ci]).collect()).collect();
        if share_with(validate, &vbase, Some((&vcol, w))) > cur_v {
            f.terms.push((cands[ci], w));
            added = Some(format!("{w:+.5}·{}", cands[ci].text()));
            break;
        }
    }
    (removed, added, tuned)
}

/// After a game (or before the first): `rounds` rounds of growth on every lesson so far.
pub fn grow(rounds: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let lessons = all_lessons();
    if lessons.is_empty() {
        return Err("no lessons yet".into());
    }
    let validate = hf_set("out/golden/grow_validate.tsv", 18..19, 2_000, 0);
    let check = check_set();
    let cands = candidates();
    let cols = table(&lessons, &cands);
    let vcols = table(&validate, &cands);
    let mut f = Function::load();
    let before = (share(&f, &lessons), share(&f, &validate), share(&f, &check), f.terms.len());
    let mut out = String::new();
    for r in 1..=rounds {
        let (removed, added, tuned) = round(&mut f, &lessons, &validate, &cols, &vcols, &cands);
        out.push_str(&format!(
            "  round {r}: removed {}; re-tuned {tuned}; added {}; {} terms; lessons {:.1}%, validate {:.1}%, check {:.1}%\n",
            if removed.is_empty() { "nothing".to_string() } else { removed.join(", ") },
            added.clone().unwrap_or_else(|| "nothing".into()),
            f.terms.len(),
            100.0 * share(&f, &lessons),
            100.0 * share(&f, &validate),
            100.0 * share(&f, &check)
        ));
        if added.is_none() && removed.is_empty() && tuned == 0 {
            break;
        }
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

/// Golden Boy's move: the opening table when the position is familiar, else the function's best (1 ply).
pub fn goldenboy_move(fen: &str) -> Result<(String, String), String> {
    let p = Position::from_fen(fen)?;
    if let Ok(text) = std::fs::read_to_string(crate::golden_table::TABLE_FILE) {
        let key = p.key64().to_string();
        for l in text.lines() {
            let mut it = l.splitn(3, '\t');
            if it.next() == Some(key.as_str()) {
                if let Some(m) = it.next() {
                    return Ok((m.to_string(), "the opening table".into()));
                }
            }
        }
    }
    let f = Function::load();
    let l = lesson(fen, "").ok_or("no legal move")?;
    Ok((pick(&f, &l).to_string(), format!("the Golden Function ({} terms)", f.terms.len())))
}

// ---------------------------------------------------------------- the search: up to 5 s instead of 1 ply

/// The function's value of position `p` for `me`, straight from the points (no allocation).
fn eval_at(f: &Function, p: &Position, me: usize) -> f32 {
    use crate::fast::bitboard::SQ64_TO_120;
    let t = crate::fast::bitboard::tables();
    let occ = p.occ;
    let mut cnt = [[0u8; 64]; 2];
    for c in 0..2 {
        for ty in 0..6 {
            let mut b = p.bb[c * 6 + ty];
            while b != 0 {
                let s = b.trailing_zeros() as usize;
                b &= b - 1;
                let mut at = match ty {
                    0 => if c == 0 { t.att.pawn_w[s] } else { t.att.pawn_b[s] },
                    1 => t.att.knight[s],
                    2 => t.bishop_attacks(s, occ),
                    3 => t.rook_attacks(s, occ),
                    4 => t.queen_attacks(s, occ),
                    _ => t.att.king[s],
                };
                while at != 0 {
                    let q = at.trailing_zeros() as usize;
                    at &= at - 1;
                    cnt[c][q] += 1;
                }
            }
        }
    }
    let mut total = 0f32;
    for s in 0..64 {
        let pc = p.board[SQ64_TO_120[s]];
        let (value, side) = if pc == 0 { (0.0, 0.0) } else { (WORTH[type_of(pc) as usize], if color_of(pc) as usize == me { 1.0 } else { -1.0 }) };
        let (am, at) = (cnt[me][s] as f32, cnt[1 - me][s] as f32);
        let b = |x: bool| x as u8 as f32;
        let x = [value, side, am, at, b(side > 0.0), b(side < 0.0), b(pc == 0), b(am > at), b(at > am), b(am > 0.0), b(at > 0.0), b(am == 0.0), b(at == 0.0)];
        for (term, w) in &f.terms {
            let mut v = *w;
            for &i in &term.0[..term.1 as usize] {
                v *= x[i as usize];
            }
            total += v;
        }
    }
    total
}

const MATE: f32 = 1.0e9;

/// Every line to `depth`, nothing cut; the value for the side to move. At a
/// leaf the function's value for the side that has just moved, so minus it.
fn search(f: &Function, p: &mut Position, depth: usize) -> f32 {
    if depth == 0 {
        if !crate::golden_table::has_legal(p) {
            return if p.in_check() { -MATE } else { 0.0 };
        }
        return -eval_at(f, p, 1 - p.turn as usize);
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

/// Golden Boy's move with the function at the leaves of every line, deepening while the next depth fits in `secs`.
pub fn goldenboy_move_deep(fen: &str, secs: f64) -> Result<(String, String), String> {
    let (m, how) = goldenboy_move(fen)?;
    if how == "the opening table" {
        return Ok((m, how));
    }
    let f = Function::load();
    let t0 = std::time::Instant::now();
    let (mut best, mut depth, mut last, mut factor) = (m, 1, 0.0f64, 35.0f64);
    for d in 2..=12 {
        let spent = t0.elapsed().as_secs_f64();
        if d > 3 && spent + last * factor > secs {
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
    Ok((best, format!("the Golden Function ({} terms), every line to depth {depth}", f.terms.len())))
}
