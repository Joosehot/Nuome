//! Beam search over plans. A plan is our next `search.plan` moves with the
//! opponent's reply between them, the reply chosen by the rules-only search
//! (what the rules of the game force, nothing more). Each step expands every
//! partial plan with the variants the rules offer in its position; a variant
//! scores
//!     weight x measure x (axes . profile) x axis_cp
//! (later plan moves x follow_weight); the position after our first move is
//! valued by the rules-only search (value_ms: what the rules of the game
//! force from there, in cp) and the judges score the partial plan after
//! every step. The theory offers and tilts; the rules value and prove. The best `search.width` plans survive. Ties keep candidate
//! order, so the result is deterministic for a given reply search.

use super::config::{Axes, Config};
use super::judges::{self, Note};
use super::parser::Situation;
use super::rules::{self, Variant};
use crate::fast::movegen::legal_moves;
use crate::fast::position::{move_to_uci, Position};
use crate::star_search::{analyse, Eval as _, RulesEval};

#[derive(Clone)]
pub struct Step {
    pub mv: u32,
    /// None = the opponent's reply
    pub variant: Option<Variant>,
    /// the other rules that offered the same move
    pub also: Vec<&'static str>,
    pub tilt: i32,
}

#[derive(Clone)]
pub struct Plan {
    pub steps: Vec<Step>,
    pub end: Position,
    /// the rules-only search's value after our first move, cp from our side
    pub material: i32,
    pub notes: Vec<Note>,
}

impl Plan {
    pub fn first(&self) -> u32 {
        self.steps[0].mv
    }
    pub fn score(&self) -> i32 {
        self.material + self.steps.iter().map(|s| s.tilt).sum::<i32>() + self.notes.iter().map(|n| n.cp).sum::<i32>()
    }
    pub fn line(&self) -> String {
        self.steps
            .iter()
            .map(|s| match &s.variant {
                Some(v) => format!("{}[{} {:+}: {}{}]", move_to_uci(s.mv), v.rule, s.tilt, v.why, s.also.iter().map(|a| format!(" +{a}")).collect::<String>()),
                None => move_to_uci(s.mv),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Variants merged per move: a move several rules offer keeps the rule with the largest tilt, the others noted.
fn merged(p: &Position, sit: &Situation, profile: &Axes, cfg: &Config, follow: f32) -> Vec<(Variant, Vec<&'static str>, i32)> {
    let mut out: Vec<(Variant, Vec<&'static str>, i32)> = Vec::new();
    for v in rules::variants(p, sit, cfg) {
        let tilt = (v.weight * v.measure * v.axes.dot(profile) * cfg.score.axis_cp * follow).round() as i32;
        match out.iter_mut().find(|x| x.0.mv == v.mv) {
            Some(x) => {
                if tilt > x.2 {
                    let mut also = vec![x.0.rule];
                    also.append(&mut x.1);
                    *x = (v, also, tilt);
                } else {
                    x.1.push(v.rule);
                }
            }
            None => out.push((v, Vec::new(), tilt)),
        }
    }
    out
}

/// Material by the rules alone, cp from side `me`.
fn material_for(p: &Position, me: usize) -> i32 {
    let v = RulesEval.eval(p);
    if p.turn as usize == me { v } else { -v }
}

pub fn search(root: &Position, root_sit: &Situation, profile: &Axes, cfg: &Config) -> Vec<Plan> {
    let me = root.turn as usize;
    let mut beam: Vec<Plan> = vec![Plan { steps: Vec::new(), end: root.clone(), material: 0, notes: Vec::new() }];
    for step in 0..cfg.search.plan {
        let mut next: Vec<Plan> = Vec::new();
        for plan in &beam {
            let mut pos = plan.end.clone();
            let mut base = plan.steps.clone();
            if step > 0 {
                // the opponent's reply by the rules alone
                if legal_moves(&mut pos).is_empty() {
                    next.push(plan.clone());
                    continue;
                }
                let Some((r, _)) = analyse(&RulesEval, None, &pos.to_fen(), cfg.search.reply_ms) else { next.push(plan.clone()); continue };
                let Some(rm) = crate::fast::movegen::uci_to_move(&mut pos, &r) else { next.push(plan.clone()); continue };
                pos.make(rm);
                base.push(Step { mv: rm, variant: None, also: Vec::new(), tilt: 0 });
            }
            if legal_moves(&mut pos).is_empty() {
                next.push(plan.clone());
                continue;
            }
            let sit_here;
            let sit = if step == 0 { root_sit } else { sit_here = Situation::read(&pos); &sit_here };
            let follow = if step == 0 { 1.0 } else { cfg.score.follow_weight };
            let mut vs = merged(&pos, sit, profile, cfg, follow);
            vs.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.mv.cmp(&b.0.mv)));
            vs.truncate(cfg.search.variants);
            for (v, also, tilt) in vs {
                let mut q = pos.clone();
                q.make(v.mv);
                let mut steps = base.clone();
                let first_is_pawn = steps.is_empty() && crate::fast::position::type_of(root.board[crate::fast::position::m_from(v.mv) as usize]) == 1;
                steps.push(Step { mv: v.mv, variant: Some(v), also, tilt });
                let mut after_first = root.clone();
                after_first.make(steps[0].mv);
                let material = if step == 0 { -analyse(&RulesEval, None, &q.to_fen(), cfg.search.value_ms).map(|r| r.1).unwrap_or(material_for(&q, me)) } else { plan.material };
                let notes = judges::judge(root, root_sit, &after_first, &q, first_is_pawn || root_first_is_pawn(root, steps[0].mv), profile, cfg);
                next.push(Plan { steps, end: q, material, notes });
            }
        }
        next.sort_by(|a, b| b.score().cmp(&a.score()).then(a.first().cmp(&b.first())));
        next.truncate(cfg.search.width);
        beam = next;
    }
    beam
}

fn root_first_is_pawn(root: &Position, m: u32) -> bool {
    crate::fast::position::type_of(root.board[crate::fast::position::m_from(m) as usize]) == 1
}
