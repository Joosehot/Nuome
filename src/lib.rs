//! Nuome: math questions in plain words, worked solutions out.
//!
//! sentence -> lexicon (tokens) -> parser (task + problem, with the words
//! behind each value) -> search (beam over solution paths: rules offer
//! steps, the profile and judges score them) -> checks (every answer is
//! proven against the original problem) -> render (numbered steps, each
//! naming its rule). No model in the loop: the same sentence and the same
//! rules.toml always give a byte-identical solution.

pub mod beal;
pub mod bitboard;
pub mod bsd;
pub mod calls;
pub mod attempt;
pub mod checks;
pub mod config;
pub mod conway;
pub mod discover;
pub mod equations;
pub mod abstract_eq;
pub mod evidence;
pub mod evolve;
pub mod explain;
pub mod expr;
pub mod goldbach;
pub mod golden;
pub mod neuro;
pub mod patterns;
pub mod retro;
pub mod golden_terms;
pub mod chess_network;
pub mod chess_rules_data;
pub mod general;
pub mod goldbach_fi;
pub mod hodge;
pub mod ideas;
pub mod lexicon;
// logic and sets (agent L)
pub mod logic;
pub mod mersenne;
pub mod mersenne_ideas;
pub mod model;
pub mod navier_stokes;
pub mod p_vs_np;
pub mod parser;
pub mod poly;
pub mod prime_formula;
pub mod prime_evolve;
pub mod prime_function;
pub mod prime_learn;
pub mod prime_shortcut;
pub mod print;
pub mod proof;
pub mod q;
pub mod render;
pub mod rules;
pub mod scoring;
pub mod search;
pub mod bigprime;
pub mod supergenius;
pub mod supergenius_shortcut;
pub mod three_cubes;
pub mod supergenius_conway;
pub mod supergenius_golden;
pub mod supergenius_eval;
pub mod addmult;
pub mod words;
pub mod yang_mills;
pub mod zeta;
// abstract algebra (agent G)
pub mod abstract_algebra;

use config::Config;
use model::Request;
use parser::{Diag, ParseOptions};
use print::Style;
use search::Outcome;

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Skip unknown words instead of failing.
    pub lenient: bool,
    pub style: Style,
}

pub struct Solved {
    pub request: Request,
    pub outcome: Outcome,
    pub text: String,
}

pub fn solve(sentence: &str, cfg: &Config, opts: &Options) -> Result<Solved, Vec<Diag>> {
    if sentence.trim().is_empty() {
        return Err(vec![Diag::new("nothing to solve").hint("try: nuome \"solve 2x + 3 = 7\"")]);
    }
    let request = parser::parse_with(sentence, &ParseOptions { lenient: opts.lenient, no_open: false }, cfg)?;
    let outcome = search::search(&request, cfg)?;
    let text = render::render(&request, &outcome, cfg, opts.style);
    Ok(Solved { request, outcome, text })
}
