//! Judges: what one move's principle cannot see alone, judged over the plan.
//! Each returns a signed cp opinion with a note. A breakable judge is
//! forgiven `discount x the profile's share of its axis`: an aggressive
//! request forgives a material deficit, a safe one does not.

use super::config::{Axes, Config};
use super::parser::Situation;
use crate::fast::position::Position;

#[derive(Clone, Debug)]
pub struct Note {
    pub cp: i32,
    pub text: String,
}

fn penalty(cfg: &Config, profile: &Axes, name: &str) -> i32 {
    let j = cfg.judge(name);
    let forgiven = match (&j.axis, j.breakable) {
        (Some(a), true) => (j.discount * profile.share(a)).min(1.0),
        _ => 0.0,
    };
    -(j.weight * (1.0 - forgiven)).round() as i32
}

/// Judge a plan: `root` and its situation, the position after our first move, the position at the end.
pub fn judge(root: &Position, sit: &Situation, after_first: &Position, end: &Position, first_is_pawn: bool, profile: &Axes, cfg: &Config) -> Vec<Note> {
    let mut out = Vec::new();
    let me = root.turn as usize;
    // threat_first: a piece of ours en prise stays en prise after the first move
    if sit.en_prise > 0.0 {
        let still = Situation::read_for(after_first, me).en_prise;
        if still >= sit.en_prise {
            let cp = penalty(cfg, profile, "threat_first");
            out.push(Note { cp, text: format!("threat_first {cp:+} ({:.1} still en prise)", still) });
        }
    }
    // material: the plan ends down material
    let end_mat = Situation::read_for(end, me).material;
    let lost = ((sit.material - end_mat) * 100.0).round() as i32;
    if lost >= cfg.judge("material").min {
        let cp = penalty(cfg, profile, "material");
        out.push(Note { cp, text: format!("material {cp:+} (the plan ends {lost} cp down)") });
    }
    // king_shield: a pawn in front of our castled king moves first
    if sit.castled && first_is_pawn {
        let before = crate::golden_theory::STATES.iter().find(|(i, _)| *i == "pawn_shield").map(|(_, f)| f(root, me)).unwrap_or(0.0);
        let after = crate::golden_theory::STATES.iter().find(|(i, _)| *i == "pawn_shield").map(|(_, f)| f(after_first, me)).unwrap_or(0.0);
        if after < before {
            let cp = penalty(cfg, profile, "king_shield");
            out.push(Note { cp, text: format!("king_shield {cp:+} (a shield pawn moves)") });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggression_forgives_the_material_judge_not_the_threat() {
        let cfg = Config::load().unwrap();
        let plain = cfg.profile;
        let sharp = cfg.profile.add(&cfg.modifiers["tal"]);
        assert!(penalty(&cfg, &sharp, "material") > penalty(&cfg, &plain, "material"));
        assert_eq!(penalty(&cfg, &sharp, "threat_first"), penalty(&cfg, &plain, "threat_first"));
    }
}
