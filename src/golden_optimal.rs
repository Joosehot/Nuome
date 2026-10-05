//! What optimal play is in the middlegame and in endgames of six or more
//! pieces: Golden Answer's abstract answer (a guess with its reasons, which
//! claims nothing).
//!
//! Golden looks at Stockfish's moves (Hugging Face agi-noobs/chess-sft-10m-
//! stockfish, out/golden/hf_mid_end.tsv) through the five checks of every
//! legal move (golden_brute::Light: safety, force, check, material, tempo;
//! legality is given, only legal moves are looked at) and sees, for each
//! check, how far Stockfish's move stands from the average legal move, in
//! units of the check's spread. Its guess F weighs each check by that: a
//! check that sets the optimal moves apart strongly counts strongly. F's
//! agreement with Stockfish on positions Golden did not look at is shown
//! for information; an abstract answer is never scored.

use crate::fast::movegen::legal_moves;
use crate::fast::position::{move_to_uci, Position};
use crate::golden_brute::light_on;

pub const QUESTION: &str = "What is optimal play in the middlegame and in endgames of six or more pieces? Give one function F of a position and a move that says what makes a move optimal, so that the move with the highest F is the optimal one. Check F against Stockfish's moves on positions it never saw.";
pub const NAMES: [&str; 5] = ["safety", "force", "check", "material", "tempo"];
const LOOK: usize = 6_000;
const SHOW: usize = 3_000;

pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// The five checks of every legal move, and the index of Stockfish's move.
fn checks(fen: &str, sf: &str) -> Option<(Vec<[f64; 5]>, usize)> {
    let mut p = Position::from_fen(fen).ok()?;
    let mut xs = Vec::new();
    let mut chosen = None;
    let before = {
        let side = p.turn as usize;
        let w = [100i32, 325, 325, 500, 975, 0];
        let mut v = [0i32; 2];
        for c in 0..2 {
            for t in 0..6 {
                v[c] += w[t] * p.bb[c * 6 + t].count_ones() as i32;
            }
        }
        v[side] - v[1 - side]
    };
    for (i, m) in legal_moves(&mut p).into_iter().enumerate() {
        if move_to_uci(m) == sf {
            chosen = Some(i);
        }
        let l = light_on(&mut p, m);
        xs.push([l.safety as f64, l.force as f64, l.check as u8 as f64, (l.material - before) as f64, l.tempo as f64]);
    }
    Some((xs, chosen?))
}

/// The positions of one phase, split by line number: the first LOOK to look at, the next SHOW to show.
fn rows(phase: &str) -> Result<Vec<(String, String)>, String> {
    let text = std::fs::read_to_string(crate::golden_table::HF_FILE).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in text.lines() {
        let mut it = line.splitn(3, '\t');
        let (Some(ph), Some(mv), Some(fen)) = (it.next(), it.next(), it.next()) else { continue };
        if ph == phase {
            out.push((fen.to_string(), mv.to_string()));
            if out.len() >= LOOK + SHOW {
                break;
            }
        }
    }
    Ok(out)
}

fn all_checks(set: &[(String, String)]) -> Vec<(Vec<[f64; 5]>, usize)> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = set.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let hs: Vec<_> = set.chunks(chunk).map(|c| s.spawn(move || c.iter().filter_map(|(f, m)| checks(f, m)).collect::<Vec<_>>())).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

/// What Golden sees in one phase: per check, how far Stockfish's move stands from the average legal move (in spreads), and F's weights.
fn look(seen: &[(Vec<[f64; 5]>, usize)]) -> ([f64; 5], [f64; 5]) {
    let (mut sum, mut sq, mut n) = ([0.0; 5], [0.0; 5], 0.0);
    for (xs, _) in seen {
        for x in xs {
            for k in 0..5 {
                sum[k] += x[k];
                sq[k] += x[k] * x[k];
            }
            n += 1.0;
        }
    }
    let sd: Vec<f64> = (0..5).map(|k| (sq[k] / n - (sum[k] / n).powi(2)).max(1e-9).sqrt()).collect();
    let mut stand = [0.0; 5];
    for (xs, c) in seen {
        let mean: Vec<f64> = (0..5).map(|k| xs.iter().map(|x| x[k]).sum::<f64>() / xs.len() as f64).collect();
        for k in 0..5 {
            stand[k] += (xs[*c][k] - mean[k]) / sd[k];
        }
    }
    let m = seen.len().max(1) as f64;
    let mut w = [0.0; 5];
    for k in 0..5 {
        stand[k] /= m;
        w[k] = stand[k] / sd[k];
    }
    (stand, w)
}

fn f_pick(w: &[f64; 5], xs: &[[f64; 5]]) -> usize {
    let mut best = (f64::NEG_INFINITY, 0);
    for (i, x) in xs.iter().enumerate() {
        let v: f64 = (0..5).map(|k| w[k] * x[k]).sum();
        if v > best.0 {
            best = (v, i);
        }
    }
    best.1
}

/// Golden Answer's answer: abstract only.
pub fn answer(abstract_: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if !abstract_ {
        return Err("only an abstract answer is asked of this question".into());
    }
    let t0 = std::time::Instant::now();
    let mut lines = Vec::new();
    let mut short = Vec::new();
    for (phase, name) in [("middlegame", "the middlegame"), ("endgame", "endgames of six or more pieces")] {
        let set = rows(phase)?;
        let (look_set, show_set) = set.split_at(LOOK.min(set.len()));
        let seen = all_checks(look_set);
        let shown = all_checks(show_set);
        let (stand, w) = look(&seen);
        let mut order: Vec<usize> = (0..5).collect();
        order.sort_by(|&a, &b| stand[b].abs().total_cmp(&stand[a].abs()));
        let f: Vec<String> = order.iter().filter(|&&k| stand[k].abs() >= 0.05).map(|&k| format!("{:+.4}·{}", w[k], NAMES[k])).collect();
        short.push(format!("in {name}: F = {}", f.join(" ")));
        lines.push(format!("{name}: Golden looked at {} positions; Stockfish's move against the average legal move, in spreads:", seen.len()));
        for &k in &order {
            lines.push(format!("    {}: {:+.2} ({})", NAMES[k], stand[k], if stand[k] > 0.05 { "the optimal move has more of it" } else if stand[k] < -0.05 { "the optimal move has less of it" } else { "it does not set the optimal move apart" }));
        }
        let hit = shown.iter().filter(|(xs, c)| f_pick(&w, xs) == *c).count();
        lines.push(format!("    for information (not a score): on {} positions it did not look at, F's move is Stockfish's in {:.1}%", shown.len(), 100.0 * hit as f64 / shown.len().max(1) as f64));
    }
    lines.push("so the guess: optimal play is the move that does most of what the optimal moves were seen to do, weighed by how clearly each sets them apart".into());
    lines.push(format!("this is an answer that claims nothing ({:.0} s)", t0.elapsed().as_secs_f64()));
    Ok((short.join("; "), lines, vec![]))
}
