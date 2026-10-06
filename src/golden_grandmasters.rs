//! GoldenGrandmasters: ten judges, each the known theory and style of one
//! grandmaster, order the candidates (the HotMotor funnel).
//!
//! The candidates are Golden's and StarNNUE's three best moves. Each gets
//! StarNNUE's search value (the same time each). The judges see and rank
//! every candidate; only a sound one, within `sound` cp of the best (the hard
//! check), may be played: the judges never play a move the search shows
//! clearly worse.
//!
//! All ten stand on the same theory (data/golden_theory.toml, one rule per
//! measured principle in src/golden_theory.rs); each one's style is its
//! weights on the principles (data/golden_grandmasters.toml). A judge scores
//! a candidate by the sum of weight x rule and ranks them; the ranks are added
//! (Borda). The move with the most points is played; ties by the search value.
//! Every judge adds `starnnue_bonus` to StarNNUE's best moves by their rank
//! (S1, S2, S3), so StarNNUE's first move is the one to beat.

use crate::fast::movegen::{legal_moves, uci_to_move};
use crate::fast::position::Position;
use crate::golden_theory::{rule, Rule, Seen};
use crate::star_search::Eval as _;

pub const GRANDMASTERS_FILE: &str = "data/golden_grandmasters.toml";

/// One grandmaster: the name and the weighted rules.
pub struct Grandmaster {
    pub name: String,
    pub weights: Vec<(String, f32, Rule)>,
}

/// The judges, `sound` and `starnnue_bonus` (cp by StarNNUE's rank) from data/golden_grandmasters.toml; every weighted id must have its rule.
pub fn load() -> Result<(i32, Vec<i32>, Vec<Grandmaster>), String> {
    let text = std::fs::read_to_string(GRANDMASTERS_FILE).map_err(|e| e.to_string())?;
    let t: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
    let sound = t.get("sound").and_then(|v| v.as_integer()).ok_or("no sound")? as i32;
    let bonus: Vec<i32> = t.get("starnnue_bonus").and_then(|v| v.as_array()).ok_or("no starnnue_bonus")?.iter().map(|x| x.as_integer().map(|i| i as i32).ok_or("starnnue_bonus: not a number")).collect::<Result<_, _>>()?;
    let order = ["Tal", "Botvinnik", "Morphy", "Caruana", "Carlsen", "Karpov", "Kasparov", "Fischer", "Anand", "Kramnik"];
    let gms = t.get("grandmaster").and_then(|v| v.as_table()).ok_or("no grandmasters")?;
    let mut names: Vec<&String> = gms.keys().collect();
    names.sort_by_key(|n| order.iter().position(|o| o == n).unwrap_or(99));
    let mut out = Vec::new();
    for name in names {
        let w = gms[name].get("weights").and_then(|v| v.as_table()).ok_or(format!("{name}: no weights"))?;
        let mut weights = Vec::new();
        for (id, v) in w {
            let x = v.as_float().or(v.as_integer().map(|i| i as f64)).ok_or(format!("{name}.{id}: not a number"))? as f32;
            weights.push((id.clone(), x, rule(id).ok_or(format!("{name}: {id} has no rule"))?));
        }
        out.push(Grandmaster { name: name.clone(), weights });
    }
    Ok((sound, bonus, out))
}

/// The value of what `m` takes in `p` (pawns), 0 for a quiet move.
fn taken(p: &Position, m: u32) -> f32 {
    match crate::fast::position::m_flag(m) {
        2 => 1.0,
        1 => p.type_at64(crate::fast::bitboard::SQ120_TO_64[crate::fast::position::m_to(m) as usize] as usize).map(|t| [1.0, 3.25, 3.25, 5.0, 9.75, 0.0][(t as usize - 1).min(5)]).unwrap_or(0.0),
        _ => 0.0,
    }
}

/// The replies at the horizon after a capture worth `v`: captures that win back at least as much.
fn worth_answering(q: &Position, replies: &[u32], v: f32) -> Vec<u32> {
    replies.iter().copied().filter(|&r| taken(q, r) >= v).collect()
}

/// A judge's theory score of `m`: its weighted rules, minus the opponent's
/// best answer by the same judge (`plies` deep); a mate is worth 1000. At
/// the horizon an exchange is played out (recaptures on the same square,
/// standing pat allowed), so it is never cut in the middle.
fn ahead(gm: &Grandmaster, p: &Position, m: u32, plies: usize) -> f32 {
    ahead_caps(gm, p, m, plies, 0)
}

fn ahead_caps(gm: &Grandmaster, p: &Position, m: u32, plies: usize, caps: usize) -> f32 {
    let s = Seen::new(p, m);
    let mine: f32 = gm.weights.iter().map(|(_, w, r)| w * r(&s)).sum();
    let mut q = s.after.clone();
    let replies = legal_moves(&mut q);
    if replies.is_empty() {
        return if q.in_check() { mine + 1000.0 } else { mine };
    }
    let best = if plies <= 1 {
        // an exchange is played out: after a capture, captures winning back at least as much (at most 4 deep), standing pat allowed
        let v = taken(p, m);
        if v > 0.0 && caps < 4 { worth_answering(&q, &replies, v).into_iter().map(|r| ahead_caps(gm, &q, r, 1, caps + 1)).fold(0.0, f32::max) } else { 0.0 }
    } else {
        replies.iter().map(|&r| ahead_caps(gm, &q, r, plies - 1, 0)).fold(f32::MIN, f32::max)
    };
    mine - best
}

/// The Texel judge's view of `moves` in `fen`: the one-ply score, the opponent's best answer by the same theory and its score, the two-ply score.
pub fn explain(fen: &str, moves: &[String]) -> Result<String, String> {
    let lw = load_texel()?;
    let gm = Grandmaster { name: "Texel".into(), weights: lw.iter().filter_map(|(id, x)| crate::golden_theory::rule(id).map(|r| (id.clone(), *x, r))).collect() };
    let mut p = Position::from_fen(fen)?;
    let mut out = String::new();
    for u in moves {
        let m = uci_to_move(&mut p, u).ok_or(format!("illegal {u}"))?;
        let s = Seen::new(&p, m);
        let mut parts: Vec<(f32, &str)> = gm.weights.iter().map(|(id, w, r)| (w * r(&s), id.as_str())).filter(|x| x.0.abs() > 0.05).collect();
        parts.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
        let one: f32 = gm.weights.iter().map(|(_, w, r)| w * r(&s)).sum();
        let mut q = s.after.clone();
        let reply = legal_moves(&mut q).into_iter().map(|r| (ahead(&gm, &q, r, 1), r)).max_by(|a, b| a.0.total_cmp(&b.0));
        let (rs, rm) = reply.map(|(v, r)| (v, crate::fast::position::move_to_uci(r))).unwrap_or((0.0, "-".into()));
        out.push_str(&format!("{u}: one ply {one:+.2} [{}]; their best answer {rm} {rs:+.2}; two plies {:+.2}
", parts.iter().take(4).map(|(v, id)| format!("{id} {v:+.2}")).collect::<Vec<_>>().join(", "), one - rs));
    }
    Ok(out)
}

/// Golden Boy's move by the ten grandmasters: (move, the explanation).
pub fn gm_move(fen: &str, ms: u64, only: &[String]) -> Result<(String, String), String> {
    if let Some(m) = crate::golden_grow::table_move_pub(fen) {
        return Ok((m, "the opening table: a known line".into()));
    }
    let gw: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    let ev = crate::star_search::GoldennetEval(gw);
    let net = crate::fast::starnnue::net()?;
    // the candidates (half the time), then each one's value (the other half, the same for each)
    let golden = crate::star_search::best_moves(&ev, None, fen, ms / 4, 3);
    let star = crate::star_search::best_moves(&ev, Some((net, 1.0)), fen, ms / 4, 3);
    let mut cands: Vec<String> = Vec::new();
    for m in star.iter().chain(golden.iter()) {
        if !cands.contains(m) {
            cands.push(m.clone());
        }
    }
    let each = (ms / 2) / cands.len().max(1) as u64;
    let mut p = Position::from_fen(fen)?;
    let mut vals: Vec<(String, i32)> = Vec::new();
    for c in &cands {
        let mv = uci_to_move(&mut p, c).ok_or("illegal")?;
        p.make(mv);
        let after = p.to_fen();
        let v = if legal_moves(&mut p).is_empty() {
            if p.in_check() { 1_000_000 } else { 0 }
        } else {
            -crate::star_search::analyse(&ev, Some((net, 1.0)), &after, each).map(|r| r.1).unwrap_or(0)
        };
        p.unmake();
        vals.push((c.clone(), v));
    }
    let (sound_cp, bonus, mut judges) = load()?;
    // only the named judges; "theory" is the shared theory alone (every rule, weight 1), "texel" the weights of gm-texel
    if !only.is_empty() {
        let mut picked: Vec<Grandmaster> = judges.into_iter().filter(|g| only.iter().any(|o| o.eq_ignore_ascii_case(&g.name))).collect();
        if only.iter().any(|o| o.eq_ignore_ascii_case("texel")) {
            let lw = load_texel()?;
            picked.push(Grandmaster { name: "Texel".into(), weights: lw.iter().filter_map(|(id, x)| crate::golden_theory::rule(id).map(|r| (id.clone(), *x, r))).collect() });
        }
        if only.iter().any(|o| o.eq_ignore_ascii_case("theory")) {
            picked.push(Grandmaster { name: "Theory".into(), weights: crate::golden_theory::RULES.iter().map(|(id, r)| (id.to_string(), 1.0, *r)).collect() });
        }
        if picked.is_empty() {
            return Err(format!("no judge named {}", only.join(" ")));
        }
        judges = picked;
    }
    let best = vals.iter().map(|v| v.1).max().unwrap_or(0);
    // the filter (the hard check): only a candidate within `sound` cp of the best may be played;
    // the judges still see and rank every candidate
    let passes: Vec<bool> = vals.iter().map(|v| sound_cp == 0 || v.1 >= best - sound_cp).collect();
    let mvs: Vec<u32> = vals.iter().map(|(c, _)| uci_to_move(&mut p, c).unwrap()).collect();
    let seen: Vec<Seen> = mvs.iter().map(|&mv| Seen::new(&p, mv)).collect();
    let cfg_t = std::fs::read_to_string(GRANDMASTERS_FILE).ok().and_then(|t| toml::from_str::<toml::Value>(&t).ok());
    let lookahead = cfg_t.as_ref().and_then(|t| t.get("lookahead").and_then(|v| v.as_integer())).unwrap_or(1).max(1) as usize;
    // the filter's cp: every candidate's gap to the best search value counts against it (per 100 cp)
    let filter_weight = cfg_t.as_ref().and_then(|t| t.get("filter_weight").and_then(|v| v.as_float())).unwrap_or(0.0) as f32;
    let gap: Vec<f32> = vals.iter().map(|v| filter_weight * (best - v.1).clamp(0, 100_000) as f32 / 100.0).collect();
    // StarNNUE's rank bonus of each candidate (pawns)
    let rank_bonus: Vec<f32> = vals.iter().map(|(c, _)| star.iter().position(|m| m == c).and_then(|r| bonus.get(r)).copied().unwrap_or(0) as f32 / 100.0).collect();
    // each judge ranks every candidate by its style; Borda points
    let n = vals.len();
    let mut points = vec![0usize; n];
    let mut picks = Vec::new();
    for gm in &judges {
        // each candidate's score and the principle that gave it most
        let scored: Vec<(f32, String)> = seen
            .iter()
            .zip(&rank_bonus)
            .map(|(s, b)| {
                let parts: Vec<(f32, &str)> = gm.weights.iter().map(|(id, w, r)| (w * r(s), id.as_str())).collect();
                let top = parts.iter().max_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1).unwrap_or("");
                let _ = s;
                (*b, top.to_string())
            })
            .collect();
        // the theory score, `lookahead` plies: our move minus the opponent's best answer by the same theory
        let scored: Vec<(f32, String)> = scored.into_iter().zip(&mvs).zip(&gap).map(|(((b, top), &m), g)| (ahead(gm, &p, m, lookahead) + b - g, top)).collect();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| scored[b].0.total_cmp(&scored[a].0).then(vals[b].1.cmp(&vals[a].1)));
        for (rank, &i) in order.iter().enumerate() {
            points[i] += n - rank;
        }
        let pick = order.iter().copied().find(|&i| passes[i]).unwrap_or(order[0]);
        let ranking = order.iter().map(|&i| format!("{}{}", vals[i].0, if passes[i] { "" } else { "✗" })).collect::<Vec<_>>().join(">");
        picks.push(format!("{} {} ({}; {ranking})", gm.name, vals[pick].0, scored[pick].1));
    }
    let win = (0..n).filter(|&i| passes[i]).max_by(|&a, &b| points[a].cmp(&points[b]).then(vals[a].1.cmp(&vals[b].1))).ok_or("no candidate")?;
    let vs = vals.iter().zip(&passes).map(|((c, v), ok)| format!("{c} {v:+}{}", if *ok { "" } else { "✗" })).collect::<Vec<_>>().join(" ");
    let ps = vals.iter().zip(&points).map(|((c, _), n)| format!("{c} {n}")).collect::<Vec<_>>().join(" ");
    Ok((vals[win].0.clone(), format!("values (✗ = filtered): {vs}; playable: {}; votes: {ps}; judges: {}", passes.iter().filter(|x| **x).count(), picks.join(", "))))
}

/// The smoke test by the grandmasters: each position reported at once.
pub fn smoke(file: &str, ms: u64, only: &[String]) -> Result<(), String> {
    use std::io::Write;
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    rows.reverse();
    let mut hit = 0;
    for (i, (fen, sf)) in rows.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let (m, how) = gm_move(fen, ms, only)?;
        hit += (m == *sf) as usize;
        println!("  {}/{}: played {m}, Stockfish {sf} {} ({:.1} s) — {how} — {hit}/{} so far", i + 1, rows.len(), if m == *sf { "RIGHT" } else { "miss" }, t0.elapsed().as_secs_f64(), i + 1);
        let _ = std::io::stdout().flush();
    }
    println!("grandmasters smoke test: {hit}/{}", rows.len());
    Ok(())
}

/// The judge's weights learned by Texel tuning (`gm-texel`).
pub const TEXEL_FILE: &str = "out/golden/judge_texel.toml";
const NR: usize = crate::golden_theory::RULES.len();

/// One position for Texel: every valued move's rule measures, its win probability, and Stockfish's move.
struct TexelRow {
    feats: Vec<[f32; NR]>,
    q: Vec<f32>,
    sf: Option<usize>,
    /// the mean logit of the moves' win probabilities (the position's level)
    level: f32,
}

fn texel_rows(set: &[crate::golden_table::Valued], skip: &[String]) -> Vec<TexelRow> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = set.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = set
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for r in part {
                        if skip.iter().any(|f| f == &r.1) {
                            continue;
                        }
                        let Ok(mut p) = Position::from_fen(&r.1) else { continue };
                        let (mut feats, mut q) = (Vec::new(), Vec::new());
                        let mut sf = None;
                        for (u, wp) in &r.3 {
                            let Some(m) = uci_to_move(&mut p, u) else { continue };
                            let s = Seen::new(&p, m);
                            let mut f = [0f32; NR];
                            for (i, (_, rule)) in crate::golden_theory::RULES.iter().enumerate() {
                                f[i] = rule(&s);
                            }
                            if *u == r.2 {
                                sf = Some(feats.len());
                            }
                            feats.push(f);
                            q.push(wp.clamp(0.01, 0.99));
                        }
                        if feats.len() < 2 {
                            continue;
                        }
                        let level = q.iter().map(|x| (x / (1.0 - x)).ln()).sum::<f32>() / q.len() as f32;
                        out.push(TexelRow { feats, q, sf, level });
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

fn texel_scores(w: &[f32; NR], r: &TexelRow) -> Vec<f32> {
    let s: Vec<f32> = r.feats.iter().map(|f| f.iter().zip(w).map(|(a, b)| a * b).sum()).collect();
    let mean = s.iter().sum::<f32>() / s.len() as f32;
    s.into_iter().map(|x| x - mean).collect()
}

fn sig(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// (mean squared error, share where the judge's best is Stockfish's move)
fn texel_measure(w: &[f32; NR], rows: &[TexelRow]) -> (f64, f64) {
    let (mut e, mut n, mut hit, mut m) = (0f64, 0usize, 0usize, 0usize);
    for r in rows {
        let s = texel_scores(w, r);
        for (x, q) in s.iter().zip(&r.q) {
            let d = (q - sig(r.level + x)) as f64;
            e += d * d;
            n += 1;
        }
        if let Some(t) = r.sf {
            m += 1;
            let best = (0..s.len()).max_by(|&a, &b| s[a].total_cmp(&s[b])).unwrap_or(0);
            hit += (best == t) as usize;
        }
    }
    (e / n.max(1) as f64, hit as f64 / m.max(1) as f64)
}

fn save_texel(w: &[f32; NR], note: &str) -> Result<(), String> {
    let mut s = format!("# The judge's weights by Texel tuning (gm-texel): {note}\n[weights]\n");
    for (i, (id, _)) in crate::golden_theory::RULES.iter().enumerate() {
        s.push_str(&format!("{id} = {:.6}\n", w[i]));
    }
    std::fs::write(TEXEL_FILE, s).map_err(|e| e.to_string())
}

/// The Texel judge's weights from TEXEL_FILE.
pub fn load_texel() -> Result<Vec<(String, f32)>, String> {
    let text = std::fs::read_to_string(TEXEL_FILE).map_err(|e| format!("{TEXEL_FILE}: {e} (run gm-texel first)"))?;
    let t: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
    let w = t.get("weights").and_then(|v| v.as_table()).ok_or("no [weights]")?;
    Ok(w.iter().filter_map(|(k, v)| v.as_float().map(|x| (k.clone(), x as f32))).collect())
}

/// The known positions: every position Stockfish has already answered in
/// the data (the HF middlegame and endgame set), by its exact key, with its
/// move. A position in it is familiar; one not in it is not, however alike.
pub const KNOWN_FILE: &str = "out/golden/known_positions.bin";

/// Build KNOWN_FILE from out/golden/hf_mid_end.tsv (phase TAB move TAB fen), the smoke positions left out.
pub fn build_known() -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let skip: std::collections::HashSet<String> = std::fs::read_to_string("out/golden/smoke_reverse.tsv").unwrap_or_default().lines().filter_map(|l| l.split_once('\t').map(|x| x.0.to_string())).collect();
    let text = std::fs::read_to_string("out/golden/hf_mid_end.tsv").map_err(|e| e.to_string())?;
    let mut out: Vec<u8> = Vec::new();
    let (mut n, mut skipped) = (0usize, 0usize);
    for line in text.lines() {
        let f: Vec<&str> = line.splitn(3, '\t').collect();
        if f.len() < 3 {
            continue;
        }
        if skip.contains(f[2]) {
            skipped += 1;
            continue;
        }
        let Ok(p) = Position::from_fen(f[2]) else { continue };
        out.extend_from_slice(&p.key64().to_le_bytes());
        let mut mv = [0u8; 8];
        for (k, b) in f[1].bytes().take(8).enumerate() {
            mv[k] = b;
        }
        out.extend_from_slice(&mv);
        n += 1;
    }
    std::fs::write(KNOWN_FILE, &out).map_err(|e| e.to_string())?;
    Ok(format!("known positions: {n} written to {KNOWN_FILE} ({skipped} smoke positions left out, {:.0} s)", t0.elapsed().as_secs_f64()))
}

pub struct Known(pub std::collections::HashMap<u64, [u8; 8]>);

impl Known {
    pub fn load() -> Result<Known, String> {
        let b = std::fs::read(KNOWN_FILE).map_err(|e| format!("{KNOWN_FILE}: {e} (run known-build first)"))?;
        let mut map = std::collections::HashMap::with_capacity(b.len() / 16);
        for c in b.chunks_exact(16) {
            let k = u64::from_le_bytes(c[0..8].try_into().unwrap());
            let mut mv = [0u8; 8];
            mv.copy_from_slice(&c[8..16]);
            map.insert(k, mv);
        }
        Ok(Known(map))
    }
    /// Stockfish's move in this exact position, when it is known.
    pub fn get(&self, p: &Position) -> Option<String> {
        self.0.get(&p.key64()).map(|m| String::from_utf8_lossy(&m[..m.iter().position(|&b| b == 0).unwrap_or(8)]).to_string())
    }
}

/// Golden Boy's move: the opening table when the position is a known line;
/// Stockfish's own move when the exact position is known (KNOWN_FILE); else
/// the judge as an engine, the theory alone in StarNet's search for `ms`.
/// Returns (move, how).
pub fn boy_move(known: &Known, fen: &str, ms: u64) -> Result<(String, String), String> {
    if let Some(m) = crate::golden_grow::table_move_pub(fen) {
        return Ok((m, "the opening table: a known line".into()));
    }
    let p = Position::from_fen(fen)?;
    if let Some(m) = known.get(&p) {
        return Ok((m, "a known position: Stockfish's move from the data".into()));
    }
    let theory = theory_eval()?;
    let (t, d, k) = crate::star_search::best_move(&theory, fen, ms).ok_or("no move")?;
    Ok((t.clone(), format!("an unknown position: the theory engine's {t} (depth {d}, {:.1}M nodes)", k as f64 / 1e6)))
}

/// The smoke test of Golden Boy's move, each position reported at once.
pub fn boy_smoke(file: &str, ms: u64) -> Result<(), String> {
    use std::io::Write;
    let known = Known::load()?;
    let text = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let mut rows: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    rows.reverse();
    let mut hit = 0;
    for (i, (fen, sf)) in rows.iter().enumerate() {
        let t0 = std::time::Instant::now();
        let (m, how) = boy_move(&known, fen, ms)?;
        hit += (m == *sf) as usize;
        println!("  {}/{}: played {m}, Stockfish {sf} {} ({:.1} s) - {how} - {hit}/{} so far", i + 1, rows.len(), if m == *sf { "RIGHT" } else { "miss" }, t0.elapsed().as_secs_f64(), i + 1);
        let _ = std::io::stdout().flush();
    }
    println!("Golden Boy smoke test: {hit}/{}", rows.len());
    Ok(())
}

/// The judge as an engine's evaluation: the theory value's weights (THEORY_VALUE_FILE) when tuned, else the move judge's (TEXEL_FILE) on the state rules.
pub fn theory_eval() -> Result<crate::star_search::TheoryEval, String> {
    let lw = load_value().or_else(|_| load_texel())?;
    let mut ws = Vec::new();
    for (i, (id, _)) in crate::golden_theory::STATES.iter().enumerate() {
        if let Some((_, w)) = lw.iter().find(|x| x.0 == *id) {
            ws.push((i, *w));
        }
    }
    // the fitted bias is left out: it is the mean badness of a random legal move
    // (the fit ran over every move), not a property of a position reached by best play
    let material = lw.iter().find(|x| x.0 == "material").map(|x| x.1).filter(|w| *w > 1e-6).unwrap_or(1.0);
    Ok(crate::star_search::TheoryEval { weights: ws, bias: 0.0, scale: 100.0 / material })
}

/// Texel tuning of the judge: every rule's weight from Stockfish's win
/// probability of each move (hf_values), the move's score centred in its
/// position; Adam, shuffled minibatches, every epoch run (no early stop),
/// the best by validate kept in TEXEL_FILE. The smoke positions are left out.
pub fn texel(n_train: usize, n_val: usize, epochs: usize, batch: usize, lr: f32) -> Result<(), String> {
    use std::io::Write;
    let say = |s: String| {
        println!("{s}");
        let _ = std::io::stdout().flush();
    };
    let t0 = std::time::Instant::now();
    let skip: Vec<String> = std::fs::read_to_string("out/golden/smoke_reverse.tsv").unwrap_or_default().lines().filter_map(|l| l.split_once('\t').map(|x| x.0.to_string())).collect();
    let (tr, va, _) = crate::golden_table::read_values(n_train, n_val, 0)?;
    say(format!("judge Texel: reading {} train / {} validate positions ({:.0} s)", tr.len(), va.len(), t0.elapsed().as_secs_f64()));
    let tr = texel_rows(&tr, &skip);
    let va = texel_rows(&va, &skip);
    say(format!("measured every move by the {NR} rules: {} train / {} validate positions ({:.0} s)", tr.len(), va.len(), t0.elapsed().as_secs_f64()));
    let mut w = [0f32; NR];
    let (e0, a0) = texel_measure(&w, &va);
    say(format!("epoch 0: validate error {e0:.5}, judge's best = Stockfish's move {:.1}%", 100.0 * a0));
    let (mut best_e, mut best_w) = (e0, w);
    let (mut m1, mut m2) = ([0f32; NR], [0f32; NR]);
    let (b1, b2, eps) = (0.9f32, 0.999f32, 1e-8f32);
    let mut step = 0i32;
    let mut order: Vec<usize> = (0..tr.len()).collect();
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    for ep in 1..=epochs {
        for i in (1..order.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            order.swap(i, (rng % (i as u64 + 1)) as usize);
        }
        for b in order.chunks(batch.max(1)) {
            let mut g = [0f32; NR];
            let mut n = 0usize;
            for &k in b {
                let r = &tr[k];
                let s = texel_scores(&w, r);
                let mut mean_f = [0f32; NR];
                for f in &r.feats {
                    for i in 0..NR {
                        mean_f[i] += f[i] / r.feats.len() as f32;
                    }
                }
                for (j, f) in r.feats.iter().enumerate() {
                    let p = sig(r.level + s[j]);
                    let d = -2.0 * (r.q[j] - p) * p * (1.0 - p);
                    for i in 0..NR {
                        g[i] += d * (f[i] - mean_f[i]);
                    }
                    n += 1;
                }
            }
            step += 1;
            for i in 0..NR {
                let gi = g[i] / n.max(1) as f32;
                m1[i] = b1 * m1[i] + (1.0 - b1) * gi;
                m2[i] = b2 * m2[i] + (1.0 - b2) * gi * gi;
                let mh = m1[i] / (1.0 - b1.powi(step));
                let vh = m2[i] / (1.0 - b2.powi(step));
                w[i] -= lr * mh / (vh.sqrt() + eps);
            }
        }
        let (e, a) = texel_measure(&w, &va);
        let (te, ta) = texel_measure(&w, &tr[..tr.len().min(20_000)]);
        let better = e < best_e;
        if better {
            best_e = e;
            best_w = w;
            save_texel(&best_w, &format!("epoch {ep}, validate error {e:.5}, best = Stockfish's move {:.1}% on {} validate positions", 100.0 * a, va.len()))?;
        }
        say(format!("epoch {ep}: train error {te:.5} ({:.1}%), validate error {e:.5}, judge's best = Stockfish's move {:.1}%{} ({:.0} s)", 100.0 * ta, 100.0 * a, if better { " — best, saved" } else { "" }, t0.elapsed().as_secs_f64()));
    }
    let mut ws: Vec<(f32, &str)> = best_w.iter().zip(crate::golden_theory::RULES.iter()).map(|(x, (id, _))| (*x, *id)).collect();
    ws.sort_by(|a, b| b.0.total_cmp(&a.0));
    say(format!("weights (best): {}", ws.iter().map(|(x, id)| format!("{id} {x:+.3}")).collect::<Vec<_>>().join(", ")));
    Ok(())
}

fn rule_feats(s: &Seen) -> [f32; NR] {
    let mut f = [0f32; NR];
    for (i, (_, rule)) in crate::golden_theory::RULES.iter().enumerate() {
        f[i] = rule(s);
    }
    f
}

fn dot(w: &[f32; NR], f: &[f32; NR]) -> f32 {
    w.iter().zip(f).map(|(a, b)| a * b).sum()
}

/// `ahead` with the weights as an array, returning the principal variation's
/// features (ours minus theirs, ply by ply) and its score: the features are
/// what the weights are fitted to, so the judge is trained at the depth it
/// plays.
fn ahead_pv(w: &[f32; NR], p: &Position, m: u32, plies: usize) -> ([f32; NR], f32) {
    ahead_pv_caps(w, p, m, plies, 0)
}

fn ahead_pv_caps(w: &[f32; NR], p: &Position, m: u32, plies: usize, caps: usize) -> ([f32; NR], f32) {
    let s = Seen::new(p, m);
    let f = rule_feats(&s);
    let mine = dot(w, &f);
    let mut q = s.after.clone();
    let replies = legal_moves(&mut q);
    if replies.is_empty() {
        return (f, mine + if q.in_check() { 1000.0 } else { 0.0 });
    }
    let pat = ([0f32; NR], 0f32);
    let best = if plies <= 1 {
        let v = taken(p, m);
        if v > 0.0 && caps < 4 {
            worth_answering(&q, &replies, v).into_iter().map(|r| ahead_pv_caps(w, &q, r, 1, caps + 1)).fold(pat, |a, b| if b.1 > a.1 { b } else { a })
        } else {
            pat
        }
    } else {
        replies.iter().map(|&r| ahead_pv_caps(w, &q, r, plies - 1, 0)).fold(([0f32; NR], f32::MIN), |a, b| if b.1 > a.1 { b } else { a })
    };
    let mut out = f;
    for i in 0..NR {
        out[i] -= best.0[i];
    }
    (out, mine - best.1)
}

/// Golden's and StarNNUE's three best moves of every position (`ms` each search), in parallel: the candidates the judge chooses among in play.
fn candidates_of(set: &[crate::golden_table::Valued], ms: u64) -> std::collections::HashMap<String, Vec<String>> {
    let gw: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).unwrap_or_default().lines().filter_map(|l| l.trim().parse().ok()).collect();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = set.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = set
            .chunks(chunk)
            .map(|part| {
                let gw = gw.clone();
                sc.spawn(move || {
                    let ev = crate::star_search::GoldennetEval(gw);
                    let net = crate::fast::starnnue::net().ok();
                    let mut out = Vec::new();
                    for r in part {
                        let mut c = crate::star_search::best_moves(&ev, net.map(|n| (n, 1.0)), &r.1, ms, 3);
                        for m in crate::star_search::best_moves(&ev, None, &r.1, ms, 3) {
                            if !c.contains(&m) {
                                c.push(m);
                            }
                        }
                        out.push((r.1.clone(), c));
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

/// How often Stockfish's move is among the candidates: `n` positions (not the
/// smoke ones), each search `ms`, k per engine for every k in `ks`; also
/// which engine finds it.
pub fn recall(n: usize, ms: u64, ks: &[usize]) -> Result<String, String> {
    let kmax = ks.iter().copied().max().unwrap_or(3);
    let (_, va, _) = crate::golden_table::read_values(0, n, 0)?;
    let gw: Vec<f32> = std::fs::read_to_string(crate::golden_table::GOLDENNET_VALUE_FILE).map_err(|e| e.to_string())?.lines().filter_map(|l| l.trim().parse().ok()).collect();
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = va.len().div_ceil(threads).max(1);
    let t0 = std::time::Instant::now();
    // (Stockfish's rank in StarNNUE's list, in Golden's list), None = not there
    let ranks: Vec<(Option<usize>, Option<usize>, usize)> = std::thread::scope(|sc| {
        let hs: Vec<_> = va
            .chunks(chunk)
            .map(|part| {
                let gw = gw.clone();
                sc.spawn(move || {
                    let ev = crate::star_search::GoldennetEval(gw);
                    let net = crate::fast::starnnue::net().ok();
                    part.iter()
                        .map(|r| {
                            let star = crate::star_search::best_moves(&ev, net.map(|n| (n, 1.0)), &r.1, ms, kmax);
                            let golden = crate::star_search::best_moves(&ev, None, &r.1, ms, kmax);
                            let legal = Position::from_fen(&r.1).map(|mut p| legal_moves(&mut p).len()).unwrap_or(0);
                            (star.iter().position(|m| *m == r.2), golden.iter().position(|m| *m == r.2), legal)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let n = ranks.len();
    let mut out = format!("recall of Stockfish's move, {n} positions, {ms} ms per search ({:.0} s)
", t0.elapsed().as_secs_f64());
    for &k in ks {
        let s = ranks.iter().filter(|r| r.0.map_or(false, |i| i < k)).count();
        let g = ranks.iter().filter(|r| r.1.map_or(false, |i| i < k)).count();
        let either = ranks.iter().filter(|r| r.0.map_or(false, |i| i < k) || r.1.map_or(false, |i| i < k)).count();
        out.push_str(&format!("  k={k}: StarNNUE {s}/{n} ({:.1}%), Golden {g}/{n} ({:.1}%), either {either}/{n} ({:.1}%)
", 100.0 * s as f64 / n as f64, 100.0 * g as f64 / n as f64, 100.0 * either as f64 / n as f64));
    }
    let s1 = ranks.iter().filter(|r| r.0 == Some(0)).count();
    let missed: Vec<String> = ranks.iter().zip(&va).filter(|(r, _)| !(r.0.map_or(false, |i| i < kmax) || r.1.map_or(false, |i| i < kmax))).take(8).map(|(r, v)| format!("{} {} ({} legal)", v.1, v.2, r.2)).collect();
    out.push_str(&format!("  StarNNUE's first = Stockfish's move {s1}/{n} ({:.1}%)
  missed at k={kmax}, first few: {}
", 100.0 * s1 as f64 / n as f64, missed.join("; ")));
    Ok(out)
}

/// Texel rows whose features are the principal variation's at `plies` under weights `w`.
/// With `cands`, only the candidates count and Stockfish's move's target is 1 (the judge's task in play).
fn texel_rows_deep(set: &[crate::golden_table::Valued], skip: &[String], w: &[f32; NR], plies: usize, cands: Option<&std::collections::HashMap<String, Vec<String>>>) -> Vec<TexelRow> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = set.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = set
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for r in part {
                        if skip.iter().any(|f| f == &r.1) {
                            continue;
                        }
                        let Ok(mut p) = Position::from_fen(&r.1) else { continue };
                        let (mut feats, mut q) = (Vec::new(), Vec::new());
                        let mut sf = None;
                        let allowed = cands.and_then(|c| c.get(&r.1));
                        for (u, wp) in &r.3 {
                            if let Some(a) = allowed {
                                if !a.contains(u) {
                                    continue;
                                }
                            }
                            let Some(m) = uci_to_move(&mut p, u) else { continue };
                            let is_sf = *u == r.2;
                            if is_sf {
                                sf = Some(feats.len());
                            }
                            feats.push(ahead_pv(w, &p, m, plies).0);
                            q.push(if is_sf && allowed.is_some() { 0.99 } else { wp.clamp(0.01, 0.99) });
                        }
                        if feats.len() < 2 {
                            continue;
                        }
                        let level = q.iter().map(|x| (x / (1.0 - x)).ln()).sum::<f32>() / q.len() as f32;
                        out.push(TexelRow { feats, q, sf, level });
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

/// Adam over the rows, every epoch run; returns the weights best by validate error.
fn texel_fit(tr: &[TexelRow], va: &[TexelRow], mut w: [f32; NR], epochs: usize, batch: usize, lr: f32, say: &dyn Fn(String)) -> [f32; NR] {
    let (mut best_e, _) = texel_measure(&w, va);
    let mut best_w = w;
    let (mut m1, mut m2) = ([0f32; NR], [0f32; NR]);
    let (b1, b2, eps) = (0.9f32, 0.999f32, 1e-8f32);
    let mut step = 0i32;
    let mut order: Vec<usize> = (0..tr.len()).collect();
    let mut rng = 0x2545_F491_4F6C_DD1Du64;
    for ep in 1..=epochs {
        for i in (1..order.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            order.swap(i, (rng % (i as u64 + 1)) as usize);
        }
        for b in order.chunks(batch.max(1)) {
            let mut g = [0f32; NR];
            let mut n = 0usize;
            for &k in b {
                let r = &tr[k];
                let s = texel_scores(&w, r);
                let mut mean_f = [0f32; NR];
                for f in &r.feats {
                    for i in 0..NR {
                        mean_f[i] += f[i] / r.feats.len() as f32;
                    }
                }
                for (j, f) in r.feats.iter().enumerate() {
                    let p = sig(r.level + s[j]);
                    let d = -2.0 * (r.q[j] - p) * p * (1.0 - p);
                    for i in 0..NR {
                        g[i] += d * (f[i] - mean_f[i]);
                    }
                    n += 1;
                }
            }
            step += 1;
            for i in 0..NR {
                let gi = g[i] / n.max(1) as f32;
                m1[i] = b1 * m1[i] + (1.0 - b1) * gi;
                m2[i] = b2 * m2[i] + (1.0 - b2) * gi * gi;
                let mh = m1[i] / (1.0 - b1.powi(step));
                let vh = m2[i] / (1.0 - b2.powi(step));
                w[i] -= lr * mh / (vh.sqrt() + eps);
            }
        }
        let (e, a) = texel_measure(&w, va);
        let better = e < best_e;
        if better {
            best_e = e;
            best_w = w;
        }
        if ep % 5 == 0 || ep == epochs || better {
            say(format!("    epoch {ep}: validate error {e:.5}, judge's best = Stockfish's move {:.1}%{}", 100.0 * a, if better { " (best)" } else { "" }));
        }
    }
    best_w
}

/// Texel tuning at depth: the weights fitted to the principal variation's
/// features at `plies` (the exchange played out), the variation chosen by
/// the weights of the round before; `rounds` such rounds, starting from
/// TEXEL_FILE's weights. Reports the validate hit rate at `plies` before and
/// after each round; the best round is kept in TEXEL_FILE.
pub fn texel_deep(n_train: usize, n_val: usize, plies: usize, rounds: usize, epochs: usize, cand_ms: u64) -> Result<(), String> {
    use std::io::Write;
    let say = |s: String| {
        println!("{s}");
        let _ = std::io::stdout().flush();
    };
    let t0 = std::time::Instant::now();
    let skip: Vec<String> = std::fs::read_to_string("out/golden/smoke_reverse.tsv").unwrap_or_default().lines().filter_map(|l| l.split_once('\t').map(|x| x.0.to_string())).collect();
    let (tr, va, _) = crate::golden_table::read_values(n_train, n_val, 0)?;
    let mut w = [0f32; NR];
    if let Ok(lw) = load_texel() {
        for (i, (id, _)) in crate::golden_theory::RULES.iter().enumerate() {
            w[i] = lw.iter().find(|x| x.0 == *id).map(|x| x.1).unwrap_or(0.0);
        }
    }
    say(format!("judge Texel at {plies} plies: {} train / {} validate positions, starting from {TEXEL_FILE}", tr.len(), va.len()));
    let cands = if cand_ms > 0 {
        let all: Vec<_> = tr.iter().chain(va.iter()).cloned().collect();
        let c = candidates_of(&all, cand_ms);
        let among = tr.iter().chain(va.iter()).filter(|r| c.get(&r.1).map_or(false, |l| l.contains(&r.2))).count();
        say(format!("candidates: Golden's and StarNNUE's three best at {cand_ms} ms each; Stockfish's move among them in {among} of {} positions ({:.0} s)", all.len(), t0.elapsed().as_secs_f64()));
        Some(c)
    } else {
        None
    };
    let vrows = texel_rows_deep(&va, &skip, &w, plies, cands.as_ref());
    let (e, a) = texel_measure(&w, &vrows);
    say(format!("round 0 (the one-ply weights at {plies} plies): validate error {e:.5}, judge's best = Stockfish's move {:.1}% ({:.0} s)", 100.0 * a, t0.elapsed().as_secs_f64()));
    let (mut best_a, mut best_w) = (a, w);
    for round in 1..=rounds {
        let trows = texel_rows_deep(&tr, &skip, &w, plies, cands.as_ref());
        say(format!("round {round}: variations chosen, {} train rows ({:.0} s)", trows.len(), t0.elapsed().as_secs_f64()));
        let fitted = texel_fit(&trows, &vrows, w, epochs, 256, 0.01, &say);
        w = fitted;
        // the real measure: the variations chosen again by the new weights
        let vrows2 = texel_rows_deep(&va, &skip, &w, plies, cands.as_ref());
        let (e, a) = texel_measure(&w, &vrows2);
        say(format!("round {round}: at {plies} plies with its own variations, validate error {e:.5}, judge's best = Stockfish's move {:.1}% ({:.0} s)", 100.0 * a, t0.elapsed().as_secs_f64()));
        if a > best_a {
            best_a = a;
            best_w = w;
            save_texel(&best_w, &format!("{plies} plies, round {round}, best = Stockfish's move {:.1}% on {} validate positions", 100.0 * a, vrows2.len()))?;
            say("    best, saved".into());
        }
    }
    let mut ws: Vec<(f32, &str)> = best_w.iter().zip(crate::golden_theory::RULES.iter()).map(|(x, (id, _))| (*x, *id)).collect();
    ws.sort_by(|a, b| b.0.total_cmp(&a.0));
    say(format!("weights (best): {}", ws.iter().map(|(x, id)| format!("{id} {x:+.3}")).collect::<Vec<_>>().join(", ")));
    Ok(())
}

/// The theory as a position value, Texel-tuned: out/golden/theory_value.toml.
pub const THEORY_VALUE_FILE: &str = "out/golden/theory_value.toml";
const NS: usize = crate::golden_theory::STATES.len();

/// The state features of `p` from `me`: state(me) - state(them) for every state rule, and a bias.
fn state_feats(p: &Position, me: usize) -> [f32; NS + 1] {
    let mut f = [0f32; NS + 1];
    for (i, (_, st)) in crate::golden_theory::STATES.iter().enumerate() {
        f[i] = st(p, me) - st(p, 1 - me);
    }
    f[NS] = 1.0;
    f
}

/// One position: every valued move's child features, its win probability, Stockfish's move.
struct ValueRow {
    feats: Vec<[f32; NS + 1]>,
    q: Vec<f32>,
    sf: Option<usize>,
}

fn value_rows(set: &[crate::golden_table::Valued], skip: &[String]) -> Vec<ValueRow> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk = set.len().div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        let hs: Vec<_> = set
            .chunks(chunk)
            .map(|part| {
                sc.spawn(move || {
                    let mut out = Vec::new();
                    for r in part {
                        if skip.iter().any(|f| f == &r.1) {
                            continue;
                        }
                        let Ok(mut p) = Position::from_fen(&r.1) else { continue };
                        let me = p.turn as usize;
                        let (mut feats, mut q, mut sf) = (Vec::new(), Vec::new(), None);
                        for (u, wp) in &r.3 {
                            let Some(m) = uci_to_move(&mut p, u) else { continue };
                            if *u == r.2 {
                                sf = Some(feats.len());
                            }
                            p.make(m);
                            feats.push(state_feats(&p, me));
                            p.unmake();
                            q.push(wp.clamp(0.01, 0.99));
                        }
                        if feats.len() >= 2 {
                            out.push(ValueRow { feats, q, sf });
                        }
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
}

fn vdot(w: &[f32; NS + 1], f: &[f32; NS + 1]) -> f32 {
    w.iter().zip(f).map(|(a, b)| a * b).sum()
}

/// (mean squared error of sigmoid(w.f) against the win probability, share where the best child is Stockfish's move)
fn value_measure(w: &[f32; NS + 1], rows: &[ValueRow]) -> (f64, f64) {
    let (mut e, mut n, mut hit, mut m) = (0f64, 0usize, 0usize, 0usize);
    for r in rows {
        let s: Vec<f32> = r.feats.iter().map(|f| vdot(w, f)).collect();
        for (x, q) in s.iter().zip(&r.q) {
            let d = (q - sig(*x)) as f64;
            e += d * d;
            n += 1;
        }
        if let Some(t) = r.sf {
            m += 1;
            let best = (0..s.len()).max_by(|&a, &b| s[a].total_cmp(&s[b])).unwrap_or(0);
            hit += (best == t) as usize;
        }
    }
    (e / n.max(1) as f64, hit as f64 / m.max(1) as f64)
}

fn save_value(w: &[f32; NS + 1], note: &str) -> Result<(), String> {
    let mut s = format!("# The theory as a position value by Texel tuning (theory-texel): {note}\n[weights]\n");
    for (i, (id, _)) in crate::golden_theory::STATES.iter().enumerate() {
        s.push_str(&format!("{id} = {:.6}\n", w[i]));
    }
    s.push_str(&format!("bias = {:.6}\n", w[NS]));
    std::fs::write(THEORY_VALUE_FILE, s).map_err(|e| e.to_string())
}

/// The theory value's weights, by state id (and "bias").
pub fn load_value() -> Result<Vec<(String, f32)>, String> {
    let text = std::fs::read_to_string(THEORY_VALUE_FILE).map_err(|e| format!("{THEORY_VALUE_FILE}: {e} (run theory-texel first)"))?;
    let t: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
    let w = t.get("weights").and_then(|v| v.as_table()).ok_or("no [weights]")?;
    Ok(w.iter().filter_map(|(k, v)| v.as_float().map(|x| (k.clone(), x as f32))).collect())
}

/// Every miss of a smoke/check log (lines "played X, Stockfish Y miss" followed by the fen):
/// Stockfish's move and the played one, each with the theory engine's value and line.
pub fn theory_misses(log: &str, ms: u64, max: usize) -> Result<String, String> {
    let text = std::fs::read_to_string(log).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    let mut n = 0;
    for (i, l) in lines.iter().enumerate() {
        if !l.contains(" miss ") || i + 1 >= lines.len() {
            continue;
        }
        let played = l.split("played ").nth(1).and_then(|x| x.split(',').next()).unwrap_or("").trim().to_string();
        let sf = l.split("Stockfish ").nth(1).and_then(|x| x.split_whitespace().next()).unwrap_or("").to_string();
        let fen = lines[i + 1].trim().to_string();
        if Position::from_fen(&fen).is_err() {
            continue;
        }
        n += 1;
        if n > max {
            break;
        }
        out.push_str(&format!("{n}. {fen}
{}", theory_pv(&fen, &[sf, played], ms, 5)?));
    }
    Ok(out)
}

/// How fast the theory value is: every state timed over the first `n` check positions (both sides), evaluations per second.
pub fn theory_bench(n: usize) -> Result<String, String> {
    let text = std::fs::read_to_string("out/golden/grow_check.tsv").map_err(|e| e.to_string())?;
    let ps: Vec<Position> = text.lines().take(n).filter_map(|l| l.split('\t').next()).filter_map(|f| Position::from_fen(f).ok()).collect();
    let ev = theory_eval()?;
    let mut acc = 0i64;
    let mut whole = 0.0;
    for pass in 0..3 {
        let t0 = std::time::Instant::now();
        for p in &ps {
            acc += ev.eval(p) as i64;
        }
        whole = t0.elapsed().as_secs_f64();
        let t1 = std::time::Instant::now();
        let mut y = 0f32;
        for p in &ps {
            y += crate::golden_theory::state_score(&ev.weights, p, p.turn as usize);
        }
        acc += y as i64;
        eprintln!("pass {pass}: eval {:.1} us, state_score {:.1} us", whole / ps.len() as f64 * 1e6, t1.elapsed().as_secs_f64() / ps.len() as f64 * 1e6);
    }
    let mut rows: Vec<(f64, &str)> = Vec::new();
    for (id, st) in crate::golden_theory::STATES.iter() {
        let t = std::time::Instant::now();
        let mut x = 0f32;
        for p in &ps {
            x += st(p, 0) - st(p, 1);
        }
        rows.push((t.elapsed().as_secs_f64() / ps.len() as f64 * 1e6, *id));
        acc += x as i64;
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total: f64 = rows.iter().map(|r| r.0).sum();
    Ok(format!("{} positions: {:.0} evaluations/s ({:.1} us each; states alone {total:.1} us); per state (us, both sides): {} [{acc}]
", ps.len(), ps.len() as f64 / whole, whole / ps.len() as f64 * 1e6, rows.iter().map(|(t, id)| format!("{id} {t:.1}")).collect::<Vec<_>>().join(", ")))
}

/// The theory value of `fen` from the side to move, state by state (weighted, in cp), largest first.
pub fn theory_static(fen: &str) -> Result<String, String> {
    let ev = theory_eval()?;
    let p = Position::from_fen(fen)?;
    let me = p.turn as usize;
    let mut parts: Vec<(f32, String)> = ev.weights.iter().map(|&(i, w)| {
        let (id, st) = crate::golden_theory::STATES[i];
        let (a, b) = (st(&p, me), st(&p, 1 - me));
        let v = w * (a - b) * ev.scale;
        (v, format!("{id} {v:+.0} (me {a:.2}, them {b:.2}, w {w:+.3})"))
    }).collect();
    parts.sort_by(|x, y| y.0.abs().total_cmp(&x.0.abs()));
    Ok(format!("eval {:+} cp (bias {:+.0}): {}
", ev.eval(&p), -ev.bias * ev.scale, parts.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join("; ")))
}

/// The theory engine's line after each of `moves` in `fen`: the value (cp,
/// from the mover) and the best replies `plies` deep, each by a search of `ms`.
pub fn theory_pv(fen: &str, moves: &[String], ms: u64, plies: usize) -> Result<String, String> {
    let ev = theory_eval()?;
    let mut out = String::new();
    for u in moves {
        let mut p = Position::from_fen(fen)?;
        let m = uci_to_move(&mut p, u).ok_or(format!("illegal {u}"))?;
        p.make(m);
        let mut line = vec![u.clone()];
        let mut value = None;
        for i in 0..plies {
            if legal_moves(&mut p).is_empty() {
                break;
            }
            let Some((r, v)) = crate::star_search::analyse(&ev, None, &p.to_fen(), if i == 0 { ms } else { ms / 2 }) else { break };
            if i == 0 {
                value = Some(-v);
            }
            let Some(rm) = uci_to_move(&mut p, &r) else { break };
            line.push(r);
            p.make(rm);
        }
        out.push_str(&format!("{u}: {} cp  line {}\n", value.map_or("?".to_string(), |v| format!("{v:+}")), line.join(" ")));
    }
    Ok(out)
}

/// Why the theory value prefers one move to another in `fen`: every state's
/// weighted difference between the two children (static, no search), largest first.
pub fn theory_why(fen: &str, a: &str, b: &str) -> Result<String, String> {
    let lw = load_value()?;
    let mut p = Position::from_fen(fen)?;
    let me = p.turn as usize;
    let child = |p: &mut Position, u: &str| -> Result<[f32; NS + 1], String> {
        let m = uci_to_move(p, u).ok_or(format!("illegal {u}"))?;
        p.make(m);
        let f = state_feats(p, me);
        p.unmake();
        Ok(f)
    };
    let (fa, fb) = (child(&mut p, a)?, child(&mut p, b)?);
    let mut parts: Vec<(f32, String)> = Vec::new();
    let (mut ta, mut tb) = (0f32, 0f32);
    for (i, (id, _)) in crate::golden_theory::STATES.iter().enumerate() {
        let w = lw.iter().find(|x| x.0 == *id).map(|x| x.1).unwrap_or(0.0);
        ta += w * fa[i];
        tb += w * fb[i];
        let d = w * (fa[i] - fb[i]);
        if d.abs() > 1e-4 {
            parts.push((d, format!("{id} {d:+.3} ({a} {:.2}, {b} {:.2}, w {w:+.3})", fa[i], fb[i])));
        }
    }
    parts.sort_by(|x, y| y.0.abs().total_cmp(&x.0.abs()));
    Ok(format!("{a} {ta:+.3} vs {b} {tb:+.3} (static, from the mover): {}
", parts.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join("; ")))
}

/// Texel tuning of the theory as a position value: the state features of
/// the position after each valued move (hf_values) against Stockfish's win
/// probability, sigmoid(w.f); Adam, shuffled minibatches, every epoch run,
/// the best by validate error kept in THEORY_VALUE_FILE. The smoke positions
/// are left out. The measure that matters: the best child = Stockfish's move.
pub fn texel_value(n_train: usize, n_val: usize, epochs: usize, batch: usize, lr: f32) -> Result<(), String> {
    use std::io::Write;
    let say = |s: String| {
        println!("{s}");
        let _ = std::io::stdout().flush();
    };
    let t0 = std::time::Instant::now();
    let skip: Vec<String> = std::fs::read_to_string("out/golden/smoke_reverse.tsv").unwrap_or_default().lines().filter_map(|l| l.split_once('\t').map(|x| x.0.to_string())).collect();
    let (tr, va, _) = crate::golden_table::read_values(n_train, n_val, 0)?;
    let tr = value_rows(&tr, &skip);
    let va = value_rows(&va, &skip);
    let kids: usize = tr.iter().map(|r| r.feats.len()).sum();
    say(format!("theory value Texel: {} train positions ({kids} children) / {} validate, {NS} states + bias ({:.0} s)", tr.len(), va.len(), t0.elapsed().as_secs_f64()));
    let mut w = [0f32; NS + 1];
    if let Ok(lw) = load_value() {
        for (i, (id, _)) in crate::golden_theory::STATES.iter().enumerate() {
            w[i] = lw.iter().find(|x| x.0 == *id).map(|x| x.1).unwrap_or(0.0);
        }
        w[NS] = lw.iter().find(|x| x.0 == "bias").map(|x| x.1).unwrap_or(0.0);
        say("starting from the weights of the file".into());
    }
    let (e0, a0) = value_measure(&w, &va);
    say(format!("epoch 0: validate error {e0:.5}, best child = Stockfish's move {:.1}%", 100.0 * a0));
    let (mut best_e, mut best_w) = (e0, w);
    let (mut m1, mut m2) = ([0f32; NS + 1], [0f32; NS + 1]);
    let (b1, b2, eps) = (0.9f32, 0.999f32, 1e-8f32);
    let mut step = 0i32;
    let mut order: Vec<usize> = (0..tr.len()).collect();
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;
    for ep in 1..=epochs {
        for i in (1..order.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            order.swap(i, (rng % (i as u64 + 1)) as usize);
        }
        for b in order.chunks(batch.max(1)) {
            let mut g = [0f32; NS + 1];
            let mut n = 0usize;
            for &k in b {
                let r = &tr[k];
                for (f, q) in r.feats.iter().zip(&r.q) {
                    let p = sig(vdot(&w, f));
                    let d = -2.0 * (q - p) * p * (1.0 - p);
                    for i in 0..=NS {
                        g[i] += d * f[i];
                    }
                    n += 1;
                }
            }
            step += 1;
            for i in 0..=NS {
                let gi = g[i] / n.max(1) as f32;
                m1[i] = b1 * m1[i] + (1.0 - b1) * gi;
                m2[i] = b2 * m2[i] + (1.0 - b2) * gi * gi;
                w[i] -= lr * (m1[i] / (1.0 - b1.powi(step))) / ((m2[i] / (1.0 - b2.powi(step))).sqrt() + eps);
            }
        }
        let (e, a) = value_measure(&w, &va);
        let better = e < best_e;
        if better {
            best_e = e;
            best_w = w;
            save_value(&best_w, &format!("epoch {ep}, validate error {e:.5}, best child = Stockfish's move {:.1}% on {} validate positions", 100.0 * a, va.len()))?;
        }
        say(format!("epoch {ep}: validate error {e:.5}, best child = Stockfish's move {:.1}%{} ({:.0} s)", 100.0 * a, if better { " - best, saved" } else { "" }, t0.elapsed().as_secs_f64()));
    }
    let mut ws: Vec<(f32, &str)> = best_w[..NS].iter().zip(crate::golden_theory::STATES.iter()).map(|(x, (id, _))| (*x, *id)).collect();
    ws.sort_by(|a, b| b.0.total_cmp(&a.0));
    say(format!("weights (best): {}; bias {:+.3}", ws.iter().map(|(x, id)| format!("{id} {x:+.3}")).collect::<Vec<_>>().join(", "), best_w[NS]));
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_ten_load_and_every_weight_has_its_rule() {
        let (sound, bonus, gms) = super::load().unwrap();
        assert!(sound >= 0);
        assert_eq!(bonus.len(), 3);
        assert_eq!(gms.len(), 10);
        assert!(gms.iter().all(|g| !g.weights.is_empty()));
        let text = std::fs::read_to_string(super::GRANDMASTERS_FILE).unwrap();
        let t: toml::Value = toml::from_str(&text).unwrap();
        let groups: Vec<&str> = t["grandmaster"].as_table().unwrap().values().map(|g| g["group"].as_str().unwrap()).collect();
        assert_eq!(groups.iter().filter(|g| **g == "attacking").count(), 5);
        assert_eq!(groups.iter().filter(|g| **g == "positional").count(), 5);
    }
}
