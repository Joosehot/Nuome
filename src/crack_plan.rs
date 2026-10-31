//! Waves as moves (the owner's): a move starts a wave, a plan running 10-20 moves ahead; the opponent's reply is a
//! wave of its own; where the opponent's wave meets ours too early (dissonance), our plan breaks, and a move that some
//! reply leaves broken is put aside. The move: the one whose worst reply leaves the best chord.
//!
//! v1 of the waves, read from the rules:
//!   a piece's wave: the moves it needs to reach every square (crack_wave::times_to, up to PLAN = 10 moves; for the plans,
//!     time counted in plies, the side to move first);
//!   a pawn's plan: its march to the queening square, one square a move;
//!   dissonance: an enemy piece reaches a square of the march no later than the pawn, and no piece of the pawn's side
//!     (its king or another) reaches that square sooner to hold it; then the plan breaks;
//!   the chord: material (Q 9, R 5, B 3, N 3, P 1), and the side whose pawn plan sounds first gains a queen (8 more);
//!     the side with the force to mate and three or more ahead wins.
//! The move: every move of ours and every reply (two plies), the chord read where the plies stop.
use crate::crack::{rules, V};
use crate::crack_wave::{times_to, UNREACHED};

/// how far the waves reach: moves (each side's), so twice as many plies
pub const PLAN: u8 = 10;
use crate::golden::Board;

/// plies: the side to move plays on plies 1, 3, 5, ...; the other on 2, 4, 6, ...
fn ply(moves: u8, side_to_move: bool) -> i32 {
    if moves == UNREACHED {
        i32::MAX
    } else if moves == 0 {
        0
    } else if side_to_move {
        2 * moves as i32 - 1
    } else {
        2 * moves as i32
    }
}

/// a pawn's march to its last rank: (square, the ply the pawn stands on it)
fn march(b: &Board, s: usize) -> Vec<(usize, i32)> {
    let white = b.sq[s] > 0;
    let moving = b.white == white;
    let (f, r) = ((s % 8) as i32, (s / 8) as i32);
    let dr = if white { 1 } else { -1 };
    let mut out = Vec::new();
    let mut rank = r;
    let mut n = 0u8;
    let first = (white && r == 1) || (!white && r == 6);
    while (white && rank < 7) || (!white && rank > 0) {
        rank += dr;
        // a double step from the first rank: the square passed and the square reached take the same move
        if first && rank == r + 2 * dr {
            out.push(((rank * 8 + f) as usize, ply(1, moving)));
            n = 1;
            continue;
        }
        n += 1;
        out.push(((rank * 8 + f) as usize, ply(n, moving)));
    }
    out
}

/// the ply a side's pieces (other than the pawn itself) first reach a square
fn arrival(waves: &[(usize, [u8; 64])], b: &Board, white: bool, sq: usize, not: usize) -> i32 {
    let moving = b.white == white;
    waves
        .iter()
        .filter(|(s, _)| *s != not && b.sq[*s] != 0 && (b.sq[*s] > 0) == white)
        .map(|(_, t)| ply(t[sq], moving))
        .min()
        .unwrap_or(i32::MAX)
}

/// v2 of the plans: every piece's wave tells two times, when it reaches a square and when it strikes it (a move
/// sooner: a piece that can step onto a square strikes it from where it stands the move before)
fn strike(waves: &[(usize, [u8; 64])], b: &Board, white: bool, sq: usize, not: usize, kings: bool) -> i32 {
    let moving = b.white == white;
    waves
        .iter()
        .filter(|(s, _)| *s != not && b.sq[*s] != 0 && (b.sq[*s] > 0) == white && (b.sq[*s].abs() == 6) == kings)
        .filter(|(s, _)| !(b.sq[*s].abs() == 1 && *s % 8 == sq % 8))
        .map(|(_, t)| if t[sq] == UNREACHED { i32::MAX } else { ply(t[sq].saturating_sub(1), moving) })
        .min()
        .unwrap_or(i32::MAX)
}

/// v2: every piece has its plan. The pawn marches to queen (a piece of its own side on its way costs it a move);
/// the king escorts its own pawn (it reaches a square in front of the pawn, one or two ranks ahead on its file or the
/// next, no later than the enemy king: then the enemy king cannot stop the march; set by hand from the key squares)
/// and blockades the enemy's (it stands on the march first); every piece stops the enemy's march by striking its
/// squares and guards its own. Dissonance: an enemy piece stands on a square of the march before the pawn, or a piece
/// other than the king strikes it by the ply after the pawn arrives (it takes the pawn, or gives itself for it), or the
/// enemy king strikes it by then and nothing of the pawn's side guards it.
fn sounding_v2(b: &Board, white: bool, waves: &[(usize, [u8; 64])]) -> Option<i32> {
    let me_king = b.king(white) as usize;
    let their_king = b.king(!white) as usize;
    let king_wave = |k: usize| waves.iter().find(|(s, _)| *s == k).map(|x| x.1);
    (0..64usize)
        .filter(|&s| b.sq[s] == if white { 1 } else { -1 })
        .filter_map(|s| {
            let mut m = march(b, s);
            // a piece of its own side on its way: the pawn waits a move for it
            let mut delay = 0;
            for x in m.iter_mut() {
                if b.sq[x.0] != 0 && (b.sq[x.0] > 0) == white {
                    delay += 2;
                }
                x.1 += delay;
            }
            // the king's escort: a square in front of the pawn reached by our king no later than by theirs
            let (f, r) = ((s % 8) as i32, (s / 8) as i32);
            let dr = if white { 1 } else { -1 };
            let escorted = match (king_wave(me_king), king_wave(their_king)) {
                (Some(mine), Some(theirs)) => (1..=2).any(|k| {
                    (f - 1..=f + 1).any(|ff| {
                        let rr = r + k * dr;
                        if !(0..8).contains(&ff) || !(0..8).contains(&rr) {
                            return false;
                        }
                        let q = (rr * 8 + ff) as usize;
                        ply(mine[q], b.white == white) <= ply(theirs[q], b.white != white)
                    })
                }),
                _ => false,
            };
            let broken = m.iter().any(|&(q, at)| {
                let occupied = waves
                    .iter()
                    .filter(|(p, _)| b.sq[*p] != 0 && (b.sq[*p] > 0) != white && !(escorted && *p == their_king))
                    .map(|(_, t)| ply(t[q], b.white != white))
                    .min()
                    .unwrap_or(i32::MAX);
                let pieces = strike(waves, b, !white, q, s, false);
                let king = if escorted { i32::MAX } else { strike(waves, b, !white, q, s, true) };
                let guard = strike(waves, b, white, q, s, false).min(strike(waves, b, white, q, s, true));
                occupied < at || pieces <= at + 1 || (king <= at + 1 && guard > at + 1)
            });
            (!broken).then(|| m.last().map_or(0, |x| x.1))
        })
        .min()
}

/// which plans the chord uses: 1 (pawns only) or 2 (every piece)
static PLANS: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);

fn sounding(b: &Board, white: bool, waves: &[(usize, [u8; 64])]) -> Option<i32> {
    if PLANS.load(std::sync::atomic::Ordering::Relaxed) == 2 {
        sounding_v2(b, white, waves)
    } else {
        sounding_v1(b, white, waves)
    }
}

/// the ply a side's pawn plan first sounds (a pawn queens unbroken), or None (v1: the pawns' plans only)
fn sounding_v1(b: &Board, white: bool, waves: &[(usize, [u8; 64])]) -> Option<i32> {
    (0..64usize)
        .filter(|&s| b.sq[s] == if white { 1 } else { -1 })
        .filter_map(|s| {
            let m = march(b, s);
            let broken = m.iter().any(|&(q, at)| {
                let enemy = arrival(waves, b, !white, q, s);
                let held = arrival(waves, b, white, q, s);
                enemy <= at && held >= enemy
            });
            (!broken).then(|| m.last().map_or(0, |x| x.1))
        })
        .min()
}

fn material(b: &Board, white: bool) -> i32 {
    (0..64)
        .filter(|&s| b.sq[s] != 0 && (b.sq[s] > 0) == white)
        .map(|s| match b.sq[s].unsigned_abs() {
            1 => 1,
            2 | 3 => 3,
            4 => 5,
            5 => 9,
            _ => 0,
        })
        .sum()
}

fn mates(b: &Board, white: bool) -> bool {
    let has = |t: i8| (0..64).any(|s| b.sq[s] == if white { t } else { -t });
    let bishops: Vec<usize> = (0..64).filter(|&s| b.sq[s] == if white { 3 } else { -3 }).collect();
    let colours = bishops.iter().map(|s| (s / 8 + s % 8) % 2).collect::<std::collections::HashSet<_>>().len();
    has(5) || has(4) || has(1) || (has(3) && has(2)) || colours == 2
}

/// the chord for the side to move: (a score, and the value it reads)
pub fn chord(b: &Board) -> (i32, V) {
    if let Some(v) = rules(b) {
        return (100 * v as i32, v);
    }
    let me = b.white;
    let waves: Vec<(usize, [u8; 64])> = (0..64).filter(|&s| b.sq[s] != 0).map(|s| (s, times_to(b, s, PLAN))).collect();
    let (mine, theirs) = (sounding(b, me, &waves), sounding(b, !me, &waves));
    let mut mm = material(b, me);
    let mut tm = material(b, !me);
    match (mine, theirs) {
        (Some(a), Some(c)) if a < c => mm += 8,
        (Some(_), Some(_)) => tm += 8,
        (Some(_), None) => mm += 8,
        (None, Some(_)) => tm += 8,
        _ => {}
    }
    let bare = |white: bool| material(b, white) == 0;
    let wins = |white: bool, m: i32, o: i32, sounds: bool| (mates(b, white) || sounds) && (bare(!white) && (m > 1 || sounds) || m - o >= 3);
    let v = if wins(me, mm, tm, mine.is_some()) {
        1
    } else if wins(!me, tm, mm, theirs.is_some()) {
        -1
    } else {
        0
    };
    (mm - tm + 50 * v as i32, v)
}

/// our moves and the opponent's waves in reply: (the chord after the worst reply, from the mover's side) per move
pub fn moves(b: &Board) -> Vec<(crate::golden::Mv, i32)> {
    b.moves()
        .into_iter()
        .map(|m| {
            let a = b.play(m);
            let score = if let Some(v) = rules(&a) {
                -100 * v as i32
            } else {
                a.moves().into_iter().map(|r| chord(&a.play(r)).0).min().unwrap_or(0)
            };
            (m, score)
        })
        .collect()
}

/// the value of a position by the waves, two plies deep
pub fn value(b: &Board) -> V {
    if let Some(v) = rules(b) {
        return v;
    }
    let best = moves(b).into_iter().map(|x| x.1).max().unwrap_or(0);
    if best >= 40 { 1 } else if best <= -40 { -1 } else { 0 }
}

/// `nuome --ideas crack-plan`: the waves as moves, on the smoke set (values) and the check set (moves)
pub fn run() -> Result<(), String> {
    let smoke: Vec<(String, Board, V)> = std::fs::read_to_string("bench/crack/smoke.tsv")
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f[0].to_string(), Board::from_fen(f[1]).unwrap(), f[2].parse::<i32>().unwrap().signum() as V)
        })
        .collect();
    let par = |n: usize, f: &(dyn Fn(usize) -> bool + Sync)| -> Vec<bool> {
        let idx: Vec<usize> = (0..n).collect();
        std::thread::scope(|sc| {
            let hs: Vec<_> = idx.chunks(n.div_ceil(12).max(1)).map(|c| sc.spawn(move || c.iter().map(|&i| f(i)).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        })
    };
    let t0 = std::time::Instant::now();
    for plans in [1u8, 2] {
    PLANS.store(plans, std::sync::atomic::Ordering::Relaxed);
    println!("
=== plans v{plans}: {}", if plans == 1 { "the pawns' plans only" } else { "every piece's plan" });
    for (name, f) in [
        ("intuition v0 + search 2 (yardstick)", &(|b: &Board| crate::crack::searched(b, 2, -1, 1)) as &(dyn Fn(&Board) -> V + Sync)),
        ("waves as moves (2 plies, the chord where they stop)", &value),
    ] {
        let ok = par(smoke.len(), &|i| f(&smoke[i].1) == smoke[i].2);
        let mut by: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for (r, k) in smoke.iter().zip(&ok) {
            let e = by.entry(r.0.as_str()).or_default();
            e.0 += *k as usize;
            e.1 += 1;
        }
        let mut cl: Vec<&str> = smoke.iter().map(|r| r.0.as_str()).collect();
        cl.dedup();
        println!("values, {name}: {}/{} ({:.1}%)\n  {}", ok.iter().filter(|x| **x).count(), smoke.len(), 100.0 * ok.iter().filter(|x| **x).count() as f64 / smoke.len() as f64, cl.iter().map(|c| format!("{c} {}/{}", by[c].0, by[c].1)).collect::<Vec<_>>().join("  "));
    }
    // the moves: does the chosen move keep the position's best value (Syzygy)?
    let check: Vec<(Board, Vec<(String, i32)>)> = std::fs::read_to_string("bench/golden_formula/check.tsv")
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut f = l.split('\t');
            let b = Board::from_fen(f.next().unwrap()).unwrap();
            let mv = f.next().unwrap_or("").split(' ').filter_map(|x| x.split_once(':')).map(|(m, v)| (m.to_string(), v.parse::<i32>().unwrap())).collect();
            (b, mv)
        })
        .collect();
    let keeps = |b: &Board, vals: &[(String, i32)], chosen: Option<crate::golden::Mv>| -> bool {
        let best = vals.iter().map(|x| x.1.signum()).max().unwrap_or(0);
        chosen.and_then(|m| vals.iter().find(|x| x.0 == m.uci())).is_some_and(|x| x.1.signum() == best)
    };
    let wave_pick = |b: &Board| moves(b).into_iter().max_by_key(|x| x.1).map(|x| x.0);
    let v0_pick = |b: &Board| b.moves().into_iter().max_by_key(|&m| -crate::crack::searched(&b.play(m), 1, -1, 1)).map(|m| m);
    for (name, pick) in [("intuition v0 + search 2 (yardstick)", &v0_pick as &(dyn Fn(&Board) -> Option<crate::golden::Mv> + Sync)), ("waves as moves", &wave_pick)] {
        let ok = par(check.len(), &|i| keeps(&check[i].0, &check[i].1, pick(&check[i].0)));
        println!("moves, {name}: keeps the best value in {}/{} ({:.1}%) of the check set's positions (bench/golden_formula/check.tsv: every one has a move that throws the value away)", ok.iter().filter(|x| **x).count(), check.len(), 100.0 * ok.iter().filter(|x| **x).count() as f64 / check.len() as f64);
    }
    }
    println!("{:.1} s", t0.elapsed().as_secs_f64());
    Ok(())
}

/// `nuome --ideas crack-plan-misses CLASS`: where the waves read the training set's positions of a class wrong
pub fn misses(class: &str) -> Result<(), String> {
    let text = std::fs::read_to_string("bench/crack/train.tsv").map_err(|e| e.to_string())?;
    let (mut n, mut wrong) = (0, 0);
    for l in text.lines().filter(|l| l.starts_with(&format!("{class}	"))) {
        let f: Vec<&str> = l.split('\t').collect();
        let b = Board::from_fen(f[1])?;
        let truth = f[2].parse::<i32>().unwrap().signum() as V;
        let got = value(&b);
        n += 1;
        if got != truth {
            wrong += 1;
            if wrong <= 12 {
                println!("{}  truth {truth:+}  waves {got:+}  chord now {:?}", f[1], chord(&b));
            }
        }
    }
    println!("{class}: {wrong}/{n} wrong on train");
    Ok(())
}
