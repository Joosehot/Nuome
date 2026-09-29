//! The rule system. Every piece of math knowledge is one file. A rule looks
//! at the current state and offers moves: each move is one step a person
//! would write down, with the variant it used, the sentence that describes
//! it and any side working. Rules contain no numbers that are taste: how
//! much each variant is worth lives in `rules.toml`.
//!
//! Which rules a task may use is also `rules.toml` (`[tasks]`), so e.g.
//! factoring never sees `distribute`, the rule that would undo it.

use crate::config::Config;
use crate::expr::{Expr, Math};
use crate::model::{Request, Task};
use crate::print::{self, Style};

pub mod cancel;
pub mod clear_denominators;
pub mod collect;
pub mod complete_square;
pub mod diff_chain;
pub mod diff_elementary;
pub mod diff_linear;
pub mod diff_power;
pub mod diff_product;
pub mod diff_quotient;
pub mod diff_squares;
pub mod distribute;
pub mod factor_common;
pub mod factor_solve;
pub mod fold;
pub mod identity;
pub mod move_term;
pub mod multiply_out;
pub mod powers;
pub mod quadratic_formula;
pub mod roots;
pub mod scale;
pub mod solutions;
pub mod square_root;
pub mod substitute;
pub mod swap;
pub mod trinomial;
pub mod verdict;
pub mod zero_product;
// algebra (agent A)
pub mod factor_theorem;
pub mod poly_divide;
pub mod rational_cancel;
pub mod add_fractions;
pub mod power_quotient;
pub mod neg_exponent;
pub mod frac_exponent;
pub mod rationalise;
pub mod compound_fraction;
pub mod log_eval;
pub mod log_laws;
pub mod log_to_exp;
pub mod power_base;
pub mod equate_exponents;
pub mod take_log;
pub mod log_domain;
pub mod ineq_add;
pub mod ineq_scale;
pub mod ineq_swap;
pub mod ineq_verdict;
pub mod sign_chart;
pub mod abs_split;
pub mod abs_check;

/// One piece of a line of explanation: words, or math printed in the
/// output's style.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    T(String),
    E(Expr),
    M(Math),
    /// ± (printed "+/-" in ASCII).
    Pm,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line(pub Vec<Piece>);

impl Line {
    pub fn new() -> Line {
        Line(Vec::new())
    }
    pub fn t(mut self, s: impl Into<String>) -> Line {
        self.0.push(Piece::T(s.into()));
        self
    }
    pub fn e(mut self, e: &Expr) -> Line {
        self.0.push(Piece::E(e.clone()));
        self
    }
    pub fn m(mut self, m: &Math) -> Line {
        self.0.push(Piece::M(m.clone()));
        self
    }
    pub fn join(mut self, other: Line) -> Line {
        self.0.extend(other.0);
        self
    }
    pub fn pm(mut self) -> Line {
        self.0.push(Piece::Pm);
        self
    }
    pub fn render(&self, s: Style) -> String {
        let mut out = String::new();
        for p in &self.0 {
            match p {
                Piece::T(t) => out.push_str(t),
                Piece::E(e) => out.push_str(&print::expr(e, s)),
                Piece::M(m) => out.push_str(&print::math(m, s)),
                Piece::Pm => out.push_str(match s {
                    Style::Ascii => "+/-",
                    Style::Unicode => "±",
                    Style::Latex => "\\pm",
                }),
            }
        }
        out
    }
}

/// One step.
#[derive(Clone, Debug)]
pub struct Move {
    pub rule: &'static str,
    pub variant: &'static str,
    pub result: Math,
    pub says: Line,
    pub work: Vec<Line>,
}

/// What a rule can see besides the state.
pub struct Cx<'a> {
    pub req: &'a Request,
    pub cfg: &'a Config,
    /// The letter being solved for or differentiated by.
    pub var: &'a str,
}

impl Cx<'_> {
    pub fn task(&self) -> Task {
        self.req.task.value
    }
}

pub trait Rule: Sync {
    fn name(&self) -> &'static str;
    /// Every variant the rule's moves can carry (for config validation).
    fn variants(&self) -> &'static [&'static str];
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move>;
}

pub fn all() -> Vec<&'static dyn Rule> {
    vec![
        &substitute::Substitute,
        &fold::Fold,
        &identity::Identity,
        &cancel::Cancel,
        &roots::Roots,
        &powers::Powers,
        &collect::Collect,
        &distribute::Distribute,
        &multiply_out::MultiplyOut,
        &factor_common::FactorCommon,
        &diff_squares::DiffSquares,
        &trinomial::Trinomial,
        &clear_denominators::ClearDenominators,
        &move_term::MoveTerm,
        &scale::Scale,
        &swap::Swap,
        &verdict::Verdict,
        &factor_solve::FactorSolve,
        &zero_product::ZeroProduct,
        &quadratic_formula::QuadraticFormula,
        &square_root::SquareRoot,
        &complete_square::CompleteSquare,
        &solutions::Solutions,
        &diff_linear::DiffLinear,
        &diff_power::DiffPower,
        &diff_elementary::DiffElementary,
        &diff_product::DiffProduct,
        &diff_quotient::DiffQuotient,
        &diff_chain::DiffChain,
        // algebra (agent A)
        &factor_theorem::FactorTheorem,
        &poly_divide::PolyDivide,
        &rational_cancel::RationalCancel,
        &add_fractions::AddFractions,
        &power_quotient::PowerQuotient,
        &neg_exponent::NegExponent,
        &frac_exponent::FracExponent,
        &rationalise::Rationalise,
        &compound_fraction::CompoundFraction,
        &log_eval::LogEval,
        &log_laws::LogLaws,
        &log_to_exp::LogToExp,
        &power_base::PowerBase,
        &equate_exponents::EquateExponents,
        &take_log::TakeLog,
        &log_domain::LogDomain,
        &ineq_add::IneqAdd,
        &ineq_scale::IneqScale,
        &ineq_swap::IneqSwap,
        &ineq_verdict::IneqVerdict,
        &sign_chart::SignChart,
        &abs_split::AbsSplit,
        &abs_check::AbsCheck,
    ]
}

pub fn by_name(name: &str) -> Option<&'static dyn Rule> {
    all().into_iter().find(|r| r.name() == name)
}

/// Where a local rewrite happens.
pub struct At<'a> {
    pub math: &'a Math,
    pub slot: usize,
    pub path: &'a [usize],
}

impl At<'_> {
    /// Is this node a whole side of an equation whose other side is 0?
    pub fn is_side_against_zero(&self) -> bool {
        self.path.is_empty()
            && match self.math {
                Math::Eq(l, r) => (if self.slot == 0 { r } else { l }).is_num(0),
                Math::Or(v) => {
                    let (l, r) = &v[self.slot / 2];
                    (if self.slot % 2 == 0 { r } else { l }).is_num(0)
                }
                Math::Ineq(l, _, r) => (if self.slot == 0 { r } else { l }).is_num(0),
                _ => false,
            }
    }
    /// Is this node inside a denominator that contains a letter? Those stay
    /// factored: (2x + 1)/(x(x + 1)), not /(x^2 + x).
    pub fn in_letter_denominator(&self) -> bool {
        let e = self.math.slots()[self.slot];
        (0..self.path.len()).any(|k| self.path[k] == 1 && matches!(e.get(&self.path[..k]), Expr::Div(_, d) if !d.vars().is_empty()))
    }
}

pub struct Rewrite {
    pub variant: &'static str,
    pub new: Expr,
    pub says: Line,
    pub work: Vec<Line>,
}

/// Run a rewrite over every node of every expression in the state. When the
/// same rewrite applies in several places (sqrt(12) in both alternatives,
/// d/dx[x] twice), it is one step that does them all.
pub fn local(rule: &'static str, m: &Math, f: impl Fn(&Expr, &At) -> Vec<Rewrite>) -> Vec<Move> {
    struct Found {
        slot: usize,
        path: Vec<usize>,
        node: Expr,
        rw: Rewrite,
    }
    let mut found: Vec<Found> = Vec::new();
    for (slot, e) in m.slots().into_iter().enumerate() {
        for (path, node) in e.walk() {
            let at = At { math: m, slot, path: &path };
            for rw in f(node, &at) {
                found.push(Found { slot, path: path.clone(), node: node.clone(), rw });
            }
        }
    }
    let mut out = Vec::new();
    let mut done = vec![false; found.len()];
    for i in 0..found.len() {
        if done[i] {
            continue;
        }
        // every other place with the same node rewritten the same way
        // (or the same step in the same words: "power rule" on every term, "cancel the common factor 2" in each alternative)
        let across = |j: usize| found[j].rw.says == found[i].rw.says && !found[i].rw.says.0.is_empty() && found[j].rw.work.is_empty() && found[i].rw.work.is_empty();
        let same: Vec<usize> = (i..found.len())
            .filter(|&j| !done[j] && found[j].rw.variant == found[i].rw.variant && ((found[j].node == found[i].node && found[j].rw.new == found[i].rw.new) || across(j)))
            .collect();
        // don't combine a node with one inside it
        let mut picked: Vec<usize> = Vec::new();
        for j in same {
            if picked.iter().all(|&k| found[k].slot != found[j].slot || !(found[j].path.starts_with(&found[k].path) || found[k].path.starts_with(&found[j].path))) {
                picked.push(j);
            }
        }
        // replace raw (the picked paths don't nest, so none moves), then tidy once
        let mut slots: Vec<Expr> = m.slots().into_iter().cloned().collect();
        for &j in &picked {
            done[j] = true;
            let fj = &found[j];
            slots[fj.slot] = slots[fj.slot].replace_raw(&fj.path, fj.rw.new.clone());
        }
        let mut result = m.clone();
        for (k, e) in slots.into_iter().enumerate() {
            result = result.with_slot(k, crate::expr::tidy(e));
        }
        let first = &found[i].rw;
        out.push(Move { rule, variant: first.variant, result, says: first.says.clone(), work: first.work.clone() });
    }
    out
}

/// What an equation turns into.
#[derive(Clone)]
pub enum Branch {
    One(Expr, Expr),
    Many(Vec<(Expr, Expr)>),
    Nothing,
    Everything,
}

#[derive(Clone)]
pub struct EqRewrite {
    pub variant: &'static str,
    pub to: Branch,
    pub says: Line,
    pub work: Vec<Line>,
}

/// Run an equation rewrite over the equation, or each alternative of an Or.
pub fn per_eq(rule: &'static str, m: &Math, f: impl Fn(&Expr, &Expr) -> Vec<EqRewrite>) -> Vec<Move> {
    let eqs: Vec<(Expr, Expr)> = match m {
        Math::Eq(l, r) => vec![(l.clone(), r.clone())],
        Math::Or(v) => v.clone(),
        _ => return vec![],
    };
    let mut out = Vec::new();
    let per: Vec<Vec<EqRewrite>> = eqs.iter().map(|(l, r)| f(l, r)).collect();
    // the same step in several alternatives ("add 2 to both sides" in each) is one step
    if eqs.len() > 1 {
        for (i, rws) in per.iter().enumerate() {
            for rw in rws {
                let Branch::One(..) = rw.to else { continue };
                let mut pick: Vec<(usize, &EqRewrite)> = vec![(i, rw)];
                for (j, other) in per.iter().enumerate().skip(i + 1) {
                    if let Some(o) = other.iter().find(|o| o.variant == rw.variant && o.says == rw.says && matches!(o.to, Branch::One(..))) {
                        pick.push((j, o));
                    }
                }
                if pick.len() < 2 || pick[0].0 != i || per[..i].iter().any(|p| p.iter().any(|o| o.variant == rw.variant && o.says == rw.says)) {
                    continue;
                }
                let mut new = eqs.clone();
                for (j, o) in &pick {
                    if let Branch::One(a, b) = &o.to {
                        new[*j] = (a.clone(), b.clone());
                    }
                }
                let lead = if pick.len() == eqs.len() && eqs.len() == 2 { "In both: " } else { "In each: " };
                out.push(Move { rule, variant: rw.variant, result: Math::Or(new), says: Line::new().t(lead).join(lower_first(&rw.says)), work: rw.work.clone() });
            }
        }
    }
    for (i, (l, r)) in eqs.iter().enumerate() {
        for rw in per[i].iter().cloned() {
            let mut rest: Vec<(Expr, Expr)> = Vec::new();
            let mut everything = false;
            for (j, eq) in eqs.iter().enumerate() {
                if j != i {
                    rest.push(eq.clone());
                    continue;
                }
                match &rw.to {
                    Branch::One(a, b) => rest.push((a.clone(), b.clone())),
                    Branch::Many(v) => rest.extend(v.iter().cloned()),
                    Branch::Nothing => {}
                    Branch::Everything => everything = true,
                }
            }
            let result = if everything {
                Math::AllReals
            } else {
                match rest.len() {
                    0 => Math::NoSolution,
                    1 => {
                        let (a, b) = rest.pop().unwrap();
                        Math::Eq(a, b)
                    }
                    _ => Math::Or(rest),
                }
            };
            let says = if eqs.len() > 1 { Line::new().t("In ").m(&Math::Eq(l.clone(), r.clone())).t(": ").join(lower_first(&rw.says)) } else { rw.says };
            out.push(Move { rule, variant: rw.variant, result, says, work: rw.work });
        }
    }
    out
}

fn lower_first(l: &Line) -> Line {
    let mut l = l.clone();
    if let Some(Piece::T(t)) = l.0.first_mut() {
        let mut c = t.chars();
        if let Some(f) = c.next() {
            *t = f.to_lowercase().collect::<String>() + c.as_str();
        }
    }
    l
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.render(Style::Ascii))
    }
}

/// Test support: a request for a sentence.
#[cfg(test)]
pub fn test_cx_req(sentence: &str) -> Request {
    crate::parser::parse(sentence, &Default::default()).unwrap_or_else(|d| panic!("{sentence}: {d:?}"))
}

/// Test support: the moves a rule offers for the sentence's starting state.
#[cfg(test)]
pub fn test_moves(rule: &dyn Rule, sentence: &str) -> Vec<String> {
    let req = test_cx_req(sentence);
    let cfg = Config::builtin();
    let cx = Cx { req: &req, cfg: &cfg, var: &req.var.value };
    rule.moves(&req.start(), &cx).iter().map(|m| print::math(&m.result, Style::Ascii)).collect()
}
