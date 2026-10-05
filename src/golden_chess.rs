//! Golden Answer's chess: "best move in <FEN>" in the kind the asker chooses.
//!
//! - logical: only what Nuome derives from the rules itself — checkmate and
//!   stalemate; the retrograde tables Nuome computes for three pieces
//!   (retro.rs); a forced mate found by exact search over every reply
//!   (neuro::supergenius). A move is called best only when that is proved.
//! - theoretical: the Syzygy tablebases (Ronald de Man), results computed by
//!   others and named: for up to five pieces, a move that keeps the
//!   position's win, draw or loss, probed through python-chess.
//! - abstract: Golden Boy's move by the patterns it discovered in
//!   Stockfish's choices; it claims nothing.

use crate::fast::bitboard::{SQ120_TO_64, SQ64_TO_120};
use crate::fast::movegen::{gen_moves, legal_moves, MAX_MOVES};
use crate::fast::position::{color_of, m_from, m_promo, m_to, move_to_uci, type_of, Position, EMPTY, OFF};
use crate::golden::Board;

pub const SYZYGY: &str = "C:/Users/joose/Tools/syzygy/345-wdl";

/// Read "best move in <FEN>" (and its other phrasings) whole; the FEN, if the question is that.
pub fn read(q: &str) -> Option<String> {
    let t = q.trim().trim_end_matches(['?', '.']).trim();
    let low = t.to_lowercase();
    const ASK: &[&str] = &[
        "what is the best move in ",
        "what is the best move for ",
        "find the best move in ",
        "best move in ",
        "best move for ",
        "solve the position ",
        "solve ",
        "mikä on paras siirto asemassa ",
        "paras siirto asemassa ",
        "paras siirto ",
        "ratkaise asema ",
    ];
    let p = ASK.iter().find(|p| low.starts_with(*p))?;
    let fen = t[p.len()..].trim().trim_matches(['"', '\'']).to_string();
    Board::from_fen(&fen).ok().map(|_| fen)
}

pub enum Kind3 {
    Logical,
    Theoretical,
    Abstract,
}

/// The answer: short (the move in UCI, or the verdict), lines, sources; None with a reason.
pub fn answer(kind: Kind3, fen: &str) -> Result<(String, Vec<String>, Vec<String>), String> {
    let b = Board::from_fen(fen).map_err(|e| format!("not a position: {e}"))?;
    let ms = b.moves();
    let side = if b.white { "white" } else { "black" };
    if ms.is_empty() {
        let v = if b.in_check() { format!("checkmate: {side} has no legal move and is in check") } else { format!("stalemate: {side} has no legal move and is not in check") };
        return match kind {
            Kind3::Logical => Ok(("no move".into(), vec![v], vec![])),
            _ => Err("the rules decide it: ask logical".into()),
        };
    }
    match kind {
        Kind3::Logical => {
            // the three-piece tables Nuome computes from the rules
            if let Some((m, v, rows)) = crate::retro::exact_move(&b) {
                let key = crate::retro::key_of(&crate::retro::pieces_of(&b));
                let mut lines = vec![format!("{key} is solved whole by Nuome's retrograde analysis from the rules (every position, every reply)")];
                lines.push(format!("{}: {}", m.uci(), crate::retro::show(v)));
                for (m2, v2) in rows.iter().skip(1).take(4) {
                    lines.push(format!("{}: {}", m2.uci(), crate::retro::show(*v2)));
                }
                return Ok((m.uci(), lines, vec![]));
            }
            // a forced mate, every reply answered, on the fast board (mate in 1, 2, 3)
            if let Ok(mut p) = Position::from_fen(fen) {
                let mut f = Fast { tt: Vec::new(), killers: Vec::new(), history: Vec::new(), nodes: 0 };
                for n in 1..=3 {
                    if let Some(m) = f.mate(&mut p, n, 300_000) {
                        let lines = vec![
                            format!("{} mates by force in {n} (an exact search answered every reply, {} positions)", move_to_uci(m), f.nodes),
                            "no shorter mate exists (mate in fewer moves was searched in full first)".to_string(),
                        ];
                        return Ok((move_to_uci(m), lines, vec![]));
                    }
                    if f.nodes > 300_000 {
                        break;
                    }
                }
            }
            // forced draws and losses: Nuome's own proof search, on small positions only (it is slow on full boards)
            let small = b.sq.iter().filter(|&&q| q != 0).count() <= 8;
            let (_, _, rows, nodes) = if small { crate::neuro::supergenius(&b, &[], 400_000).ok_or("no legal move")? } else { (ms[0], crate::neuro::Proof::Unknown, Vec::new(), 0) };
            use crate::neuro::Proof;
            let wins: Vec<(String, u32)> = rows.iter().filter_map(|(m, p, _)| if let Proof::Win(n) = p { Some((m.uci(), *n)) } else { None }).collect();
            if let Some((m, n)) = wins.iter().min_by_key(|w| w.1) {
                let lines = vec![
                    format!("{m} mates by force in {} (an exact search answered every reply, {nodes} positions)", (n + 1) / 2),
                    if wins.len() > 1 { format!("{} moves mate by force; {m} is the fastest found", wins.len()) } else { "it is the only move found that mates by force".into() },
                ];
                return Ok((m.clone(), lines, vec![]));
            }
            let losses = rows.iter().filter(|r| matches!(r.1, Proof::Loss(_))).count();
            let draws: Vec<String> = rows.iter().filter(|r| matches!(r.1, Proof::Draw)).map(|r| r.0.uci()).collect();
            if !draws.is_empty() && draws.len() + losses == rows.len() {
                return Ok((draws[0].clone(), vec![format!("{} holds a draw by force, and every other move is a forced draw or loss ({nodes} positions)", draws[0])], vec![]));
            }
            if !rows.is_empty() && losses == rows.len() {
                let slow = rows.iter().max_by_key(|r| if let Proof::Loss(n) = r.1 { n } else { 0 }).map(|r| r.0.uci()).unwrap_or_default();
                return Ok((slow.clone(), vec![format!("every move loses by force; {slow} lasts longest ({nodes} positions)")], vec![]));
            }
            if STRONG_ONLY.with(|c| c.get()) {
                return Err("no strong proof".into());
            }
            // 6-32 pieces: a proof with its scope said: the best move of a bounded game, every move searched
            let (rows, n, depth) = deepen(&b);
            let (m, v) = rows[0].clone();
            let second = rows.get(1).cloned();
            let who = if b.white { "white" } else { "black" };
            let mut lines = vec![
                format!("no forced mate and no solved class: what is proved is the bounded game (scope: {depth} plies of every legal move by both sides, then captures traded until quiet, each side free to stop; {}, mate ±100000)", leaf_words()),
                format!("in that game {m} gives {who} {v:+} centipawns, and no other move gives more ({n} positions, every reply searched)"),
            ];
            if let Some((m2, v2)) = second {
                lines.push(format!("the next best is {m2} at {v2:+}"));
            }
            lines.push("this proves the move best in the bounded game, not best in chess".into());
            Ok((m, lines, vec![]))
        }
        Kind3::Theoretical => {
            let pieces = b.sq.iter().filter(|&&p| p != 0).count();
            if pieces > 5 {
                return Err(format!("no known result covers it: the Syzygy tables here reach 5 pieces, the position has {pieces}"));
            }
            let rows = syzygy_rows(fen)?;
            let best = rows.iter().map(|r| r.1).max().ok_or("no moves probed")?;
            let keep: Vec<&String> = rows.iter().filter(|r| r.1 == best).map(|r| &r.0).collect();
            let what = match best {
                2 => "win",
                1 => "win (cursed: drawn by the 50-move rule with best defence)",
                0 => "draw",
                -1 => "loss (blessed: saved by the 50-move rule)",
                _ => "loss",
            };
            let lines = vec![
                format!("the position is a {what} for {side} with best play"),
                format!("moves that keep it: {}", keep.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")),
                "the tables give win, draw or loss only (WDL), not the fastest way".into(),
            ];
            Ok((keep[0].clone(), lines, vec![format!("Syzygy endgame tablebases, 3-5 pieces (Ronald de Man), probed with python-chess from {SYZYGY}")]))
        }
        Kind3::Abstract => {
            let rosters = crate::patterns::load();
            let (m, w, rows) = crate::patterns::play(&b, &[], &rosters).ok_or("no legal move")?;
            let mut lines = vec![format!("Golden Boy's move by the patterns it discovered in Stockfish's choices ({} patterns)", rosters.len())];
            if let Some((_, _, hits)) = rows.first() {
                if !hits.is_empty() {
                    lines.push(format!("because: {}", hits.iter().take(3).map(|p| p.words()).collect::<Vec<_>>().join("; ")));
                }
            }
            if w.is_infinite() {
                lines.push("its calculation found a forced mate first".into());
            }
            Ok((m.uci(), lines, vec![]))
        }
    }
}

const MATE: i64 = 100_000;

thread_local! {
    /// Only the strong proofs (mate, solved class, forced draw or loss): the bounded game is left to the caller.
    static STRONG_ONLY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// When the deepening must stop; a depth not finished in time is not used.
    static DEADLINE: std::cell::Cell<Option<std::time::Instant>> = const { std::cell::Cell::new(None) };
    static ABORTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The fast search's table, kept for the deepening of one question (cleared per question).
    static FAST: std::cell::Cell<Option<Fast>> = const { std::cell::Cell::new(None) };
}

/// The logical answer's strong proofs only.
fn strong(fen: &str) -> Option<(String, Vec<String>)> {
    STRONG_ONLY.with(|c| c.set(true));
    let r = answer(Kind3::Logical, fen);
    STRONG_ONLY.with(|c| c.set(false));
    r.ok().map(|(m, l, _)| (m, l))
}

/// The bounded game searched deeper and deeper (1, 2, ... up to GOLDEN_PROOF_PLIES, default 4)
/// while time lasts (GOLDEN_TIME_MS, default 2000); the deepest depth finished in full is the one used.
fn deepen(b: &Board) -> (Vec<(String, i64)>, u64, u32) {
    let max = std::env::var("GOLDEN_PROOF_PLIES").ok().and_then(|x| x.parse().ok()).unwrap_or(4);
    let ms = std::env::var("GOLDEN_TIME_MS").ok().and_then(|x| x.parse().ok()).unwrap_or(2000u64);
    let t0 = std::time::Instant::now();
    FAST.with(|c| c.set(None));
    let (mut rows, mut nodes) = bounded_rows(b, 1);
    let mut done = 1;
    for d in 2..=max {
        DEADLINE.with(|c| c.set(Some(t0 + std::time::Duration::from_millis(ms))));
        ABORTED.with(|c| c.set(false));
        let (r, n) = bounded_rows(b, d);
        DEADLINE.with(|c| c.set(None));
        nodes += n;
        if ABORTED.with(|c| c.get()) {
            break;
        }
        rows = r;
        done = d;
    }
    (rows, nodes, done)
}

/// The static eval at the bounded game's quiet leaves: Golden heart when it
/// has been cloned (GOLDEN_LEAF=material keeps the plain material), else material.
fn leaf(b: &Board) -> i64 {
    if std::env::var("GOLDEN_LEAF").map_or(true, |v| v != "material") {
        if let Some(w) = crate::golden_heart::frozen() {
            return crate::golden_heart::eval(w, b);
        }
    }
    crate::supergenius_eval::material(b)
}

/// What the leaves are scored by, for the logical answer's stated scope.
fn leaf_words() -> &'static str {
    if std::env::var("GOLDEN_LEAF").map_or(true, |v| v != "material") && crate::golden_heart::frozen().is_some() {
        "scored at the quiet leaves by Golden heart (the static eval cloned from Stockfish's games, out/golden/heart_weights.txt)"
    } else {
        "scored by material, Kaufman 100/325/325/500/975"
    }
}

/// The bounded game's value for the side to move: every move for `depth`
/// plies, then captures (and promotions) until quiet with standing pat;
/// material at the leaves. Alpha-beta cuts only lines that cannot change
/// the value, so the value is the full minimax value of this game.
fn game(b: &Board, depth: u32, mut alpha: i64, beta: i64, nodes: &mut u64) -> i64 {
    *nodes += 1;
    if *nodes % 1024 == 0 {
        if let Some(t) = DEADLINE.with(|c| c.get()) {
            if std::time::Instant::now() > t {
                ABORTED.with(|c| c.set(true));
            }
        }
    }
    if ABORTED.with(|c| c.get()) {
        return 0;
    }
    let ms = b.moves();
    if ms.is_empty() {
        return if b.in_check() { -MATE } else { 0 };
    }
    let quiet = depth == 0;
    if quiet {
        let stand = leaf(b);
        if stand >= beta {
            return stand;
        }
        alpha = alpha.max(stand);
    }
    // captures first: the order changes only the speed, never the value
    let mut order: Vec<(i64, crate::golden::Mv)> = ms
        .into_iter()
        .map(|m| {
            let victim = b.sq[m.to as usize].unsigned_abs() as i64;
            let ep = b.sq[m.from as usize].abs() == 1 && Some(m.to) == b.ep;
            ((victim + ep as i64) * 10 + (m.promo != 0) as i64 * 50, m)
        })
        .collect();
    if quiet {
        order.retain(|(score, _)| *score > 0);
    }
    order.sort_by_key(|o| std::cmp::Reverse(o.0));
    let mut best = if quiet { alpha } else { -MATE - 1 };
    for (_, m) in order {
        let v = -game(&b.play(m), depth.saturating_sub(1), -beta, -alpha.max(best), nodes);
        if v > best {
            best = v;
        }
        if best >= beta {
            break;
        }
    }
    best
}

/// Every root move's value in the bounded game (full window each, best first), and the positions searched.
/// The game is searched on Steldens' fast board (make/unmake); GOLDEN_SLOW_BOARD=1 uses Nuome's own board.
fn bounded_rows(b: &Board, depth: u32) -> (Vec<(String, i64)>, u64) {
    let mut nodes = 0u64;
    if std::env::var("GOLDEN_SLOW_BOARD").is_err() {
        if let Ok(mut p) = Position::from_fen(&b.fen()) {
            let mut f = FAST.with(|c| c.take()).unwrap_or_else(Fast::new);
            f.nodes = 0;
            let mut rows = Vec::new();
            for m in legal_moves(&mut p) {
                p.make(m);
                let v = -f.game(&mut p, depth.saturating_sub(1), 1, -MATE - 1, MATE + 1);
                p.unmake();
                rows.push((move_to_uci(m), v));
            }
            rows.sort_by_key(|r| std::cmp::Reverse(r.1));
            let n = f.nodes;
            FAST.with(|c| c.set(Some(f)));
            return (rows, n);
        }
    }
    let mut rows: Vec<(String, i64)> = b
        .moves()
        .into_iter()
        .map(|m| (m.uci(), -game(&b.play(m), depth.saturating_sub(1), -MATE - 1, MATE + 1, &mut nodes)))
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1));
    (rows, nodes)
}

/// Nuome's board from the fast one (for Golden heart at the leaves).
fn to_board(p: &Position) -> Board {
    let mut sq = [0i8; 64];
    for (s, x) in sq.iter_mut().enumerate() {
        let pc = p.board[SQ64_TO_120[s]];
        if pc != EMPTY && pc != OFF {
            let t = type_of(pc) as i8;
            *x = if color_of(pc) == 0 { t } else { -t };
        }
    }
    let ep = if p.ep >= 0 { let s = SQ120_TO_64[p.ep as usize]; (s >= 0).then_some(s as u8) } else { None };
    Board { sq, white: p.turn == 0, castle: p.castle, ep, half: p.halfmove, full: 1 }
}

/// Material for the side to move, as supergenius_eval::material counts it (Kaufman, bishop pair 50).
fn material_fast(p: &Position) -> i64 {
    const W: [i64; 6] = [100, 325, 325, 500, 975, 0];
    let mut v = [0i64; 2];
    for c in 0..2 {
        for t in 0..6 {
            v[c] += W[t] * p.bb[c * 6 + t].count_ones() as i64;
        }
        if p.bb[c * 6 + 2].count_ones() >= 2 {
            v[c] += 50;
        }
    }
    let me = p.turn as usize;
    v[me] - v[1 - me]
}

fn leaf_fast(p: &Position) -> i64 {
    if std::env::var("GOLDEN_LEAF").map_or(true, |v| v != "material") {
        if let Some(w) = crate::golden_heart::frozen() {
            return crate::golden_heart::eval_fast(w, p);
        }
    }
    material_fast(p)
}

/// The fast search's memory: a table of positions already valued (used only
/// for the same remaining depth, so the value stays the bounded game's), the
/// killer moves and the history (they change only the order of the moves,
/// never a value).
pub struct Fast {
    tt: Vec<(u64, u8, i64, u8, u32)>,
    killers: Vec<[u32; 2]>,
    history: Vec<u32>,
    pub nodes: u64,
}

const TT_BITS: u32 = 21;
const EXACT: u8 = 0;
const LOWER: u8 = 1;
const UPPER: u8 = 2;

impl Fast {
    pub fn new() -> Fast {
        Fast { tt: vec![(0, 0, 0, 0, 0); 1 << TT_BITS], killers: vec![[0; 2]; 64], history: vec![0; 120 * 120], nodes: 0 }
    }

    /// The same bounded game as `game`, on the fast board: every legal move for
    /// `depth` plies, then captures and promotions until quiet with standing pat.
    pub fn game(&mut self, p: &mut Position, depth: u32, ply: usize, mut alpha: i64, beta: i64) -> i64 {
        self.nodes += 1;
        if self.nodes % 1024 == 0 {
            if let Some(t) = DEADLINE.with(|c| c.get()) {
                if std::time::Instant::now() > t {
                    ABORTED.with(|c| c.set(true));
                }
            }
        }
        if ABORTED.with(|c| c.get()) {
            return 0;
        }
        let key = p.key64();
        let slot = (key as usize) & ((1 << TT_BITS) - 1);
        let (tk, td, tv, tf, tm) = self.tt[slot];
        let mut tt_move = 0;
        if tk == key && td == depth as u8 + 1 {
            tt_move = tm;
            if tf == EXACT || (tf == LOWER && tv >= beta) || (tf == UPPER && tv <= alpha) {
                return tv;
            }
        }
        let alpha0 = alpha;
        let quiet = depth == 0;
        let mut list = [0u32; MAX_MOVES];
        let n = gen_moves(p, &mut list, false);
        let legal_one = |p: &mut Position, m: u32| {
            p.make(m);
            let ok = !p.in_check_c(1 - p.turn);
            p.unmake();
            ok
        };
        let k = self.killers.get(ply).copied().unwrap_or([0; 2]);
        let order = |p: &Position, m: u32, h: &[u32]| -> i64 {
            let victim = type_of(p.board[m_to(m) as usize]) as i64;
            if m == tt_move {
                1 << 40
            } else if victim > 0 || m_promo(m) != 0 {
                (1 << 30) + victim * 10 + (m_promo(m) != 0) as i64 * 50
            } else if m == k[0] || m == k[1] {
                1 << 29
            } else {
                h[m_from(m) as usize * 120 + m_to(m) as usize] as i64
            }
        };
        let mut moves: Vec<(i64, u32)> = Vec::with_capacity(n);
        if quiet {
            if !list[..n].iter().any(|&m| legal_one(p, m)) {
                return if p.in_check() { -MATE } else { 0 };
            }
            let stand = leaf_fast(p);
            if stand >= beta {
                return stand;
            }
            alpha = alpha.max(stand);
            let c = gen_moves(p, &mut list, true);
            for &m in &list[..c] {
                if legal_one(p, m) {
                    moves.push((order(p, m, &self.history), m));
                }
            }
        } else {
            for &m in &list[..n] {
                if legal_one(p, m) {
                    moves.push((order(p, m, &self.history), m));
                }
            }
            if moves.is_empty() {
                return if p.in_check() { -MATE } else { 0 };
            }
        }
        moves.sort_by_key(|o| std::cmp::Reverse(o.0));
        let mut best = if quiet { alpha } else { -MATE - 1 };
        let mut best_move = 0;
        for (_, m) in moves {
            p.make(m);
            let v = -self.game(p, depth.saturating_sub(1), ply + 1, -beta, -alpha.max(best));
            p.unmake();
            if v > best {
                best = v;
                best_move = m;
            }
            if best >= beta {
                if !quiet && type_of(p.board[m_to(m) as usize]) == 0 && m_promo(m) == 0 && ply < self.killers.len() {
                    let ks = &mut self.killers[ply];
                    if ks[0] != m {
                        ks[1] = ks[0];
                        ks[0] = m;
                    }
                    self.history[m_from(m) as usize * 120 + m_to(m) as usize] += depth * depth;
                }
                break;
            }
        }
        if !ABORTED.with(|c| c.get()) {
            let flag = if best <= alpha0 { UPPER } else if best >= beta { LOWER } else { EXACT };
            self.tt[slot] = (key, depth as u8 + 1, best, flag, best_move);
        }
        best
    }

    /// A mate the side to move forces within `n` of its moves: every reply answered. The mating move, if any.
    pub fn mate(&mut self, p: &mut Position, n: u32, budget: u64) -> Option<u32> {
        for m in legal_moves(p) {
            p.make(m);
            let ok = self.mated(p, n - 1, budget);
            p.unmake();
            if ok {
                return Some(m);
            }
            if self.nodes > budget {
                return None;
            }
        }
        None
    }

    /// The side to move is mated now, or every reply leaves a mate within `n` more moves.
    fn mated(&mut self, p: &mut Position, n: u32, budget: u64) -> bool {
        self.nodes += 1;
        let replies = legal_moves(p);
        if replies.is_empty() {
            return p.in_check();
        }
        if n == 0 || self.nodes > budget {
            return false;
        }
        for r in replies {
            p.make(r);
            let ok = self.mate(p, n, budget).is_some();
            p.unmake();
            if !ok {
                return false;
            }
        }
        true
    }
}

/// Every legal move with its Syzygy WDL for the mover (2 win, 0 draw, -2 loss; ±1 cursed/blessed).
fn syzygy_rows(fen: &str) -> Result<Vec<(String, i32)>, String> {
    let script = format!(
        "import chess, chess.syzygy\nb = chess.Board('{fen}')\nwith chess.syzygy.open_tablebase('{SYZYGY}') as tb:\n    for m in b.legal_moves:\n        b.push(m)\n        v = -2 if b.is_checkmate() else (0 if b.is_stalemate() or len(b.piece_map()) == 2 else tb.probe_wdl(b))\n        print(m.uci(), -v)\n        b.pop()\n"
    );
    let out = std::process::Command::new("python").arg("-c").arg(&script).output().map_err(|e| format!("python-chess could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!("the Syzygy probe failed: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().filter_map(|l| l.split_once(' ').and_then(|(m, v)| v.trim().parse().ok().map(|v| (m.to_string(), v)))).collect())
}

/// Golden Boy plays with Golden Answer's kinds, each move saying what it
/// rests on: a strong proof (mate, solved class) first; in positions of up
/// to five pieces, among the moves that keep the Syzygy result
/// (theoretical), the one its patterns rank first; otherwise the bounded
/// game's best (logical, proved in its scope), the patterns choosing among
/// equals. (move, kind, lines)
pub fn play(fen: &str) -> Option<(String, &'static str, Vec<String>)> {
    let b = Board::from_fen(fen).ok()?;
    if b.moves().is_empty() {
        return None;
    }
    // 1. a strong proof: a forced mate, a solved class, a forced draw or loss
    if let Some((m, lines)) = strong(fen) {
        if m != "no move" {
            return Some((m, "logical", lines));
        }
    }
    let rosters = crate::patterns::load();
    let ranked: Vec<String> = crate::patterns::play(&b, &[], &rosters).map(|(_, _, rows)| rows.iter().map(|r| r.0.uci()).collect()).unwrap_or_default();
    if b.sq.iter().filter(|&&p| p != 0).count() <= 5 {
        if let Ok(rows) = syzygy_rows(fen) {
            if let Some(best) = rows.iter().map(|r| r.1).max() {
                let keep: Vec<&String> = rows.iter().filter(|r| r.1 == best).map(|r| &r.0).collect();
                let pick = ranked.iter().find(|m| keep.contains(m)).cloned().unwrap_or_else(|| keep[0].clone());
                let what = match best {
                    2 | 1 => "win",
                    0 => "draw",
                    _ => "loss",
                };
                let lines = vec![format!("Syzygy: the position is a {what}; {} moves keep it, and Golden Boy's patterns rank {pick} first among them", keep.len())];
                return Some((pick, "theoretical", lines));
            }
        }
    }
    // 3. the bounded game, proved; among its equally best moves the patterns choose
    let (rows, nodes, depth) = deepen(&b);
    let top = rows.first()?.1;
    let equal: Vec<&String> = rows.iter().filter(|r| r.1 == top).map(|r| &r.0).collect();
    let pick = ranked.iter().find(|m| equal.contains(m)).cloned().unwrap_or_else(|| equal[0].clone());
    let lines = vec![format!(
        "the bounded game ({depth} plies of every move, captures to quiet; {}) gives {top:+} at best ({nodes} positions); {} moves reach it, and Golden Boy's patterns choose {pick} among them",
        leaf_words(),
        equal.len()
    )];
    Some((pick, "logical (bounded) + patterns among equals", lines))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_question() {
        assert!(read("best move in 6k1/8/6K1/8/8/8/8/R7 w - - 0 1").is_some());
        assert!(read("paras siirto asemassa 6k1/8/6K1/8/8/8/8/R7 w - - 0 1").is_some());
        assert!(read("best move in the opening").is_none());
    }

    #[test]
    fn logical_mates_and_tables() {
        // KRvK mate in 1: the table
        let (m, _, _) = answer(Kind3::Logical, "6k1/8/6K1/8/8/8/8/R7 w - - 0 1").unwrap();
        assert_eq!(m, "a1a8");
        // a back-rank mate in 1 on a full board: the exact search
        let (m, lines, _) = answer(Kind3::Logical, "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1").unwrap();
        assert_eq!(m, "d1d8");
        assert!(lines[0].contains("mates by force"));
        // checkmate: no move
        assert_eq!(answer(Kind3::Logical, "R5k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1").unwrap().0, "no move");
    }
}
