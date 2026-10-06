//! The rules: one principle of data/golden_theory.toml each. The principle's
//! reading of a move (what it measures, with its pass/fail test) lives in
//! src/golden_theory.rs under the same id; here each principle says when the
//! situation asks for it and offers the moves that follow it. Its weight and
//! axes are data/chess_rules.toml.

use super::config::{Axes, Config};
use super::parser::Situation;
use crate::fast::movegen::legal_moves;
use crate::fast::position::Position;
use crate::golden_theory::{Rule as Measure, Seen, RULES};

/// One offered move.
#[derive(Clone, Debug)]
pub struct Variant {
    pub mv: u32,
    pub rule: &'static str,
    /// the fact that asked for it
    pub why: String,
    /// how far the move follows the principle (about one per pawn)
    pub measure: f32,
    pub axes: Axes,
    pub weight: f32,
}

/// When a principle is asked for: the fact of the situation, or None.
fn asked(id: &str, sit: &Situation) -> Option<String> {
    let every = || Some("every position".to_string());
    let opening = || (sit.phase > 0.8 && sit.pieces >= 12).then(|| format!("the opening (phase {:.2})", sit.phase));
    let endgame = || (sit.phase < 0.35).then(|| format!("the endgame (phase {:.2})", sit.phase));
    match id {
        "material" | "space" | "center" | "initiative" | "restriction" | "pawn_structure" | "coordination" | "fork" | "pin" | "double_check" | "sacrifice" | "hanging" | "knight_outpost" | "bishops_open" | "bad_bishop" | "rook_seventh" | "attack_king" | "pawn_demolition" | "back_rank" | "combinations" => every(),
        "defend_in_advance" => Some(if sit.en_prise > 0.0 { format!("{:.1} of ours en prise", sit.en_prise) } else { "every position".into() }),
        "exchanges" => (sit.material >= 1.5).then(|| format!("ahead {:+.1}", sit.material)),
        "rook_behind_passer" | "passed_pawn" => (sit.my_passers || sit.their_passers).then(|| "a passed pawn on the board".to_string()),
        "queen_not_early" | "development" | "castle_early" | "classical_hypermodern" => opening(),
        "king_safety_middlegame" => (sit.phase >= 0.35).then(|| format!("the middlegame, king danger {:.1}", sit.king_danger)),
        "pawn_shield" => sit.castled.then(|| "our king is castled".to_string()),
        "king_active" | "outside_passer" | "connected_passers" | "promotion_goal" | "rule_of_square" | "extra_pawn_decides" | "rook_endgame_initiative" => endgame(),
        _ => None,
    }
}

/// Every variant every asking rule offers in `p`: the legal moves that follow the principle (measure > 0).
pub fn variants(p: &Position, sit: &Situation, cfg: &Config) -> Vec<Variant> {
    let mut q = p.clone();
    let moves = legal_moves(&mut q);
    let seen: Vec<(u32, Seen)> = moves.iter().map(|&m| (m, Seen::new(p, m))).collect();
    let mut out = Vec::new();
    for (id, measure) in RULES.iter() {
        let Some(why) = asked(id, sit) else { continue };
        let rc = cfg.rule(id);
        let f: Measure = *measure;
        for (m, s) in &seen {
            let x = f(s);
            if x > 0.0 {
                out.push(Variant { mv: *m, rule: id, why: why.clone(), measure: x, axes: rc.axes(), weight: rc.weight });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_asks_for_development_the_endgame_for_the_king() {
        let cfg = Config::load().unwrap();
        let start = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let sit = Situation::read(&start);
        let v = variants(&start, &sit, &cfg);
        assert!(v.iter().any(|x| x.rule == "development"));
        assert!(!v.iter().any(|x| x.rule == "king_active"));
        let end = Position::from_fen("8/8/8/4k3/8/8/8/K7 w - - 0 1").unwrap();
        let sit = Situation::read(&end);
        let v = variants(&end, &sit, &cfg);
        assert!(v.iter().any(|x| x.rule == "king_active"));
    }
}
