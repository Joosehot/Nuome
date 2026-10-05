//! The Golden formula for chess, made like the Goldbach bound: the question
//! first, word for word; the answer by the open-problem way of thinking
//! (CLAUDE.md): what the conditions force, the choice left, a short exact
//! search of that choice, and a plain verdict, checked on a set it never saw.
//!
//! The question (QUESTION): P a legal position, s to move; every legal move m
//! and reply r, Q = P·m·r; find p(m, r) of five quantities of Q — legality,
//! safety, force, material, tempo — such that the move maximising
//! min over r of p(m, r) keeps the value of P under perfect play.
//!
//! The truth is known exactly only up to five pieces (Syzygy), so the
//! condition is searched on bench/golden_formula/search.tsv and checked on
//! check.tsv (tools/golden_formula_truth.py; both written once).

use crate::fast::movegen::{gen_moves, legal_moves, uci_to_move, MAX_MOVES};
use crate::fast::position::{m_to, move_to_uci, type_of, Position};

pub const QUESTION: &str = "Let P be a legal chess position and s the side to move. For every legal move m of s and every legal reply r, let Q = P·m·r. Find the function p(m, r) of five quantities of Q — legality L, safety S, force F, material M, tempo T — such that the move m that maximises min over r of p(m, r) keeps the value of P under perfect play (win stays win, draw stays at least draw).";
pub const SEARCH_SET: &str = "bench/golden_formula/search.tsv";
pub const CHECK_SET: &str = "bench/golden_formula/check.tsv";
pub const FILE: &str = "out/golden/formula_chess.txt";
const SECONDS: f64 = 5.0;

/// The quantities of Q read by the rules, from s's side (s is to move in Q).
pub const NAMES: [&str; 9] = ["M_pawn", "M_knight", "M_bishop", "M_rook", "M_queen", "S", "F", "T", "1"];
const NX: usize = 9;
const INF: i64 = i64::MAX / 4;

/// What Q is: decided by legality alone, or the quantities.
#[derive(Clone, Copy)]
enum Q {
    Lost,
    Won,
    Draw,
    X([i32; NX]),
}

/// The pieces of the other side that the side to move can capture with a legal move (distinct squares).
fn captures(p: &mut Position) -> i32 {
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, true);
    let mut seen = 0u128;
    for &m in &list[..n] {
        let t = p.board[m_to(m) as usize];
        if t == 0 || type_of(t) == 6 {
            continue;
        }
        p.make(m);
        let ok = !p.in_check_c(me);
        p.unmake();
        if ok {
            seen |= 1u128 << m_to(m);
        }
    }
    seen.count_ones() as i32
}

fn read_q(p: &mut Position) -> Q {
    let s = p.turn as usize;
    let mine = legal_moves(p).len() as i32;
    if mine == 0 {
        return if p.in_check() { Q::Lost } else { Q::Draw };
    }
    if p.insufficient() || p.halfmove >= 100 {
        return Q::Draw;
    }
    let mut x = [0i32; NX];
    for t in 0..5 {
        x[t] = p.bb[s * 6 + t].count_ones() as i32 - p.bb[(1 - s) * 6 + t].count_ones() as i32;
    }
    // force: what s can take; safety: what the other side could take, were it to move; tempo: moves
    x[6] = captures(p);
    p.make_null();
    x[5] = -captures(p);
    let theirs = legal_moves(p).len() as i32;
    p.unmake_null();
    x[7] = mine - theirs;
    x[8] = 1;
    Q::X(x)
}

/// One position of a set: per move, whether it keeps the value, and its pairs (or the end the move itself makes).
struct Pos {
    fen: String,
    moves: Vec<(String, bool, Result<Vec<Q>, Q>)>,
}

fn load_set(path: &str) -> Result<Vec<Pos>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e} (run tools/golden_formula_truth.py)"))?;
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let (fen, rest) = line.split_once('\t').ok_or("bad line")?;
        let vals: Vec<(&str, i32)> = rest.split_whitespace().filter_map(|t| t.split_once(':').and_then(|(m, v)| v.parse().ok().map(|v| (m, v)))).collect();
        let best = vals.iter().map(|v| v.1).max().unwrap_or(0);
        let mut p = Position::from_fen(fen)?;
        let mut moves = Vec::new();
        for (u, v) in vals {
            let m = uci_to_move(&mut p, u).ok_or(format!("{u} in {fen}"))?;
            moves.push((u.to_string(), v == best, pairs(&mut p, m)));
        }
        out.push(Pos { fen: fen.to_string(), moves });
    }
    Ok(out)
}

/// Every reply's Q after m, or the end m makes at once.
fn pairs(p: &mut Position, m: u32) -> Result<Vec<Q>, Q> {
    p.make(m);
    let replies = legal_moves(p);
    let r = if replies.is_empty() {
        Err(if p.in_check() { Q::Won } else { Q::Draw })
    } else if p.insufficient() {
        Err(Q::Draw)
    } else {
        Ok(replies
            .into_iter()
            .map(|r| {
                p.make(r);
                let q = read_q(p);
                p.unmake();
                q
            })
            .collect())
    };
    p.unmake();
    r
}

fn value(w: &[i64; NX], q: &Q) -> i64 {
    match q {
        Q::Lost => -INF,
        Q::Won => INF,
        Q::Draw => 0,
        Q::X(x) => x.iter().zip(w).map(|(a, b)| *a as i64 * b).sum(),
    }
}

fn f_of(w: &[i64; NX], pairs: &Result<Vec<Q>, Q>) -> i64 {
    match pairs {
        Err(end) => value(w, end),
        Ok(qs) => qs.iter().map(|q| value(w, q)).min().unwrap_or(0),
    }
}

/// Whether the condition holds on the position: every move with the largest f keeps the value.
fn holds(w: &[i64; NX], pos: &Pos) -> bool {
    let fs: Vec<i64> = pos.moves.iter().map(|m| f_of(w, &m.2)).collect();
    let top = *fs.iter().max().unwrap();
    pos.moves.iter().zip(&fs).all(|(m, &f)| f != top || m.1)
}

fn count(w: &[i64; NX], set: &[Pos]) -> usize {
    set.iter().filter(|p| holds(w, p)).count()
}

/// The exact search of the choice left: integer weights, local steps from many starts, for `seconds`.
fn search(set: &[Pos], seconds: f64) -> ([i64; NX], usize) {
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let t0 = std::time::Instant::now();
    let size = |w: &[i64; NX]| w.iter().map(|x| x.abs()).sum::<i64>();
    let mut best = ([0i64; NX], count(&[0; NX], set));
    while t0.elapsed().as_secs_f64() < seconds && best.1 < set.len() {
        let mut w = [0i64; NX];
        for x in w.iter_mut() {
            *x = (next() % 21) as i64 - 10;
        }
        let mut c = count(&w, set);
        loop {
            let mut moved = false;
            for i in 0..NX {
                for d in [-1i64, 1, -5, 5] {
                    let mut v = w;
                    v[i] += d;
                    let cv = count(&v, set);
                    if cv > c || (cv == c && size(&v) < size(&w)) {
                        w = v;
                        c = cv;
                        moved = true;
                    }
                }
            }
            if !moved || t0.elapsed().as_secs_f64() > seconds {
                break;
            }
        }
        if c > best.1 || (c == best.1 && size(&w) < size(&best.0)) {
            best = (w, c);
        }
    }
    best
}

pub fn written(w: &[i64; NX]) -> String {
    let terms: Vec<String> = w.iter().zip(NAMES).filter(|(x, _)| **x != 0).map(|(x, n)| if n == "1" { format!("{x:+}") } else { format!("{x:+}·{n}") }).collect();
    format!("p(m, r) = {}", if terms.is_empty() { "0".into() } else { terms.join(" ") })
}

pub fn load() -> Option<[i64; NX]> {
    let text = std::fs::read_to_string(FILE).ok()?;
    let v: Vec<i64> = text.lines().next()?.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    v.try_into().ok()
}

/// Golden Answer's answer to QUESTION. Logical: the four steps. Theoretical:
/// none (no known result names such a function). Abstract: the frozen p, claiming nothing.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if kind_abstract {
        let g = guess()?;
        let mut lines = g.reasons.clone();
        lines.push("this is an answer that claims nothing".into());
        return Ok((format!("probably {}", g.p_text), lines, vec![]));
    }
    if !kind_logical {
        return theoretical();
    }
    let search_set = load_set(SEARCH_SET)?;
    let check_set = load_set(CHECK_SET)?;
    let mut lines = vec![
        "1. what the condition forces (legality decides first, by the rules): a Q where s has no legal move and is in check is lost, so p there is below every other value (-∞); a move that mates at once has +∞; a stalemate, or a draw by insufficient material or the fifty-move rule, is a draw: p = 0, the value every non-drawn Q is measured against".to_string(),
        "   the other four read on Q by the rules, from s's side (s to move in Q): material M = s's pieces minus the other's, per kind (pawn, knight, bishop, rook, queen); force F = the other side's pieces s can capture with a legal move; safety S = minus the pieces of s the other side could capture so, were it to move; tempo T = s's legal moves minus the other side's".to_string(),
        "2. the choice left: how the quantities combine. Searched here: p = w · (M_pawn, M_knight, M_bishop, M_rook, M_queen, S, F, T, 1) with whole-number weights".to_string(),
    ];
    // validate on a known case first: KQ against K alone
    let kqk: Vec<Pos> = load_set(SEARCH_SET)?.into_iter().filter(|p| {
        let b = p.fen.split(' ').next().unwrap_or("");
        b.chars().filter(|c| c.is_ascii_alphabetic()).count() == 3 && b.contains('Q')
    }).collect();
    let (wk, ck) = search(&kqk, SECONDS / 5.0);
    lines.push(format!("   validated on a known case first: KQ against K, {ck}/{} of its search positions with {}", kqk.len(), written(&wk)));
    let (w, c) = search(&search_set, SECONDS);
    lines.push(format!("3. searched for {SECONDS:.0} s on {SEARCH_SET}: the best p satisfies the condition on {c}/{} positions: {}", search_set.len(), written(&w)));
    let failed: Vec<&Pos> = check_set.iter().filter(|p| !holds(&w, p)).collect();
    lines.push(format!("   checked on {CHECK_SET}, never searched: {}/{}", check_set.len() - failed.len(), check_set.len()));
    let save = w.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
    let _ = std::fs::write(FILE, format!("{save}\n# {}\n# search {c}/{}, check {}/{}\n", written(&w), search_set.len(), check_set.len() - failed.len(), check_set.len()));
    let verdict = if c == search_set.len() && failed.is_empty() {
        format!("4. built and checked: {} holds on all {} positions searched and all {} checked (up to five pieces; beyond that the truth is not known, so not decided)", written(&w), search_set.len(), check_set.len())
    } else {
        let first = failed.first().map(|p| {
            let fs: Vec<(String, i64, bool)> = p.moves.iter().map(|m| (m.0.clone(), f_of(&w, &m.2), m.1)).collect();
            let top = fs.iter().map(|x| x.1).max().unwrap();
            let bad = fs.iter().find(|x| x.1 == top && !x.2).map(|x| x.0.clone()).unwrap_or_default();
            format!("; first counterexample in the check set: {} — p prefers {bad}, which does not keep the value", p.fen)
        }).unwrap_or_default();
        format!("4. undecided: no p of this form found that holds everywhere (search {c}/{}, check {}/{}){first}", search_set.len(), check_set.len() - failed.len(), check_set.len())
    };
    lines.push(verdict.clone());
    let short = if c == search_set.len() && failed.is_empty() { written(&w) } else { "undecided".to_string() };
    Ok((short, lines, vec!["the rules of chess; the truth from the Syzygy tables (Ronald de Man), 3-5 pieces".into()]))
}

/// The question, read whole.
pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

// ---------------------------------------------------------------- the theoretical answer

/// Shannon's evaluation (1950) of Q from the side to move, in tenths of a pawn:
/// 200(K-K') + 9(Q-Q') + 5(R-R') + 3(B-B'+N-N') + (P-P') - 0.5(D-D'+S-S'+I-I') + 0.1(M-M').
/// D doubled, S backward, I isolated pawns; M legal moves.
pub fn shannon(p: &mut Position) -> i64 {
    let s = p.turn as usize;
    let bb = p.bb;
    let count = |c: usize, t: usize| bb[c * 6 + t].count_ones() as i64;
    let side = |c: usize| -> i64 {
        2000 * count(c, 5) + 90 * count(c, 4) + 50 * count(c, 3) + 30 * (count(c, 2) + count(c, 1)) + 10 * count(c, 0)
    };
    // pawn weaknesses: doubled, backward, isolated (each a pawn)
    let weak = |c: usize| -> i64 {
        let pawns = bb[c * 6];
        let file = |f: i32| -> u64 { if (0..8).contains(&f) { 0x0101_0101_0101_0101u64 << f } else { 0 } };
        let mut n = 0;
        let mut b = pawns;
        while b != 0 {
            let sq = b.trailing_zeros() as i32;
            b &= b - 1;
            let (f, r) = (sq % 8, sq / 8);
            if (pawns & file(f)).count_ones() > 1 {
                n += 1; // doubled
            }
            let beside = pawns & (file(f - 1) | file(f + 1));
            if beside == 0 {
                n += 1; // isolated
            } else {
                // backward: every pawn beside it is further up the board
                let behind_or_level = (0..64).filter(|&q| beside & (1u64 << q) != 0).any(|q: i32| if c == 0 { q / 8 <= r } else { q / 8 >= r });
                if !behind_or_level {
                    n += 1;
                }
            }
        }
        n
    };
    let mine = legal_moves(p).len() as i64;
    p.make_null();
    let theirs = legal_moves(p).len() as i64;
    p.unmake_null();
    side(s) - side(1 - s) - 5 * (weak(s) - weak(1 - s)) + (mine - theirs)
}

/// p(m, r) by the known results: legality by the rules, then Shannon's function on Q.
fn theory_pairs(p: &mut Position, m: u32) -> i64 {
    p.make(m);
    let replies = legal_moves(p);
    let v = if replies.is_empty() {
        if p.in_check() { INF } else { 0 }
    } else if p.insufficient() {
        0
    } else {
        let mut worst = INF;
        for r in replies {
            p.make(r);
            let v = if legal_moves(p).is_empty() {
                if p.in_check() { -INF } else { 0 }
            } else if p.insufficient() || p.halfmove >= 100 {
                0
            } else {
                shannon(p)
            };
            p.unmake();
            worst = worst.min(v);
        }
        worst
    };
    p.unmake();
    v
}

fn theory_holds(pos: &Pos) -> bool {
    let Ok(mut p) = Position::from_fen(&pos.fen) else { return false };
    let fs: Vec<i64> = pos.moves.iter().map(|(u, _, _)| uci_to_move(&mut p, u).map_or(-INF, |m| theory_pairs(&mut p, m))).collect();
    let top = *fs.iter().max().unwrap();
    pos.moves.iter().zip(&fs).all(|(m, &f)| f != top || m.1)
}

/// The theoretical answer: the tables where they reach, Shannon's function beyond.
fn theoretical() -> Result<(String, Vec<String>, Vec<String>), String> {
    let check_set = load_set(CHECK_SET)?;
    let shannon_ok = check_set.iter().filter(|p| theory_holds(p)).count();
    let short = "p(m, r) = the value of Q in the Syzygy tables when Q has at most five pieces; otherwise Shannon's function of Q: 200ΔK + 9ΔQ + 5ΔR + 3(ΔB + ΔN) + ΔP − 0.5(ΔD + ΔS + ΔI) + 0.1ΔM".to_string();
    let lines = vec![
        "legality (the rules): mated in Q is −∞, mate at once is +∞, stalemate and draws by rule are 0".to_string(),
        "up to five pieces: the tables give the value of Q under perfect play; the move maximising min over r keeps the value of P, since the tables are exact (de Man)".to_string(),
        "more pieces: Shannon's evaluation function — material M (200, 9, 5, 3, 3, 1), safety S as pawn weakness (doubled D, backward S, isolated I, −0.5 each), tempo T as mobility (0.1 a legal move); force F has no term of its own: Shannon puts the forcing moves (checks, captures) in the lines followed, not in the function".to_string(),
        "claim: right up to five pieces; beyond, Shannon's function is a known approximation and is not known to keep the value".to_string(),
        format!("for information: Shannon's function alone (without the tables) keeps the value on {shannon_ok}/{} positions of {CHECK_SET}", check_set.len()),
    ];
    Ok((short, lines, vec![
        "Syzygy endgame tablebases, 3-5 pieces (Ronald de Man)".into(),
        "C. E. Shannon, Programming a Computer for Playing Chess, Philosophical Magazine 41 (1950)".into(),
        "the rules of chess (FIDE Laws, articles 3, 5, 9)".into(),
    ]))
}

/// Every legal move with f by the theoretical answer (Shannon's function past the tables), best first.
pub fn rank(fen: &str) -> Option<Vec<(String, i64)>> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut rows: Vec<(String, i64)> = legal_moves(&mut p).into_iter().map(|m| (move_to_uci(m), theory_pairs(&mut p, m))).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    Some(rows)
}

// ---------------------------------------------------------------- how p is used

pub const USE_QUESTION: &str = "Given p(m, r) from the previous answer, how is p used on the line m₁ r₁ m₂ r₂ … from P — how many pairs, which moves are followed, and where the value is read — so that the move chosen keeps the value of P under perfect play?";

pub fn read_use(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(USE_QUESTION).trim_end_matches(['.', '?', '!'])
}

/// Forcing plies followed past the pair, the checks among them, and the positions one root move may take.
pub const FORCING_PLIES: usize = 20;
const CHECK_PLIES: usize = 4;
const NODES: u64 = 20_000;

/// Shannon's type B from Q: the forceful variations (captures, promotions,
/// and checks in the first plies; every reply in check) followed, each side
/// free to stand on the position as it is; p read where they are quiet.
/// The value for the side to move.
fn quiet(p: &mut Position, plies: usize, depth: usize, nodes: &mut u64) -> i64 {
    *nodes += 1;
    let me = p.turn;
    let check = p.in_check();
    let all = legal_moves(p);
    if all.is_empty() {
        return if check { -INF } else { 0 };
    }
    if p.insufficient() || p.halfmove >= 100 {
        return 0;
    }
    let stand = if check { -INF } else { shannon(p) };
    if plies == 0 || *nodes > NODES {
        return if check { shannon(p) } else { stand };
    }
    let mut best = stand;
    for m in all {
        let capture = p.board[m_to(m) as usize] != 0 || crate::fast::position::m_flag(m) == 2;
        let promo = crate::fast::position::m_promo(m) != 0;
        p.make(m);
        let gives_check = depth < CHECK_PLIES && p.in_check();
        if check || capture || promo || gives_check {
            let v = -quiet(p, plies - 1, depth + 1, nodes);
            best = best.max(v);
        }
        p.unmake();
        let _ = me;
    }
    best
}

/// f(m) the theoretical way of use: the pair in full, then the forceful variations to quiet, both sides choosing.
fn use_f(p: &mut Position, m: u32) -> i64 {
    p.make(m);
    let replies = legal_moves(p);
    let v = if replies.is_empty() {
        if p.in_check() { INF } else { 0 }
    } else if p.insufficient() {
        0
    } else {
        let mut worst = INF;
        let mut nodes = 0;
        for r in replies {
            p.make(r);
            let v = quiet(p, FORCING_PLIES, 0, &mut nodes);
            p.unmake();
            worst = worst.min(v);
        }
        worst
    };
    p.unmake();
    v
}

/// Every legal move by the theoretical way of use, best first.
pub fn rank_use(fen: &str) -> Option<Vec<(String, i64)>> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut rows: Vec<(String, i64)> = legal_moves(&mut p).into_iter().map(|m| (move_to_uci(m), use_f(&mut p, m))).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    Some(rows)
}

fn use_holds(pos: &Pos) -> bool {
    let Ok(mut p) = Position::from_fen(&pos.fen) else { return false };
    let fs: Vec<i64> = pos.moves.iter().map(|(u, _, _)| uci_to_move(&mut p, u).map_or(-INF, |m| use_f(&mut p, m))).collect();
    let top = *fs.iter().max().unwrap();
    pos.moves.iter().zip(&fs).all(|(m, &f)| f != top || m.1)
}

/// Golden Answer's answer to USE_QUESTION.
pub fn answer_use(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if kind_logical {
        return Err("not decided; what the condition forces:\n  - the value of P under perfect play is, by definition, the line followed to the end of the game with both sides choosing their best (every pair, every move); used that way any p that is right at the ends keeps the value\n  - how short a line suffices with this p, and which moves may be left out, is not forced by the rules: undecided".into());
    }
    if kind_abstract {
        let g = guess()?;
        let mut lines = g.reasons.clone();
        lines.push("this is an answer that claims nothing".into());
        return Ok((format!("probably {}", g.use_text), lines, vec![]));
    }
    let check_set = load_set(CHECK_SET)?;
    let ok = check_set.iter().filter(|p| use_holds(p)).count();
    Ok((
        format!("Shannon's type B: the pair (m, r) in full, then only the forceful variations — captures, promotions, checks (the first {CHECK_PLIES} plies), every reply in check — up to {FORCING_PLIES} plies, each side free to stand; p is read where the line is quiet; each side chooses its best on the line (minimax)"),
        vec![
            "how many pairs: one in full; past it only forceful moves, until quiet (Shannon: \"examine forceful variations out as far as possible and evaluate only at reasonable positions, where some quasi-stability has been established\")".into(),
            "which moves: every move and reply in the pair; then captures, promotions, checks, and every move out of check".into(),
            "where the value is read: at the quiet ends, by p; up the line each side takes its best (Shannon's minimax)".into(),
            "Shannon's type A (every move to a fixed depth) is the other known way; he names it weak, since the depth cannot go far".into(),
            "claim: the way of use is Shannon's; that it keeps the value is not known past five pieces".into(),
            format!("for information: on {CHECK_SET} this keeps the value on {ok}/{} positions (Shannon's p alone, no tables)", check_set.len()),
        ],
        vec!["C. E. Shannon, Programming a Computer for Playing Chess, Philosophical Magazine 41 (1950): minimax; strategies of type A and B".into()],
    ))
}

// ---------------------------------------------------------------- the abstract answer

/// The p's Golden has: the one its logical answer searched, and Shannon's from its theoretical answer.
#[derive(Clone, Copy)]
enum Pv {
    Searched([i64; NX]),
    Shannon,
}

/// p on a position that is not over, from the side to move.
fn judge(p: &mut Position, pv: Pv) -> i64 {
    match pv {
        Pv::Shannon => shannon(p),
        Pv::Searched(w) => value(&w, &read_q(p)),
    }
}

fn quiet_g(p: &mut Position, pv: Pv, plies: usize, depth: usize, nodes: &mut u64) -> i64 {
    *nodes += 1;
    let check = p.in_check();
    let all = legal_moves(p);
    if all.is_empty() {
        return if check { -INF } else { 0 };
    }
    if p.insufficient() || p.halfmove >= 100 {
        return 0;
    }
    if plies == 0 || *nodes > NODES {
        return judge(p, pv);
    }
    let mut best = if check { -INF } else { judge(p, pv) };
    for m in all {
        let capture = p.board[m_to(m) as usize] != 0 || crate::fast::position::m_flag(m) == 2;
        let promo = crate::fast::position::m_promo(m) != 0;
        p.make(m);
        if check || capture || promo || (depth < CHECK_PLIES && p.in_check()) {
            best = best.max(-quiet_g(p, pv, plies - 1, depth + 1, nodes));
        }
        p.unmake();
    }
    best
}

/// f(m) with p = pv, used on one pair (type_b false) or Shannon's type B.
fn f_g(p: &mut Position, m: u32, pv: Pv, type_b: bool) -> i64 {
    p.make(m);
    let replies = legal_moves(p);
    let v = if replies.is_empty() {
        if p.in_check() { INF } else { 0 }
    } else if p.insufficient() {
        0
    } else {
        let mut worst = INF;
        let mut nodes = 0;
        for r in replies {
            p.make(r);
            let v = if type_b {
                quiet_g(p, pv, FORCING_PLIES, 0, &mut nodes)
            } else if legal_moves(p).is_empty() {
                if p.in_check() { -INF } else { 0 }
            } else if p.insufficient() || p.halfmove >= 100 {
                0
            } else {
                judge(p, pv)
            };
            p.unmake();
            worst = worst.min(v);
        }
        worst
    };
    p.unmake();
    v
}

fn holds_g(pos: &Pos, pv: Pv, type_b: bool) -> bool {
    let Ok(mut p) = Position::from_fen(&pos.fen) else { return false };
    let fs: Vec<i64> = pos.moves.iter().map(|(u, _, _)| uci_to_move(&mut p, u).map_or(-INF, |m| f_g(&mut p, m, pv, type_b))).collect();
    let top = *fs.iter().max().unwrap();
    pos.moves.iter().zip(&fs).all(|(m, &f)| f != top || m.1)
}

pub struct Guess {
    pub p_text: String,
    pub use_text: String,
    pub reasons: Vec<String>,
}

/// Golden's guess: of its own p's (the logical answer's searched one, the
/// theoretical answer's Shannon) and its two ways of use (one pair, type B),
/// the pairing that held most often where it has seen the truth (the search
/// set; the check set is not looked at). Claims nothing.
pub fn guess() -> Result<Guess, String> {
    let set = load_set(SEARCH_SET)?;
    let searched = load().ok_or("the logical answer has not searched its p yet: ask logical first")?;
    let cands = [
        (Pv::Searched(searched), written(&searched), "the logical answer's searched p"),
        (Pv::Shannon, "Shannon's function (1950)".to_string(), "the theoretical answer's p"),
    ];
    let mut rows = Vec::new();
    std::thread::scope(|sc| {
        let hs: Vec<_> = cands
            .iter()
            .flat_map(|c| [false, true].map(|b| (c.clone(), b)))
            .map(|((pv, text, from), b)| {
                let set = &set;
                sc.spawn(move || (set.iter().filter(|x| holds_g(x, pv, b)).count(), text, from, b))
            })
            .collect();
        for h in hs {
            rows.push(h.join().unwrap());
        }
    });
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    let use_name = |b: bool| if b { "used Shannon's type B way (the pair, then the forceful moves to quiet)" } else { "used on one pair (m, r)" };
    let (n, text, from, b) = rows[0].clone();
    let mut reasons = vec![format!("Golden has two p's of its own ({}, {}) and two ways to use them; where it has seen the truth ({} positions of {SEARCH_SET}), this one held most often: {n}", cands[0].2, cands[1].2, set.len())];
    for (n2, t2, _, b2) in &rows[1..] {
        reasons.push(format!("  {t2}, {}: {n2}", use_name(*b2)));
    }
    reasons.push(format!("so the guess is that {from}, {}, keeps the value more often than the others also where the truth is not known", use_name(b)));
    // the range: how often it keeps the value where the truth is not seen (Wilson, 95%)
    let (lo, hi) = wilson(n, set.len());
    reasons.push(format!("range: probably keeps the value in {:.0}% to {:.0}% of such positions (it held on {n} of {}; Wilson's 95% interval)", 100.0 * lo, 100.0 * hi, set.len()));
    // the region: each weight of the searched p moved alone, the whole numbers where it holds as often
    if let (Pv::Searched(w), false) = (cands.iter().find(|c| c.1 == text).map(|c| c.0).unwrap_or(Pv::Shannon), b) {
        let best = count(&w, &set);
        let mut parts = Vec::new();
        for i in 0..NX {
            let ok = |d: i64| {
                let mut v = w;
                v[i] = d;
                count(&v, &set) >= best
            };
            let (mut a, mut z) = (w[i], w[i]);
            while a > w[i] - 50 && ok(a - 1) {
                a -= 1;
            }
            while z < w[i] + 50 && ok(z + 1) {
                z += 1;
            }
            let show = |x: i64, edge: i64| if x == edge { "…".to_string() } else { x.to_string() };
            parts.push(format!("{} in [{}, {}]", NAMES[i], show(a, w[i] - 50), show(z, w[i] + 50)));
        }
        reasons.push(format!("region: each weight moved alone, it holds as often for {} (… = no edge within 50)", parts.join(", ")));
    }
    let text = text.trim_start_matches("p(m, r) = ").to_string();
    Ok(Guess { p_text: format!("p(m, r) = {text}"), use_text: format!("p {}", use_name(b)).replacen("p used", "p is used", 1), reasons })
}

/// Wilson's 95% score interval for k successes in n.
fn wilson(k: usize, n: usize) -> (f64, f64) {
    let (k, n, z) = (k as f64, n.max(1) as f64, 1.96f64);
    let ph = k / n;
    let c = (ph + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let h = z * (ph * (1.0 - ph) / n + z * z / (4.0 * n * n)).sqrt() / (1.0 + z * z / n);
    ((c - h).max(0.0), (c + h).min(1.0))
}
