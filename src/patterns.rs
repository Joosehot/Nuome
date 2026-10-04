//! The discovering genius: Golden Boy finds, in Stockfish's choices, the
//! patterns Stockfish plays again and again, and plays by them. Nothing is
//! written for it: it is given the positions (Stockfish's best move known,
//! depth 20), the rules' facts about every legal move (what the move does
//! to the network: captures, checks, the squares struck, how the move
//! counts change, whether the piece lands where it can be taken, ...), and
//! it composes conditions from those facts.
//!
//! A pattern is a condition (one fact, or two together) that holds for the
//! move Stockfish chose far more often than for the other legal moves in
//! the same positions - a lift - and recurs across thousands of positions.
//! Found on the first positions, tested on unseen ones; kept only when the
//! lift survives there. Every kept pattern is written in words with its numbers.
//!
//! Golden Boy's move: among the moves the exact calculation does not
//! refute, the one carrying the strongest surviving patterns (the sum of
//! their log-lifts). No eval.

use crate::golden::{atoms, Board, Mv, ATOMS};

/// The facts of a move, each a yes/no the rules decide.
pub const FACTS: [&str; 44] = [
    "pawn moves", "knight moves", "bishop moves", "rook moves", "queen moves", "king moves",
    "captures", "promotes", "gives check", "castles",
    "lands where they strike", "lands where they do not strike", "lands guarded by me", "lands unguarded",
    "leaves a square they strike", "lands on a square their pawn strikes",
    "to the centre", "to the extended centre", "to the edge",
    "forward", "backward",
    "my moves increase", "my moves decrease", "their moves increase", "their moves decrease",
    "their king's escapes decrease", "their king's escapes increase", "my king's escapes decrease",
    "i strike more of theirs", "they strike fewer of mine", "they strike more of mine",
    "my captures increase", "their captures increase", "their captures decrease",
    "takes an unstruck piece", "takes a cheaper piece", "takes a dearer or equal piece",
    "rook to a file without my pawns", "minor off the back rank", "king move with pieces on",
    "pawn to the 6th or 7th", "passed pawn advance", "a piece to where their pawn can step and strike", "lands where their cheaper piece strikes",
];
pub const N_FACTS: usize = FACTS.len();

fn worth(t: i8) -> i64 {
    match t.abs() {
        1 => 100,
        2 | 3 => 325,
        4 => 500,
        5 => 975,
        _ => 0,
    }
}

/// The facts of every legal move of `b`, as bit masks (bit i = FACTS[i]).
pub fn facts_of(b: &Board) -> Vec<(Mv, u64)> {
    let ms = b.moves();
    let before = atoms(b, &ms, u32::MAX);
    let e_before = crate::golden::attack_edges(b);
    let (me, them) = if b.white { (0usize, 1usize) } else { (1, 0) };
    let pieces_on = b.sq.iter().filter(|&&p| p != 0 && p.abs() != 6).count();
    let my_pawn_files: Vec<usize> = (0..64).filter(|&n| b.sq[n] == if b.white { 1 } else { -1 }).map(|n| n % 8).collect();
    let idx = |name: &str| FACTS.iter().position(|f| *f == name).expect("fact") as u64;
    let their_pawn = if b.white { -1 } else { 1 };
    let dir: i32 = if b.white { 1 } else { -1 };
    // the cheapest piece of theirs striking each node
    let mut cheapest = [i64::MAX; 64];
    for n in 0..64 {
        let p = b.sq[n];
        if p != 0 && ((p < 0) == b.white) {
            let mut s = crate::golden::strikes_of(b, n);
            while s != 0 {
                let t = s.trailing_zeros() as usize;
                s &= s - 1;
                cheapest[t] = cheapest[t].min(worth(p).max(1));
            }
        }
    }
    let mut out = Vec::with_capacity(ms.len());
    for &m in &ms {
        let mut f = 0u64;
        let set = |f: &mut u64, name: &str| *f |= 1 << idx(name);
        let p = b.sq[m.from as usize];
        let t = p.abs();
        set(&mut f, ["", "pawn moves", "knight moves", "bishop moves", "rook moves", "queen moves", "king moves"][t as usize]);
        let victim = b.sq[m.to as usize];
        let ep = t == 1 && Some(m.to) == b.ep;
        if victim != 0 || ep {
            set(&mut f, "captures");
            let vw = if ep { 100 } else { worth(victim) };
            if e_before[them][m.to as usize] == 0 {
                set(&mut f, "takes an unstruck piece");
            }
            if vw < worth(p) {
                set(&mut f, "takes a cheaper piece");
            } else {
                set(&mut f, "takes a dearer or equal piece");
            }
        }
        if m.promo != 0 {
            set(&mut f, "promotes");
        }
        if t == 6 && (m.to as i32 - m.from as i32).abs() == 2 {
            set(&mut f, "castles");
        }
        let a = b.play(m);
        if a.in_check() {
            set(&mut f, "gives check");
        }
        let e_after = crate::golden::attack_edges(&a);
        let to = m.to as usize;
        set(&mut f, if e_after[them][to] > 0 { "lands where they strike" } else { "lands where they do not strike" });
        set(&mut f, if e_after[me][to] > 0 { "lands guarded by me" } else { "lands unguarded" });
        if e_before[them][m.from as usize] > 0 {
            set(&mut f, "leaves a square they strike");
        }
        if cheapest[to] < worth(p) && t != 6 {
            set(&mut f, "lands where their cheaper piece strikes");
        }
        let (tf, tr) = ((to % 8) as i32, (to / 8) as i32);
        let pawn_at = |x: i32, y: i32| (0..8).contains(&x) && (0..8).contains(&y) && b.sq[(y * 8 + x) as usize] == their_pawn;
        if pawn_at(tf - 1, tr + dir) || pawn_at(tf + 1, tr + dir) {
            set(&mut f, "lands on a square their pawn strikes");
        }
        if t != 1 && (pawn_at(tf - 1, tr + 2 * dir) || pawn_at(tf + 1, tr + 2 * dir)) && (0..8).contains(&(tr + dir)) && b.sq[((tr + dir) * 8 + tf) as usize] == 0 {
            set(&mut f, "a piece to where their pawn can step and strike");
        }
        let ring = (tf - 3).max(4 - tf - 1).max((tr - 3).max(4 - tr - 1));
        match ring {
            0 => set(&mut f, "to the centre"),
            1 => set(&mut f, "to the extended centre"),
            3 => set(&mut f, "to the edge"),
            _ => {}
        }
        let fr = (m.from / 8) as i32;
        if (tr - fr) * dir > 0 {
            set(&mut f, "forward");
        } else if (tr - fr) * dir < 0 {
            set(&mut f, "backward");
        }
        // the counts after, from the mover's side: `a` is seen by the other side, so the names swap
        let ams = a.moves();
        let after = atoms(&a, &ams, u32::MAX);
        let ai = |name: &str| ATOMS.iter().position(|x| *x == name).expect("atom");
        let my_moves_after = after[ai("their_moves")];
        let their_moves_after = after[ai("my_moves")];
        if my_moves_after > before[ai("my_moves")] {
            set(&mut f, "my moves increase");
        } else if my_moves_after < before[ai("my_moves")] {
            set(&mut f, "my moves decrease");
        }
        if their_moves_after > before[ai("their_moves")] {
            set(&mut f, "their moves increase");
        } else if their_moves_after < before[ai("their_moves")] {
            set(&mut f, "their moves decrease");
        }
        let their_esc_after = after[ai("my_escapes")];
        if their_esc_after < before[ai("their_escapes")] {
            set(&mut f, "their king's escapes decrease");
        } else if their_esc_after > before[ai("their_escapes")] {
            set(&mut f, "their king's escapes increase");
        }
        if after[ai("their_escapes")] < before[ai("my_escapes")] {
            set(&mut f, "my king's escapes decrease");
        }
        if after[ai("they_attack")] > before[ai("i_attack")] {
            set(&mut f, "i strike more of theirs");
        }
        if after[ai("i_attack")] < before[ai("they_attack")] {
            set(&mut f, "they strike fewer of mine");
        } else if after[ai("i_attack")] > before[ai("they_attack")] {
            set(&mut f, "they strike more of mine");
        }
        if after[ai("their_captures")] > before[ai("my_captures")] {
            set(&mut f, "my captures increase");
        }
        if after[ai("my_captures")] > before[ai("their_captures")] {
            set(&mut f, "their captures increase");
        } else if after[ai("my_captures")] < before[ai("their_captures")] {
            set(&mut f, "their captures decrease");
        }
        if t == 4 && !my_pawn_files.contains(&(to % 8)) {
            set(&mut f, "rook to a file without my pawns");
        }
        let home = if b.white { 0 } else { 7 };
        if (t == 2 || t == 3) && (m.from / 8) as i32 == home && pieces_on >= 10 {
            set(&mut f, "minor off the back rank");
        }
        if t == 6 && pieces_on >= 10 && (m.to as i32 - m.from as i32).abs() != 2 {
            set(&mut f, "king move with pieces on");
        }
        if t == 1 {
            let rel = if b.white { tr } else { 7 - tr };
            if rel >= 5 {
                set(&mut f, "pawn to the 6th or 7th");
            }
            let passed = !(0..64).any(|n| b.sq[n] == their_pawn && ((n % 8) as i32 - tf).abs() <= 1 && ((n / 8) as i32 - tr) * dir > 0);
            if passed {
                set(&mut f, "passed pawn advance");
            }
        }
        out.push((m, f));
    }
    out
}

/// A pattern: one fact, or two together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pattern(pub u64);

impl Pattern {
    pub fn holds(self, facts: u64) -> bool {
        facts & self.0 == self.0
    }
    pub fn words(self) -> String {
        (0..N_FACTS).filter(|&i| self.0 >> i & 1 == 1).map(|i| FACTS[i]).collect::<Vec<_>>().join(" and ")
    }
}

pub struct Found {
    pub pattern: Pattern,
    pub lift_a: f64,
    pub lift_b: f64,
    /// positions where Stockfish's move carried it (first set)
    pub support: usize,
}

/// One position: the facts of every legal move, and which one Stockfish chose.
struct Case {
    facts: Vec<u64>,
    chosen: usize,
}

/// Read Stockfish's positions (one JSON object a line: "fen", "best_moves": [{"move": uci, ...}]).
fn read_positions(path: &str, skip: usize, take: usize) -> Result<Vec<Case>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let lines: Vec<&str> = text.lines().skip(skip).take(take).collect();
    // the facts of every move of every position, on every core
    let threads = 12;
    let chunk = lines.len().div_ceil(threads).max(1);
    let parts: Vec<Vec<Case>> = std::thread::scope(|sc| {
        let hs: Vec<_> = lines
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for line in part {
                        let fen = line.split("\"fen\": \"").nth(1).and_then(|s| s.split('"').next());
                        let best = line.split("\"move\": \"").nth(1).and_then(|s| s.split('"').next());
                        let (Some(fen), Some(best)) = (fen, best) else { continue };
                        let Ok(b) = Board::from_fen(fen) else { continue };
                        let fs = facts_of(&b);
                        if fs.len() < 2 {
                            continue;
                        }
                        let Some(ci) = fs.iter().position(|(m, _)| m.uci() == best) else { continue };
                        out.push(Case { facts: fs.iter().map(|x| x.1).collect(), chosen: ci });
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("facts")).collect()
    });
    Ok(parts.into_iter().flatten().collect())
}

fn lift(cases: &[Case], p: Pattern) -> (f64, usize) {
    let (mut chosen_hits, mut all_hits, mut all) = (0usize, 0usize, 0usize);
    for c in cases {
        for (i, &f) in c.facts.iter().enumerate() {
            all += 1;
            if p.holds(f) {
                all_hits += 1;
                if i == c.chosen {
                    chosen_hits += 1;
                }
            }
        }
    }
    let p_chosen = chosen_hits as f64 / cases.len().max(1) as f64;
    let p_any = all_hits as f64 / all.max(1) as f64;
    (if p_any > 0.0 { p_chosen / p_any } else { 0.0 }, chosen_hits)
}

/// Discover: every single fact and every pair, lifts on the first positions,
/// kept when the lift survives on the unseen ones.
pub fn discover(path: &str, n: usize, log: &mut dyn FnMut(&str)) -> Result<Vec<Found>, String> {
    let t0 = std::time::Instant::now();
    let a = read_positions(path, 0, n)?;
    let b = read_positions(path, n, n)?;
    log(&format!("{} positions to discover on, {} unseen ({:.0} s to read the facts of every move)", a.len(), b.len(), t0.elapsed().as_secs_f64()));
    let mut candidates: Vec<Pattern> = (0..N_FACTS).map(|i| Pattern(1 << i)).collect();
    for i in 0..N_FACTS {
        for j in i + 1..N_FACTS {
            candidates.push(Pattern(1 << i | 1 << j));
        }
    }
    let min_support = (a.len() / 100).max(30);
    let chunk = candidates.len().div_ceil(12).max(1);
    let mut found: Vec<Found> = std::thread::scope(|sc| {
        let hs: Vec<_> = candidates
            .chunks(chunk)
            .map(|part| {
                let (a, b) = (&a, &b);
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for &p in part {
                        let (la, support) = lift(a, p);
                        if support < min_support || la < 1.5 {
                            continue;
                        }
                        let (lb, _) = lift(b, p);
                        if lb >= 1.3 {
                            out.push(Found { pattern: p, lift_a: la, lift_b: lb, support });
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("lifts")).collect()
    });
    found.sort_by(|x, y| y.lift_b.partial_cmp(&x.lift_b).unwrap());
    // a pair stays only when it beats both of its parts on the unseen positions
    let singles: std::collections::HashMap<u64, f64> = found.iter().filter(|f| f.pattern.0.count_ones() == 1).map(|f| (f.pattern.0, f.lift_b)).collect();
    found.retain(|f| {
        if f.pattern.0.count_ones() == 1 {
            return true;
        }
        (0..N_FACTS).map(|i| 1u64 << i).filter(|bit| f.pattern.0 & bit != 0).all(|p| singles.get(&p).map_or(true, |&l| f.lift_b > l * 1.1))
    });
    log(&format!("{} patterns survive ({:.0} s)", found.len(), t0.elapsed().as_secs_f64()));
    Ok(found)
}

pub const PATTERNS_FILE: &str = "out/golden/patterns.txt";

pub fn save(found: &[Found]) -> Result<(), String> {
    let text: String = found.iter().map(|f| format!("{}\t{:.3}\t{:.3}\t{}\t{}\n", f.pattern.0, f.lift_a, f.lift_b, f.support, f.pattern.words())).collect();
    std::fs::write(PATTERNS_FILE, text).map_err(|e| e.to_string())
}

pub fn load() -> Vec<(Pattern, f64)> {
    std::fs::read_to_string(PATTERNS_FILE)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut it = l.split('\t');
            let bits: u64 = it.next()?.parse().ok()?;
            let _la: f64 = it.next()?.parse().ok()?;
            let lb: f64 = it.next()?.parse().ok()?;
            Some((Pattern(bits), lb))
        })
        .collect()
}

/// Golden Boy's move by the patterns: a forced mate first (the exact
/// calculation); moves proved lost are left alone; among the rest the one
/// whose matching patterns weigh most (the sum of unseen log-lifts); ties
/// stay in the rules' order.
pub fn play(b: &Board, history: &[u64], patterns: &[(Pattern, f64)]) -> Option<(Mv, f64, Vec<(Mv, f64, Vec<Pattern>)>)> {
    let (_, _, rows, _) = crate::neuro::supergenius(b, history, 100_000)?;
    if let Some((m, _, _)) = rows.iter().find(|r| matches!(r.1, crate::neuro::Proof::Win(_))) {
        return Some((*m, f64::INFINITY, vec![]));
    }
    let losing: Vec<Mv> = rows.iter().filter(|r| matches!(r.1, crate::neuro::Proof::Loss(_))).map(|r| r.0).collect();
    let fs = facts_of(b);
    let mut scored: Vec<(Mv, f64, Vec<Pattern>)> = fs
        .iter()
        .filter(|(m, _)| !losing.contains(m) || losing.len() == fs.len())
        .map(|&(m, f)| {
            let hits: Vec<Pattern> = patterns.iter().filter(|(p, _)| p.holds(f)).map(|x| x.0).collect();
            let w: f64 = patterns.iter().filter(|(p, _)| p.holds(f)).map(|(_, l)| l.ln()).sum();
            (m, w, hits)
        })
        .collect();
    if scored.is_empty() {
        return None;
    }
    scored.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    Some((scored[0].0, scored[0].1, scored))
}

pub fn report(path: &str, n: usize) -> Result<String, String> {
    let mut out = String::from("Golden Boy, the discovering genius, reads Stockfish's choices\n");
    let found = discover(path, n, &mut |l| out.push_str(&format!("  {l}\n")))?;
    save(&found)?;
    out.push_str(&format!("\n{} patterns recur and survive the unseen positions (lift >= 1.5 found, >= 1.3 unseen):\n", found.len()));
    for f in found.iter().take(40) {
        out.push_str(&format!("  x{:.2} (unseen x{:.2}, {} times): {}\n", f.lift_a, f.lift_b, f.support, f.pattern.words()));
    }
    if found.len() > 40 {
        out.push_str(&format!("  ... and {} more, all in {PATTERNS_FILE}\n", found.len() - 40));
    }
    Ok(out)
}
