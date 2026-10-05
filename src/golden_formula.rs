//! The Golden formula for chess, made like the Goldbach bound: the problem
//! defined first, then the formula built only from Golden Answer's own
//! answers, then checked exactly where the truth is known.
//!
//! The problem.
//!   object:  a legal position P, s the side to move
//!   truth:   V(P), win / draw / loss for s with perfect play, by the rules alone
//!   pair:    every move m of s and every reply r to it
//!   score:   the five checks — legality, safety, force, material, tempo —
//!            made at once on every pair; how each is scored is asked of
//!            Golden Answer (`QUESTIONS`), nothing is written in for it
//!   demand:  the formula orders the moves as V does
//!   check:   exactly where V is known (Syzygy tables, up to five pieces)
//!
//! A part Golden Answer does not answer stays empty, and the formula is not
//! made until every part has an answer.

use crate::golden_answer::{answer as ask, Kind};

pub const QUESTIONS: [&str; 6] = [
    "how is a chess move and its reply scored for legality",
    "how is a chess move and its reply scored for safety",
    "how is a chess move and its reply scored for force",
    "how is a chess move and its reply scored for material",
    "how is a chess move and its reply scored for tempo",
    "how do the five scores combine into one formula",
];

/// Golden Answer's own answer to each question: the first kind (logical,
/// theoretical) that answers it, or None. Abstract answers claim nothing,
/// so they are shown but never built on.
pub fn parts() -> Vec<(&'static str, Option<(Kind, String)>, String)> {
    QUESTIONS
        .iter()
        .map(|q| {
            let got = [Kind::Logical, Kind::Theoretical].into_iter().find_map(|k| {
                let a = ask(k, q);
                a.short.clone().map(|s| (k, s))
            });
            let abs = ask(Kind::Abstract, q).short.unwrap_or_default();
            (*q, got, abs)
        })
        .collect()
}

/// The answer to "the Golden formula for chess": made from the parts, or why not yet.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let ps = parts();
    let open: Vec<&str> = ps.iter().filter(|p| p.1.is_none()).map(|p| p.0).collect();
    if !open.is_empty() && !kind_abstract {
        return Err(format!("not made yet: Golden Answer does not answer {} of the {} parts itself:\n  - {}", open.len(), ps.len(), open.join("\n  - ")));
    }
    let _ = kind_logical;
    let lines: Vec<String> = ps.iter().map(|(q, got, abs)| match got {
        Some((k, s)) => format!("{q}: {s} ({k:?})"),
        None => format!("{q}: no answer (abstract, claiming nothing: {abs})"),
    }).collect();
    let short = if open.is_empty() { "the formula of the parts below".to_string() } else { "no formula yet".to_string() };
    Ok((short, lines, vec![]))
}

/// The question, read whole.
pub fn read(q: &str) -> bool {
    let t: String = q.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let t = t.trim_end_matches(['.', '?', '!']);
    [
        "give one function of a chess position that checks legality, safety, force, material and tempo at the same time",
        "the golden formula for chess",
        "what is the golden formula for chess",
    ]
    .contains(&t)
}

/// Every legal move with the formula's value: none until the formula is made.
pub fn rank(_fen: &str) -> Option<Vec<(String, i64)>> {
    None
}
