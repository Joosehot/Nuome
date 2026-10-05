//! Golden's brute force: every line, on Golden's own board, with no
//! evaluation at all. Only what the rules prove is returned: a mate within
//! the depth, a stalemate, or nothing proven. Every line is visited; nothing
//! is cut. The depth grows 1, 2, 3, ... up to PLIES (6: three moves each),
//! each depth searched whole; when the time is up, the last whole depth
//! stands.

use crate::golden::Board;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const PLIES: usize = 6;
pub const SECONDS: f64 = 10.0;

/// What the rules prove for the side to move, within the plies searched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proven {
    /// mates in this many plies
    Win(usize),
    /// is mated in this many plies
    Loss(usize),
    Draw,
    Unknown,
}

/// When to stop: the deadline, and a flag every thread sees once it has passed.
pub struct Clock {
    pub until: Instant,
    pub over: AtomicBool,
}

impl Clock {
    fn out(&self, nodes: u64) -> bool {
        if nodes & 4095 == 0 && Instant::now() >= self.until {
            self.over.store(true, Ordering::Relaxed);
        }
        self.over.load(Ordering::Relaxed)
    }
}

/// Every line from `b` to `plies` more; `nodes` counts positions visited.
/// With the time up the answer is meaningless (the caller drops the depth).
pub fn prove(b: &Board, plies: usize, nodes: &mut u64, clock: &Clock) -> Proven {
    *nodes += 1;
    if clock.out(*nodes) {
        return Proven::Unknown;
    }
    if plies == 0 {
        // at the end of the line only mate and stalemate are read: is there any legal move at all
        if b.has_legal() {
            return Proven::Unknown;
        }
        return if b.in_check() { Proven::Loss(0) } else { Proven::Draw };
    }
    let ms = b.moves();
    if ms.is_empty() {
        return if b.in_check() { Proven::Loss(0) } else { Proven::Draw };
    }
    let mut win: Option<usize> = None;
    let mut all_lost = true;
    let mut longest_loss = 0;
    let mut all_known = true;
    let mut any_draw = false;
    for m in ms {
        match prove(&b.play(m), plies - 1, nodes, clock) {
            Proven::Loss(n) => win = Some(win.map_or(n + 1, |w| w.min(n + 1))),
            Proven::Win(n) => longest_loss = longest_loss.max(n + 1),
            Proven::Draw => {
                all_lost = false;
                any_draw = true;
            }
            Proven::Unknown => {
                all_lost = false;
                all_known = false;
            }
        }
    }
    if let Some(n) = win {
        Proven::Win(n)
    } else if all_lost {
        Proven::Loss(longest_loss)
    } else if all_known && any_draw {
        Proven::Draw
    } else {
        Proven::Unknown
    }
}

/// Every root move with what is proven after it (for the mover) at one whole
/// depth, and the positions visited; None when the time ran out first.
fn one_depth(b: &Board, plies: usize, clock: &Clock) -> (Option<Vec<(String, Proven)>>, u64) {
    let ms = b.moves();
    let (rows, nodes): (Vec<_>, Vec<u64>) = std::thread::scope(|s| {
        let hs: Vec<_> = ms
            .iter()
            .map(|&m| {
                s.spawn(move || {
                    let mut n = 0;
                    let v = match prove(&b.play(m), plies - 1, &mut n, clock) {
                        Proven::Loss(k) => Proven::Win(k + 1),
                        Proven::Win(k) => Proven::Loss(k + 1),
                        x => x,
                    };
                    ((m.uci(), v), n)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).unzip()
    });
    let n = nodes.iter().sum();
    if clock.over.load(Ordering::Relaxed) {
        (None, n)
    } else {
        (Some(rows), n)
    }
}

/// Deepening to `plies` within `seconds`: the last whole depth, its results, and all positions visited.
pub fn root(fen: &str, plies: usize, seconds: f64) -> Result<(usize, Vec<(String, Proven)>, u64), String> {
    let b = Board::from_fen(fen)?;
    let clock = Clock { until: Instant::now() + Duration::from_secs_f64(seconds), over: AtomicBool::new(false) };
    let (mut depth, mut rows, mut nodes) = (0, Vec::new(), 0);
    for d in 1..=plies {
        let (r, n) = one_depth(&b, d, &clock);
        nodes += n;
        match r {
            Some(r) => {
                depth = d;
                let decided = r.iter().any(|x| matches!(x.1, Proven::Win(_)));
                rows = r;
                if decided {
                    break; // a mate is proven: the shortest is found
                }
            }
            None => break,
        }
    }
    Ok((depth, rows, nodes))
}

pub fn report(fen: &str, plies: usize, seconds: f64) -> Result<String, String> {
    let t0 = Instant::now();
    let (depth, rows, nodes) = root(fen, plies, seconds)?;
    let secs = t0.elapsed().as_secs_f64();
    let wins: Vec<String> = rows.iter().filter_map(|(m, v)| if let Proven::Win(n) = v { Some(format!("{m} (mate in {n} plies)")) } else { None }).collect();
    let losses = rows.iter().filter(|r| matches!(r.1, Proven::Loss(_))).count();
    let draws = rows.iter().filter(|r| r.1 == Proven::Draw).count();
    let unknown = rows.iter().filter(|r| r.1 == Proven::Unknown).count();
    Ok(format!(
        "{fen}\n  every line searched to {depth} plies (of {plies}) in {secs:.1} s of {seconds:.0}; {nodes} positions ({:.1} M/s)\n  proven: {} winning{}, {losses} losing, {draws} drawn, {unknown} not proven\n",
        nodes as f64 / secs.max(1e-9) / 1e6,
        wins.len(),
        if wins.is_empty() { String::new() } else { format!(" ({})", wins.join(", ")) }
    ))
}

// ---------------------------------------------------------------- the lumberjack

/// A tree that leads nowhere after this many plies is felled (when others lead somewhere).
pub const FELL_AT: usize = 20;

/// One root move's tree: the move, what its brute force proves, and how deep it has grown.
#[derive(Clone)]
pub struct Tree {
    pub mv: String,
    pub proven: Proven,
    pub depth: usize,
    /// the light eval of the move (the five checks), once read
    pub light: Option<Light>,
}

// ---------------------------------------------------------------- the light eval: the five checks

/// The five checks of one root move, from the mover's side, in centipawns
/// (pawn 100, knight and bishop 325, rook 500, queen 975).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    /// legality: the brute force's proof (mate, stalemate) stands above the rest
    pub legal: bool,
    /// safety: what the other side wins by its captures after the move (0 or less)
    pub safety: i32,
    /// force: what the move threatens to win by captures if the other side did nothing, and whether it checks
    pub force: i32,
    pub check: bool,
    /// material: mine minus theirs once the captures after the move are traded out
    pub material: i32,
    /// tempo: my legal moves minus theirs after the move
    pub tempo: i32,
}

const LIGHT_WORTH: [i32; 7] = [0, 100, 325, 325, 500, 975, 0];
/// A move is felled by the light eval when it ends this much below the best move's material.
pub const LIGHT_MARGIN: i32 = 100;

fn light_material(p: &crate::fast::position::Position) -> i32 {
    let side = p.turn as usize;
    let mut v = [0i32; 2];
    for c in 0..2 {
        for t in 0..6 {
            v[c] += LIGHT_WORTH[t + 1] * p.bb[c * 6 + t].count_ones() as i32;
        }
    }
    v[side] - v[1 - side]
}

/// The captures traded out, each side free to stop: the material for the side to move.
fn light_trade(p: &mut crate::fast::position::Position, plies: usize) -> i32 {
    use crate::fast::movegen::{gen_moves, MAX_MOVES};
    let stand = light_material(p);
    if plies == 0 {
        return stand;
    }
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, true);
    let mut best = stand;
    for &m in &list[..n] {
        p.make(m);
        if !p.in_check_c(me) {
            best = best.max(-light_trade(p, plies - 1));
        }
        p.unmake();
    }
    best
}

/// The five checks of `uci` in the position `fen`.
pub fn light(fen: &str, uci: &str) -> Option<Light> {
    use crate::fast::movegen::uci_to_move;
    let mut p = crate::fast::position::Position::from_fen(fen).ok()?;
    let m = uci_to_move(&mut p, uci)?;
    Some(light_on(&mut p, m))
}

/// The five checks of move `m` in position `p` (p is left as it was).
pub fn light_on(p: &mut crate::fast::position::Position, m: u32) -> Light {
    use crate::fast::movegen::legal_moves;
    p.make(m);
    let check = p.in_check();
    let here = -light_material(p);
    // material and safety: the other side to move, the captures traded out
    let material = -light_trade(p, 6);
    let safety = (material - here).min(0);
    // force: were the other side to pass, what my captures would win
    let force = if check {
        0
    } else {
        p.make_null();
        let f = light_trade(p, 6) - light_material(p);
        p.unmake_null();
        f
    };
    // tempo
    let theirs = legal_moves(p).len() as i32;
    let mine = if check {
        0
    } else {
        p.make_null();
        let k = legal_moves(p).len() as i32;
        p.unmake_null();
        k
    };
    p.unmake();
    Light { legal: true, safety, force, check, material, tempo: mine - theirs }
}

/// The lumberjack (6c80019, without any function): every root move grows its
/// own tree, one ply a round, all together, each searched whole by brute
/// force. A move proved losing is felled at once; a tree that leads nowhere
/// (no proof) after FELL_AT plies is felled when another proves something;
/// the rest grow until the time ends. A proved mate ends it (the shortest).
/// The move is the best proof left: mate, then a proved draw, then not
/// proven; ties by the rules' order. If every tree falls, the one felled
/// last (the longest loss) stays.
pub fn lumberjack(fen: &str, seconds: f64) -> Result<(Option<Tree>, Vec<Tree>, Vec<Tree>, u64), String> {
    let b = Board::from_fen(fen)?;
    let clock = Clock { until: Instant::now() + Duration::from_secs_f64(seconds), over: AtomicBool::new(false) };
    let mut standing: Vec<(crate::golden::Mv, Tree)> = b.moves().into_iter().map(|m| (m, Tree { mv: m.uci(), proven: Proven::Unknown, depth: 0, light: None })).collect();
    let mut felled: Vec<Tree> = Vec::new();
    let mut nodes = 0u64;
    let mut depth = 1;
    while !standing.is_empty() && !clock.over.load(Ordering::Relaxed) && depth <= FELL_AT {
        // every standing tree grows one ply, in parallel
        let grown: Vec<(Option<Proven>, u64)> = std::thread::scope(|s| {
            let hs: Vec<_> = standing
                .iter()
                .map(|(m, t)| {
                    let (b, clock, m, done) = (&b, &clock, *m, t.proven != Proven::Unknown);
                    s.spawn(move || {
                        if done {
                            return (None, 0);
                        }
                        let mut n = 0;
                        let v = match prove(&b.play(m), depth - 1, &mut n, clock) {
                            Proven::Loss(k) => Proven::Win(k + 1),
                            Proven::Win(k) => Proven::Loss(k + 1),
                            x => x,
                        };
                        (if clock.over.load(Ordering::Relaxed) { None } else { Some(v) }, n)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut next = Vec::new();
        for ((m, mut t), (v, n)) in standing.into_iter().zip(grown) {
            nodes += n;
            if let Some(v) = v {
                t.proven = v;
                t.depth = depth;
            }
            if matches!(t.proven, Proven::Loss(_)) {
                felled.push(t);
            } else {
                next.push((m, t));
            }
        }
        standing = next;
        if standing.iter().any(|t| matches!(t.1.proven, Proven::Win(_))) {
            break;
        }
        if depth == FELL_AT && standing.iter().any(|t| t.1.proven != Proven::Unknown) {
            let (keep, fall): (Vec<_>, Vec<_>) = standing.into_iter().partition(|t| t.1.proven != Proven::Unknown);
            felled.extend(fall.into_iter().map(|t| t.1));
            standing = keep;
        }
        depth += 1;
    }
    // the light eval: the five checks of every tree left; a tree whose material
    // after the captures ends LIGHT_MARGIN below the best one is felled (a proved
    // mate is never felled, and the light eval never fells every tree)
    if !standing.iter().any(|t| matches!(t.1.proven, Proven::Win(_))) {
        for t in standing.iter_mut() {
            t.1.light = light(fen, &t.1.mv);
        }
        if let Some(best) = standing.iter().filter_map(|t| t.1.light.map(|l| l.material)).max() {
            let (keep, fall): (Vec<_>, Vec<_>) = standing.into_iter().partition(|t| t.1.light.map_or(true, |l| l.material >= best - LIGHT_MARGIN));
            felled.extend(fall.into_iter().map(|t| t.1));
            standing = keep;
        }
    }
    let rank = |p: Proven| match p {
        Proven::Win(n) => (3i64, -(n as i64)),
        Proven::Draw => (2, 0),
        Proven::Unknown => (1, 0),
        Proven::Loss(n) => (0, n as i64),
    };
    let mut trees: Vec<Tree> = standing.into_iter().map(|t| t.1).collect();
    // stable: ties keep the rules' order
    trees.sort_by(|x, y| rank(y.proven).cmp(&rank(x.proven)));
    let pick = trees.first().cloned().or_else(|| felled.iter().max_by_key(|t| rank(t.proven)).cloned());
    Ok((pick, trees, felled, nodes))
}

pub fn lumberjack_report(fen: &str, seconds: f64) -> Result<String, String> {
    let t0 = Instant::now();
    let (pick, standing, felled, nodes) = lumberjack(fen, seconds)?;
    let secs = t0.elapsed().as_secs_f64();
    let deepest = standing.iter().chain(&felled).map(|t| t.depth).max().unwrap_or(0);
    let show = |t: &Tree| match t.proven {
        Proven::Win(n) => format!("{} mate in {n}", t.mv),
        Proven::Loss(n) => format!("{} mated in {n}", t.mv),
        Proven::Draw => format!("{} draw", t.mv),
        Proven::Unknown => format!("{} ({} plies)", t.mv, t.depth),
    };
    Ok(format!(
        "{fen}\n  lumberjack in {secs:.1} s of {seconds:.0}: {nodes} positions, trees grown to {deepest} plies\n  move: {}\n  standing {}: {}\n  felled {}: {}\n",
        pick.as_ref().map_or("none".to_string(), show),
        standing.len(),
        standing.iter().take(8).map(show).collect::<Vec<_>>().join(", ") + if standing.len() > 8 { ", ..." } else { "" },
        felled.len(),
        felled.iter().map(show).collect::<Vec<_>>().join(", "),
    ))
}

// ---------------------------------------------------------------- brute force, the lumberjack and StarNet's eval

/// The share of the time for the brute force and the lumberjack; the rest is the eval's.
pub const PROOF_SHARE: f64 = 0.75;
const MATE_CP: i32 = 100_000;
const QS_PLIES: usize = 4;

/// The captures (and promotions; every reply in check) traded to quiet, each
/// side free to stand, StarNNUE at the quiet ends: the side to move's value.
fn quiet(net: &crate::fast::starnnue::Net, p: &mut crate::fast::position::Position, plies: usize, until: Instant) -> i32 {
    use crate::fast::movegen::{gen_moves, MAX_MOVES};
    let check = p.in_check();
    let me = p.turn;
    let mut list = [0u32; MAX_MOVES];
    let n = gen_moves(p, &mut list, !check);
    let stand = if check { -MATE_CP } else { net.eval(p) };
    if plies == 0 || Instant::now() >= until {
        return if check { net.eval(p) } else { stand };
    }
    let mut best = stand;
    let mut any = false;
    for &m in &list[..n] {
        // out of check every move; otherwise only captures that do not lose material
        if !check && crate::fast::see::see(p, m) < 0 {
            continue;
        }
        p.make(m);
        if p.in_check_c(me) {
            p.unmake();
            continue;
        }
        any = true;
        let v = -quiet(net, p, plies - 1, until);
        p.unmake();
        best = best.max(v);
    }
    if check && !any {
        return -MATE_CP;
    }
    best
}

/// A move's value by StarNet's eval: its worst reply, each traded to quiet.
fn starnet_value(net: &crate::fast::starnnue::Net, fen: &str, uci: &str, until: Instant) -> Option<i32> {
    use crate::fast::movegen::{legal_moves, uci_to_move};
    let mut p = crate::fast::position::Position::from_fen(fen).ok()?;
    let m = uci_to_move(&mut p, uci)?;
    p.make(m);
    let replies = legal_moves(&mut p);
    let v = if replies.is_empty() {
        if p.in_check() { MATE_CP } else { 0 }
    } else {
        let mut worst = i32::MAX;
        for r in replies {
            p.make(r);
            worst = worst.min(quiet(net, &mut p, QS_PLIES, until));
            p.unmake();
        }
        worst
    };
    Some(v)
}

/// Golden Boy's move: the brute force proves and the lumberjack fells; a
/// proved mate is played; otherwise StarNet's eval (StarNNUE) chooses among
/// the trees left standing in the best proof class.
pub fn golden_boy(fen: &str, seconds: f64) -> Result<(String, Vec<String>), String> {
    let t0 = Instant::now();
    let net = crate::fast::starnnue::net()?;
    let (pick, standing, felled, nodes) = lumberjack(fen, seconds * PROOF_SHARE)?;
    let pick = pick.ok_or("no legal move")?;
    let mut lines = vec![format!(
        "brute force and lumberjack: {nodes} positions in {:.1} s, trees to {} plies; {} felled ({}), {} standing",
        t0.elapsed().as_secs_f64(),
        standing.iter().chain(&felled).map(|t| t.depth).max().unwrap_or(0),
        felled.len(),
        felled.iter().map(|t| t.mv.as_str()).collect::<Vec<_>>().join(" "),
        standing.len()
    )];
    if let Proven::Win(n) = pick.proven {
        lines.push(format!("{} mates in {n} plies (proved by brute force)", pick.mv));
        return Ok((pick.mv, lines));
    }
    if standing.is_empty() {
        lines.push(format!("every move is proved losing; {} loses slowest", pick.mv));
        return Ok((pick.mv, lines));
    }
    let until = t0 + Duration::from_secs_f64(seconds);
    let class = pick.proven;
    let candidates: Vec<String> = standing.iter().filter(|t| t.proven == class).map(|t| t.mv.clone()).collect();
    let values: Vec<(String, i32)> = std::thread::scope(|s| {
        let hs: Vec<_> = candidates.iter().map(|m| s.spawn(move || (m.clone(), starnet_value(net, fen, m, until).unwrap_or(-MATE_CP)))).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    // the best value; ties keep the rules' order
    let (m, v) = values.iter().fold(None::<&(String, i32)>, |b, x| match b { Some(b) if b.1 >= x.1 => Some(b), _ => Some(x) }).cloned().ok_or("no candidate")?;
    lines.push(format!("StarNet's eval chooses among the {} standing ({}): {m} at {v:+} cp (worst reply, captures traded to quiet; {:.1} s)", candidates.len(), if class == Proven::Draw { "proved draws" } else { "not proven" }, t0.elapsed().as_secs_f64()));
    Ok((m, lines))
}

pub fn golden_boy_report(fen: &str, seconds: f64) -> Result<String, String> {
    let (m, lines) = golden_boy(fen, seconds)?;
    Ok(format!("{fen}\n  move: {m}\n  {}\n", lines.join("\n  ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lumberjack_fells_and_mates() {
        // scholar's mate: the lumberjack plays Qxf7#
        let (pick, _, _, _) = lumberjack("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4", 5.0).unwrap();
        assert_eq!(pick.unwrap().mv, "h5f7");
        // black to move must not allow Qxf7#: every move that allows it is felled (Nxh5 saves it)
        let (pick, _, felled, _) = lumberjack("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 4 4", 5.0).unwrap();
        assert!(felled.iter().any(|t| t.mv == "a7a6"));
        assert_ne!(pick.unwrap().proven, Proven::Loss(2));
    }

    use super::*;

    #[test]
    fn finds_the_mates() {
        // scholar's mate: Qxf7# at once
        let (d, rows, _) = root("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4", 2, 10.0).unwrap();
        assert_eq!(d, 1);
        assert_eq!(rows.iter().find(|r| r.0 == "h5f7").unwrap().1, Proven::Win(1));
        // checkmated: no move
        let clock = Clock { until: Instant::now() + Duration::from_secs(10), over: AtomicBool::new(false) };
        let mut n = 0;
        assert_eq!(prove(&Board::from_fen("R5k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1").unwrap(), 2, &mut n, &clock), Proven::Loss(0));
    }
}
