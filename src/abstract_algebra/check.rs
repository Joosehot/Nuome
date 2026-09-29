//! Checking a proof about a group or a ring, without trusting the rules:
//!
//! - steps: for every step, the two lines are compared: exactly one side
//!   changed, and at one position the old and new subterms are an instance
//!   of an equation of the law the step names (found again by matching),
//!   with its condition an instance of a hypothesis or an axiom; or the
//!   whole goal is an instance of a lemma's conclusion and the new goal the
//!   same instance of its condition
//! - lemmas: every lemma cited comes before the statement in the order of
//!   lemmas, and has a proof of its own that passes these checks
//! - models: the statement holds in concrete groups (or rings) for every
//!   assignment of elements to its letters

use super::models;
use super::parse::{Eqn, Statement};
use super::term::{self, Subst, Term};
use super::{laws, sides, Law, Setting};
use crate::checks::Check;
use crate::config::Config;
use crate::expr::Math;
use crate::model::Request;
use crate::search::{Path, Step};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

fn ck(name: &'static str, ok: bool, detail: impl Into<String>) -> Check {
    Check { name, ok, detail: detail.into() }
}

pub fn checks(req: &Request, cfg: &Config, path: &Path, out: &mut Vec<Check>) {
    let Some(st) = &req.structure else {
        out.push(ck("proof", false, "no structure to check the proof in"));
        return;
    };
    out.push(steps(st, path, cfg.abstract_algebra.max_assignments));
    let cited: Vec<&'static Law> = {
        let mut seen: Vec<&'static Law> = Vec::new();
        for s in &path.steps {
            if let Some(l) = laws().iter().find(|l| l.def.rule == s.mv.rule && l.def.lemma) {
                if !seen.iter().any(|x| std::ptr::eq(*x, l)) {
                    seen.push(l);
                }
            }
        }
        seen
    };
    if !cited.is_empty() {
        out.push(lemmas(st, &cited, cfg));
    }
    out.push(model_check(req, st, cfg));
}

/// Where the hypotheses are used, for the steps check.
fn steps(st: &Setting, path: &Path, max: usize) -> Check {
    let mut used: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, s) in path.steps.iter().enumerate() {
        match verify(st, s) {
            Ok(Some(h)) => used.entry(h).or_default().push(k + 1),
            Ok(None) => {}
            Err(e) => return ck("steps", false, format!("step {} ({}): {e}", k + 1, s.mv.rule)),
        }
        if let Err(e) = holds_in_models(st, s, max) {
            return ck("steps", false, format!("step {} ({}) is false in a model: {e}", k + 1, s.mv.rule));
        }
    }
    let n = path.steps.len();
    let mut detail = if n == 1 { format!("the step is one instance of the law it names, found again by matching, and holds in every test {}{}", st.kind.name(), if st.hyps.is_empty() { "" } else { " where the hypotheses do" }) } else { format!("each of the {n} steps is one instance of the axiom, lemma or hypothesis it names, found again by matching the two lines, and holds in every test {}{}", st.kind.name(), if st.hyps.is_empty() { "" } else { " where the hypotheses do" }) };
    for (h, at) in &used {
        let steps: Vec<String> = at.iter().map(|k| k.to_string()).collect();
        detail += &format!("; the hypothesis {} is used in step{} {}", show_eqn(&st.hyps[*h]), if at.len() > 1 { "s" } else { "" }, steps.join(", "));
    }
    let unused: Vec<String> = (0..st.hyps.len()).filter(|h| !used.contains_key(h)).map(|h| show_eqn(&st.hyps[h])).collect();
    if !unused.is_empty() {
        detail += &format!("; not needed: {}", unused.join(", "));
    }
    ck("steps", true, detail)
}

fn show_eqn(e: &Eqn) -> String {
    let all = if e.all.is_empty() { String::new() } else { format!(" (for all {})", e.all.join(" and ")) };
    format!("{} = {}{all}", term::show(&e.l, crate::print::Style::Ascii), term::show(&e.r, crate::print::Style::Ascii))
}

/// Is the step what it says? Ok(Some(k)) when it used hypothesis k.
fn verify(st: &Setting, s: &Step) -> Result<Option<usize>, String> {
    let rule = s.mv.rule;
    let Some((l0, r0)) = sides(&s.before) else { return Err("the line before isn't an equation of the structure".into()) };
    if rule == "sides_equal" {
        return if s.mv.result == Math::Proved && l0 == r0 { Ok(None) } else { Err("the sides are not the same".into()) };
    }
    let Some((l1, r1)) = sides(&s.mv.result) else { return Err("the new line isn't an equation of the structure".into()) };
    // the equations the step may use
    let (eqs, conds, vars, hyp): (Vec<(Term, Term)>, Vec<(Term, Term)>, BTreeSet<String>, bool) = if rule == "use_hypothesis" {
        let mut vars = BTreeSet::new();
        let mut eqs = Vec::new();
        for (l, r, v) in st.hyp_patterns() {
            eqs.push((l, r));
            vars.extend(v);
        }
        (eqs, vec![], vars, true)
    } else {
        let Some(law) = laws().iter().find(|l| l.def.rule == rule) else { return Err("no such axiom or lemma".into()) };
        if !st.allows(law) {
            return Err(format!("{} can't be used here (a lemma may only cite the lemmas proved before it)", law.def.title));
        }
        (law.eqs.clone(), law.conds.clone(), law.vars.clone(), false)
    };
    // one side rewritten at one position
    let changed: Vec<(&Term, &Term)> = [(l0, l1), (r0, r1)].into_iter().filter(|(a, b)| a != b).collect();
    if let [(before, after)] = changed[..] {
        let p = term::diff(before, after).expect("they differ");
        for k in (0..=p.len()).rev() {
            let (x, y) = (before.get(&p[..k]), after.get(&p[..k]));
            if before.replace(&p[..k], y.clone()) != *after {
                continue;
            }
            for (i, (a, b)) in eqs.iter().enumerate() {
                for (from, to) in [(a, b), (b, a)] {
                    let mut s = Subst::new();
                    if !(term::matches(from, x, &vars, &mut s) && term::matches(to, y, &vars, &mut s)) {
                        continue;
                    }
                    if hyp {
                        return Ok(Some(i));
                    }
                    let found: Option<Vec<Option<usize>>> = conds.iter().map(|(c, d)| holds_as_fact(st, &c.subst(&s), &d.subst(&s), &vars)).collect();
                    if let Some(found) = found {
                        return Ok(found.into_iter().flatten().next());
                    }
                }
            }
        }
    }
    // the goal replaced by a lemma's condition
    if let [(c, d)] = &conds[..] {
        for (a, b) in &eqs {
            for (x, y) in [(a, b), (b, a)] {
                for (p, q) in [(c, d), (d, c)] {
                    let mut s = Subst::new();
                    if term::matches(x, l0, &vars, &mut s) && term::matches(y, r0, &vars, &mut s) && term::matches(p, l1, &vars, &mut s) && term::matches(q, r1, &vars, &mut s) {
                        return Ok(None);
                    }
                }
            }
        }
    }
    Err(format!("{} doesn't turn the line before into the line after", if hyp { "no hypothesis".to_string() } else { format!("the law \"{}\"", s.mv.rule) }))
}

/// Is `c = d` (whose letters in `vars` may still be free) an instance of a
/// hypothesis (Some(Some(k))) or of an axiom of the structure (Some(None))?
fn holds_as_fact(st: &Setting, c: &Term, d: &Term, vars: &BTreeSet<String>) -> Option<Option<usize>> {
    let hyps = st.hyps.len();
    let mut facts: Vec<(Term, Term, BTreeSet<String>)> = st.hyp_patterns();
    for law in laws().iter().filter(|l| !l.def.lemma && l.conds.is_empty() && st.allows(l)) {
        let ren: BTreeSet<String> = law.vars.clone();
        for (l, r) in &law.eqs {
            facts.push((l.rename(&ren, "&"), r.rename(&ren, "&"), ren.iter().map(|v| format!("&{v}")).collect()));
        }
    }
    let k = facts.iter().position(|(l, r, fv)| {
        let all: BTreeSet<String> = vars.union(fv).cloned().collect();
        [(l, r), (r, l)].iter().any(|(x, y)| {
            let mut s = Subst::new();
            term::unify(c, x, &all, &mut s) && term::unify(d, y, &all, &mut s)
        })
    })?;
    Some((k < hyps).then_some(k))
}

/// Proofs of lemmas, found and checked once per configuration.
fn lemma_proof(rule: &str, cfg: &Config) -> Result<usize, String> {
    static CACHE: Mutex<Option<HashMap<(String, u64), Result<usize, String>>>> = Mutex::new(None);
    let key = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{cfg:?}").hash(&mut h);
        (rule.to_string(), h.finish())
    };
    if let Some(r) = CACHE.lock().ok().and_then(|c| c.as_ref().and_then(|m| m.get(&key).cloned())) {
        return r;
    }
    let law = laws().iter().find(|l| l.def.rule == rule).ok_or("no such lemma")?;
    let sentence = super::lemma_sentence(law.def);
    let r = match crate::solve(&sentence, cfg, &crate::Options::default()) {
        Ok(s) => match &s.request.structure {
            Some(own) if own.lemmas == law.index => Ok(s.outcome.path().steps.len()),
            _ => Err(format!("\"{sentence}\" wasn't read as the lemma itself")),
        },
        Err(d) => Err(d.iter().map(|d| d.message.clone()).collect::<Vec<_>>().join("; ")),
    };
    if let Ok(mut c) = CACHE.lock() {
        c.get_or_insert_with(HashMap::new).insert(key, r.clone());
    }
    r
}

fn lemmas(st: &Setting, cited: &[&'static Law], cfg: &Config) -> Check {
    let mut parts = Vec::new();
    for l in cited {
        if !st.allows(l) {
            return ck("lemmas", false, format!("{} is proved after this statement, so it can't be cited", l.def.title));
        }
        match lemma_proof(l.def.rule, cfg) {
            Ok(n) => parts.push(format!("{} ({}, {n} step{})", l.def.text, l.def.title.to_lowercase(), if n == 1 { "" } else { "s" })),
            Err(e) => return ck("lemmas", false, format!("{} has no checked proof: {e}", l.def.title)),
        }
    }
    let what = if parts.len() == 1 { "the lemma".to_string() } else { format!("each of the {} lemmas", parts.len()) };
    ck("lemmas", true, format!("{what} cited has its own checked proof from the axioms and earlier lemmas: {}", parts.join("; ")))
}

fn model_check(req: &Request, st: &Setting, cfg: &Config) -> Check {
    let Some((l, r)) = sides(&req.problem.value) else { return ck("models", false, "the statement isn't an equation") };
    let goal = Eqn { l: l.clone(), r: r.clone(), all: vec![], words: String::new() };
    let stmt = Statement { hyps: st.hyps.clone(), goals: vec![goal.clone()] };
    let v = models::check(st.kind, &stmt, &goal, cfg.abstract_algebra.max_assignments);
    if let Some(c) = v.counterexample {
        return ck("models", false, format!("false: {c}"));
    }
    let kinds = format!("{}s", st.kind.name());
    let skipped = if v.skipped.is_empty() { String::new() } else { format!("; {} too big to try with this many letters", list(&v.skipped.iter().map(String::as_str).collect::<Vec<_>>())) };
    if v.held.is_empty() && v.vacuous.is_empty() {
        return ck("models", true, format!("every test {} is too big to try with this many letters, so this check says nothing; the proof rests on the steps", st.kind.name()));
    }
    if v.held.is_empty() {
        return ck("models", true, format!("the hypotheses hold in none of the test {kinds} ({}), so this check says nothing; the proof rests on the steps", v.vacuous.join(", ")));
    }
    let total: usize = v.held.iter().map(|h| h.1).sum();
    let names: Vec<&str> = v.held.iter().map(|h| h.0.as_str()).collect();
    let kinds = if names.len() == 1 { st.kind.name().to_string() } else { kinds };
    let mut d = format!("holds in the {kinds} {} at all {total} assignments of elements to the letters{}", list(&names), if st.hyps.is_empty() { "" } else { " where the hypotheses hold" });
    if !v.vacuous.is_empty() {
        let vac: Vec<&str> = v.vacuous.iter().map(String::as_str).collect();
        d += &format!(" (in {} they never hold)", list(&vac));
    }
    ck("models", true, d + &skipped)
}

fn list(v: &[&str]) -> String {
    match v {
        [] => String::new(),
        [a] => a.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// A step, tried in the models: each side keeps its value wherever the
/// hypotheses hold; or, for "it is enough to show", the new equation
/// implies the old one.
fn holds_in_models(st: &Setting, s: &Step, max: usize) -> Result<(), String> {
    let (Some((l0, r0)), Some((l1, r1))) = (sides(&s.before), sides(&s.mv.result)) else { return Ok(()) };
    let eq = |l: &Term, r: &Term| Eqn { l: l.clone(), r: r.clone(), all: vec![], words: String::new() };
    let keeps = |a: &Term, b: &Term| {
        let goal = eq(a, b);
        models::check(st.kind, &Statement { hyps: st.hyps.clone(), goals: vec![goal.clone()] }, &goal, max).counterexample
    };
    let Some(c) = keeps(l0, l1).or_else(|| keeps(r0, r1)) else { return Ok(()) };
    let mut hyps = st.hyps.clone();
    hyps.push(eq(l1, r1));
    let goal = eq(l0, r0);
    match models::check(st.kind, &Statement { hyps, goals: vec![goal.clone()] }, &goal, max).counterexample {
        None => Ok(()),
        Some(_) => Err(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Line, Move};

    fn req(s: &str) -> Request {
        crate::parser::parse(s, &Default::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"))
    }

    /// A line of a proof, true or not (a whole problem would be refused if false).
    fn state(s: &str) -> Math {
        let e = super::super::parse::equation(&crate::lexicon::lex(s).unwrap(), super::super::Kind::Group).unwrap();
        Math::Eq(crate::expr::Expr::Alg(e.l), crate::expr::Expr::Alg(e.r))
    }

    fn step(rule: &'static str, before: &str, after: &str) -> Step {
        let mv = Move { rule, variant: "x", result: state(after), says: Line::new(), work: vec![] };
        Step { mv, before: state(before), local: 0.0, notes: vec![] }
    }

    #[test]
    fn steps_are_matched_not_trusted() {
        let r = req("in a group, prove that if ab = ac then b = c");
        let st = r.structure.as_ref().unwrap();
        // one associativity step, one inverse step, the hypothesis
        assert!(verify(st, &step("group_assoc", "(ab)c = e", "a(bc) = e")).is_ok());
        assert!(verify(st, &step("group_inverse", "(aa^-1)b = b", "eb = b")).is_ok());
        assert_eq!(verify(st, &step("use_hypothesis", "a^-1 (ab) = c", "a^-1 (ac) = c")), Ok(Some(0)));
        // the right change under the wrong name, two changes at once, a false step
        assert!(verify(st, &step("group_identity", "(aa^-1)b = b", "eb = b")).is_err());
        assert!(verify(st, &step("group_assoc", "((ab)c)d = e", "a(b(cd)) = e")).is_err());
        assert!(verify(st, &step("group_assoc", "(ab)c = e", "(ba)c = e")).is_err());
        assert!(verify(st, &step("use_hypothesis", "ab = c", "ba = c")).is_err());
        // a step that matches a law but is false in a model is caught there too
        let plain = req("in a group, prove (ab)^-1 = b^-1 a^-1");
        let plain = plain.structure.as_ref().unwrap();
        assert!(holds_in_models(plain, &step("group_assoc", "(ab)c = e", "(ba)c = e"), 50_000).is_err());
        assert!(holds_in_models(plain, &step("group_assoc", "(ab)c = e", "a(bc) = e"), 50_000).is_ok());
    }

    #[test]
    fn a_lemma_cannot_prove_itself() {
        let r = req("in a group, prove (a^-1)^-1 = a");
        let st = r.structure.as_ref().unwrap();
        let s = step("group_inverse_inverse", "(a^-1)^-1 = a", "a = a");
        assert!(verify(st, &s).unwrap_err().contains("only cite the lemmas proved before it"));
        let r = req("in a group, prove ((a^-1)^-1)b = ab");
        assert!(verify(r.structure.as_ref().unwrap(), &step("group_inverse_inverse", "((a^-1)^-1)b = ab", "ab = ab")).is_ok());
    }

    #[test]
    fn false_statements_come_back_with_elements() {
        let d = crate::parser::parse("in a group, prove ab = ba", &Default::default()).unwrap_err();
        assert_eq!(d[0].hint.as_deref(), Some("in S3 (permutations of 1, 2, 3; ab is b, then a), a = (1 2), b = (1 3): ab = (1 3 2) but ba = (1 2 3)"));
        let d = crate::parser::parse("in a ring, prove ab = ba", &Default::default()).unwrap_err();
        assert!(d[0].hint.as_deref().unwrap().starts_with("in 2x2 matrices over Z/2"));
    }

    #[test]
    fn statements_parse_with_hypotheses_and_styles() {
        let r = req("prove that in a group, if a^2 = e for all a then ab = ba");
        let st = r.structure.as_ref().unwrap();
        assert_eq!(st.hyps[0].all, vec!["a".to_string()]);
        assert_eq!(crate::render::header(&r, crate::print::Style::Ascii), "Prove in a group: if aa = e for all a then ab = ba");
        let r = req("in a group, prove (ab)^-1 = b^-1 a^-1");
        assert_eq!(crate::print::math(&r.problem.value, crate::print::Style::Unicode), "(ab)⁻¹ = b⁻¹a⁻¹");
        assert_eq!(crate::print::math(&r.problem.value, crate::print::Style::Latex), r"\left(ab\right)^{-1} = b^{-1}a^{-1}");
        // ordinary algebra is untouched: no group words, no group
        assert!(req("prove (a + b)^2 = a^2 + 2ab + b^2").structure.is_none());
        // a lemma is recognised up to its letters, so it can't cite itself
        assert_eq!(req("in a group, prove that if pq = e then q = p^-1").structure.unwrap().lemmas, 0);
    }
}
