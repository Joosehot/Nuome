//! Golden heart, a HotMotor engine for the eval, cloned from Stockfish's
//! games the way Vuuvee clones a voice from a recording.
//!
//!   games (the recording) -> listen: every one of Stockfish's own moves,
//!       every rule's inputs after every legal move (nothing is decided yet)
//!   -> only after every game is heard: the seeds — each input's worth,
//!       measured as how much more Stockfish's move has of it than the
//!       average legal move
//!   -> rules offer variants (how strongly each speaks) -> beam search over
//!       the variants, scored by the copy on part of the recording, the
//!       judges vetoing what cannot be right
//!   -> copy: replay the games never heard and measure the distance to
//!       Stockfish, by phase and by the rule that pulled the copy away
//!   -> freeze: out/golden/heart_rules.toml, every number readable
//!
//! No Stockfish evaluation is used, only the moves.

pub mod rules;

use crate::golden::{Board, Mv};
use rules::{read_all, width, RULES, VARIANTS};

pub const RULES_FILE: &str = "out/golden/heart_rules.toml";
pub const WEIGHTS_FILE: &str = "out/golden/heart_weights.txt";

static FROZEN: std::sync::OnceLock<Option<Vec<f64>>> = std::sync::OnceLock::new();

/// The frozen heart, loaded once (None when it has not been cloned yet).
pub fn frozen() -> Option<&'static [f64]> {
    FROZEN
        .get_or_init(|| {
            let w: Vec<f64> = std::fs::read_to_string(WEIGHTS_FILE).ok()?.lines().filter_map(|l| l.trim().parse().ok()).collect();
            (w.len() == width()).then_some(w)
        })
        .as_deref()
}

/// The static eval: the position's worth for the side to move, in hundredths of
/// the heart's units. The heart was measured from the side that had just moved,
/// so it is read from there and turned around.
pub fn eval_fast(w: &[f64], p: &crate::fast::position::Position) -> i64 {
    let b = crate::fast::board_of(p);
    let sh = rules::Shared { edges: crate::fast::edges_of(p) };
    (-score(w, &rules::read_all_with(&b, !b.white, &sh)) * 100.0).round() as i64
}

pub fn eval(w: &[f64], b: &Board) -> i64 {
    (-score(w, &read_all(b, !b.white)) * 100.0).round() as i64
}
const PHASES: [&str; 4] = ["openings", "middlegames", "endgames", "pawn endings"];

fn phase(b: &Board) -> usize {
    let pieces = b.sq.iter().filter(|&&p| p != 0 && p.abs() != 1 && p.abs() != 6).count();
    match pieces {
        0 => 3,
        1..=4 => 2,
        _ if b.sq.iter().filter(|&&p| p != 0).count() >= 28 => 0,
        _ => 1,
    }
}

/// One of Stockfish's moves as heard: every legal move's inputs (seen from the mover) and which it played.
#[derive(Clone)]
pub struct Decision {
    pub after: Vec<Vec<(u16, f32)>>,
    pub chosen: usize,
    pub phase: usize,
}

fn parse(line: &str) -> Option<((bool, bool), Vec<&str>)> {
    let parts: Vec<&str> = line.split('\t').collect();
    let (colours, moves) = match parts.len() {
        3 => ((parts[0].contains('W'), parts[0].contains('B')), parts[2]),
        2 => ((true, true), parts[1]),
        _ => return None,
    };
    Some((colours, moves.split_whitespace().collect()))
}

/// Walk the games and hand every one of Stockfish's decisions to `f`, on every core.
fn walk<T: Send>(lines: &[&str], init: impl Fn() -> T + Sync, f: impl Fn(&mut T, Decision) + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = lines.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = lines
            .chunks(chunk)
            .map(|part| {
                let (init, f) = (&init, &f);
                sc.spawn(move || {
                    let mut acc = init();
                    for line in part {
                        let Some(((sw, sb), moves)) = parse(line) else { continue };
                        let mut b = Board::start();
                        for u in moves {
                            let ms: Vec<Mv> = b.moves();
                            let Some(ci) = ms.iter().position(|m| m.uci() == u) else { break };
                            if (if b.white { sw } else { sb }) && ms.len() > 1 {
                                let me = b.white;
                                let after = ms.iter().map(|&m| read_all(&b.play(m), me)).collect();
                                f(&mut acc, Decision { after, chosen: ci, phase: phase(&b) });
                            }
                            b = b.play(ms[ci]);
                        }
                    }
                    acc
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("walk")).collect()
    })
}

/// Everything heard: per input, (Stockfish's move) minus (the mean of all moves), summed; and a sample of decisions for the search.
struct Heard {
    diff: Vec<f64>,
    n: usize,
    sample: Vec<Decision>,
}

fn listen(lines: &[&str], sample_every: usize) -> Heard {
    let w = width();
    let parts = walk(
        lines,
        || Heard { diff: vec![0.0; w], n: 0, sample: Vec::new() },
        |h, d| {
            for &(i, v) in &d.after[d.chosen] {
                h.diff[i as usize] += v as f64;
            }
            let k = d.after.len() as f64;
            for x in &d.after {
                for &(i, v) in x {
                    h.diff[i as usize] -= v as f64 / k;
                }
            }
            h.n += 1;
            if h.n % sample_every == 0 {
                h.sample.push(d);
            }
        },
    );
    let mut all = Heard { diff: vec![0.0; w], n: 0, sample: Vec::new() };
    for p in parts {
        for i in 0..w {
            all.diff[i] += p.diff[i];
        }
        all.n += p.n;
        all.sample.extend(p.sample);
    }
    all
}

fn score(w: &[f64], x: &[(u16, f32)]) -> f64 {
    x.iter().map(|&(i, v)| w[i as usize] * v as f64).sum()
}

fn pick(w: &[f64], d: &Decision) -> usize {
    let mut best = (f64::NEG_INFINITY, 0);
    for (i, x) in d.after.iter().enumerate() {
        let s = score(w, x);
        if s > best.0 {
            best = (s, i);
        }
    }
    best.1
}

/// The weights a choice of variants gives: each rule's seeds times its strength.
fn weights(seeds: &[f64], strength: &[f32]) -> Vec<f64> {
    let mut w = vec![0.0; seeds.len()];
    for (r, rule) in RULES.iter().enumerate() {
        for i in rule.start..rule.start + rule.len {
            w[i] = seeds[i] * strength[r] as f64;
        }
    }
    w
}

/// The seeds measured as the likeliest account of Stockfish's choices: the
/// worths under which the moves it played are most probably the best of all
/// legal moves (a conditional logit), from the decisions heard across every
/// game. Fitted only after the whole recording has been heard.
fn fit_seeds(sample: &[Decision], width: usize, epochs: usize) -> Vec<f64> {
    let mut w = vec![0.0f64; width];
    for e in 0..epochs {
        let lr = 0.05 / (1.0 + e as f64 * 0.3);
        for d in sample {
            let s: Vec<f64> = d.after.iter().map(|x| score(&w, x)).collect();
            let m = s.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let ex: Vec<f64> = s.iter().map(|v| (v - m).exp()).collect();
            let z: f64 = ex.iter().sum();
            for (j, x) in d.after.iter().enumerate() {
                let g = (j == d.chosen) as u8 as f64 - ex[j] / z;
                if g.abs() < 1e-7 {
                    continue;
                }
                for &(i, v) in x {
                    w[i as usize] += lr * g * v as f64;
                }
            }
        }
        // a little shrinkage keeps rarely heard inputs near zero
        for x in w.iter_mut() {
            *x *= 1.0 - 1e-4;
        }
    }
    w
}

/// A piece's worth as the heart sees it: its material seed plus the mean of its placement seeds (mine minus theirs, halved).
fn piece_worth(seeds: &[f64], t: usize) -> f64 {
    let pl = &RULES[1];
    let mean = |whose: usize| (0..64).map(|sq| seeds[pl.start + (whose * 6 + t) * 64 + sq]).sum::<f64>() / 64.0;
    seeds[t] + (mean(0) - mean(1)) / 2.0
}

/// The judges: a heart where material is silent, or values a queen below a rook or a minor piece below a pawn, cannot be right.
fn judges(seeds: &[f64], strength: &[f32]) -> Option<&'static str> {
    if strength[0] == 0.0 {
        return Some("material is silent");
    }
    let m: Vec<f64> = (0..5).map(|t| piece_worth(seeds, t)).collect();
    if !(m[4] > m[3] && m[3] > m[1].max(m[2]) && m[1].min(m[2]) > m[0] && m[0] > 0.0) {
        return Some("the measured piece worths are out of order (Q > R > N, B > P > 0)");
    }
    None
}

/// Beam search over the rules' variants, in the rules' order, scored by how often the copy plays Stockfish's move on the sample.
fn search(seeds: &[f64], sample: &[Decision], beam: usize) -> (Vec<f32>, f64, Option<&'static str>) {
    let agree = |s: &[f32]| {
        let w = weights(seeds, s);
        sample.iter().filter(|d| pick(&w, d) == d.chosen).count() as f64 / sample.len().max(1) as f64
    };
    let mut partial: Vec<(Vec<f32>, f64)> = vec![(Vec::new(), 0.0)];
    for r in 0..RULES.len() {
        let mut next = Vec::new();
        for (p, _) in &partial {
            for &v in &VARIANTS {
                let mut s = p.clone();
                s.push(v);
                // rules not yet decided speak at full strength while this one is tried
                let mut full = s.clone();
                full.resize(RULES.len(), 1.0);
                if judges(seeds, &full).is_some() {
                    continue;
                }
                next.push((s, agree(&full)));
            }
        }
        // ties keep the order the variants were offered in
        next.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        next.truncate(beam);
        if next.is_empty() {
            let mut full = vec![1.0; RULES.len()];
            full[0] = 1.0;
            return (full.clone(), agree(&full), judges(seeds, &full));
        }
        partial = next;
    }
    let (s, a) = partial[0].clone();
    (s, a, None)
}

pub struct Report {
    pub text: String,
}

pub fn clone_heart(path: &str, max_games: usize) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).take(max_games).collect();
    let cut = if lines.len() < 100 { lines.len() * 3 / 4 } else { lines.len() * 95 / 100 };
    let (voice, check) = lines.split_at(cut);
    // 1. listen to every game of the voice; nothing is decided while listening
    // a sample spread over every game (about 40 000 of Stockfish's moves) is kept for the fitting and the search
    let heard = listen(voice, (voice.len() * 60 / 40_000).max(1));
    // 2. only now, every game heard, the seeds: the likeliest worths behind Stockfish's choices
    let seeds = fit_seeds(&heard.sample, width(), 12);
    let mut out = format!("Golden heart, HotMotor, cloned from {} of Stockfish's games ({} of its moves heard, every game before any seed; {} games never heard kept for the check)\n", voice.len(), heard.n, check.len());
    out.push_str(&format!("measured worths, relative to a pawn (fitted on {} of its moves sampled across every game):\n", heard.sample.len()));
    let pawn = piece_worth(&seeds, 0).abs().max(1e-9);
    for t in 0..5 {
        out.push_str(&format!("  {}: {:.2}\n", ["pawn", "knight", "bishop", "rook", "queen"][t], piece_worth(&seeds, t) / pawn));
    }
    out.push_str(&format!("  bishop pair: {:.2}\n", seeds[5] / pawn));
    // 3. the beam search over the variants, the judges vetoing
    let (strength, sample_agree, veto) = search(&seeds, &heard.sample, 8);
    if let Some(v) = veto {
        out.push_str(&format!("the judges vetoed every chain: {v}\n"));
    }
    out.push_str(&format!("chosen strengths (on {} sampled moves of the voice: the copy plays Stockfish's move in {:.1}%):\n", heard.sample.len(), 100.0 * sample_agree));
    for (r, rule) in RULES.iter().enumerate() {
        out.push_str(&format!("  {}: x{}\n", rule.name, strength[r]));
    }
    let w = weights(&seeds, &strength);
    // 4. copy: the games never heard; the distance by phase and by the rule that pulled the copy away
    let parts = walk(
        check,
        || ([0usize; 4], [0usize; 4], vec![0usize; RULES.len()]),
        |acc, d| {
            acc.0[d.phase] += 1;
            let p = pick(&w, &d);
            if p == d.chosen {
                acc.1[d.phase] += 1;
            } else {
                // the rule whose score favours the copy's move over Stockfish's the most
                let per_rule = |x: &[(u16, f32)], r: usize| x.iter().filter(|&&(i, _)| (i as usize) >= RULES[r].start && (i as usize) < RULES[r].start + RULES[r].len).map(|&(i, v)| w[i as usize] * v as f64).sum::<f64>();
                let worst = (0..RULES.len()).max_by(|&a, &b| {
                    let da = per_rule(&d.after[p], a) - per_rule(&d.after[d.chosen], a);
                    let db = per_rule(&d.after[p], b) - per_rule(&d.after[d.chosen], b);
                    da.partial_cmp(&db).unwrap()
                });
                if let Some(r) = worst {
                    acc.2[r] += 1;
                }
            }
        },
    );
    let (mut n, mut ok, mut blame) = ([0usize; 4], [0usize; 4], vec![0usize; RULES.len()]);
    for (a, b, c) in parts {
        for p in 0..4 {
            n[p] += a[p];
            ok[p] += b[p];
        }
        for r in 0..RULES.len() {
            blame[r] += c[r];
        }
    }
    let total: usize = n.iter().sum();
    let agreed: usize = ok.iter().sum();
    out.push_str(&format!("copy on the {} games never heard: Stockfish's move in {:.1}% of {} moves\n", check.len(), 100.0 * agreed as f64 / total.max(1) as f64, total));
    for p in 0..4 {
        if n[p] > 0 {
            out.push_str(&format!("  {}: {:.1}% of {}\n", PHASES[p], 100.0 * ok[p] as f64 / n[p] as f64, n[p]));
        }
    }
    out.push_str("where the copy left Stockfish, the rule that pulled it away most:\n");
    for (r, rule) in RULES.iter().enumerate() {
        out.push_str(&format!("  {}: {} moves\n", rule.name, blame[r]));
    }
    // 5. freeze
    let mut toml = format!("# Golden heart, measured from {} of Stockfish's moves in {} games ({path})\n", heard.n, voice.len());
    for (r, rule) in RULES.iter().enumerate() {
        toml.push_str(&format!("\n[{}]\nstrength = {}\n", rule.name, strength[r]));
        for i in 0..rule.len {
            toml.push_str(&format!("\"{}\" = {:.6}\n", (rule.label)(i), seeds[rule.start + i]));
        }
    }
    std::fs::write(RULES_FILE, toml).map_err(|e| e.to_string())?;
    // the frozen weights (seeds times strength) for the eval, one a line in input order
    std::fs::write(WEIGHTS_FILE, w.iter().map(|x| format!("{x:.8}\n")).collect::<String>()).map_err(|e| e.to_string())?;
    out.push_str(&format!("frozen in {RULES_FILE} ({:.0} s)\n", t0.elapsed().as_secs_f64()));
    Ok(out)
}
