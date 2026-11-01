//! The wave model of a position (the owner's): every piece's four things (place, movement, value, relation) make a
//! pattern on the board; the patterns meet and make waves; the waves give the value, and the move.
//!
//! A piece sends a wave to every square: its amplitude the piece's worth, fading with distance (decay per move); its
//! phase the time, the moves the piece needs to reach the square from its place (omega per move); its sign its side
//! (the side to move +, the other -). All the waves added are the position's wave Psi on the 64 squares. Its power
//! |Psi|^2 holds a cross term for every pair of pieces: two arriving together strengthen each other, a wave of the
//! other side cancels them; that is the relation, and it is not a sum of the pieces.
//!
//! The waves are read where they decide: the enemy king and its squares, one's own king and its squares, the
//! queening squares of the best pawns, the pieces' own squares. Each read three ways: the wave's direction (the real
//! part, whose it is), its power, and how many squares are one's own (real part above a half) less the other's.
//! Golden invents the frequency, the fading and how the readings combine, on bench/crack/train.tsv only; judged on
//! bench/crack/smoke.tsv. The move: the one after which the position's waves read best for the mover.
use crate::crack::{rules, searched_with, V};
use crate::crack_invent::Invented;
use crate::golden::Board;

/// how many moves ahead a piece's wave reaches
pub const HORIZON: u8 = 4;
pub const UNREACHED: u8 = u8::MAX;

/// the moves a piece needs to reach every square, by its own way of moving, the other pieces standing still (a
/// slider stops at the first piece; a square held by its own side is not entered; a pawn's wave goes to the squares
/// it pushes to and the squares it strikes)
pub fn times(b: &Board, from: usize) -> [u8; 64] {
    times_to(b, from, HORIZON)
}

/// the same, `horizon` moves ahead
pub fn times_to(b: &Board, from: usize, horizon: u8) -> [u8; 64] {
    let p = b.sq[from];
    let white = p > 0;
    let t = p.unsigned_abs();
    let mut out = [UNREACHED; 64];
    out[from] = 0;
    let mut front = vec![from];
    let own = |s: usize| b.sq[s] != 0 && (b.sq[s] > 0) == white && s != from;
    for step in 1..=horizon {
        let mut next = Vec::new();
        for &s in &front {
            let (f, r) = ((s % 8) as i32, (s / 8) as i32);
            let mut reach = |nf: i32, nr: i32, next: &mut Vec<usize>| -> bool {
                if !(0..8).contains(&nf) || !(0..8).contains(&nr) {
                    return false;
                }
                let n = (nr * 8 + nf) as usize;
                if own(n) {
                    return false;
                }
                if out[n] == UNREACHED {
                    out[n] = step;
                    next.push(n);
                }
                b.sq[n] == 0 || n == from
            };
            match t {
                1 => {
                    let dr = if white { 1 } else { -1 };
                    for df in [-1, 1] {
                        reach(f + df, r + dr, &mut next);
                    }
                    if reach(f, r + dr, &mut next) && ((white && r == 1) || (!white && r == 6)) && step == 1 {
                        reach(f, r + 2 * dr, &mut next);
                    }
                }
                2 => {
                    for (a, c) in [(1, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1)] {
                        reach(f + a, r + c, &mut next);
                    }
                }
                6 => {
                    for a in -1..=1 {
                        for c in -1..=1 {
                            if a != 0 || c != 0 {
                                reach(f + a, r + c, &mut next);
                            }
                        }
                    }
                }
                _ => {
                    let dirs: &[(i32, i32)] = match t {
                        3 => &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
                        4 => &[(1, 0), (-1, 0), (0, 1), (0, -1)],
                        _ => &[(1, 1), (1, -1), (-1, 1), (-1, -1), (1, 0), (-1, 0), (0, 1), (0, -1)],
                    };
                    for &(a, c) in dirs {
                        let (mut nf, mut nr) = (f + a, r + c);
                        while reach(nf, nr, &mut next) {
                            nf += a;
                            nr += c;
                        }
                    }
                }
            }
        }
        front = next;
        if front.is_empty() {
            break;
        }
    }
    out
}

/// the wave's settings Golden chooses: the phase per move, the fading per move, a king's worth as a wave
#[derive(Clone, Copy, Debug)]
pub struct Wave {
    pub omega: f64,
    pub decay: f64,
    pub king: f64,
}

fn worth(t: u8, king: f64) -> f64 {
    match t {
        1 => 1.0,
        2 | 3 => 3.0,
        4 => 5.0,
        5 => 9.0,
        _ => king,
    }
}

/// the position's wave: Psi on the 64 squares, (re, im), the side to move +
pub fn psi(b: &Board, w: Wave) -> [(f64, f64); 64] {
    let mut out = [(0.0, 0.0); 64];
    for s in 0..64 {
        let p = b.sq[s];
        if p == 0 {
            continue;
        }
        let sign = if (p > 0) == b.white { 1.0 } else { -1.0 };
        let amp = worth(p.unsigned_abs(), w.king) * sign;
        for (q, &t) in times(b, s).iter().enumerate() {
            if t == UNREACHED {
                continue;
            }
            let a = amp * w.decay.powi(t as i32);
            let ph = w.omega * t as f64;
            out[q].0 += a * ph.cos();
            out[q].1 += a * ph.sin();
        }
    }
    out
}

pub const READINGS: [&str; 6] = ["their king's squares", "my king's squares", "my pawn's queening square", "their pawn's queening square", "my pieces' squares", "their pieces' squares"];
/// raw numbers: 6 regions x 3 readings, then the pieces counted (the value part, as before), the bare kings, check
pub const N: usize = 18 + 8;

fn ring(k: i32) -> Vec<usize> {
    let mut v = Vec::new();
    for a in -1..=1 {
        for c in -1..=1 {
            let (f, r) = (k % 8 + a, k / 8 + c);
            if (0..8).contains(&f) && (0..8).contains(&r) {
                v.push((r * 8 + f) as usize);
            }
        }
    }
    v
}

fn queening(b: &Board, white: bool) -> Vec<usize> {
    (0..64usize)
        .filter(|&s| b.sq[s] == if white { 1 } else { -1 })
        .map(|s| if white { 56 + s % 8 } else { s % 8 })
        .collect()
}

/// the readings of the waves, scaled to whole numbers for Golden's weights
pub fn readings(b: &Board, w: Wave) -> [i32; crate::crack_invent::N] {
    let me = b.white;
    let ps = psi(b, w);
    let regions: [Vec<usize>; 6] = [
        ring(b.king(!me)),
        ring(b.king(me)),
        queening(b, me),
        queening(b, !me),
        (0..64).filter(|&s| b.sq[s] != 0 && (b.sq[s] > 0) == me && b.sq[s].abs() != 6).collect(),
        (0..64).filter(|&s| b.sq[s] != 0 && (b.sq[s] > 0) != me && b.sq[s].abs() != 6).collect(),
    ];
    let mut out = [0i32; crate::crack_invent::N];
    for (i, reg) in regions.iter().enumerate() {
        let re: f64 = reg.iter().map(|&q| ps[q].0).sum();
        let pw: f64 = reg.iter().map(|&q| ps[q].0 * ps[q].0 + ps[q].1 * ps[q].1).sum();
        let ctrl: i32 = reg.iter().map(|&q| if ps[q].0 > 0.5 { 1 } else if ps[q].0 < -0.5 { -1 } else { 0 }).sum();
        out[3 * i] = re.round() as i32;
        out[3 * i + 1] = (pw / 4.0).round() as i32;
        out[3 * i + 2] = ctrl;
    }
    let count = |white: bool, t: i8| (0..64).filter(|&s| b.sq[s] == if white { t } else { -t }).count() as i32;
    for t in 1..=5i8 {
        out[17 + t as usize] = count(me, t) - count(!me, t);
    }
    let bare = |white: bool| (1..=5).all(|t| count(white, t) == 0) as i32;
    out[23] = bare(me);
    out[24] = bare(!me);
    out[25] = b.in_check() as i32;
    out
}

pub fn names() -> Vec<String> {
    let mut v = Vec::new();
    for r in READINGS {
        v.push(format!("direction at {r}"));
        v.push(format!("power at {r}"));
        v.push(format!("squares mine less theirs at {r}"));
    }
    for t in ["pawns", "knights", "bishops", "rooks", "queens"] {
        v.push(format!("{t} (me - them)"));
    }
    v.push("I have a bare king".into());
    v.push("they have a bare king".into());
    v.push("I am in check".into());
    v
}

pub fn load(path: &str) -> Result<Vec<(String, Board, V)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Ok((f[0].to_string(), Board::from_fen(f[1])?, f[2].parse::<i32>().map_err(|e| e.to_string())?.signum() as V))
        })
        .collect()
}

/// the waves read by an invented function
pub fn value(b: &Board, w: Wave, f: &Invented) -> V {
    rules(b).unwrap_or_else(|| f.value(&readings(b, w)))
}

/// `nuome --ideas crack-wave [DEPTH]`: Golden invents the wave and its reading on train; judged on smoke
pub fn run(depth: usize) -> Result<(), String> {
    assert_eq!(N, crate::crack_invent::N - 1, "the readings fill the invention's numbers");
    let train = load("bench/crack/train.tsv")?;
    let smoke = load("bench/crack/smoke.tsv")?;
    let t0 = std::time::Instant::now();
    let mut best: Option<(f64, Wave, Invented)> = None;
    for &omega in &[0.0, std::f64::consts::FRAC_PI_4, std::f64::consts::FRAC_PI_2, std::f64::consts::PI] {
        for &decay in &[0.5, 0.7, 0.9] {
            for &king in &[1.0, 3.0] {
                let w = Wave { omega, decay, king };
                let read: Vec<(String, Board, V)> = train.clone();
                let (f, s) = crate::crack_invent::invent_on(&read, &|b: &Board| readings(b, w), 12, 2500);
                println!("  wave omega {omega:.2} decay {decay} king {king}: training score {s:.3}");
                if best.as_ref().map_or(true, |x| s > x.0) {
                    best = Some((s, w, f));
                }
            }
        }
    }
    let (s, w, f) = best.unwrap();
    let names = names();
    println!("\nGolden chose the wave omega {:.2}, decay {}, king {} (training score {s:.3}, {:.0} s); its reading:", w.omega, w.decay, w.king, t0.elapsed().as_secs_f64());
    for (k, wt) in f.w.iter().enumerate() {
        if *wt != 0 && k < names.len() {
            println!("  {wt:+} x [{}]", names[k]);
        }
    }
    println!("  win when S >= {}, loss when S <= -{}, else draw", f.win_at, f.loss_at);
    let judge = |name: &str, g: &(dyn Fn(&Board) -> V + Sync)| {
        let right: Vec<bool> = std::thread::scope(|sc| {
            let hs: Vec<_> = smoke.chunks(smoke.len().div_ceil(12)).map(|c| sc.spawn(move || c.iter().map(|r| g(&r.1) == r.2).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut by: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
        for (r, ok) in smoke.iter().zip(&right) {
            let e = by.entry(r.0.as_str()).or_default();
            e.0 += *ok as usize;
            e.1 += 1;
        }
        let total = right.iter().filter(|x| **x).count();
        let mut classes: Vec<&str> = smoke.iter().map(|r| r.0.as_str()).collect();
        classes.dedup();
        println!("\n{name}: {total}/{} ({:.1}%)\n  {}", smoke.len(), 100.0 * total as f64 / smoke.len() as f64, classes.iter().map(|c| format!("{c} {}/{}", by[c].0, by[c].1)).collect::<Vec<_>>().join("  "));
    };
    judge("intuition v0 (yardstick)", &crate::crack::intuition);
    judge("the waves", &|b: &Board| value(b, w, &f));
    for d in 1..=depth {
        judge(&format!("intuition v0 + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &crate::crack::intuition));
        judge(&format!("the waves + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &|x: &Board| value(x, w, &f)));
    }
    Ok(())
}
