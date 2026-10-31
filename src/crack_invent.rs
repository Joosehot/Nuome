//! Golden invents the function of a position's code from the owner's four parts: place, movement, value, relation.
//!
//! The raw numbers are read from the code by the rules alone, each named by its part (from the side to move's view,
//! "me" against "them"). Golden then invents how they combine: a weighted sum S of whole-number weights and two
//! thresholds (S at or above the first: a win; at or below minus the second: a loss; between: a draw), found by
//! climbing from random starts on bench/crack/train.tsv only (2000 positions); a shorter function wins a tie. The
//! invention is judged on bench/crack/smoke.tsv, never seen while inventing, alone and with a short search, against
//! the intuition v0.
use crate::crack::{rules, searched_with, V};
use crate::golden::{attack_edges, Board};

/// the raw numbers: (name, part)
pub const FEATURES: [(&str, &str); 27] = [
    ("queens (me - them)", "value"),
    ("rooks (me - them)", "value"),
    ("bishops (me - them)", "value"),
    ("knights (me - them)", "value"),
    ("pawns (me - them)", "value"),
    ("I have a bare king", "value"),
    ("they have a bare king", "value"),
    ("squares I attack - squares they attack", "movement"),
    ("moves my best pawn needs to queen", "movement"),
    ("moves their best pawn needs to queen", "movement"),
    ("my best pawn's rank", "place"),
    ("their best pawn's rank", "place"),
    ("my king's distance from the edge", "place"),
    ("their king's distance from the edge", "place"),
    ("my best pawn on an edge file", "place"),
    ("their best pawn on an edge file", "place"),
    ("race: their first arrival at my pawn's queening square - my pawn's moves", "relation"),
    ("race: my first arrival at their pawn's queening square - their pawn's moves", "relation"),
    ("my pieces they attack and I do not defend", "relation"),
    ("their pieces I attack and they do not defend", "relation"),
    ("distance between the kings", "relation"),
    ("my king's distance to my best pawn", "relation"),
    ("their king's distance to my best pawn", "relation"),
    ("their king's distance to their best pawn", "relation"),
    ("my king's distance to their best pawn", "relation"),
    ("kings face each other, two squares apart, and I must move", "relation"),
    ("I am in check", "relation"),
];
pub const N: usize = FEATURES.len();

fn dist(a: i32, b: i32) -> i32 {
    ((a % 8) - (b % 8)).abs().max((a / 8 - b / 8).abs())
}

fn edge(s: i32) -> i32 {
    (s % 8).min(7 - s % 8).min(s / 8).min(7 - s / 8)
}

/// a side's most advanced pawn: (square, moves to queen with the side to move's tempo counted), or None
fn best_pawn(b: &Board, white: bool) -> Option<(i32, i32)> {
    (0..64)
        .filter(|&s| b.sq[s as usize] == if white { 1 } else { -1 })
        .map(|s| {
            let rank = s / 8;
            let to_go = if white { 7 - rank } else { rank };
            let first = if white { rank == 1 } else { rank == 6 };
            (s, to_go - first as i32 - (b.white == white) as i32)
        })
        .min_by_key(|x| x.1)
}

/// the raw numbers of a position, from the side to move's view
pub fn features(b: &Board) -> [i32; N] {
    let me = b.white;
    let e = attack_edges(b);
    let (mi, ti) = (if me { 0 } else { 1 }, if me { 1 } else { 0 });
    let count = |white: bool, t: i8| (0..64).filter(|&s| b.sq[s] == if white { t } else { -t }).count() as i32;
    let bare = |white: bool| (1..=5).all(|t| count(white, t) == 0) as i32;
    let attacked = |side: usize| e[side].iter().filter(|&&n| n > 0).count() as i32;
    let (mk, tk) = (b.king(me), b.king(!me));
    let (mp, tp) = (best_pawn(b, me), best_pawn(b, !me));
    // the first arrival of a side's pieces at a square: its king's steps; a piece that already strikes it, 1
    let arrival = |white: bool, sq: i32| -> i32 {
        let side = if white { 0 } else { 1 };
        let k = dist(b.king(white), sq);
        if e[side][sq as usize] > 0 && b.sq.iter().enumerate().any(|(s, &p)| p != 0 && (p > 0) == white && p.abs() != 6 && p.abs() != 1 && s as i32 != sq) {
            k.min(1)
        } else {
            k
        }
    };
    let race = |pawn: Option<(i32, i32)>, white: bool| -> i32 {
        match pawn {
            Some((s, t)) => {
                let q = (if white { 56 } else { 0 }) + s % 8;
                (arrival(!white, q) - t).clamp(-6, 6)
            }
            None => 0,
        }
    };
    let hanging = |white: bool| -> i32 {
        let (own, other) = if white { (0, 1) } else { (1, 0) };
        (0..64).filter(|&s| b.sq[s] != 0 && (b.sq[s] > 0) == white && b.sq[s].abs() != 6 && e[other][s] > 0 && e[own][s] == 0).count() as i32
    };
    let rank_of = |p: Option<(i32, i32)>, white: bool| p.map_or(0, |(s, _)| if white { s / 8 } else { 7 - s / 8 });
    let faces = {
        let (df, dr) = ((mk % 8 - tk % 8).abs(), (mk / 8 - tk / 8).abs());
        ((df == 0 && dr == 2) || (dr == 0 && df == 2)) as i32
    };
    let dk = |k: i32, p: Option<(i32, i32)>| p.map_or(0, |(s, _)| dist(k, s));
    [
        count(me, 5) - count(!me, 5),
        count(me, 4) - count(!me, 4),
        count(me, 3) - count(!me, 3),
        count(me, 2) - count(!me, 2),
        count(me, 1) - count(!me, 1),
        bare(me),
        bare(!me),
        attacked(mi) - attacked(ti),
        mp.map_or(9, |p| p.1),
        tp.map_or(9, |p| p.1),
        rank_of(mp, me),
        rank_of(tp, !me),
        edge(mk),
        edge(tk),
        mp.map_or(0, |(s, _)| (s % 8 == 0 || s % 8 == 7) as i32),
        tp.map_or(0, |(s, _)| (s % 8 == 0 || s % 8 == 7) as i32),
        race(mp, me),
        race(tp, !me),
        hanging(me),
        hanging(!me),
        dist(mk, tk),
        dk(mk, mp),
        dk(tk, mp),
        dk(tk, tp),
        dk(mk, tp),
        faces,
        b.in_check() as i32,
    ]
}

/// an invented function: whole-number weights and the two thresholds
#[derive(Clone)]
pub struct Invented {
    pub w: [i32; N],
    pub win_at: i32,
    pub loss_at: i32,
}

impl Invented {
    pub fn value(&self, x: &[i32; N]) -> V {
        let s: i32 = self.w.iter().zip(x).map(|(a, b)| a * b).sum();
        if s >= self.win_at { 1 } else if s <= -self.loss_at { -1 } else { 0 }
    }

    pub fn of(&self, b: &Board) -> V {
        rules(b).unwrap_or_else(|| self.value(&features(b)))
    }

    pub fn size(&self) -> usize {
        self.w.iter().filter(|&&w| w != 0).count()
    }

    pub fn show(&self) -> String {
        let mut by_part: Vec<String> = Vec::new();
        for part in ["place", "movement", "value", "relation"] {
            let terms: Vec<String> = self.w.iter().zip(FEATURES).filter(|(w, f)| **w != 0 && f.1 == part).map(|(w, f)| format!("{w:+} x [{}]", f.0)).collect();
            if !terms.is_empty() {
                by_part.push(format!("  {part}:\n    {}", terms.join("\n    ")));
            }
        }
        format!("S =\n{}\nwin when S >= {}, loss when S <= -{}, else draw", by_part.join("\n"), self.win_at, self.loss_at)
    }
}

/// a small fixed generator (xorshift), so every invention can be made again
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo + 1) as u64) as i32
    }
}

fn load(path: &str) -> Result<Vec<(String, Board, V)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Ok((f[0].to_string(), Board::from_fen(f[1])?, f[2].parse::<i32>().map_err(|e| e.to_string())?.signum() as V))
        })
        .collect()
}

/// Golden's invention: climbing from random starts on the training set; the score is the share right, and a shorter
/// function wins a tie
pub fn invent(train: &[(String, Board, V)], restarts: usize, steps: usize) -> (Invented, f64) {
    invent_on(train, &features, restarts, steps)
}

/// the same invention on any raw numbers of a position (the waves' readings, ...)
pub fn invent_on(train: &[(String, Board, V)], read: &(dyn Fn(&Board) -> [i32; N] + Sync), restarts: usize, steps: usize) -> (Invented, f64) {
    let xs: Vec<([i32; N], V, Option<V>)> = std::thread::scope(|sc| {
        let hs: Vec<_> = train.chunks(train.len().div_ceil(12).max(1)).map(|c| sc.spawn(move || c.iter().map(|r| (read(&r.1), r.2, rules(&r.1))).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let score = |f: &Invented| -> f64 {
        let right = xs.iter().filter(|(x, v, ru)| ru.unwrap_or_else(|| f.value(x)) == *v).count();
        right as f64 / xs.len() as f64 - 0.0005 * f.size() as f64
    };
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut best = Invented { w: [0; N], win_at: 1, loss_at: 1 };
    let mut best_s = score(&best);
    for _ in 0..restarts {
        let mut f = Invented { w: [0; N], win_at: rng.range(1, 20), loss_at: rng.range(1, 20) };
        for w in f.w.iter_mut() {
            if rng.range(0, 3) == 0 {
                *w = rng.range(-4, 4);
            }
        }
        let mut s = score(&f);
        for _ in 0..steps {
            let mut g = f.clone();
            match rng.range(0, N as i32 + 1) {
                k if (k as usize) < N => g.w[k as usize] = (g.w[k as usize] + rng.range(-2, 2)).clamp(-9, 9),
                k if k as usize == N => g.win_at = (g.win_at + rng.range(-3, 3)).max(1),
                _ => g.loss_at = (g.loss_at + rng.range(-3, 3)).max(1),
            }
            let gs = score(&g);
            if gs >= s {
                f = g;
                s = gs;
            }
        }
        if s > best_s {
            best = f;
            best_s = s;
        }
    }
    (best, best_s)
}

/// `nuome --ideas crack-invent [DEPTH]`: Golden invents on train, judged on smoke
pub fn run(depth: usize) -> Result<(), String> {
    let train = load("bench/crack/train.tsv")?;
    let smoke = load("bench/crack/smoke.tsv")?;
    let t0 = std::time::Instant::now();
    let (f, s) = invent(&train, 40, 4000);
    println!("Golden invented on {} training positions in {:.1} s (training score {:.3}, {} of {} raw numbers used):\n{}", train.len(), t0.elapsed().as_secs_f64(), s, f.size(), N, f.show());
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
        println!("\n{name}: {total}/{} right ({:.1}%) on the smoke set, never seen while inventing", smoke.len(), 100.0 * total as f64 / smoke.len() as f64);
        let mut classes: Vec<&str> = smoke.iter().map(|r| r.0.as_str()).collect();
        classes.dedup();
        println!("  {}", classes.iter().map(|c| format!("{c} {}/{}", by[c].0, by[c].1)).collect::<Vec<_>>().join("  "));
    };
    judge("intuition v0 (yardstick)", &crate::crack::intuition);
    judge("Golden's invention", &|b: &Board| f.of(b));
    for d in 1..=depth {
        judge(&format!("intuition v0 + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &crate::crack::intuition));
        judge(&format!("Golden's invention + search {d}"), &|b: &Board| searched_with(b, d, -1, 1, &|x: &Board| f.of(x)));
    }
    Ok(())
}
