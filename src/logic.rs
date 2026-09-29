//! Propositional logic and the algebra of sets (agent L).
//!
//! A statement of logic ("prove (p -> q) <-> (~q -> ~p)") or about sets
//! ("prove (A union B)' = A' intersect B'") has its own small grammar here,
//! so the rest of Nuome's phrasing is unaffected: "and" is a connective, not
//! a separator, "->" is "implies", not "approaches", and capital letters are
//! sets. The sentence only comes here when it asks for a proof and uses the
//! words or symbols of logic or sets; a sentence that doesn't parse as one
//! goes back to the ordinary parser unless its symbols leave no doubt.
//!
//! The laws of logic and of sets are the same laws of a Boolean algebra:
//! not/complement, and/intersection, or/union, T/U, F/empty. `view` reads an
//! expression as that algebra, so each law is written once (one file per law
//! in `src/rules/`) and works on both.
//!
//! Truth values are only ever computed here by `eval`, which the checks use
//! and the rules don't: every proof is confirmed on every row of the truth
//! table (every region of the Venn diagram), which for finitely many letters
//! is an exact, exhaustive proof.

use crate::config::Config;
use crate::expr::{Conn, Expr, Math, SetOp};
use crate::lexicon::{self, Tok};
use crate::model::{Calc, Modifier, Request, Said, Task};
use crate::parser::Diag;
use crate::print::{self, Style};
use crate::rules::{self, Cx, Line, Move, Rewrite};
use std::collections::BTreeSet;

// ---- the Boolean view -------------------------------------------------------

/// Which algebra an expression lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    Logic,
    Sets,
}

/// An expression read as a Boolean algebra: not/complement, and/intersection,
/// or/union, T/U and F/empty.
pub enum B<'a> {
    Not(&'a Expr),
    And(&'a [Expr]),
    Or(&'a [Expr]),
    Const(bool),
}

pub fn view(e: &Expr) -> Option<(Flavor, B<'_>)> {
    match e {
        Expr::Logic(Conn::Not, v) => Some((Flavor::Logic, B::Not(&v[0]))),
        Expr::Logic(Conn::And, v) => Some((Flavor::Logic, B::And(v))),
        Expr::Logic(Conn::Or, v) => Some((Flavor::Logic, B::Or(v))),
        Expr::Truth(b) => Some((Flavor::Logic, B::Const(*b))),
        Expr::Set(SetOp::Complement, v) => Some((Flavor::Sets, B::Not(&v[0]))),
        Expr::Set(SetOp::Inter, v) => Some((Flavor::Sets, B::And(v))),
        Expr::Set(SetOp::Union, v) => Some((Flavor::Sets, B::Or(v))),
        Expr::SetConst(b) => Some((Flavor::Sets, B::Const(*b))),
        _ => None,
    }
}

pub fn not(f: Flavor, a: Expr) -> Expr {
    match f {
        Flavor::Logic => Expr::Logic(Conn::Not, vec![a]),
        Flavor::Sets => Expr::Set(SetOp::Complement, vec![a]),
    }
}

/// and / intersection of the parts, flat; one part is itself, none is T / U.
pub fn and(f: Flavor, v: Vec<Expr>) -> Expr {
    match v.len() {
        0 => konst(f, true),
        _ => crate::expr::tidy(match f {
            Flavor::Logic => Expr::Logic(Conn::And, v),
            Flavor::Sets => Expr::Set(SetOp::Inter, v),
        }),
    }
}

/// or / union of the parts, flat; one part is itself, none is F / empty.
pub fn or(f: Flavor, v: Vec<Expr>) -> Expr {
    match v.len() {
        0 => konst(f, false),
        _ => crate::expr::tidy(match f {
            Flavor::Logic => Expr::Logic(Conn::Or, v),
            Flavor::Sets => Expr::Set(SetOp::Union, v),
        }),
    }
}

pub fn konst(f: Flavor, b: bool) -> Expr {
    match f {
        Flavor::Logic => Expr::Truth(b),
        Flavor::Sets => Expr::SetConst(b),
    }
}

pub fn implies(a: Expr, b: Expr) -> Expr {
    Expr::Logic(Conn::Implies, vec![a, b])
}

/// A canonical spelling up to the order of the parts of and, or, union and
/// intersection (they commute and associate: the parts are kept flat).
pub fn canon(e: &Expr) -> String {
    match e {
        Expr::Logic(c @ (Conn::And | Conn::Or), v) => {
            let mut parts: Vec<String> = v.iter().map(canon).collect();
            parts.sort();
            format!("{c:?}[{}]", parts.join(","))
        }
        Expr::Set(o @ (SetOp::Union | SetOp::Inter), v) => {
            let mut parts: Vec<String> = v.iter().map(canon).collect();
            parts.sort();
            format!("{o:?}[{}]", parts.join(","))
        }
        Expr::Logic(c, v) => format!("{c:?}({})", v.iter().map(canon).collect::<Vec<_>>().join(",")),
        Expr::Set(o, v) => format!("{o:?}({})", v.iter().map(canon).collect::<Vec<_>>().join(",")),
        Expr::Member(x, a) => format!("{x} in {}", canon(a)),
        e => print::expr(e, Style::Ascii),
    }
}

pub fn same(a: &Expr, b: &Expr) -> bool {
    canon(a) == canon(b)
}

/// The parts of an and (a lone statement is a one-part and).
pub fn conjuncts(e: &Expr) -> Vec<Expr> {
    match view(e) {
        Some((_, B::And(v))) => v.to_vec(),
        _ => vec![e.clone()],
    }
}

/// The parts of an or (a lone statement is a one-part or).
pub fn disjuncts(e: &Expr) -> Vec<Expr> {
    match view(e) {
        Some((_, B::Or(v))) => v.to_vec(),
        _ => vec![e.clone()],
    }
}

/// Placeholder letters for stating a law: P, Q, R (logic) or X, Y, Z (sets).
pub fn ph(f: Flavor, k: usize) -> Expr {
    let names = match f {
        Flavor::Logic => ["P", "Q", "R"],
        Flavor::Sets => ["X", "Y", "Z"],
    };
    Expr::Var(names[k].to_string())
}

/// "De Morgan's law: ~(P and Q) <=> ~P or ~Q." (sets: "... = ...").
pub fn law_says(f: Flavor, name: &str, lhs: Expr, rhs: Expr) -> Line {
    let m = match f {
        Flavor::Logic => Math::Equiv(lhs, rhs),
        Flavor::Sets => Math::Eq(lhs, rhs),
    };
    Line::new().t(format!("{name}: ")).m(&m).t(".")
}

// ---- statements ----------------------------------------------------------------

/// Is this expression a set (built from sets, or a capital letter)?
pub fn is_set(e: &Expr) -> bool {
    match e {
        Expr::Set(..) | Expr::SetConst(_) => true,
        Expr::Var(v) => v.chars().next().is_some_and(|c| c.is_uppercase()),
        _ => false,
    }
}

/// Is this a statement of logic or about sets (a state these rules work on)?
pub fn is_statement(m: &Math) -> bool {
    match m {
        Math::Taut(_) | Math::Equiv(..) | Math::Entails(..) | Math::Subset(..) => true,
        Math::Eq(l, r) => is_set(l) && is_set(r),
        _ => false,
    }
}

/// Do the laws of logic apply here: a proof, of a statement of logic or sets?
/// And does the method the sentence insists on leave room for `rule`? "by
/// truth table" leaves only the table; "by element chasing" leaves only the
/// chase for the statement about sets (the laws of logic then finish).
pub fn applies(rule: &str, m: &Math, cx: &Cx) -> bool {
    let allowed = match cx.req.method.as_ref().map(|s| s.value.as_str()) {
        Some("truth_table") => rule == "truth_table",
        Some("element_chase") => rule == "element_chase" || !matches!(m, Math::Eq(..) | Math::Subset(..)),
        _ => true,
    };
    cx.task() == Task::Prove && is_statement(m) && allowed
}

/// Run a law over every node, like `rules::local`. A law doesn't prove
/// itself: at the start, a move that would finish the proof in one
/// application (the statement *is* the law) is dropped, and the truth table
/// proves it instead (see `table_first`).
pub fn law(rule: &'static str, m: &Math, cx: &Cx, f: impl Fn(&Expr, Flavor, B) -> Vec<Rewrite>) -> Vec<Move> {
    law_on(rule, m, cx, |e| match view(e) {
        Some((fl, b)) => f(e, fl, b),
        None => vec![],
    })
}

/// `law` for rewrites of any node (implications, differences).
pub fn law_on(rule: &'static str, m: &Math, cx: &Cx, f: impl Fn(&Expr) -> Vec<Rewrite>) -> Vec<Move> {
    if !applies(rule, m, cx) {
        return vec![];
    }
    let mut moves = rules::local(rule, m, |e, _| f(e));
    // spreading and/or over each other can grow a statement without end
    let limit = cx.cfg.logic.growth * size(&cx.req.start()) as f64 + 2.0;
    moves.retain(|mv| (size(&mv.result) as f64) <= limit);
    if table_first(m, cx) {
        moves.retain(|mv| !closes(&mv.result));
    }
    moves
}

/// The size of a statement, "x in A" counting as the one letter A (so a
/// chased set statement is as big as the sets it came from).
pub fn size(m: &Math) -> usize {
    m.slots().iter().map(|e| e.walk().iter().filter(|(_, n)| !matches!(n, Expr::Member(..))).count()).sum()
}

/// Is this the statement as asked, small enough for its truth table? Then a
/// law that would prove it in one step (it *is* the law: De Morgan's,
/// commutativity, "an intersection is inside each of its sets") gives way to
/// the truth table, which proves it without assuming it.
pub fn table_first(m: &Math, cx: &Cx) -> bool {
    *m == cx.req.start() && letters(m).len() <= cx.cfg.logic.table_letters
}

/// Would the proof end right here: both sides the same, T, or the goal among
/// the assumptions?
pub fn closes(m: &Math) -> bool {
    match m {
        Math::Equiv(l, r) | Math::Eq(l, r) => same(l, r),
        Math::Taut(e) => *e == Expr::Truth(true),
        Math::Entails(l, r) | Math::Subset(l, r) => entails_by_parts(l, r).is_some(),
        _ => false,
    }
}

/// Why the left side plainly gives the right: the same; the right is among
/// the left's and-parts; the left is among the right's or-parts; or the
/// right is T / the left is F.
pub fn entails_by_parts(l: &Expr, r: &Expr) -> Option<&'static str> {
    if same(l, r) {
        return Some("same");
    }
    if matches!(view(r), Some((_, B::Const(true)))) || matches!(view(l), Some((_, B::Const(false)))) {
        return Some("trivial");
    }
    let ls = conjuncts(l);
    if conjuncts(r).iter().all(|c| ls.iter().any(|x| same(x, c))) {
        return Some("conjunct");
    }
    let rs = disjuncts(r);
    if disjuncts(l).iter().all(|d| rs.iter().any(|x| same(x, d))) {
        return Some("disjunct");
    }
    if shared_part(l, r).is_some() {
        return Some("part");
    }
    None
}

/// An and-part of `l` that is one of the or-parts of `r`: from p and q, p or r.
pub fn shared_part(l: &Expr, r: &Expr) -> Option<Expr> {
    let rs = disjuncts(r);
    conjuncts(l).into_iter().find(|c| rs.iter().any(|d| same(c, d)))
}

// ---- truth values (for the checks and the truth table) ------------------------

/// The value of a statement (or whether an element of the region lies in a
/// set) when each letter has the value `env` gives it.
pub fn eval(e: &Expr, env: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
    let all = |v: &[Expr]| v.iter().try_fold(true, |a, x| Some(eval(x, env)? && a));
    let any = |v: &[Expr]| v.iter().try_fold(false, |a, x| Some(eval(x, env)? || a));
    match e {
        Expr::Var(v) => env(v),
        Expr::Truth(b) | Expr::SetConst(b) => Some(*b),
        Expr::Member(_, a) => eval(a, env),
        Expr::Logic(Conn::Not, v) | Expr::Set(SetOp::Complement, v) => Some(!eval(&v[0], env)?),
        Expr::Logic(Conn::And, v) | Expr::Set(SetOp::Inter, v) => all(v),
        Expr::Logic(Conn::Or, v) | Expr::Set(SetOp::Union, v) => any(v),
        Expr::Logic(Conn::Implies, v) => Some(!eval(&v[0], env)? || eval(&v[1], env)?),
        Expr::Logic(Conn::Iff, v) => Some(eval(&v[0], env)? == eval(&v[1], env)?),
        Expr::Set(SetOp::Diff, v) => Some(eval(&v[0], env)? && !eval(&v[1], env)?),
        _ => None,
    }
}

/// Whether the statement holds in one row (one region).
pub fn holds(m: &Math, env: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
    match m {
        Math::Taut(e) => eval(e, env),
        Math::Equiv(l, r) | Math::Eq(l, r) => Some(eval(l, env)? == eval(r, env)?),
        Math::Entails(l, r) | Math::Subset(l, r) => Some(!eval(l, env)? || eval(r, env)?),
        Math::Proved => Some(true),
        _ => None,
    }
}

/// The letters of a statement, in order.
pub fn letters(m: &Math) -> Vec<String> {
    let set: BTreeSet<String> = m.slots().iter().flat_map(|e| e.vars()).collect();
    set.into_iter().collect()
}

/// Row k of the table over n letters: the first row is all true, the last
/// all false, the first letter changing slowest (the textbook order).
pub fn row(n: usize, k: usize) -> Vec<bool> {
    (0..n).map(|i| (k >> (n - 1 - i)) & 1 == 0).collect()
}

/// The value of a statement in a row.
pub fn at(m: &Math, names: &[String], vals: &[bool]) -> Option<bool> {
    holds(m, &|v| names.iter().position(|n| n == v).map(|i| vals[i]))
}

pub fn value_at(e: &Expr, names: &[String], vals: &[bool]) -> Option<bool> {
    eval(e, &|v| names.iter().position(|n| n == v).map(|i| vals[i]))
}

/// A row where the statement fails, if there is one.
pub fn counterexample(m: &Math) -> Option<Vec<bool>> {
    let names = letters(m);
    let n = names.len();
    (0..1usize << n).map(|k| row(n, k)).find(|vals| at(m, &names, vals) == Some(false))
}

/// "p = true, q = false" or "an element in A but not in B".
pub fn describe_row(names: &[String], vals: &[bool], sets: bool) -> String {
    if sets {
        let ins: Vec<&str> = names.iter().zip(vals).filter(|(_, v)| **v).map(|(n, _)| n.as_str()).collect();
        let outs: Vec<&str> = names.iter().zip(vals).filter(|(_, v)| !**v).map(|(n, _)| n.as_str()).collect();
        let join = |v: &[&str]| match v.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
            _ => v.join(""),
        };
        return match (ins.is_empty(), outs.is_empty()) {
            (false, false) => format!("an element in {} but not in {}", join(&ins), join(&outs)),
            (false, true) => format!("an element in {}", join(&ins)),
            (true, _) => format!("an element in none of {}", join(&outs)),
        };
    }
    names.iter().zip(vals).map(|(n, v)| format!("{n} = {v}")).collect::<Vec<_>>().join(", ")
}

/// How far a state is from proved, roughly in steps: implications,
/// biconditionals and differences still to rewrite, negations of whole
/// statements still to push in, and the size of the sides.
pub fn distance(m: &Math) -> f64 {
    fn pending(e: &Expr) -> f64 {
        e.walk()
            .iter()
            .map(|(_, n)| match n {
                Expr::Logic(Conn::Implies | Conn::Iff, _) => 2.0,
                Expr::Set(SetOp::Diff, _) => 1.5,
                Expr::Logic(Conn::Not, v) | Expr::Set(SetOp::Complement, v) if !matches!(v[0], Expr::Var(_) | Expr::Member(..)) => 1.5,
                Expr::Member(_, a) if !matches!(**a, Expr::Var(_)) => 1.0,
                _ => 0.0,
            })
            .sum()
    }
    let side = |e: &Expr| e.size() as f64 * 0.5 + pending(e);
    if closes(m) {
        return 0.5;
    }
    match m {
        Math::Taut(e) => side(e) + 1.0,
        Math::Equiv(l, r) | Math::Eq(l, r) => side(l) + side(r),
        Math::Entails(l, r) | Math::Subset(l, r) => side(l) + side(r),
        _ => 0.0,
    }
}

// ---- the grammar ------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum L {
    Letter(String),
    Not,
    And,
    Or,
    Implies,
    Iff,
    /// "is equivalent to", "≡": between the two statements.
    Equiv,
    True,
    False,
    Union,
    Inter,
    /// "complement of": before a set.
    Comp,
    /// "'", "^c": after a set.
    Prime,
    Diff,
    Subset,
    Eq,
    Empty,
    Open,
    Close,
    /// "is a tautology", "is always true".
    Taut,
    Prove,
    Method(&'static str),
    Mod(Modifier),
    Skip,
    Unknown,
}

#[derive(Clone, Debug)]
struct Lt {
    l: L,
    words: String,
    /// Written with a symbol or word that only logic or sets use.
    strong: bool,
}

/// Split the sentence into logic tokens; `None` if a symbol isn't one of ours.
fn scan(s: &str) -> Option<Vec<Lt>> {
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<Lt> = Vec::new();
    let mut words: Vec<(String, usize)> = Vec::new(); // pending run of words
    let tok = |l: L, w: &str, strong: bool| Lt { l, words: w.to_string(), strong };
    let mut i = 0;
    let flush = |words: &mut Vec<(String, usize)>, out: &mut Vec<Lt>| {
        let mut k = 0;
        while k < words.len() {
            // the longest known phrase
            let mut best: Option<(usize, &'static Tok)> = None;
            let mut phrase = String::new();
            for j in k..words.len().min(k + 7) {
                if j > k {
                    phrase.push(' ');
                }
                phrase.push_str(&words[j].0.to_lowercase());
                if let Some(t) = lexicon::phrase(&phrase) {
                    best = Some((j + 1 - k, t));
                }
            }
            let one = &words[k].0;
            let text: String = words[k..k + best.map_or(1, |b| b.0)].iter().map(|w| w.0.as_str()).collect::<Vec<_>>().join(" ");
            let is_letter = one.chars().count() == 1;
            let l = match best {
                Some((n, _)) if n == 1 && is_letter => (L::Letter(one.clone()), false),
                Some((_, t)) => match t {
                    Tok::Task(Task::Prove) => (L::Prove, false),
                    Tok::Logic(k) => match *k {
                        "not" => (L::Not, false),
                        "or" => (L::Or, false),
                        "implies" => (L::Implies, true),
                        "iff" => (L::Iff, true),
                        "equiv" => (L::Equiv, true),
                        "tautology" => (L::Taut, true),
                        "true" => (L::True, false),
                        "false" => (L::False, false),
                        "union" => (L::Union, true),
                        "inter" => (L::Inter, true),
                        "complement" => (L::Comp, true),
                        "subset" => (L::Subset, true),
                        "empty" => (L::Empty, true),
                        _ => (L::Unknown, false),
                    },
                    Tok::Sep if text == "and" => (L::And, false),
                    // words that stand for sets only inside a statement about sets
                    Tok::Topic(_) if text == "intersection" || text == "intersected with" => (L::Inter, false),
                    Tok::Topic(_) if text == "subset" => (L::Subset, false),
                    Tok::Op('-') => (L::Diff, false),
                    Tok::Op('=') => (L::Eq, false),
                    Tok::Method(m) => (L::Method(m), true),
                    Tok::Mod(m) => (L::Mod(*m), false),
                    Tok::Filler | Tok::Is | Tok::Sep | Tok::To | Tok::Of | Tok::For | Tok::When => (L::Skip, false),
                    _ => (L::Unknown, false),
                },
                None if is_letter => (L::Letter(one.clone()), false),
                None => (L::Unknown, false),
            };
            out.push(Lt { l: l.0, words: text, strong: l.1 });
            k += best.map_or(1, |b| b.0);
        }
        words.clear();
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let next2 = chars.get(i + 2).copied();
        if c.is_alphabetic() {
            let st = i;
            while i < chars.len() && (chars[i].is_alphabetic() || (chars[i] == '-' && i > st && chars.get(i + 1).is_some_and(|x| x.is_alphabetic()))) {
                i += 1;
            }
            words.push((chars[st..i].iter().collect(), st));
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        flush(&mut words, &mut out);
        let (l, n, strong) = match (c, next, next2) {
            ('<', Some('-'), Some('>')) | ('<', Some('='), Some('>')) => (L::Iff, 3, true),
            ('-', Some('>'), _) | ('=', Some('>'), _) => (L::Implies, 2, false),
            ('=', Some('='), _) => (L::Equiv, 2, true),
            ('/', Some('\\'), _) => (L::And, 2, true),
            ('\\', Some('/'), _) => (L::Or, 2, true),
            ('^', Some('c'), n2) if !n2.is_some_and(|x| x.is_alphabetic()) => (L::Prime, 2, true),
            ('{', Some('}'), _) => (L::Empty, 2, true),
            ('~' | '¬', ..) => (L::Not, 1, true),
            ('!', ..) => (L::Not, 1, false),
            ('∧', ..) => (L::And, 1, true),
            ('&' | '^', ..) => (L::And, 1, false),
            ('∨', ..) => (L::Or, 1, true),
            ('|', ..) => (L::Or, 1, false),
            ('→' | '⇒' | '⟹', ..) => (L::Implies, 1, true),
            ('↔' | '⇔' | '⟺', ..) => (L::Iff, 1, true),
            ('≡', ..) => (L::Equiv, 1, true),
            ('∪', ..) => (L::Union, 1, true),
            ('∩', ..) => (L::Inter, 1, false),
            ('\'' | '′' | 'ᶜ', ..) => (L::Prime, 1, false),
            ('\\' | '∖', ..) => (L::Diff, 1, true),
            ('-' | '−', ..) => (L::Diff, 1, false),
            ('⊆' | '⊂', ..) => (L::Subset, 1, true),
            ('=', ..) => (L::Eq, 1, false),
            ('∅', ..) => (L::Empty, 1, true),
            ('⊤', ..) => (L::True, 1, true),
            ('⊥', ..) => (L::False, 1, true),
            ('(' | '[', ..) => (L::Open, 1, false),
            (')' | ']', ..) => (L::Close, 1, false),
            (',' | '.' | ';' | ':' | '?', ..) => (L::Skip, 1, false),
            _ => return None,
        };
        out.push(tok(l, &chars[i..i + n].iter().collect::<String>(), strong));
        i += n;
    }
    flush(&mut words, &mut out);
    // "A U B": a U between two sets is a union (anywhere else it is the universal set)
    for k in 1..out.len().saturating_sub(1) {
        let set_before = matches!(&out[k - 1].l, L::Letter(v) if v.chars().all(|c| c.is_uppercase())) || matches!(out[k - 1].l, L::Close | L::Prime);
        let set_after = matches!(&out[k + 1].l, L::Letter(v) if v.chars().all(|c| c.is_uppercase())) || matches!(out[k + 1].l, L::Open | L::Comp);
        if out[k].l == L::Letter("U".into()) && set_before && set_after {
            out[k].l = L::Union;
        }
    }
    Some(out)
}

struct P<'a> {
    ts: &'a [Lt],
    i: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<&L> {
        self.ts.get(self.i).map(|t| &t.l)
    }
    fn err(&self, what: &str) -> Diag {
        let all: Vec<&str> = self.ts.iter().map(|t| t.words.as_str()).collect();
        match self.ts.get(self.i) {
            Some(t) => Diag::new(format!("{what} at \"{}\" in \"{}\"", t.words, all.join(" "))),
            None => Diag::new(format!("{what} at the end of \"{}\"", all.join(" "))),
        }
    }
    fn expr(&mut self) -> Result<Expr, Diag> {
        let mut a = self.imp()?;
        while self.peek() == Some(&L::Iff) {
            self.i += 1;
            let b = self.imp()?;
            a = Expr::Logic(Conn::Iff, vec![a, b]);
        }
        Ok(a)
    }
    fn imp(&mut self) -> Result<Expr, Diag> {
        let a = self.or()?;
        if self.peek() == Some(&L::Implies) {
            self.i += 1;
            let b = self.imp()?;
            return Ok(implies(a, b));
        }
        Ok(a)
    }
    fn or(&mut self) -> Result<Expr, Diag> {
        let mut a = self.and()?;
        loop {
            let op = match self.peek() {
                Some(L::Or) => Expr::Logic(Conn::Or, vec![]),
                Some(L::Union) => Expr::Set(SetOp::Union, vec![]),
                Some(L::Diff) => Expr::Set(SetOp::Diff, vec![]),
                _ => return Ok(a),
            };
            self.i += 1;
            let b = self.and()?;
            a = crate::expr::tidy(match op {
                Expr::Logic(c, _) => Expr::Logic(c, vec![a, b]),
                Expr::Set(o, _) => Expr::Set(o, vec![a, b]),
                _ => unreachable!("an operation"),
            });
        }
    }
    fn and(&mut self) -> Result<Expr, Diag> {
        let mut a = self.unary()?;
        loop {
            let set = match self.peek() {
                Some(L::And) => false,
                Some(L::Inter) => true,
                _ => return Ok(a),
            };
            self.i += 1;
            let b = self.unary()?;
            a = crate::expr::tidy(if set { Expr::Set(SetOp::Inter, vec![a, b]) } else { Expr::Logic(Conn::And, vec![a, b]) });
        }
    }
    fn unary(&mut self) -> Result<Expr, Diag> {
        match self.peek() {
            Some(L::Not) => {
                self.i += 1;
                Ok(Expr::Logic(Conn::Not, vec![self.unary()?]))
            }
            // "the complement of A intersect B" is (A intersect B)'
            Some(L::Comp) => {
                self.i += 1;
                Ok(Expr::Set(SetOp::Complement, vec![self.and()?]))
            }
            _ => {
                let mut a = self.atom()?;
                while self.peek() == Some(&L::Prime) {
                    self.i += 1;
                    a = Expr::Set(SetOp::Complement, vec![a]);
                }
                Ok(a)
            }
        }
    }
    fn atom(&mut self) -> Result<Expr, Diag> {
        let Some(t) = self.ts.get(self.i) else { return Err(self.err("expected a letter")) };
        self.i += 1;
        match &t.l {
            L::Letter(v) => Ok(Expr::Var(v.clone())),
            L::True => Ok(Expr::Truth(true)),
            L::False => Ok(Expr::Truth(false)),
            L::Empty => Ok(Expr::SetConst(false)),
            L::Open => {
                let e = self.expr()?;
                if self.peek() != Some(&L::Close) {
                    return Err(self.err("missing )"));
                }
                self.i += 1;
                Ok(e)
            }
            _ => {
                self.i -= 1;
                Err(self.err("expected a letter"))
            }
        }
    }
}

/// Letters become sets (capitals; U is the universal set) or statements
/// (T and F are true and false); a statement may not mix the two.
fn settle(e: Expr, sets: bool) -> Result<Expr, Diag> {
    let kids = |v: Vec<Expr>| v.into_iter().map(|x| settle(x, sets)).collect::<Result<Vec<_>, _>>();
    let mixed = || Diag::new("this mixes sets and statements of logic").hint("sets use union, intersection, complement (A'), difference (A \\ B); statements use not, and, or, ->, <->");
    Ok(match e {
        Expr::Var(v) if sets => match v.as_str() {
            "U" => Expr::SetConst(true),
            _ if v.chars().all(|c| c.is_uppercase()) => Expr::Var(v),
            _ => return Err(Diag::new(format!("\"{v}\": sets are capital letters")).hint("e.g. prove A intersect (B union C) = (A intersect B) union (A intersect C)")),
        },
        Expr::Var(v) => match v.as_str() {
            "T" => Expr::Truth(true),
            "F" => Expr::Truth(false),
            _ => Expr::Var(v),
        },
        Expr::Truth(_) if sets => return Err(mixed()),
        Expr::Logic(..) if sets => return Err(mixed()),
        Expr::Set(..) | Expr::SetConst(_) if !sets => return Err(mixed()),
        Expr::Logic(c, v) => Expr::Logic(c, kids(v)?),
        Expr::Set(o, v) => Expr::Set(o, kids(v)?),
        e => e,
    })
}

/// Does the sentence look like a statement of logic or about sets? `None`:
/// not ours. Otherwise the request, or why it can't be one.
pub fn parse(sentence: &str, cfg: &Config) -> Option<Result<Request, Vec<Diag>>> {
    let toks = scan(sentence)?;
    let asks = toks.iter().any(|t| matches!(t.l, L::Prove | L::Taut));
    let ours = toks.iter().any(|t| !matches!(t.l, L::Letter(_) | L::Open | L::Close | L::Prove | L::Mod(_) | L::Skip | L::Unknown));
    if !asks || !ours {
        return None;
    }
    let strong = toks.iter().any(|t| t.strong);
    // a sentence that reads as a statement is ours even when it is refused
    match build(sentence, &toks, cfg) {
        Ok(r) => Some(Ok(r)),
        Err((parsed, d)) if parsed || strong => Some(Err(d)),
        Err(_) => None,
    }
}

fn build(sentence: &str, toks: &[Lt], cfg: &Config) -> Result<Request, (bool, Vec<Diag>)> {
    let mut task = None;
    let mut method: Option<Said<String>> = None;
    let mut modifiers = Vec::new();
    let mut math: Vec<Lt> = Vec::new();
    let mut taut = None;
    for t in toks {
        match &t.l {
            L::Prove => task = task.or(Some(Said::new(Task::Prove, t.words.clone()))),
            L::Taut => taut = Some(t.words.clone()),
            L::Method(m) => method = Some(Said::new(m.to_string(), t.words.clone())),
            L::Mod(m) => modifiers.push(Said::new(*m, t.words.clone())),
            L::Skip => {}
            L::Unknown => {
                let mut d = Diag::new(format!("unknown word \"{}\"", t.words));
                d.hint = Some(match lexicon::suggest(&t.words.to_lowercase()) {
                    Some(h) => format!("{h} (--vocabulary lists every word)"),
                    None => "a statement of logic uses letters, not, and, or, -> and <->; one about sets uses capital letters, union, intersection, ' and \\".into(),
                });
                return Err((false, vec![d]));
            }
            _ => math.push(t.clone()),
        }
    }
    let task = task.unwrap_or_else(|| Said::new(Task::Prove, taut.clone().unwrap_or_default()));
    // the relation between the two sides
    let rels: Vec<usize> = math.iter().enumerate().filter(|(_, t)| matches!(t.l, L::Equiv | L::Eq | L::Subset)).map(|(i, _)| i).collect();
    if rels.len() > 1 {
        return Err((false, vec![Diag::new("more than one =, <=> or subset in the statement").hint("one statement at a time")]));
    }
    let side = |ts: &[Lt]| -> Result<Expr, Diag> {
        if ts.is_empty() {
            return Err(Diag::new("one side of the statement is empty"));
        }
        let mut p = P { ts, i: 0 };
        let e = p.expr()?;
        if p.i < ts.len() {
            return Err(p.err("didn't expect this"));
        }
        Ok(e)
    };
    let sets = math.iter().any(|t| matches!(t.l, L::Union | L::Inter | L::Comp | L::Prime | L::Diff | L::Subset | L::Empty));
    let problem = match rels.first() {
        Some(&k) => {
            let (l, r) = (settle(side(&math[..k]).map_err(|d| (false, vec![d]))?, sets).map_err(|d| (false, vec![d]))?, settle(side(&math[k + 1..]).map_err(|d| (false, vec![d]))?, sets).map_err(|d| (false, vec![d]))?);
            match (&math[k].l, sets) {
                (L::Subset, true) => Math::Subset(l, r),
                (L::Eq, true) => Math::Eq(l, r),
                (L::Equiv | L::Eq, false) => Math::Equiv(l, r),
                _ => return Err((false, vec![Diag::new(format!("\"{}\" doesn't fit between two sets", math[k].words)).hint("sets are equal (=) or one is a subset of the other")])),
            }
        }
        None => match settle(side(&math).map_err(|d| (false, vec![d]))?, sets).map_err(|d| (false, vec![d]))? {
            _ if sets => return Err((false, vec![Diag::new("a statement about sets needs = or subset").hint("e.g. prove A \\ B = A intersect B'")])),
            Expr::Logic(Conn::Iff, v) if taut.is_none() => Math::Equiv(v[0].clone(), v[1].clone()),
            e => Math::Taut(e),
        },
    };
    let words = math.iter().map(|t| t.words.as_str()).collect::<Vec<_>>().join(" ");
    let problem = Said::new(problem, words);
    let names = letters(&problem.value);
    let shown = print::math(&problem.value, Style::Ascii);
    if names.len() > cfg.logic.check_letters {
        return Err((true, vec![Diag::new(format!("{shown} has {} letters", names.len())).hint(format!("Nuome proves statements with up to {} letters, so that every row of the truth table can be checked", cfg.logic.check_letters))]));
    }
    if let Some(m) = &method {
        if m.value == "element_chase" && !sets {
            return Err((true, vec![Diag::new(format!("\"{}\" is a way to prove statements about sets", m.words)).hint("for logic, try \"by truth table\"")]));
        }
    }
    // a false statement is refused with the row that shows it
    if let Some(vals) = counterexample(&problem.value) {
        let row = describe_row(&names, &vals, sets);
        let (what, why) = match &problem.value {
            Math::Taut(_) => ("isn't a tautology", format!("{row} makes it false")),
            Math::Equiv(l, _) => ("isn't an equivalence", format!("{row} makes the left side {} and the right side {}", value_at(l, &names, &vals) == Some(true), value_at(l, &names, &vals) != Some(true))),
            Math::Eq(l, _) => ("isn't true for every choice of sets", format!("{row} is in the {} side only", if value_at(l, &names, &vals) == Some(true) { "left" } else { "right" })),
            _ => ("isn't true for every choice of sets", format!("{row} is in the left side but not the right")),
        };
        return Err((true, vec![Diag::new(format!("{shown} {what}, so it can't be proved")).hint(why)]));
    }
    let var = if sets { Said::new("x".to_string(), "(an element of the sets)") } else { Said::new(names.first().cloned().unwrap_or_else(|| "p".into()), "(first letter)") };
    Ok(Request { sentence: sentence.trim().to_string(), task, problem, var, given: vec![], method, modifiers, decimals: None, notes: vec![], calc: Calc::default() })
}

/// The tokens of a logic sentence, for --explain.
pub fn tokens(sentence: &str) -> Vec<String> {
    scan(sentence).unwrap_or_default().iter().map(|t| format!("{:?} <- \"{}\"", t.l, t.words)).collect()
}

// ---- membership ---------------------------------------------------------------------

/// What "x is in the set" says, down to the letters: x in A intersect B is
/// x in A and x in B, x in A' is x not in A, x in A \ B is x in A and x not in B.
pub fn member(x: &str, s: &Expr) -> Expr {
    let m = |e: &Expr| member(x, e);
    match s {
        Expr::Set(SetOp::Union, v) => or(Flavor::Logic, v.iter().map(m).collect()),
        Expr::Set(SetOp::Inter, v) => and(Flavor::Logic, v.iter().map(m).collect()),
        Expr::Set(SetOp::Complement, v) => not(Flavor::Logic, m(&v[0])),
        Expr::Set(SetOp::Diff, v) => and(Flavor::Logic, vec![m(&v[0]), not(Flavor::Logic, m(&v[1]))]),
        Expr::SetConst(b) => Expr::Truth(*b),
        e => Expr::Member(x.to_string(), Box::new(e.clone())),
    }
}

/// Test support: the state a rule sees for a logic sentence.
#[cfg(test)]
pub fn test_moves(rule: &dyn rules::Rule, sentence: &str) -> Vec<String> {
    let cfg = Config::builtin();
    let req = match parse(sentence, &cfg) {
        Some(Ok(r)) => r,
        other => panic!("{sentence}: {:?}", other.map(|r| r.err())),
    };
    let cx = Cx { req: &req, cfg: &cfg, var: &req.var.value };
    rule.moves(&req.start(), &cx).iter().map(|m| print::math(&m.result, Style::Ascii)).collect()
}

/// Test support: a statement or expression written the way a sentence would.
#[cfg(test)]
pub fn test_expr(s: &str) -> Expr {
    let toks = scan(s).expect("symbols");
    let sets = toks.iter().any(|t| matches!(t.l, L::Union | L::Inter | L::Comp | L::Prime | L::Diff | L::Subset | L::Empty));
    let mut p = P { ts: &toks, i: 0 };
    settle(p.expr().expect("parses"), sets).expect("settles")
}

/// Test support: the moves a rule offers from a given state, for a sentence.
#[cfg(test)]
pub fn test_moves_from(rule: &dyn rules::Rule, sentence: &str, st: Math) -> Vec<String> {
    let cfg = Config::builtin();
    let req = match parse(sentence, &cfg) {
        Some(Ok(r)) => r,
        other => panic!("{sentence}: {:?}", other.map(|r| r.err())),
    };
    let cx = Cx { req: &req, cfg: &cfg, var: &req.var.value };
    rule.moves(&st, &cx).iter().map(|m| print::math(&m.result, Style::Ascii)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(s: &str) -> String {
        match parse(s, &Config::builtin()) {
            Some(Ok(r)) => format!("{} | {}", r.task.value.key(), print::math(&r.problem.value, Style::Ascii)),
            Some(Err(d)) => format!("error: {}", d[0]),
            None => "not logic".into(),
        }
    }

    #[test]
    fn statements_parse() {
        assert_eq!(show("prove (p -> q) <-> (~q -> ~p)"), "prove | (p -> q) <=> (~q -> ~p)");
        assert_eq!(show("prove not (p and q) is equivalent to not p or not q"), "prove | ~(p and q) <=> ~p or ~q");
        assert_eq!(show("show that p or not p is a tautology"), "prove | p or ~p");
        assert_eq!(show("prove ((p -> q) and p) -> q"), "prove | ((p -> q) and p) -> q");
        assert_eq!(show("prove A ∩ (B ∪ C) = (A ∩ B) ∪ (A ∩ C)"), "prove | A intersect (B union C) = (A intersect B) union (A intersect C)");
        assert_eq!(show("prove (A ∪ B)' = A' ∩ B'"), "prove | (A union B)' = A' intersect B'");
        assert_eq!(show("prove A \\ B = A ∩ B'"), "prove | A \\ B = A intersect B'");
        assert_eq!(show("prove A ∩ B ⊆ A"), "prove | A intersect B subset of A");
        assert_eq!(show("prove p ^ ~p = F"), "prove | p and ~p <=> F");
    }

    #[test]
    fn other_sentences_are_not_logic() {
        assert_eq!(show("prove (a + b)^2 = a^2 + 2ab + b^2"), "not logic");
        assert_eq!(show("solve x + 1 = 2"), "not logic");
        assert_eq!(show("lim x->0 sin x / x"), "not logic");
        assert_eq!(show("prove the Hodge conjecture"), "not logic");
    }

    #[test]
    fn false_statements_are_refused_with_a_row() {
        assert!(show("prove (p -> q) -> p").contains("p = false, q = true makes it false"));
        assert!(show("prove A union B = A").contains("an element in B but not in A"));
    }
}
