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
pub const FACTS: [&str; 68] = [
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
    "pins a piece of theirs", "strikes two of their pieces", "check and strikes another piece",
    "king nearer my passed pawn", "king into the square of their passed pawn", "takes the opposition",
    "rook behind a passed pawn", "rook check from afar", "rook on the file next to their king",
    "guards a piece of mine that hung", "moves a struck piece to safety", "develops toward the centre",
    "blocks a line to my king", "attacks a pinned piece",
    "lands where only their pinned pieces strike", "skewers two of their pieces", "pawn break: steps to strike their pawn",
    "king strikes their pawn", "pawn outruns their king", "creates a passed pawn",
    "wins material by exchange", "loses material by exchange", "strikes a dearer piece", "check and strikes a dearer piece",
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

/// The pieces of `white` pinned to their king (a slider of the other side
/// behind exactly one piece on a line to the king), as a bitboard.
fn pinned(b: &Board, white: bool) -> u64 {
    let k = b.king(white);
    if k < 0 {
        return 0;
    }
    let (kf, kr) = (k % 8, k / 8);
    let mut out = 0u64;
    for (dirs, kinds) in [([(1, 0), (-1, 0), (0, 1), (0, -1)], [4i8, 5]), ([(1, 1), (1, -1), (-1, 1), (-1, -1)], [3i8, 5])] {
        for (df, dr) in dirs {
            let (mut x, mut y, mut own, mut own_n) = (kf + df, kr + dr, 0, 0usize);
            while (0..8).contains(&x) && (0..8).contains(&y) {
                let n = (y * 8 + x) as usize;
                let p = b.sq[n];
                if p != 0 {
                    if (p > 0) == white {
                        own += 1;
                        own_n = n;
                        if own > 1 {
                            break;
                        }
                    } else {
                        if own == 1 && kinds.contains(&p.abs()) {
                            out |= 1 << own_n;
                        }
                        break;
                    }
                }
                x += df;
                y += dr;
            }
        }
    }
    out
}

/// What the side to move gains by capturing on `sq` and letting the
/// exchange run, cheapest capturer first, every capture a legal move
/// (each side may stop when going on loses).
fn exchange_gain(b: &Board, sq: usize, depth: u32) -> i64 {
    if b.sq[sq] == 0 || depth == 0 {
        return 0;
    }
    let order = |p: i8| if p.abs() == 6 { 10_000 } else { worth(p) };
    let Some(m) = b.moves().into_iter().filter(|m| m.to as usize == sq).min_by_key(|m| order(b.sq[m.from as usize])) else { return 0 };
    (worth(b.sq[sq]) - exchange_gain(&b.play(m), sq, depth - 1)).max(0)
}

fn cheb(a: usize, b: usize) -> i32 {
    ((a % 8) as i32 - (b % 8) as i32).abs().max(((a / 8) as i32 - (b / 8) as i32).abs())
}

/// My passed pawns (nodes) and their queening nodes, for `white`.
fn passed_pawns(b: &Board, white: bool) -> Vec<(usize, usize)> {
    let (mine, theirs) = if white { (1i8, -1i8) } else { (-1, 1) };
    let mut out = Vec::new();
    for n in 0..64 {
        if b.sq[n] != mine {
            continue;
        }
        let (f, r) = ((n % 8) as i32, (n / 8) as i32);
        let dir = if white { 1 } else { -1 };
        let passed = !(0..64).any(|m| b.sq[m] == theirs && ((m % 8) as i32 - f).abs() <= 1 && ((m / 8) as i32 - r) * dir > 0);
        if passed {
            out.push((n, if white { 56 + f as usize } else { f as usize }));
        }
    }
    out
}

/// The facts of every legal move of `b`, as bit masks (bit i = FACTS[i]).
pub fn facts_of(b: &Board) -> Vec<(Mv, u128)> {
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
    let their_pinned_before = pinned(b, !b.white).count_ones();
    let my_king = b.king(b.white).max(0) as usize;
    let their_king = b.king(!b.white).max(0) as usize;
    let my_passed = passed_pawns(b, b.white);
    let their_passed = passed_pawns(b, !b.white);
    // my pieces struck by them and unguarded by me, before
    let hung_before: Vec<usize> = (0..64).filter(|&n| b.sq[n] != 0 && ((b.sq[n] > 0) == b.white) && b.sq[n].abs() != 6 && e_before[them][n] > 0 && e_before[me][n] == 0).collect();
    let mut out = Vec::with_capacity(ms.len());
    for &m in &ms {
        let mut f = 0u128;
        let set = |f: &mut u128, name: &str| *f |= 1u128 << idx(name);
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
        // ── the themes' words ──
        // pins: a piece of theirs newly pinned to their king
        if pinned(&a, !b.white).count_ones() > their_pinned_before {
            set(&mut f, "pins a piece of theirs");
        }
        if t != 6 && t != 1 {
            let hits = crate::golden::strikes_of(&a, to);
            let targets = (0..64).filter(|&n| hits >> n & 1 == 1 && a.sq[n] != 0 && ((a.sq[n] > 0) != b.white) && a.sq[n].abs() != 1).count();
            if targets >= 2 {
                set(&mut f, "strikes two of their pieces");
            }
            if a.in_check() && targets >= 2 {
                set(&mut f, "check and strikes another piece");
            }
            // attacks a pinned piece of theirs
            let pin_a = pinned(&a, !b.white);
            if hits & pin_a != 0 {
                set(&mut f, "attacks a pinned piece");
            }
        }
        if t == 6 {
            if let Some(&(pn, _)) = my_passed.iter().min_by_key(|(pn, _)| cheb(my_king, *pn)) {
                if cheb(to, pn) < cheb(my_king, pn) {
                    set(&mut f, "king nearer my passed pawn");
                }
            }
            for &(pn, qn) in &their_passed {
                let steps = cheb(pn, qn);
                if cheb(to, qn) <= steps && cheb(my_king, qn) > steps {
                    set(&mut f, "king into the square of their passed pawn");
                    break;
                }
            }
            // the opposition: kings on one file or rank, one node between, and they must move
            let (tf2, tr2) = (to % 8, to / 8);
            let (kf2, kr2) = (their_king % 8, their_king / 8);
            if (tf2 == kf2 && (tr2 as i32 - kr2 as i32).abs() == 2) || (tr2 == kr2 && (tf2 as i32 - kf2 as i32).abs() == 2) {
                set(&mut f, "takes the opposition");
            }
        }
        if t == 4 {
            // behind a passed pawn of either side on the same file
            let dir_w = |white: bool| if white { 1i32 } else { -1 };
            for &(pn, _) in my_passed.iter().chain(their_passed.iter()) {
                if pn % 8 == to % 8 {
                    let pawn_white = b.sq[pn] > 0 || a.sq[pn] > 0;
                    let behind = ((pn / 8) as i32 - (to / 8) as i32) * dir_w(pawn_white) > 0;
                    if behind {
                        set(&mut f, "rook behind a passed pawn");
                        break;
                    }
                }
            }
            if a.in_check() && cheb(to, their_king) >= 4 {
                set(&mut f, "rook check from afar");
            }
            if ((to % 8) as i32 - (their_king % 8) as i32).abs() == 1 {
                set(&mut f, "rook on the file next to their king");
            }
        }
        if !hung_before.is_empty() {
            let hung_after = hung_before.iter().filter(|&&n| n != m.from as usize && e_after[me][n] > 0).count();
            if hung_after > 0 {
                set(&mut f, "guards a piece of mine that hung");
            }
            if hung_before.contains(&(m.from as usize)) && e_after[them][to] == 0 {
                set(&mut f, "moves a struck piece to safety");
            }
        }
        if (t == 2 || t == 3) && (m.from / 8) as i32 == home && ring <= 1 {
            set(&mut f, "develops toward the centre");
        }
        // blocks a line to my king: a slider of theirs had my king in sight, and now the moved piece stands between
        if t != 6 {
            let my_pinned_after = pinned(&a, b.white);
            if my_pinned_after >> to & 1 == 1 {
                set(&mut f, "blocks a line to my king");
            }
        }
        // ── what the exchange on the landing node leaves, by the rules' captures ──
        let taken = if ep { 100 } else { worth(victim) };
        let balance = taken - exchange_gain(&a, to, 8);
        if balance > 0 {
            set(&mut f, "wins material by exchange");
        } else if balance < 0 {
            set(&mut f, "loses material by exchange");
        }
        if t != 6 {
            let hits = crate::golden::strikes_of(&a, to);
            let mine = worth(a.sq[to]);
            let dearer = (0..64).any(|n| hits >> n & 1 == 1 && a.sq[n] != 0 && ((a.sq[n] > 0) != b.white) && a.sq[n].abs() != 6 && worth(a.sq[n]) > mine);
            if dearer {
                set(&mut f, "strikes a dearer piece");
                if a.in_check() {
                    set(&mut f, "check and strikes a dearer piece");
                }
            }
        }
        // ── the clear words: pins that do not guard, skewers, pawn endings ──
        let pin_a = pinned(&a, !b.white);
        if e_after[them][to] > 0 {
            let free = (0..64).any(|n| a.sq[n] != 0 && ((a.sq[n] > 0) != b.white) && pin_a >> n & 1 == 0 && crate::golden::strikes_of(&a, n) >> to & 1 == 1);
            if !free {
                set(&mut f, "lands where only their pinned pieces strike");
            }
        }
        if (3..=5).contains(&t) {
            let dirs: &[(i32, i32)] = match t {
                3 => &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
                4 => &[(1, 0), (-1, 0), (0, 1), (0, -1)],
                _ => &[(1, 1), (1, -1), (-1, 1), (-1, -1), (1, 0), (-1, 0), (0, 1), (0, -1)],
            };
            let (tf, tr) = ((to % 8) as i32, (to / 8) as i32);
            'dirs: for &(df, dr) in dirs {
                let (mut x, mut y) = (tf + df, tr + dr);
                let mut first: i8 = 0;
                while (0..8).contains(&x) && (0..8).contains(&y) {
                    let q = a.sq[(y * 8 + x) as usize];
                    if q != 0 {
                        if (q > 0) == b.white {
                            continue 'dirs;
                        }
                        if first == 0 {
                            if q.abs() < 4 {
                                continue 'dirs;
                            }
                            first = q;
                        } else {
                            if q.abs() != 1 && (first.abs() == 6 || worth(first) > worth(q)) {
                                set(&mut f, "skewers two of their pieces");
                                break 'dirs;
                            }
                            continue 'dirs;
                        }
                    }
                    x += df;
                    y += dr;
                }
            }
        }
        if t == 1 && crate::golden::strikes_of(&a, to) & (0..64).filter(|&n| a.sq[n] == their_pawn).fold(0u64, |acc, n| acc | 1 << n) != 0 {
            set(&mut f, "pawn break: steps to strike their pawn");
        }
        if t == 6 && crate::golden::strikes_of(&a, to) & (0..64).filter(|&n| a.sq[n] == their_pawn).fold(0u64, |acc, n| acc | 1 << n) != 0 {
            set(&mut f, "king strikes their pawn");
        }
        let passed_a = passed_pawns(&a, b.white);
        if passed_a.len() > my_passed.len() {
            set(&mut f, "creates a passed pawn");
        }
        let their_pieces = (0..64).any(|n| a.sq[n] != 0 && ((a.sq[n] > 0) != b.white) && a.sq[n].abs() != 1 && a.sq[n].abs() != 6);
        if t == 1 && !their_pieces {
            if let Some(&(_, q)) = passed_a.iter().find(|(pn, _)| *pn == to) {
                let start = if b.white { 1 } else { 6 };
                let steps = cheb(to, q) - if to / 8 == start { 1 } else { 0 };
                let k = a.king(!b.white);
                if k >= 0 && cheb(k as usize, q) > steps {
                    set(&mut f, "pawn outruns their king");
                }
            }
        }
        out.push((m, f));
    }
    out
}

/// A pattern: one fact, or two together.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pattern(pub u128);

impl Pattern {
    pub fn holds(self, facts: u128) -> bool {
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
    facts: Vec<u128>,
    chosen: usize,
    phase: Phase,
}

/// The phase of a position, by the pieces on the board: each phase gets its own roster.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// kings and pawns only
    Pawns,
    /// at most four pieces besides kings and pawns
    Endgame,
    Middle,
}

pub const PHASES: [Phase; 3] = [Phase::Pawns, Phase::Endgame, Phase::Middle];

pub fn phase_of(b: &Board) -> Phase {
    let pieces = b.sq.iter().filter(|&&p| p != 0 && p.abs() != 1 && p.abs() != 6).count();
    match pieces {
        0 => Phase::Pawns,
        1..=4 => Phase::Endgame,
        _ => Phase::Middle,
    }
}

impl Phase {
    pub fn file(self) -> &'static str {
        match self {
            Phase::Pawns => "out/golden/patterns_pawns.txt",
            Phase::Endgame => "out/golden/patterns_endgame.txt",
            Phase::Middle => "out/golden/patterns_middle.txt",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Phase::Pawns => "pawn endings",
            Phase::Endgame => "endgames",
            Phase::Middle => "openings and middlegames",
        }
    }
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
                        out.push(Case { facts: fs.iter().map(|x| x.1).collect(), chosen: ci, phase: phase_of(&b) });
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
fn discover(a: &[Case], b: &[Case], log: &mut dyn FnMut(&str)) -> Vec<Found> {
    let t0 = std::time::Instant::now();
    let mut candidates: Vec<Pattern> = (0..N_FACTS).map(|i| Pattern(1u128 << i)).collect();
    for i in 0..N_FACTS {
        for j in i + 1..N_FACTS {
            candidates.push(Pattern(1u128 << i | 1u128 << j));
        }
    }
    // rare tactics count too (the unseen half still checks every one); a small phase scales it down
    let min_support = (a.len() / 1500).clamp(30, 200);
    let chunk = candidates.len().div_ceil(12).max(1);
    let mut found: Vec<Found> = std::thread::scope(|sc| {
        let hs: Vec<_> = candidates
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for &p in part {
                        let (la, support) = lift(a, p);
                        // the themes' words (facts 44 on) always stand in the roster as singles, however simple
                        let theme_single = p.0.count_ones() == 1 && p.0 >> 44 != 0 && support > 0;
                        if theme_single {
                            let (lb, _) = lift(b, p);
                            out.push(Found { pattern: p, lift_a: la, lift_b: lb, support });
                            continue;
                        }
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
    let singles: std::collections::HashMap<u128, f64> = found.iter().filter(|f| f.pattern.0.count_ones() == 1 && f.lift_b >= 1.3).map(|f| (f.pattern.0, f.lift_b)).collect();
    found.retain(|f| {
        if f.pattern.0.count_ones() == 1 {
            return true;
        }
        (0..N_FACTS).map(|i| 1u128 << i).filter(|bit| f.pattern.0 & bit != 0).all(|p| singles.get(&p).map_or(true, |&l| f.lift_b > l * 1.1))
    });
    log(&format!("{} patterns survive ({:.0} s)", found.len(), t0.elapsed().as_secs_f64()));
    found
}

pub const PATTERNS_FILE: &str = "out/golden/patterns.txt";

pub fn save(file: &str, found: &[Found]) -> Result<(), String> {
    let text: String = found.iter().map(|f| format!("{}\t{:.3}\t{:.3}\t{}\t{}\n", f.pattern.0, f.lift_a, f.lift_b, f.support, f.pattern.words())).collect();
    std::fs::write(file, text).map_err(|e| e.to_string())
}

/// Every roster: the whole one, and one a phase (empty when not discovered).
pub struct Rosters {
    pub all: Vec<(Pattern, f64)>,
    pub phase: Vec<(Phase, Vec<(Pattern, f64)>)>,
}

impl Rosters {
    /// The roster for this position: its phase's own when it has one.
    pub fn for_board(&self, b: &Board) -> &[(Pattern, f64)] {
        let ph = phase_of(b);
        match self.phase.iter().find(|(p, r)| *p == ph && !r.is_empty()) {
            Some((_, r)) => r,
            None => &self.all,
        }
    }
    pub fn len(&self) -> usize {
        self.all.len() + self.phase.iter().map(|(_, r)| r.len()).sum::<usize>()
    }
}

pub fn load() -> Rosters {
    Rosters { all: load_file(PATTERNS_FILE), phase: PHASES.iter().map(|&p| (p, load_file(p.file()))).collect() }
}

fn load_file(file: &str) -> Vec<(Pattern, f64)> {
    std::fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut it = l.split('\t');
            let bits: u128 = it.next()?.parse().ok()?;
            let _la: f64 = it.next()?.parse().ok()?;
            let lb: f64 = it.next()?.parse().ok()?;
            Some((Pattern(bits), lb))
        })
        .collect()
}

const MATE: i64 = 100_000;

/// The exact calculation of the forcing lines: the material (the heart's
/// Kaufman values) the side to move ends with when the attacker plays its
/// captures, promotions and checks and the defender its captures and
/// promotions (in check: every reply), each side free to stop when going on
/// would lose. Every line is followed to its end within `depth` plies (no
/// pruning); `nodes` bounds the whole calculation.
fn forced(b: &Board, depth: u32, attacker: bool, nodes: &mut usize) -> i64 {
    *nodes += 1;
    let ms = b.moves();
    let check = b.in_check();
    if ms.is_empty() {
        return if check { -MATE } else { 0 };
    }
    let stand = crate::supergenius_eval::material(b);
    if depth == 0 || *nodes > FORCED_NODES {
        return stand;
    }
    let mut best = if check { -MATE } else { stand };
    for m in ms {
        let capture = b.sq[m.to as usize] != 0 || (b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep);
        let a = b.play(m);
        let gives_check = attacker && a.in_check();
        if !(check || capture || m.promo != 0 || gives_check) {
            continue;
        }
        let v = -forced(&a, depth - 1, !attacker, nodes);
        if v > best {
            best = v;
        }
    }
    best
}

const FORCED_NODES: usize = 60_000;
const FORCED_DEPTH: u32 = 6;

/// Golden Boy's move by the patterns: a forced mate first (the exact
/// calculation); moves proved lost are left alone; among the rest the one
/// whose matching patterns weigh most (the sum of unseen log-lifts); ties
/// stay in the rules' order.
pub fn play(b: &Board, history: &[u64], rosters: &Rosters) -> Option<(Mv, f64, Vec<(Mv, f64, Vec<Pattern>)>)> {
    let (_, _, rows, _) = crate::neuro::supergenius(b, history, 100_000)?;
    if let Some((m, _, _)) = rows.iter().find(|r| matches!(r.1, crate::neuro::Proof::Win(_))) {
        return Some((*m, f64::INFINITY, vec![]));
    }
    let losing: Vec<Mv> = rows.iter().filter(|r| matches!(r.1, crate::neuro::Proof::Loss(_))).map(|r| r.0).collect();
    let mut fs = facts_of(b);
    fs.retain(|(m, _)| !losing.contains(m) || losing.len() == rows.len());
    // the forcing lines, calculated to their end: a move that wins material by
    // force leaves only its equals; a move that loses it by force is left out
    let reached: Vec<i64> = fs
        .iter()
        .map(|(m, _)| {
            let mut nodes = 0;
            -forced(&b.play(*m), FORCED_DEPTH - 1, false, &mut nodes)
        })
        .collect();
    if let Some(&top) = reached.iter().max() {
        let mut i = 0;
        fs.retain(|_| {
            i += 1;
            reached[i - 1] >= top - 90
        });
    }
    let patterns = rosters.for_board(b);
    let mut scored: Vec<(Mv, f64, Vec<Pattern>)> = fs
        .iter()
        .map(|&(m, f)| {
            let hits: Vec<Pattern> = patterns.iter().filter(|(p, _)| p.holds(f)).map(|x| x.0).collect();
            let w: f64 = patterns.iter().filter(|(p, _)| p.holds(f)).map(|(_, l)| l.ln()).fold(0.0, f64::max);
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
    let t0 = std::time::Instant::now();
    let a = read_positions(path, 0, n)?;
    let b = read_positions(path, n, n)?;
    out.push_str(&format!("  {} positions to discover on, {} unseen ({:.0} s to read the facts of every move)\n", a.len(), b.len(), t0.elapsed().as_secs_f64()));
    for ph in PHASES {
        let pick = |cs: &[Case]| -> Vec<Case> { cs.iter().filter(|c| c.phase == ph).map(|c| Case { facts: c.facts.clone(), chosen: c.chosen, phase: c.phase }).collect() };
        let (pa, pb) = (pick(&a), pick(&b));
        let found = discover(&pa, &pb, &mut |_| {});
        save(ph.file(), &found)?;
        out.push_str(&format!("\n{}: {} positions + {} unseen, {} patterns; the strongest:\n", ph.name(), pa.len(), pb.len(), found.len()));
        for f in found.iter().take(12) {
            out.push_str(&format!("  x{:.2} (unseen x{:.2}, {} times): {}\n", f.lift_a, f.lift_b, f.support, f.pattern.words()));
        }
    }
    out.push_str("\nall phases together:\n");
    let found = discover(&a, &b, &mut |l| out.push_str(&format!("  {l}\n")));
    save(PATTERNS_FILE, &found)?;
    out.push_str(&format!("\n{} patterns recur and survive the unseen positions (lift >= 1.5 found, >= 1.3 unseen):\n", found.len()));
    for f in found.iter().take(40) {
        out.push_str(&format!("  x{:.2} (unseen x{:.2}, {} times): {}\n", f.lift_a, f.lift_b, f.support, f.pattern.words()));
    }
    if found.len() > 40 {
        out.push_str(&format!("  ... and {} more, all in {PATTERNS_FILE}\n", found.len() - 40));
    }
    Ok(out)
}
