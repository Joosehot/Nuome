//! Abstract algebra (agent G): proofs from the axioms of a group or a ring.
//!
//! "In a group, prove (ab)^-1 = b^-1 a^-1" becomes an equation between
//! group terms (`term::Term`, wrapped in one `Expr::Alg` leaf so no rule
//! for numbers ever looks inside). The proof is a chain found by the beam
//! search: each step rewrites one subterm by one axiom, one lemma or one
//! hypothesis, or replaces the goal by what a lemma says is enough to show.
//! Lemmas are laws with their own proofs from the axioms (and the lemmas
//! before them); a proof may cite a lemma only if that lemma's proof checks.
//!
//! Checks (`check.rs`), independent of the rules: every step is found again
//! by matching the named law against the two lines; every cited lemma is
//! proved again; the statement is tried in concrete groups (or rings) for
//! every assignment of elements, and a false statement is refused with the
//! elements that break it.

pub mod check;
pub mod models;
pub mod moves;
pub mod parse;
pub mod term;

use crate::config::Config;
use crate::expr::{Expr, Math};
use crate::lexicon::{Tok, Token};
use crate::model::{Calc, Request, Said, Task};
use crate::parser::Diag;
use crate::print::Style;
use parse::{Eqn, Statement};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;
use term::Term;

/// What the letters are elements of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Group,
    /// A group whose product commutes.
    Abelian,
    Ring,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Group => "group",
            Kind::Abelian => "abelian group",
            Kind::Ring => "ring",
        }
    }
    pub fn example(self) -> &'static str {
        match self {
            Kind::Group | Kind::Abelian => "(ab)^-1 = b^-1 a^-1",
            Kind::Ring => "a0 = 0",
        }
    }
    fn family(self) -> Kind {
        match self {
            Kind::Abelian => Kind::Group,
            k => k,
        }
    }
}

/// Tunable numbers ([abstract_algebra] in rules.toml).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cfg {
    /// No step may make a side bigger than this many symbols.
    pub max_size: usize,
    /// Distance to done per symbol of both sides (ranks unfinished proofs).
    pub size_weight: f64,
    /// Distance to done while the sides differ.
    pub mismatch: f64,
    /// Distance added while the sides' normal forms differ (the freely
    /// reduced words of a group, the multiplied-out sums of a ring): only a
    /// hypothesis can close that gap.
    pub normal_gap: f64,
    /// Distance taken off while a side holds an instance of a side of a
    /// hypothesis (one step from using it).
    pub hypothesis_pull: f64,
    /// The model check skips a model with more assignments than this.
    pub max_assignments: usize,
}

/// The [tasks] lists of a group proof and a ring proof.
pub const TASK_KEYS: [&str; 2] = ["prove_group", "prove_ring"];

pub fn task_key(kind: Kind) -> &'static str {
    if kind == Kind::Ring {
        TASK_KEYS[1]
    } else {
        TASK_KEYS[0]
    }
}

pub fn validate(cfg: &Config) -> anyhow::Result<()> {
    let c = &cfg.abstract_algebra;
    if c.max_size < 5 || c.max_assignments < 1000 {
        anyhow::bail!("[abstract_algebra]: max_size must be at least 5 and max_assignments at least 1000");
    }
    if [c.size_weight, c.mismatch, c.normal_gap, c.hypothesis_pull].iter().any(|x| !(x.is_finite() && *x >= 0.0)) {
        anyhow::bail!("[abstract_algebra]: size_weight, mismatch, normal_gap and hypothesis_pull must be numbers >= 0");
    }
    for k in TASK_KEYS {
        if !cfg.tasks.contains_key(k) {
            anyhow::bail!("[tasks] has no entry for {k}");
        }
    }
    // the search stops once a proof is finished, which is right only if no
    // step can raise a proof's score, under any profile the words can ask for
    let mut profiles = vec![cfg.profile];
    profiles.extend(cfg.modifiers.values().map(|m| cfg.profile.add(m)));
    for key in TASK_KEYS {
        for name in &cfg.tasks[key] {
            let Some(rc) = cfg.rules.get(name) else { continue };
            for (v, axes) in &rc.variants {
                for p in &profiles {
                    let cost = cfg.steps.cost + cfg.steps.brevity_cost * p.brevity.max(0.0);
                    let cost = if rc.minor { cost * cfg.steps.minor_share } else { cost };
                    if rc.weight * axes.dot(p) > cost {
                        anyhow::bail!("[rules.{name}.variants.{v}] is worth more than a step costs; a group or ring proof step must not gain score");
                    }
                }
            }
        }
    }
    for d in LAWS {
        let key = task_key(d.family);
        if !cfg.tasks[key].iter().any(|r| r == d.rule) {
            anyhow::bail!("[tasks] {key} doesn't list {}", d.rule);
        }
    }
    Ok(())
}

/// An axiom or a lemma: which rule applies it and what it says.
pub struct LawDef {
    pub rule: &'static str,
    pub title: &'static str,
    pub text: &'static str,
    pub family: Kind,
    pub lemma: bool,
    /// Only in an abelian group (commutativity).
    pub abelian: bool,
}

const fn axiom(rule: &'static str, title: &'static str, text: &'static str, family: Kind) -> LawDef {
    LawDef { rule, title, text, family, lemma: false, abelian: false }
}
const fn lemma(rule: &'static str, title: &'static str, text: &'static str, family: Kind) -> LawDef {
    LawDef { rule, title, text, family, lemma: true, abelian: false }
}

use crate::rules as r;

/// Every law, lemmas in the order they are proved: a lemma's proof may
/// cite only the lemmas before it.
pub const LAWS: &[LawDef] = &[
    axiom("group_assoc", "Associativity", r::group_assoc::LAW, Kind::Group),
    axiom("group_identity", "Identity", r::group_identity::LAW, Kind::Group),
    axiom("group_inverse", "Inverse", r::group_inverse::LAW, Kind::Group),
    LawDef { rule: "group_comm", title: "Commutativity", text: r::group_comm::LAW, family: Kind::Group, lemma: false, abelian: true },
    lemma("group_inverse_unique", "Uniqueness of inverses", r::group_inverse_unique::LAW, Kind::Group),
    lemma("group_cancel_left", "Left cancellation", r::group_cancel_left::LAW, Kind::Group),
    lemma("group_cancel_right", "Right cancellation", r::group_cancel_right::LAW, Kind::Group),
    lemma("group_inverse_inverse", "Inverse of an inverse", r::group_inverse_inverse::LAW, Kind::Group),
    lemma("group_inverse_identity", "Inverse of the identity", r::group_inverse_identity::LAW, Kind::Group),
    lemma("group_inverse_product", "Inverse of a product", r::group_inverse_product::LAW, Kind::Group),
    axiom("ring_add_assoc", "Associativity of +", r::ring_add_assoc::LAW, Kind::Ring),
    axiom("ring_add_comm", "Commutativity of +", r::ring_add_comm::LAW, Kind::Ring),
    axiom("ring_zero", "Zero", r::ring_zero::LAW, Kind::Ring),
    axiom("ring_neg", "Negative", r::ring_neg::LAW, Kind::Ring),
    axiom("ring_mul_assoc", "Associativity of multiplication", r::ring_mul_assoc::LAW, Kind::Ring),
    axiom("ring_distrib", "Distributivity", r::ring_distrib::LAW, Kind::Ring),
    lemma("ring_add_cancel", "Cancellation for +", r::ring_add_cancel::LAW, Kind::Ring),
    lemma("ring_neg_unique", "Uniqueness of negatives", r::ring_neg_unique::LAW, Kind::Ring),
    lemma("ring_mul_zero", "Times zero", r::ring_mul_zero::LAW, Kind::Ring),
    lemma("ring_zero_mul", "Zero times", r::ring_zero_mul::LAW, Kind::Ring),
    lemma("ring_neg_neg", "Negative of a negative", r::ring_neg_neg::LAW, Kind::Ring),
    lemma("ring_neg_mul", "Negative times", r::ring_neg_mul::LAW, Kind::Ring),
    lemma("ring_mul_neg", "Times a negative", r::ring_mul_neg::LAW, Kind::Ring),
];

/// A law ready to match: pattern variables are renamed "?x" so they never
/// meet the letters of a problem.
pub struct Law {
    pub def: &'static LawDef,
    /// The equations (the conclusion, for a law with a condition).
    pub eqs: Vec<(Term, Term)>,
    pub conds: Vec<(Term, Term)>,
    pub vars: BTreeSet<String>,
    /// As written, for printing.
    pub shown: Statement,
    /// Place among the lemmas of its family.
    pub index: usize,
}

pub const VAR: &str = "?";

fn build_law(def: &'static LawDef, index: usize) -> Law {
    let toks = crate::lexicon::lex(def.text).expect("a law lexes");
    let (st, _) = parse::read(&toks, def.family).unwrap_or_else(|d| panic!("law {}: {d:?}", def.rule));
    let mut letters = BTreeSet::new();
    for e in st.hyps.iter().chain(&st.goals) {
        letters.extend(e.l.letters());
        letters.extend(e.r.letters());
    }
    let ren = |t: &Term| t.rename(&letters, VAR);
    Law {
        def,
        eqs: st.goals.iter().map(|e| (ren(&e.l), ren(&e.r))).collect(),
        conds: st.hyps.iter().map(|e| (ren(&e.l), ren(&e.r))).collect(),
        vars: letters.iter().map(|v| format!("{VAR}{v}")).collect(),
        shown: st,
        index,
    }
}

pub fn laws() -> &'static [Law] {
    static L: OnceLock<Vec<Law>> = OnceLock::new();
    L.get_or_init(|| {
        let mut count = std::collections::BTreeMap::new();
        LAWS.iter()
            .map(|d| {
                let k = count.entry(d.family).or_insert(0usize);
                let i = *k;
                if d.lemma {
                    *k += 1;
                }
                build_law(d, i)
            })
            .collect()
    })
}

pub fn law(rule: &str) -> Option<&'static Law> {
    laws().iter().find(|l| l.def.rule == rule)
}

/// The sentence that proves a lemma on its own.
pub fn lemma_sentence(def: &LawDef) -> String {
    format!("in a {}, prove {}", def.family.name(), def.text)
}

/// A problem about a group or a ring: what the search and the checks need
/// besides the goal.
#[derive(Clone, Debug)]
pub struct Setting {
    pub kind: Kind,
    pub hyps: Vec<Eqn>,
    /// Lemmas of the family with a place below this may be cited.
    pub lemmas: usize,
    /// Terms a step may bring in (e = aa^-1 needs an a): every piece of the
    /// statement.
    pub pool: Vec<Term>,
    /// The letters alone: what an axiom writes in (e = a^-1 a).
    pub letters: Vec<Term>,
    /// [abstract_algebra] from rules.toml (the distance to done needs it).
    pub cfg: Cfg,
}

impl Setting {
    /// May a proof in this setting use `law`?
    pub fn allows(&self, law: &Law) -> bool {
        law.def.family == self.kind.family() && (!law.def.abelian || self.kind == Kind::Abelian) && (!law.def.lemma || law.index < self.lemmas)
    }
    /// The hypotheses as patterns: "for all" letters renamed "!a".
    pub fn hyp_patterns(&self) -> Vec<(Term, Term, BTreeSet<String>)> {
        self.hyps
            .iter()
            .map(|h| {
                let all: BTreeSet<String> = h.all.iter().cloned().collect();
                (h.l.rename(&all, "!"), h.r.rename(&all, "!"), all.iter().map(|v| format!("!{v}")).collect())
            })
            .collect()
    }
}

/// Is the state an equation between terms of a structure?
pub fn sides(m: &Math) -> Option<(&Term, &Term)> {
    match m {
        Math::Eq(Expr::Alg(l), Expr::Alg(r)) => Some((l, r)),
        _ => None,
    }
}

pub fn alg(t: &Term) -> Expr {
    Expr::Alg(t.clone())
}

/// A statement's letters renamed in order of appearance, for comparing
/// a problem with a lemma up to the names of the letters.
fn canonical(hyps: &[(Term, Term, Vec<String>)], goal: &(Term, Term)) -> String {
    let mut order: Vec<String> = Vec::new();
    let mut note = |t: &Term| {
        for (_, n) in t.walk() {
            if let Term::El(v) = n {
                if !order.contains(v) {
                    order.push(v.clone());
                }
            }
        }
    };
    for (l, r, _) in hyps {
        note(l);
        note(r);
    }
    note(&goal.0);
    note(&goal.1);
    let name = |t: &Term| term::show(&t.map_letters(&|v| format!("#{}", order.iter().position(|o| o == v).unwrap_or(0))), Style::Ascii);
    let hs: Vec<String> = hyps.iter().map(|(l, r, all)| format!("{}={}{:?}", name(l), name(r), all.iter().map(|a| order.iter().position(|v| v == a)).collect::<Vec<_>>())).collect();
    format!("{} => {}={}", hs.join(";"), name(&goal.0), name(&goal.1))
}

/// Every orientation of every equation of a statement, canonically named.
fn spellings(st: &Statement) -> Vec<String> {
    let Some(g) = st.goals.first() else { return vec![] };
    let n = st.hyps.len() + 1;
    (0..1usize << n)
        .map(|mask| {
            let hyps: Vec<(Term, Term, Vec<String>)> = st.hyps.iter().enumerate().map(|(i, h)| if mask >> (i + 1) & 1 == 1 { (h.r.clone(), h.l.clone(), h.all.clone()) } else { (h.l.clone(), h.r.clone(), h.all.clone()) }).collect();
            let goal = if mask & 1 == 1 { (g.r.clone(), g.l.clone()) } else { (g.l.clone(), g.r.clone()) };
            canonical(&hyps, &goal)
        })
        .collect()
}

/// Which lemma a statement is, if it is one (up to letters and sides).
pub fn which_lemma(kind: Kind, st: &Statement) -> Option<&'static Law> {
    let mine = spellings(st);
    laws().iter().filter(|l| l.def.lemma && l.def.family == kind.family()).find(|l| {
        let theirs = spellings(&l.shown);
        theirs.first().is_some_and(|t| mine.contains(t))
    })
}

/// Every piece of the statement, smallest first: what a step may bring in.
fn pool(st: &Statement) -> Vec<Term> {
    let mut set: BTreeSet<(usize, Term)> = BTreeSet::new();
    for e in st.hyps.iter().chain(&st.goals) {
        let all: BTreeSet<String> = e.all.iter().cloned().collect();
        for side in [&e.l, &e.r] {
            for (_, t) in side.walk() {
                if !matches!(t, Term::E | Term::Zero) && !t.has_letter_in(&all) {
                    set.insert((t.size(), t.clone()));
                }
            }
        }
    }
    set.into_iter().map(|(_, t)| t).collect()
}

/// The sentence is about a group or a ring: read it with this grammar.
/// None when it isn't (the ordinary parser takes it).
pub fn request(sentence: &str, toks: &[Token], cfg: &Config) -> Option<Result<Request, Vec<Diag>>> {
    let named: Vec<(Kind, &str)> = toks
        .iter()
        .filter_map(|t| match t.tok {
            Tok::Structure(k) | Tok::Statement(k, _) => Some((k, t.words.as_str())),
            Tok::InverseOf => Some((Kind::Group, t.words.as_str())),
            _ => None,
        })
        .collect();
    if named.is_empty() {
        return None;
    }
    Some(build(sentence, toks, &named, cfg))
}

fn build(sentence: &str, toks: &[Token], named: &[(Kind, &str)], cfg: &Config) -> Result<Request, Vec<Diag>> {
    // the structure: an abelian group is a group; a group and a ring clash
    let structures: Vec<(Kind, &str)> = toks.iter().filter_map(|t| if let Tok::Structure(k) = t.tok { Some((k, t.words.as_str())) } else { None }).collect();
    let pick = if structures.is_empty() { named } else { &structures[..] };
    let kind = if pick.iter().any(|(k, _)| *k == Kind::Ring) {
        if let Some((_, w)) = pick.iter().chain(named).find(|(k, _)| *k != Kind::Ring) {
            return Err(vec![Diag::new(format!("\"{w}\" is about groups, but the sentence is about a ring")).hint("one structure at a time")]);
        }
        Kind::Ring
    } else if pick.iter().any(|(k, _)| *k == Kind::Abelian) {
        Kind::Abelian
    } else {
        Kind::Group
    };
    let toks = parse::splice(toks).map_err(|d| vec![d])?;
    let (st, ex) = parse::read(&toks, kind)?;
    let goal = match &st.goals[..] {
        [g] => g.clone(),
        [] => return Err(vec![Diag::new(format!("prove what about a {}?", kind.name())).hint(format!("state an equation, e.g. \"in a {}, prove {}\"", kind.name(), kind.example()))]),
        gs => {
            let all: Vec<String> = gs.iter().map(|g| format!("\"{}\"", g.words)).collect();
            return Err(vec![Diag::new(format!("more than one claim: {}", all.join(", "))).hint("one statement at a time; put what is given after \"if\"")]);
        }
    };
    // a statement that fails in a concrete structure is refused before any search
    let v = models::check(kind, &st, &goal, cfg.abstract_algebra.max_assignments);
    if let Some(c) = v.counterexample {
        let shown = statement_text(&st, &goal, Style::Ascii);
        return Err(vec![Diag::new(format!("{shown} is not true in every {}", kind.name())).hint(c)]);
    }
    let lemmas = which_lemma(kind, &st).map_or(usize::MAX, |l| l.index);
    let setting = Setting { kind, hyps: st.hyps.clone(), lemmas, letters: pool(&st).into_iter().filter(|t| matches!(t, Term::El(_))).collect(), pool: pool(&st), cfg: cfg.abstract_algebra.clone() };
    let task = ex.task.map_or_else(|| Said::new(Task::Prove, "(a statement: prove)"), |t| Said::new(Task::Prove, t.words));
    let modifiers = ex.modifiers.iter().filter_map(|t| if let Tok::Mod(m) = t.tok { Some(Said::new(m, t.words.clone())) } else { None }).collect();
    let first = goal.l.letters().into_iter().chain(goal.r.letters()).next().unwrap_or_else(|| "x".into());
    Ok(Request {
        sentence: sentence.trim().to_string(),
        task,
        problem: Said::new(Math::Eq(alg(&goal.l), alg(&goal.r)), goal.words.clone()),
        var: Said::new(first, "(the first letter)"),
        given: vec![],
        method: None,
        modifiers,
        decimals: None,
        notes: vec![],
        calc: Calc::default(),
        structure: Some(setting),
    })
}

fn eqn_text(e: &Eqn, s: Style) -> String {
    let all = if e.all.is_empty() { String::new() } else { format!(" for all {}", e.all.join(" and ")) };
    format!("{} = {}{all}", term::show(&e.l, s), term::show(&e.r, s))
}

pub fn statement_text(st: &Statement, goal: &Eqn, s: Style) -> String {
    let g = eqn_text(goal, s);
    if st.hyps.is_empty() {
        return g;
    }
    let hs: Vec<String> = st.hyps.iter().map(|h| eqn_text(h, s)).collect();
    format!("if {} then {g}", hs.join(" and "))
}

/// "Prove in a group: if ab = ac then b = c".
pub fn header(req: &Request, s: Style) -> Option<String> {
    let st = req.structure.as_ref()?;
    let (l, r) = sides(&req.problem.value)?;
    let goal = Eqn { l: l.clone(), r: r.clone(), all: vec![], words: String::new() };
    let text = statement_text(&Statement { hyps: st.hyps.clone(), goals: vec![] }, &goal, s);
    Some(format!("Prove in {} {}: {text}", if st.kind == Kind::Abelian { "an" } else { "a" }, st.kind.name()))
}

/// Why the search found nothing, for a statement about a structure.
pub fn why_not(req: &Request, cfg: &Config) -> Option<String> {
    let st = req.structure.as_ref()?;
    Some(format!(
        "it holds in every test {} tried, but no chain of at most {} steps, each one axiom, lemma or hypothesis, was found; Nuome refuses rather than guess",
        st.kind.name(),
        cfg.search.max_steps
    ))
}

/// How far a proof state is from both sides reading the same.
pub fn distance(l: &Term, r: &Term, req: &Request) -> f64 {
    let Some(st) = &req.structure else { return 0.0 };
    if l == r {
        return 0.0;
    }
    // the beam sorts by this many times over: remember it (per thread, same answer every time)
    type Memo = std::collections::HashMap<([u64; 4], Kind, Vec<Eqn>, Term, Term), f64>;
    thread_local!(static MEMO: std::cell::RefCell<Memo> = std::cell::RefCell::new(Memo::new()));
    let c = &st.cfg;
    let weights = [c.size_weight, c.mismatch, c.normal_gap, c.hypothesis_pull].map(f64::to_bits);
    let key = (weights, st.kind, st.hyps.clone(), l.clone(), r.clone());
    if let Some(d) = MEMO.with(|m| m.borrow().get(&key).copied()) {
        return d;
    }
    let d = measure(l, r, st);
    MEMO.with(|m| {
        let mut m = m.borrow_mut();
        if m.len() > 200_000 {
            m.clear();
        }
        m.insert(key, d);
    });
    d
}

fn measure(l: &Term, r: &Term, st: &Setting) -> f64 {
    let c = &st.cfg;
    let differ = if st.kind == Kind::Ring { term::monomials(l) != term::monomials(r) } else { term::word(l) != term::word(r) };
    let d = c.size_weight * (l.size() + r.size()) as f64 + c.mismatch + if differ { c.normal_gap } else { 0.0 };
    // a side that holds a side of a hypothesis is one step from using it
    let pats = st.hyp_patterns();
    let near = pats.iter().any(|(hl, hr, vars)| {
        [l, r].iter().any(|side| side.walk().iter().any(|(_, t)| [hl, hr].iter().any(|p| !p.children().is_empty() && term::matches(p, t, vars, &mut Default::default()))))
    });
    // half as near: a hypothesis's letters stand side by side, a regrouping away
    let adjacent = pats.iter().any(|(hl, hr, vars)| {
            [hl, hr].iter().any(|p| {
                let w = term::flat(p);
                p.size() > 1 && !p.has_letter_in(vars) && [l, r].iter().any(|side| term::flat(side).windows(w.len()).any(|x| x == w.as_slice()))
            })
        });
    if near {
        (d - c.hypothesis_pull).max(0.0)
    } else if adjacent {
        (d - c.hypothesis_pull / 2.0).max(0.0)
    } else {
        d
    }
}

/// Rules a structure's proofs may use.
pub fn rules(req: &Request, cfg: &Config) -> Option<Vec<&'static dyn crate::rules::Rule>> {
    let st = req.structure.as_ref()?;
    Some(cfg.tasks[task_key(st.kind)].iter().filter_map(|n| crate::rules::by_name(n)).collect())
}
