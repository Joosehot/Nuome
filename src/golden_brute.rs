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
    let mut standing: Vec<(crate::golden::Mv, Tree)> = b.moves().into_iter().map(|m| (m, Tree { mv: m.uci(), proven: Proven::Unknown, depth: 0 })).collect();
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
