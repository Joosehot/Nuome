//! Cracking a position's code: every position has one exact value (Zermelo), a function of its code (the pieces on
//! their squares and the side to move). A table only looks the value up, and knows nothing past what it holds; a
//! cracker computes the value from the code itself. Different ways of cracking, each answering win, draw, loss, or
//! "I don't know", measured where the truth is known (up to five pieces, Syzygy), before going on to the unknown.
//!
//!   rules:     what the rules decide at once (mate, stalemate, too little to mate): exact, or silent;
//!   proof d:   every line d plies deep, nothing but the rules: a win or loss proved inside the horizon, or silent;
//!   intuition: always answers, from the code alone, with no search;
//!   intuition + search d: every line d plies deep, the intuition where the lines stop.
//!
//! `nuome crack-smoke` runs each on bench/crack/smoke.tsv (500 positions, 20 classes, written once).
use crate::golden::Board;

/// A position's value for the side to move: 1 win, 0 draw, -1 loss.
pub type V = i8;

/// What the rules decide at once.
pub fn rules(b: &Board) -> Option<V> {
    if b.insufficient() {
        return Some(0);
    }
    if !b.has_legal() {
        return Some(if b.in_check() { -1 } else { 0 });
    }
    None
}

/// Every line `d` plies deep by the rules alone: Some(value) when the value is proved inside the horizon (a win is a
/// move to a proved loss; a loss is every move to a proved win; a draw only when the rules end the game drawn).
pub fn proof(b: &Board, d: usize) -> Option<V> {
    if let Some(v) = rules(b) {
        return Some(v);
    }
    if d == 0 {
        return None;
    }
    let mut all_won = true;
    for m in b.moves() {
        match proof(&b.play(m), d - 1) {
            Some(-1) => return Some(1),
            Some(1) => {}
            _ => all_won = false,
        }
    }
    all_won.then_some(-1)
}

/// The pieces of one side, by type (1 pawn, 2 knight, 3 bishop, 4 rook, 5 queen, 6 king), with the bishops' square
/// colours.
struct Side {
    count: [u8; 7],
    light_bishop: bool,
    dark_bishop: bool,
}

fn side(b: &Board, white: bool) -> Side {
    let mut s = Side { count: [0; 7], light_bishop: false, dark_bishop: false };
    for (sq, &p) in b.sq.iter().enumerate() {
        if p != 0 && (p > 0) == white {
            let t = p.unsigned_abs() as usize;
            s.count[t] += 1;
            if t == 3 {
                if (sq / 8 + sq % 8) % 2 == 1 { s.light_bishop = true } else { s.dark_bishop = true }
            }
        }
    }
    s
}

impl Side {
    /// can these pieces force mate on a bare king? (known theory, set by hand: a queen, a rook, a pawn that may queen,
    /// bishop and knight, two bishops on both colours)
    fn mates(&self) -> bool {
        self.count[5] > 0 || self.count[4] > 0 || self.count[1] > 0 || (self.count[3] > 0 && self.count[2] > 0) || (self.light_bishop && self.dark_bishop)
    }

    /// material in pawns (Q 9, R 5, B 3, N 3, P 1)
    fn material(&self) -> i32 {
        self.count[1] as i32 + 3 * (self.count[2] + self.count[3]) as i32 + 5 * self.count[4] as i32 + 9 * self.count[5] as i32
    }

    fn bare(&self) -> bool {
        self.count[1..6].iter().all(|&c| c == 0)
    }
}

/// The intuition, v0: the value read from the code without searching. The side with the force to mate and the
/// clearly larger material wins; with no clear edge, a draw. (Every rule here is known theory, set by hand: v0 is the
/// yardstick the inventions must beat.)
pub fn intuition(b: &Board) -> V {
    if let Some(v) = rules(b) {
        return v;
    }
    let (me, them) = (side(b, b.white), side(b, !b.white));
    let wins = |a: &Side, d: &Side| a.mates() && (d.bare() || a.material() - d.material() >= 3);
    if wins(&me, &them) {
        1
    } else if wins(&them, &me) {
        -1
    } else {
        0
    }
}

/// Every line `d` plies deep, the intuition where the lines stop (negamax with alpha-beta on -1..1).
pub fn searched(b: &Board, d: usize, mut alpha: V, beta: V) -> V {
    if let Some(v) = rules(b) {
        return v;
    }
    if d == 0 {
        return intuition(b);
    }
    let mut best = -1;
    for m in b.moves() {
        let v = -searched(&b.play(m), d - 1, -beta, -alpha);
        if v > best {
            best = v;
        }
        if best > alpha {
            alpha = best;
        }
        if alpha >= beta {
            break;
        }
    }
    best
}

fn truth_of(wdl: i32) -> V {
    wdl.signum() as V
}

/// One cracker's tally on the set: answered, right, per class.
#[derive(Default, Clone)]
struct Tally {
    n: usize,
    answered: usize,
    right: usize,
}

/// `nuome crack-smoke [DEPTH]`: every cracker on the smoke set, overall and by class.
pub fn smoke(path: &str, depth: usize) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let rows: Vec<(String, Board, V)> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Ok((f[0].to_string(), Board::from_fen(f[1])?, truth_of(f[2].parse::<i32>().map_err(|e| e.to_string())?)))
        })
        .collect::<Result<_, String>>()?;
    type Cracker<'a> = (String, Box<dyn Fn(&Board) -> Option<V> + Sync + 'a>);
    let mut crackers: Vec<Cracker> = vec![("rules".into(), Box::new(rules))];
    for d in [2, 4] {
        crackers.push((format!("proof {d}"), Box::new(move |b: &Board| proof(b, d))));
    }
    crackers.push(("intuition".into(), Box::new(|b: &Board| Some(intuition(b)))));
    for d in 1..=depth {
        crackers.push((format!("intuition + search {d}"), Box::new(move |b: &Board| Some(searched(b, d, -1, 1)))));
    }
    let mut classes: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    classes.dedup();
    println!("{} positions, {} classes ({path}); truth: Syzygy, value for the side to move", rows.len(), classes.len());
    for (name, crack) in &crackers {
        let t0 = std::time::Instant::now();
        let got: Vec<Option<V>> = std::thread::scope(|sc| {
            let chunks: Vec<_> = rows.chunks(rows.len().div_ceil(12).max(1)).map(|c| sc.spawn(|| c.iter().map(|r| crack(&r.1)).collect::<Vec<_>>())).collect();
            chunks.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut all = Tally::default();
        let mut by: std::collections::BTreeMap<&str, Tally> = Default::default();
        for (r, g) in rows.iter().zip(&got) {
            for t in [&mut all, by.entry(r.0.as_str()).or_default()] {
                t.n += 1;
                if let Some(v) = g {
                    t.answered += 1;
                    t.right += (*v == r.2) as usize;
                }
            }
        }
        let pct = |a: usize, b: usize| 100.0 * a as f32 / b.max(1) as f32;
        println!("\n{name}: answered {}/{} ({:.0}%), right {}/{} of those answered ({:.1}%), {:.1} s", all.answered, all.n, pct(all.answered, all.n), all.right, all.answered, pct(all.right, all.answered), t0.elapsed().as_secs_f64());
        let line: Vec<String> = classes.iter().map(|c| { let t = &by[c.as_str()]; format!("{c} {}/{}", t.right, t.answered) }).collect();
        println!("  {}", line.join("  "));
    }
    Ok(())
}
