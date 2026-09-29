//! Steps from laws. A rule file holds one law; these helpers turn it into
//! moves: rewrite one subterm by one equation of the law (either way), or,
//! for a law with a condition, rewrite when the condition is a known fact,
//! or replace the goal by the condition ("it is enough to show ...").

use super::term::{self, Subst, Term};
use super::{alg, law, sides, Law, Setting};
use crate::expr::Math;
use crate::rules::{Cx, Line, Move};
use std::collections::BTreeSet;

/// Every way to rewrite one subterm of `side` from `a` to `b`. Letters of
/// `b` that `a` doesn't bind are filled from the pool.
pub fn rewrites(side: &Term, a: &Term, b: &Term, vars: &BTreeSet<String>, pool: &[Term]) -> Vec<(Vec<usize>, Term, Term)> {
    let mut out = Vec::new();
    for (path, node) in side.walk() {
        let mut s = Subst::new();
        if !term::matches(a, node, vars, &mut s) {
            continue;
        }
        // writing e next to e (e -> ee, ey -> e(ey)) never helps a proof
        if matches!(a, Term::El(_)) && (matches!(node, Term::E | Term::Zero) || node.children().iter().any(|c| matches!(c, Term::E | Term::Zero))) {
            continue;
        }
        for s in fill(b, vars, s, pool) {
            let new = b.subst(&s);
            if &new != node {
                out.push((path.clone(), node.clone(), new));
            }
        }
    }
    out
}

/// Complete `s` for the variables of `t` from the pool, every way.
fn fill(t: &Term, vars: &BTreeSet<String>, s: Subst, pool: &[Term]) -> Vec<Subst> {
    let free: Vec<String> = t.letters().into_iter().filter(|v| vars.contains(v) && !s.contains_key(v)).collect();
    let mut all = vec![s];
    for v in free {
        all = all.into_iter().flat_map(|s| pool.iter().map(move |p| (s.clone(), p.clone())).map({
            let v = v.clone();
            move |(mut s, p)| {
                s.insert(v.clone(), p);
                s
            }
        })).collect();
    }
    all
}

fn ground(t: &Term) -> bool {
    t.letters().iter().all(|v| !v.starts_with(['?', '!', '#']))
}

/// A letter of a hypothesis or a law as written: "!a" -> "a".
fn plain(t: &Term) -> Term {
    t.map_letters(&|v| v.trim_start_matches(['?', '!', '#']).to_string())
}

/// A fact a condition may be: a hypothesis or an axiom of the structure.
pub struct Fact {
    pub l: Term,
    pub r: Term,
    pub vars: BTreeSet<String>,
    /// "hypothesis", "inverse axiom".
    pub source: String,
}

/// Is old -> new one instance of an axiom of the structure?
fn axiom_step(st: &Setting, old: &Term, new: &Term) -> bool {
    super::laws().iter().filter(|l| !l.def.lemma && l.conds.is_empty() && st.allows(l)).any(|l| {
        l.eqs.iter().any(|(a, b)| {
            [(a, b), (b, a)].iter().any(|(x, y)| {
                let mut s = Subst::new();
                term::matches(x, old, &l.vars, &mut s) && term::matches(y, new, &l.vars, &mut s)
            })
        })
    })
}

/// Is old -> new one instance of a hypothesis (that step says it plainly)?
fn hypothesis_step(st: &Setting, old: &Term, new: &Term) -> bool {
    st.hyp_patterns().iter().any(|(a, b, vars)| {
        [(a, b), (b, a)].iter().any(|(x, y)| {
            let mut s = Subst::new();
            term::matches(x, old, vars, &mut s) && term::matches(y, new, vars, &mut s)
        })
    })
}

pub fn facts(st: &Setting) -> Vec<Fact> {
    let mut out: Vec<Fact> = st.hyp_patterns().into_iter().map(|(l, r, vars)| Fact { l, r, vars, source: "hypothesis".into() }).collect();
    for law in super::laws().iter().filter(|l| !l.def.lemma && l.conds.is_empty() && st.allows(l)) {
        for (l, r) in &law.eqs {
            let re = |t: &Term| t.map_letters(&|v| v.replacen('?', "#", 1));
            out.push(Fact { l: re(l), r: re(r), vars: law.vars.iter().map(|v| v.replacen('?', "#", 1)).collect(), source: format!("{} axiom", law.def.title.to_lowercase()) });
        }
    }
    out
}

fn eq_line(l: &Term, r: &Term) -> Math {
    Math::Eq(alg(&plain(l)), alg(&plain(r)))
}

fn with_side(l: &Term, r: &Term, side: usize, new: Term) -> Math {
    if side == 0 {
        Math::Eq(alg(&new), alg(r))
    } else {
        Math::Eq(alg(l), alg(&new))
    }
}

/// The law as written, for a step's sentence: "(xy)^-1 = y^-1 x^-1" or
/// "if xy = e then y = x^-1".
fn law_line(law: &Law) -> Line {
    let mut line = Line::new();
    if !law.shown.hyps.is_empty() {
        line = line.t("if ");
        for (i, h) in law.shown.hyps.iter().enumerate() {
            if i > 0 {
                line = line.t(" and ");
            }
            line = line.m(&Math::Eq(alg(&h.l), alg(&h.r)));
        }
        line = line.t(" then ");
    }
    for (i, g) in law.shown.goals.iter().enumerate() {
        if i > 0 {
            line = line.t(" and ");
        }
        line = line.m(&Math::Eq(alg(&g.l), alg(&g.r)));
    }
    line
}

/// Moves for the law of `rule`: `fwd` rewrites left to right, `back` right
/// to left; for a law with a condition, `fwd` rewrites where the condition
/// is a fact and `back` (if given) reduces the goal to the condition.
pub fn law_moves(rule: &'static str, m: &Math, cx: &Cx, fwd: &'static str, back: Option<&'static str>) -> Vec<Move> {
    let Some(st) = &cx.req.structure else { return vec![] };
    let Some((l, r)) = sides(m) else { return vec![] };
    let Some(law) = law(rule) else { return vec![] };
    if !st.allows(law) {
        return vec![];
    }
    let max = cx.cfg.abstract_algebra.max_size;
    let mut out = Vec::new();
    let title = law.def.title;
    if law.conds.is_empty() {
        for (a, b) in &law.eqs {
            for (variant, from, to) in [(Some(fwd), a, b), (back, b, a)] {
                let Some(variant) = variant else { continue };
                for (side, t) in [l, r].into_iter().enumerate() {
                    for (path, old, new) in rewrites(t, from, to, &law.vars, &st.letters) {
                        let whole = t.replace(&path, new.clone());
                        // an axiom may write something in; a lemma simplifies or restates, adding
                        // at most one symbol ((xy)^-1 -> y^-1 x^-1, never x -> (x^-1)^-1)
                        if whole.size() > max || (law.def.lemma && new.size() > old.size() + 1) {
                            continue;
                        }
                        let says = if law.def.lemma { Line::new().t(format!("{title}, ")).join(law_line(law)).t(": ") } else { Line::new().t(format!("{title}: ")) };
                        let says = says.m(&Math::Eq(alg(&old), alg(&new))).t(".");
                        out.push(Move { rule, variant, result: with_side(l, r, side, whole), says, work: vec![] });
                    }
                }
            }
        }
        return out;
    }
    let facts = facts(st);
    let [(p, q)] = &law.conds[..] else { return out };
    for (a, b) in &law.eqs {
        // rewrite a subterm, the condition being a fact
        for (from, to) in [(a, b), (b, a)] {
            for (side, t) in [l, r].into_iter().enumerate() {
                for (path, node) in t.walk() {
                    let mut s = Subst::new();
                    if !term::matches(from, node, &law.vars, &mut s) {
                        continue;
                    }
                    for f in &facts {
                        let vars: BTreeSet<String> = law.vars.union(&f.vars).cloned().collect();
                        for (fl, fr) in [(&f.l, &f.r), (&f.r, &f.l)] {
                            let mut u = s.clone();
                            if !(term::unify(p, fl, &vars, &mut u) && term::unify(q, fr, &vars, &mut u)) {
                                continue;
                            }
                            let new = to.subst(&u);
                            // a rewrite an axiom makes by itself is that axiom's step
                            if !ground(&new) || &new == node || axiom_step(st, node, &new) || hypothesis_step(st, node, &new) {
                                continue;
                            }
                            // an axiom as the condition only ever simplifies: (a^-1)^-1 -> a, never
                            // a -> (a^-1)^-1; a hypothesis may add an inverse (ab -> (ab)^-1), no more
                            let grow = new.size() as i64 - node.size() as i64;
                            if grow > if f.source == "hypothesis" { 1 } else { 0 } {
                                continue;
                            }
                            let whole = t.replace(&path, new.clone());
                            if whole.size() > max {
                                continue;
                            }
                            let says = Line::new().t(format!("{title}: ")).m(&eq_line(&p.subst(&u), &q.subst(&u))).t(format!(" ({}), so ", f.source)).m(&Math::Eq(alg(node), alg(&new))).t(".");
                            out.push(Move { rule, variant: fwd, result: with_side(l, r, side, whole), says, work: vec![] });
                        }
                    }
                }
            }
        }
        // the whole goal is an instance of the conclusion: show the condition instead
        let Some(reduce) = back else { continue };
        for (x, y) in [(a, b), (b, a)] {
            let mut s = Subst::new();
            if !(term::matches(x, l, &law.vars, &mut s) && term::matches(y, r, &law.vars, &mut s)) {
                continue;
            }
            let both = super::term::mul(p.clone(), q.clone());
            for s in fill(&both, &law.vars, s, &st.pool) {
                let (np, nq) = (p.subst(&s), q.subst(&s));
                if np == nq || np.size() > max || nq.size() > max {
                    continue;
                }
                let result = Math::Eq(alg(&np), alg(&nq));
                let says = Line::new().t(format!("{title}, ")).join(law_line(law)).t(": it is enough to show ").m(&result).t(".");
                out.push(Move { rule, variant: reduce, result, says, work: vec![] });
            }
        }
    }
    out
}

/// Moves that use a hypothesis, either way round.
pub fn hypothesis_moves(rule: &'static str, m: &Math, cx: &Cx, fwd: &'static str, back: &'static str) -> Vec<Move> {
    let Some(st) = &cx.req.structure else { return vec![] };
    let Some((l, r)) = sides(m) else { return vec![] };
    let max = cx.cfg.abstract_algebra.max_size;
    let mut out = Vec::new();
    for (k, (hl, hr, vars)) in st.hyp_patterns().into_iter().enumerate() {
        let h = &st.hyps[k];
        for (variant, from, to) in [(fwd, &hl, &hr), (back, &hr, &hl)] {
            for (side, t) in [l, r].into_iter().enumerate() {
                for (path, old, new) in rewrites(t, from, to, &vars, &st.pool) {
                    let whole = t.replace(&path, new.clone());
                    if whole.size() > max {
                        continue;
                    }
                    let mut says = Line::new().t("By the hypothesis");
                    if !h.all.is_empty() {
                        // which value each "for all" letter takes here
                        let mut s = Subst::new();
                        term::matches(from, &old, &vars, &mut s);
                        term::matches(to, &new, &vars, &mut s);
                        // the value each "for all" letter takes, unless it is the letter itself
                        let with: Vec<(&String, &Term)> = h.all.iter().filter_map(|v| s.get(&format!("!{v}")).filter(|t| **t != term::el(v)).map(|t| (v, t))).collect();
                        says = says.t(" ").m(&Math::Eq(alg(&h.l), alg(&h.r))).t(format!(" for all {}", h.all.join(" and ")));
                        if !with.is_empty() {
                            says = says.t(" (with ");
                            for (i, (v, t)) in with.iter().enumerate() {
                                if i > 0 {
                                    says = says.t(", ");
                                }
                                says = says.e(&alg(&term::el(v))).t(" = ").e(&alg(t));
                            }
                            says = says.t(")");
                        }
                    }
                    let says = says.t(", ").m(&Math::Eq(alg(&old), alg(&new))).t(".");
                    out.push(Move { rule, variant, result: with_side(l, r, side, whole), says, work: vec![] });
                }
            }
        }
    }
    out
}
