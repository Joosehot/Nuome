//! Checks: prove a finalist before it is played, independently of the rules
//! that offered it, as the mathematics is proven against the original
//! problem. The prover knows the rules of the game only (RulesEval: material
//! and what the search sees of captures, checks and mates), no theory:
//!
//!   1. no forced mate against us follows the move;
//!   2. the move loses at most checks.material cp by force beyond what the
//!      best move by the rules alone loses (the reference).
//!
//! The first finalist that passes is played; when none passes the engine
//! says so and plays the reference move.

use super::config::Config;
use crate::fast::movegen::{legal_moves, uci_to_move};
use crate::fast::position::{move_to_uci, Position};
use crate::star_search::{analyse, search_root_pub, RulesEval};

const MATE_ZONE: i32 = 1_000_000_000 - 1_000;

pub struct Reference {
    pub mv: String,
    pub value: i32,
}

/// The best move and value by the rules alone, from the side to move.
pub fn reference(fen: &str, cfg: &Config) -> Option<Reference> {
    search_root_pub(&RulesEval, fen, cfg.checks.reference_ms).map(|(m, _, _, v)| Reference { mv: m, value: v })
}

pub struct Checked {
    pub passed: bool,
    pub note: String,
}

/// The value of `mv` by the rules alone (from the mover), the opponent's forced play `ms` deep.
pub fn forced_value(p: &Position, mv: u32, ms: u64) -> i32 {
    let mut q = p.clone();
    q.make(mv);
    if legal_moves(&mut q).is_empty() {
        return if q.in_check() { 1_000_000_000 } else { 0 };
    }
    -analyse(&RulesEval, None, &q.to_fen(), ms).map(|r| r.1).unwrap_or(0)
}

pub fn check(p: &Position, mv: u32, reference: &Reference, cfg: &Config) -> Checked {
    let v = forced_value(p, mv, cfg.checks.mate_ms);
    let mated = v <= -MATE_ZONE;
    let loss = reference.value - v;
    let loses = !mated && loss > cfg.checks.material;
    let passed = !mated && !loses;
    let note = format!(
        "{}: {}; by force {v:+} cp against the reference {} {:+} (loss {loss}, limit {}) {}",
        move_to_uci(mv),
        if mated { "MATED by force" } else { "no forced mate" },
        reference.mv,
        reference.value,
        cfg.checks.material,
        if passed { "ok" } else { "FAILS" }
    );
    Checked { passed, note }
}

/// For the tests: check a uci move in a fen.
pub fn check_uci(fen: &str, uci: &str, cfg: &Config) -> Result<Checked, String> {
    let mut p = Position::from_fen(fen)?;
    let m = uci_to_move(&mut p, uci).ok_or("illegal")?;
    let r = reference(fen, cfg).ok_or("no reference")?;
    Ok(check(&p, m, &r, cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fools_mate_fails_and_a_sound_move_passes() {
        let cfg = Config::load().unwrap();
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/8/5P2/PPPPP1PP/RNBQKBNR w KQkq - 0 2";
        assert!(!check_uci(fen, "g2g4", &cfg).unwrap().passed);
        assert!(check_uci(fen, "e2e4", &cfg).unwrap().passed);
    }

    #[test]
    fn hanging_a_rook_fails() {
        let cfg = Config::load().unwrap();
        // the bishop on g2 covers a8: the rook to a8 is lost by force
        let fen = "4k3/8/8/8/8/8/6b1/R3K3 w - - 0 1";
        assert!(!check_uci(fen, "a1a8", &cfg).unwrap().passed);
        assert!(check_uci(fen, "a1a7", &cfg).unwrap().passed);
    }
}
