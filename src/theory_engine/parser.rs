//! The request (the position and the profile the words ask for, each word
//! kept) and the situation (the facts of the position the rules ask about).

use super::config::{Axes, Config};
use super::lexicon::{tokens, Token};
use crate::fast::position::Position;

pub struct Request {
    pub fen: String,
    pub profile: Axes,
    /// the modifiers that shifted the profile
    pub words: Vec<String>,
}

pub fn parse(sentence: &str, cfg: &Config) -> Result<Request, String> {
    let mut fen = None;
    let mut profile = cfg.profile;
    let mut words = Vec::new();
    for t in tokens(sentence, cfg)? {
        match t {
            Token::Fen(f) => fen = Some(f),
            Token::Modifier(m) => {
                profile = profile.add(&cfg.modifiers[&m]);
                words.push(m);
            }
            Token::Filler => {}
        }
    }
    let fen = fen.ok_or("no position: give a FEN")?;
    Position::from_fen(&fen)?;
    Ok(Request { fen, profile, words })
}

/// The facts of the position, from the side to move, each read by a state of the theory.
pub struct Situation {
    pub phase: f32,
    /// material, ours minus theirs, in pawns (the theory's baseline values)
    pub material: f32,
    /// our pieces en prise, in pawns
    pub en_prise: f32,
    pub my_passers: bool,
    pub their_passers: bool,
    pub king_danger: f32,
    pub pieces: u32,
    pub castled: bool,
}

fn state(id: &str, p: &Position, c: usize) -> f32 {
    crate::golden_theory::STATES.iter().find(|(i, _)| *i == id).map(|(_, f)| f(p, c)).unwrap_or(0.0)
}

impl Situation {
    pub fn read(p: &Position) -> Situation {
        Situation::read_for(p, p.turn as usize)
    }
    /// The facts from side `me`, whoever is to move.
    pub fn read_for(p: &Position, me: usize) -> Situation {
        let them = 1 - me;
        let mat = |c| state("material_pawn", p, c) + 3.0 * (state("material_knight", p, c) + state("material_bishop", p, c)) + 5.0 * state("material_rook", p, c) + 9.0 * state("material_queen", p, c);
        let k = p.king_sq64(me as u8).unwrap_or(0);
        let castled = (k / 8 == if me == 0 { 0 } else { 7 }) && (k % 8 <= 2 || k % 8 >= 6);
        Situation {
            phase: crate::golden_theory::phase(p),
            material: mat(me) - mat(them),
            en_prise: -state("defend_in_advance", p, me),
            my_passers: state("passed_pawn", p, me) > 0.0,
            their_passers: state("passed_pawn", p, them) > 0.0,
            king_danger: -state("king_danger", p, me),
            pieces: (1..5).map(|t| (p.bb[t] | p.bb[6 + t]).count_ones()).sum(),
            castled,
        }
    }
    pub fn describe(&self) -> String {
        format!(
            "phase {:.2}, {} pieces, material {:+.1}, en prise {:.1}, king danger {:.1}{}{}{}",
            self.phase,
            self.pieces,
            self.material,
            self.en_prise,
            self.king_danger,
            if self.my_passers { ", our passed pawn" } else { "" },
            if self.their_passers { ", their passed pawn" } else { "" },
            if self.castled { ", castled" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_shift_the_profile() {
        let cfg = Config::load().unwrap();
        let plain = parse("best move in rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &cfg).unwrap();
        let sharp = parse("best move in rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 like Tal", &cfg).unwrap();
        assert!(sharp.profile.initiative > plain.profile.initiative);
        assert_eq!(sharp.words, vec!["tal".to_string()]);
        assert!(parse("best move here", &cfg).is_err());
    }
}
