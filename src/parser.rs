//! Tokens -> a structured request. The math itself is a small Pratt parser
//! with the conventions people use on paper: 2x is 2*x, sin 2x is sin(2x),
//! -x^2 is -(x^2), 15% of 80 is 15/100 * 80.

use crate::expr::{self, Expr, Func, Konst, Math};
use crate::lexicon::{self, Tok, Token};
use crate::model::{Calc, Request, Said, Task};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Diag {
    pub message: String,
    pub hint: Option<String>,
}

impl Diag {
    pub fn new(m: impl Into<String>) -> Diag {
        Diag { message: m.into(), hint: None }
    }
    pub fn hint(mut self, h: impl Into<String>) -> Diag {
        self.hint = Some(h.into());
        self
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)?;
        if let Some(h) = &self.hint {
            write!(f, "\n  hint: {h}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ParseOptions {
    pub lenient: bool,
}

fn is_math(t: &Tok) -> bool {
    matches!(t, Tok::Num(_) | Tok::Var(_) | Tok::Const(_) | Tok::Func(_) | Tok::Op(_) | Tok::Squared | Tok::Cubed | Tok::Percent | Tok::Infix(_) | Tok::Bang | Tok::Built(_) | Tok::Rel(_) | Tok::Log | Tok::LogBase | Tok::Degrees)
}

/// How many tokens from the start form a math span.
fn math_len(ts: &[Token]) -> usize {
    ts.iter().take_while(|t| is_math(&t.tok)).count()
}

/// How many tokens form the point a limit approaches: 2, -1, pi/2, 2pi,
/// infinity, -infinity. It stops there, so "lim x->0 sin x/x" keeps its
/// function.
fn point_len(ts: &[Token]) -> usize {
    let tok = |k: usize| ts.get(k).map(|t| &t.tok);
    let mut k = 0;
    if matches!(tok(k), Some(Tok::Op('-') | Tok::Op('+'))) {
        k += 1;
    }
    if !matches!(tok(k), Some(Tok::Num(_) | Tok::Const(_))) {
        return 0;
    }
    k += 1;
    if matches!(tok(k), Some(Tok::Const(_))) {
        k += 1;
    }
    if matches!(tok(k), Some(Tok::Op('/'))) && matches!(tok(k + 1), Some(Tok::Num(_) | Tok::Const(_))) {
        k += 2;
    }
    k
}

/// A bound or a point: "3", or "x = 3" (the value).
fn value_of(ts: &[Token]) -> Result<Expr, Diag> {
    match parse_math(ts)? {
        Math::Expr(e) => Ok(e),
        Math::Eq(Expr::Var(_), e) => Ok(e),
        _ => Err(Diag::new(format!("can't use \"{}\" as a bound", words(ts))).hint("e.g. \"from 0 to 3\"")),
    }
}

fn words(ts: &[Token]) -> String {
    ts.iter().map(|t| t.words.as_str()).collect::<Vec<_>>().join(" ")
}

pub fn parse(sentence: &str, opts: &ParseOptions) -> Result<Request, Vec<Diag>> {
    parse_with(sentence, opts, &crate::config::Config::builtin())
}

pub fn parse_with(sentence: &str, opts: &ParseOptions, cfg: &crate::config::Config) -> Result<Request, Vec<Diag>> {
    let toks = lexicon::lex(sentence).map_err(|e| vec![Diag::new(e)])?;
    // a famous problem: name it, say where it stands, and why Nuome stops
    // written out in full: Hodge classes and algebraic cycles together are the Hodge conjecture
    let said = |w: &str| toks.iter().any(|t| matches!(t.tok, Tok::Topic(_)) && t.words.starts_with(w));
    let spelled_out = (said("hodge") && said("algebraic cycle")).then_some("hodge");
    if let Some(key) = toks.iter().find_map(|t| if let Tok::Open(k) = t.tok { Some(k) } else { None }).or(spelled_out) {
        let p = &cfg.open[key];
        let named = if toks.iter().any(|t| matches!(t.tok, Tok::Open(_))) { p.name.clone() } else { format!("{} (recognised from its statement)", p.name) };
        let mut m = format!("{named} is not something Nuome can prove: {}\n  statement: {}", p.status, p.statement);
        for k in &p.known {
            m.push_str(&format!("\n  known: {k}"));
        }
        return Err(vec![Diag::new(m).hint("Nuome prints only what its rules derive and its checks confirm; no rule set derives this, and a proof could not be checked by sampling or substitution")]);
    }
    let topics: Vec<&Token> = toks.iter().filter(|t| matches!(t.tok, Tok::Topic(_))).collect();
    if let Some(Token { tok: Tok::Topic(area), .. }) = topics.first() {
        let words: Vec<String> = topics.iter().map(|t| format!("\"{}\"", t.words)).collect();
        return Err(vec![Diag::new(format!("{}: {area} has no rules in Nuome", words.join(", "))).hint("Nuome covers arithmetic, algebra, calculus, trigonometry, number theory basics, statistics and sequences")]);
    }
    let (toks, notes) = crate::words::rewrite(toks, cfg).map_err(|d| vec![d])?;
    let mut diags = Vec::new();
    let mut task: Option<Said<Task>> = None;
    let mut var: Option<Said<String>> = None;
    let mut method = None;
    let mut modifiers = Vec::new();
    let mut decimals: Option<Said<u32>> = None;
    let mut exact: Option<String> = None;
    let mut spans: Vec<Vec<Token>> = Vec::new();
    let mut given_spans: Vec<Vec<Token>> = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    let mut in_given = false;
    let mut order: Option<Said<u32>> = None;
    let mut bounds: Option<Said<(Expr, Expr)>> = None;
    let mut point: Option<Said<Expr>> = None;

    let flush = |cur: &mut Vec<Token>, in_given: &mut bool, spans: &mut Vec<Vec<Token>>, given: &mut Vec<Vec<Token>>| {
        if !cur.is_empty() {
            if *in_given {
                given.push(std::mem::take(cur));
            } else {
                spans.push(std::mem::take(cur));
            }
        }
        *in_given = false;
    };

    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        // "of" after a percentage multiplies: 15% of 80
        if t.tok == Tok::Of {
            if cur.last().is_some_and(|p| p.tok == Tok::Percent) {
                cur.push(Token { tok: Tok::Op('*'), words: t.words.clone() });
            }
            i += 1;
            continue;
        }
        // "in degrees" with no number before it: only says how angles are measured
        if t.tok == Tok::Degrees && cur.is_empty() {
            i += 1;
            continue;
        }
        // "as x approaches 2", "lim x->0": the letter before, the point after
        if t.tok == Tok::Approaches {
            let Some(Token { tok: Tok::Var(v), words: vw }) = cur.last().cloned() else {
                diags.push(Diag::new(format!("\"{}\" needs a letter before it", t.words)).hint("e.g. \"as x approaches 2\""));
                i += 1;
                continue;
            };
            cur.pop();
            // lim(x -> 0) ...
            let open = cur.last().is_some_and(|p| p.tok == Tok::Op('('));
            if open {
                cur.pop();
            }
            let n = point_len(&toks[i + 1..]);
            let span = &toks[i + 1..i + 1 + n];
            let said = format!("{vw} {} {}", t.words, words(span));
            match value_of(span) {
                Ok(p) if n > 0 => point = Some(Said::new(p, said.clone())),
                _ => diags.push(Diag::new(format!("\"{}\" needs a point after it", t.words)).hint("e.g. \"as x approaches 2\" or \"as x approaches infinity\"")),
            }
            if var.is_none() {
                var = Some(Said::new(v, said));
            }
            i += 1 + n;
            if open && toks.get(i).is_some_and(|t| t.tok == Tok::Op(')')) {
                i += 1;
            }
            continue;
        }
        // "to 3 decimal places", "3 dp"
        if let Tok::Num(n) = &t.tok {
            if toks.get(i + 1).is_some_and(|n| n.tok == Tok::Places) {
                if !n.is_int() || n.is_neg() || n.num() > 12 {
                    diags.push(Diag::new(format!("can't round to {} decimal places", t.words)).hint("use 0 to 12 places"));
                }
                let lead = if cur.last().is_some_and(|p| p.tok == Tok::To) { cur.pop().map(|p| format!("{} ", p.words)).unwrap_or_default() } else { String::new() };
                decimals = Some(Said::new(n.num().clamp(0, 12) as u32, format!("{lead}{} {}", t.words, toks[i + 1].words)));
                i += 2;
                continue;
            }
        }
        // "divide A by B" is (A)/(B)
        if t.tok == Tok::By {
            if !cur.is_empty() && toks.get(i + 1).is_some_and(|n| is_math(&n.tok)) {
                let mut j = i + 1;
                while j < toks.len() && is_math(&toks[j].tok) {
                    j += 1;
                }
                let op = |c: char, w: &str| Token { tok: Tok::Op(c), words: w.to_string() };
                let mut wrapped = vec![op('(', "(")];
                wrapped.append(&mut cur);
                wrapped.extend([op(')', ")"), op('/', &t.words), op('(', "(")]);
                wrapped.extend(toks[i + 1..j].iter().cloned());
                wrapped.push(op(')', ")"));
                cur = wrapped;
                i = j;
            } else {
                i += 1;
            }
            continue;
        }
        if is_math(&t.tok) {
            cur.push(t.clone());
            i += 1;
            continue;
        }
        match &t.tok {
            Tok::To => {
                // only meaningful before "3 decimal places"; otherwise filler
                if matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Num(_))) && toks.get(i + 2).is_some_and(|t| t.tok == Tok::Places) {
                    flush(&mut cur, &mut in_given, &mut spans, &mut given_spans);
                    cur.push(t.clone());
                    in_given = false;
                }
                i += 1;
                continue;
            }
            _ => flush(&mut cur, &mut in_given, &mut spans, &mut given_spans),
        }
        match &t.tok {
            Tok::Task(k) => {
                if let Some(prev) = &task {
                    if prev.value != *k {
                        diags.push(Diag::new(format!("two tasks: \"{}\" and \"{}\"", prev.words, t.words)).hint("ask for one thing at a time"));
                    }
                } else {
                    task = Some(Said::new(*k, t.words.clone()));
                }
            }
            Tok::For => match toks.get(i + 1) {
                Some(Token { tok: Tok::Var(v), words }) => {
                    var = Some(Said::new(v.clone(), format!("{} {words}", t.words)));
                    i += 1;
                }
                _ => diags.push(Diag::new(format!("\"{}\" needs a letter after it", t.words)).hint("e.g. \"solve 2y + 1 = 5 for y\"")),
            },
            Tok::When => in_given = true,
            Tok::Method(m) => {
                if method.is_some() {
                    diags.push(Diag::new(format!("two methods asked for (\"{}\")", t.words)));
                }
                method = Some(Said::new(m.to_string(), t.words.clone()));
            }
            Tok::Mod(m) => modifiers.push(Said::new(*m, t.words.clone())),
            Tok::Decimal => {
                if decimals.is_none() {
                    decimals = Some(Said::new(3, t.words.clone()));
                }
            }
            Tok::Places => diags.push(Diag::new(format!("\"{}\" needs a number", t.words)).hint("e.g. \"to 3 decimal places\"")),
            Tok::Exact => exact = Some(t.words.clone()),
            Tok::Order(n) => {
                order = Some(Said::new(*n, t.words.clone()));
                if task.is_none() {
                    task = Some(Said::new(Task::Differentiate, t.words.clone()));
                }
            }
            Tok::From => {
                // "from 0 to 3", "between 0 and pi"
                let lo = math_len(&toks[i + 1..]);
                let sep = i + 1 + lo;
                let joined = toks.get(sep).is_some_and(|s| s.tok == Tok::To || (s.tok == Tok::Sep && s.words == "and"));
                let hi = if joined { math_len(&toks[sep + 1..]) } else { 0 };
                if lo == 0 || hi == 0 {
                    diags.push(Diag::new(format!("\"{}\" needs two bounds", t.words)).hint("e.g. \"from 0 to 3\" or \"between 0 and pi\""));
                    i += 1;
                    continue;
                }
                match (value_of(&toks[i + 1..sep]), value_of(&toks[sep + 1..sep + 1 + hi])) {
                    (Ok(a), Ok(b)) => bounds = Some(Said::new((a, b), words(&toks[i..sep + 1 + hi]))),
                    (Err(d), _) | (_, Err(d)) => diags.push(d),
                }
                i = sep + 1 + hi;
                continue;
            }
            Tok::Approaches | Tok::Degrees => unreachable!("handled above"),
            Tok::Sep | Tok::Filler | Tok::Is | Tok::To | Tok::Of => {}
            Tok::Unsupported(what) => diags.push(Diag::new(format!("\"{}\": {what} is not in Nuome v0", t.words)).hint("Nuome covers arithmetic, algebra, calculus, trigonometry, number theory basics, statistics and sequences; `--vocabulary` lists every word")),
            Tok::Unknown => {
                if !opts.lenient {
                    let mut d = Diag::new(format!("unknown word \"{}\"", t.words));
                    d.hint = Some(match lexicon::suggest(&t.words) {
                        Some(h) => format!("{h} (--vocabulary lists every word, --lenient skips unknown ones)"),
                        None => "--vocabulary lists every word, --lenient skips unknown ones".into(),
                    });
                    diags.push(d);
                }
            }
            // word-grammar tokens left over: they didn't fit a construction
            Tok::List(_) | Tok::Ordinal(_) | Tok::SumOf | Tok::First | Tok::TermsOf | Tok::TermOf | Tok::InfSum | Tok::Pick(_) | Tok::Remainder | Tok::Grow | Tok::Per | Tok::Period(_) | Tok::Every(_) | Tok::Change(_) | Tok::By | Tok::WhatPct | Tok::IsWhatPct | Tok::PctChange => {
                diags.push(Diag::new(format!("\"{}\" doesn't fit here", t.words)).hint("--vocabulary lists every word; see the README for the phrasings Nuome understands"));
            }
            _ => unreachable!("math tokens handled above"),
        }
        i += 1;
    }
    flush(&mut cur, &mut in_given, &mut spans, &mut given_spans);
    if let (Some(e), Some(d)) = (&exact, &decimals) {
        diags.push(Diag::new(format!("\"{e}\" and \"{}\" ask for different answers", d.words)));
    }
    // a stray "to" span (from "to 3 dp" handled above) is dropped
    spans.retain(|s| !(s.len() == 1 && s[0].tok == Tok::To));

    let mut problems = Vec::new();
    for s in &spans {
        match parse_math(s) {
            Ok(m) => problems.push(Said::new(m, words(s))),
            Err(d) => diags.push(d),
        }
    }
    // "find x if ...", "x where ...": a lone letter beside an equation names the unknown
    if problems.len() == 1 && matches!(problems[0].value, Math::Expr(Expr::Var(_))) && (!given_spans.is_empty() || spans.len() > 1) {
        let p = problems.remove(0);
        if var.is_none() {
            if let Math::Expr(Expr::Var(v)) = p.value {
                var = Some(Said::new(v, p.words));
            }
        }
    }
    // "find x if 7 = 3x - 2": with no other problem, the condition is the problem
    if problems.is_empty() && given_spans.len() == 1 {
        let s = given_spans.pop().expect("one span");
        match parse_math(&s) {
            Ok(m) => problems.push(Said::new(m, words(&s))),
            Err(d) => diags.push(d),
        }
    }
    let mut given = Vec::new();
    for s in &given_spans {
        match parse_math(s) {
            Ok(Math::Eq(Expr::Var(v), val)) if !val.has_var(&v) && val.vars().is_empty() => given.push(Said::new((v, val), words(s))),
            Ok(_) => diags.push(Diag::new(format!("can't use \"{}\" as a condition", words(s))).hint("write it as a letter and a number, e.g. \"when x = 3\"")),
            Err(d) => diags.push(d),
        }
    }
    if !diags.is_empty() {
        return Err(diags);
    }
    // several equations are a system, solved together
    if problems.len() > 1 && problems.iter().all(|p| matches!(p.value, Math::Eq(..))) {
        let words = problems.iter().map(|p| p.words.clone()).collect::<Vec<_>>().join(" and ");
        let eqs = problems
            .drain(..)
            .filter_map(|p| match p.value {
                Math::Eq(l, r) => Some((l, r)),
                _ => None,
            })
            .collect();
        problems.push(Said::new(Math::System(eqs), words));
    }
    let problem = match problems.len() {
        0 => return Err(vec![Diag::new("no math in the sentence").hint("try: nuome \"solve 2x + 3 = 7\"")]),
        1 => problems.pop().unwrap(),
        _ => {
            let all: Vec<String> = problems.iter().map(|p| format!("\"{}\"", p.words)).collect();
            let mut d = Diag::new(format!("more than one problem: {}", all.join(", ")));
            d.hint = Some(if problems.iter().filter(|p| matches!(p.value, Math::Eq(..))).count() > 1 { "solving systems of equations is not in Nuome v0; one equation at a time".into() } else { "one problem at a time".into() });
            return Err(vec![d]);
        }
    };
    let letters = match &problem.value {
        Math::Expr(e) => e.vars(),
        Math::Eq(l, r) => l.vars().union(&r.vars()).cloned().collect(),
        Math::Ineq(l, _, r) => l.vars().union(&r.vars()).cloned().collect(),
        Math::System(eqs) => eqs.iter().flat_map(|(l, r)| l.vars().into_iter().chain(r.vars())).collect(),
        _ => Default::default(),
    };
    // calculus and trig (agent B): a tangent's "y = f(x)" names only x, and
    // "at x = 1" is where it touches, not a value to substitute
    let tangent = task.as_ref().is_some_and(|t| t.value == Task::Tangent);
    let letters = match &problem.value {
        Math::Eq(Expr::Var(y), f) if tangent && !f.has_var(y) => f.vars(),
        _ => letters,
    };
    if tangent && point.is_none() {
        if let Some(k) = given.iter().position(|g| letters.contains(&g.value.0) || letters.is_empty()) {
            let g = given.remove(k);
            if var.is_none() {
                var = Some(Said::new(g.value.0.clone(), g.words.clone()));
            }
            point = Some(Said::new(g.value.1, g.words));
        }
    }
    let task = task.unwrap_or_else(|| match &problem.value {
        Math::Eq(..) => Said::new(Task::Solve, "(an equation: solve)"),
        Math::Ineq(..) => Said::new(Task::Solve, "(an inequality: solve)"),
        Math::System(_) => Said::new(Task::Solve, "(equations: solve them together)"),
        _ if letters.is_empty() || !given.is_empty() => Said::new(Task::Evaluate, "(no letters: evaluate)"),
        _ => Said::new(Task::Simplify, "(letters: simplify)"),
    });
    // which letter
    let var = match var {
        Some(v) => {
            if task.value == Task::Solve && !letters.contains(&v.value) {
                return Err(vec![Diag::new(format!("{} doesn't appear in {}", v.value, problem.words))]);
            }
            v
        }
        None => {
            let free: Vec<&String> = letters.iter().filter(|l| !given.iter().any(|g| &g.value.0 == *l)).collect();
            match free.len() {
                0 => Said::new("x".to_string(), "(default x)"),
                1 => Said::new(free[0].clone(), "(the only letter)"),
                _ if matches!(problem.value, Math::System(_)) => Said::new(free[0].clone(), "(the first letter of the system)"),
                _ if task.value == Task::Solve || task.value == Task::Differentiate => {
                    let names: Vec<&str> = free.iter().map(|s| s.as_str()).collect();
                    return Err(vec![Diag::new(format!("which letter? {} has {}", problem.words, names.join(", "))).hint(format!("say \"for {}\" or \"with respect to {}\"", names[0], names[0]))]);
                }
                _ => Said::new(free[0].clone(), "(first letter)"),
            }
        }
    };
    // "divide 7 by 2" is arithmetic
    let task = if task.value == Task::Divide && letters.is_empty() { Said::new(Task::Evaluate, task.words) } else { task };
    let shown = crate::print::math(&problem.value, crate::print::Style::Ascii);
    let math_ok = match (task.value, &problem.value) {
        (Task::Prove, Math::Eq(..) | Math::Ineq(..) | Math::System(_)) => Ok(()),
        (Task::Prove, _) => Err(Diag::new(format!("prove what about {shown}?")).hint("state it as an equation or an inequality, e.g. \"prove (a + b)^2 = a^2 + 2ab + b^2\"")),
        (Task::Solve, Math::Eq(..)) => Ok(()),
        (Task::Solve, Math::Ineq(..)) => Ok(()),
        (Task::Solve, Math::System(eqs)) => system_ok(eqs, &letters),
        (_, Math::Ineq(..) | Math::System(_)) => Err(Diag::new(format!("{shown} can only be solved, not {}", task.value.key())).hint(format!("say \"solve {shown}\""))),
        (Task::Divide, Math::Expr(e)) => divide_ok(e, &var.value),
        (Task::Solve, _) => Err(Diag::new(format!("{shown} isn't an equation")).hint(format!("to find where it is zero, write \"solve {shown} = 0\""))),
        (Task::Tangent, Math::Eq(Expr::Var(y), f)) if !f.has_var(y) => Ok(()),
        (_, Math::Eq(..)) => Err(Diag::new(format!("{shown} is an equation; {} works on an expression", task.value.key())).hint(format!("to find {}, say \"solve {shown}\"", var.value))),
        (Task::Evaluate, Math::Expr(e)) => {
            let unset: Vec<String> = e.vars().into_iter().filter(|l| !given.iter().any(|g| &g.value.0 == l)).collect();
            if unset.is_empty() {
                Ok(())
            } else {
                Err(Diag::new(format!("can't evaluate: {} has no value", unset.join(", "))).hint(format!("give one (\"when {} = 3\") or say \"simplify\"; for multiplication write * or times, since x is a letter", unset[0])))
            }
        }
        _ => Ok(()),
    };
    math_ok.map_err(|d| vec![d])?;
    if let Some(m) = &method {
        if task.value != Task::Solve {
            return Err(vec![Diag::new(format!("\"{}\" is a way to solve an equation, not to {}", m.words, task.value.key()))]);
        }
    }
    if let Some(d) = &decimals {
        if matches!(task.value, Task::Factor | Task::Expand | Task::Differentiate | Task::Divide) && given.is_empty() {
            return Err(vec![Diag::new(format!("\"{}\": a {} has no single value to round", d.words, task.value.key()))]);
        }
    }
    // calculus and trig (agent B): the extra words must fit the task
    let degrees = toks.iter().find(|t| t.tok == Tok::Degrees).map(|t| t.words.clone());
    let calc = Calc { order, bounds, point, degrees };
    if let Some(o) = &calc.order {
        if task.value != Task::Differentiate {
            return Err(vec![Diag::new(format!("\"{}\" goes with differentiating, not with {}", o.words, task.value.verb()))]);
        }
    }
    if let Some(b) = &calc.bounds {
        if !matches!(task.value, Task::Integrate | Task::Solve) {
            return Err(vec![Diag::new(format!("\"{}\": bounds go with an integral or with solving in an interval", b.words))]);
        }
        if b.value.0.has_var(&var.value) || b.value.1.has_var(&var.value) || [&b.value.0, &b.value.1].iter().any(|e| e.walk().iter().any(|(_, n)| matches!(n, Expr::Const(crate::expr::Konst::Inf)))) {
            return Err(vec![Diag::new(format!("\"{}\": the bounds must be numbers", b.words)).hint("improper integrals (to infinity) are not in Nuome v0")]);
        }
    }
    match (&calc.point, task.value) {
        (None, Task::Limit) => return Err(vec![Diag::new("a limit needs a point").hint(format!("say where {} goes, e.g. \"as {} approaches 2\"", var.value, var.value))]),
        (None, Task::Tangent) => return Err(vec![Diag::new("a tangent needs a point").hint(format!("say where it touches, e.g. \"at {} = 1\"", var.value))]),
        (Some(p), t) if !matches!(t, Task::Limit | Task::Tangent) => return Err(vec![Diag::new(format!("\"{}\" goes with a limit", p.words)).hint("e.g. \"limit of sin x / x as x approaches 0\"")]),
        _ => {}
    }
    if let Some(d) = &decimals {
        if (task.value == Task::Integrate && calc.bounds.is_none()) || task.value == Task::Tangent {
            return Err(vec![Diag::new(format!("\"{}\": the answer is a function, with no single value to round", d.words))]);
        }
    }
    Ok(Request { sentence: sentence.trim().to_string(), task, problem, var, given, method, modifiers, decimals, notes, calc })
}

/// A system: two or three linear equations in at most three letters.
fn system_ok(eqs: &[(Expr, Expr)], letters: &std::collections::BTreeSet<String>) -> Result<(), Diag> {
    let vars: Vec<String> = letters.iter().cloned().collect();
    if eqs.len() > 3 || vars.len() > 3 {
        return Err(Diag::new(format!("{} equations in {} letters is more than Nuome solves", eqs.len(), vars.len())).hint("systems of up to three equations in up to three letters"));
    }
    for (l, r) in eqs {
        if crate::poly::linear_form(l, r, &vars).is_none() {
            let shown = crate::print::math(&Math::Eq(l.clone(), r.clone()), crate::print::Style::Ascii);
            return Err(Diag::new(format!("{shown} isn't linear")).hint("Nuome solves systems of linear equations (no powers or products of letters)"));
        }
    }
    Ok(())
}

/// Polynomial division needs "A by B" with polynomials in one letter.
fn divide_ok(e: &Expr, v: &str) -> Result<(), Diag> {
    let Expr::Div(a, b) = e else {
        return Err(Diag::new("divide what by what?").hint("e.g. \"divide x^3 - 1 by x - 1\""));
    };
    if e.vars().len() > 1 || crate::poly::from_expr(a, v).is_none() || crate::poly::from_expr(b, v).is_none_or(|p| p.deg().is_none_or(|d| d == 0)) {
        let shown = crate::print::expr(e, crate::print::Style::Ascii);
        return Err(Diag::new(format!("can't divide {shown} as polynomials")).hint("both parts must be polynomials in one letter, and the divisor must contain the letter"));
    }
    Ok(())
}

/// Parse one math span: an expression, or an equation with one "=".
pub fn parse_math(ts: &[Token]) -> Result<Math, Diag> {
    // an inequality: one sign, no "="
    let rels: Vec<usize> = ts.iter().enumerate().filter(|(_, t)| matches!(t.tok, Tok::Rel(_))).map(|(i, _)| i).collect();
    if let Some(&k) = rels.first() {
        if rels.len() > 1 || ts.iter().any(|t| t.tok == Tok::Op('=')) {
            return Err(Diag::new(format!("\"{}\" has more than one sign", words(ts))).hint("one inequality at a time; write a double inequality as two"));
        }
        let Tok::Rel(rel) = ts[k].tok else { unreachable!("found above") };
        let (l, r) = (&ts[..k], &ts[k + 1..]);
        if l.is_empty() || r.is_empty() {
            return Err(Diag::new(format!("\"{}\": one side of {} is empty", words(ts), ts[k].words)));
        }
        return Ok(Math::Ineq(parse_expr(l)?, rel, parse_expr(r)?));
    }
    let eqs: Vec<usize> = ts.iter().enumerate().filter(|(_, t)| t.tok == Tok::Op('=')).map(|(i, _)| i).collect();
    match eqs.len() {
        0 => Ok(Math::Expr(parse_expr(ts)?)),
        1 => {
            let (l, r) = (&ts[..eqs[0]], &ts[eqs[0] + 1..]);
            if l.is_empty() || r.is_empty() {
                return Err(Diag::new(format!("\"{}\": one side of = is empty", words(ts))));
            }
            Ok(Math::Eq(parse_expr(l)?, parse_expr(r)?))
        }
        _ => Err(Diag::new(format!("\"{}\" has more than one =", words(ts))).hint("one equation at a time")),
    }
}

pub fn parse_expr(ts: &[Token]) -> Result<Expr, Diag> {
    let mut p = P { ts, i: 0, bars: 0 };
    let e = p.sum()?;
    if p.i < ts.len() {
        return Err(Diag::new(format!("didn't expect \"{}\" in \"{}\"", ts[p.i].words, words(ts))));
    }
    Ok(expr::tidy(e))
}

struct P<'a> {
    ts: &'a [Token],
    i: usize,
    /// How many |...| are open: inside one, "|" closes it.
    bars: usize,
}

impl P<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.ts.get(self.i).map(|t| &t.tok)
    }
    fn err(&self, what: &str) -> Diag {
        let all = words(self.ts);
        match self.ts.get(self.i) {
            Some(t) => Diag::new(format!("{what} at \"{}\" in \"{all}\"", t.words)),
            None => Diag::new(format!("{what} at the end of \"{all}\"")),
        }
    }
    fn starts_primary(&self) -> bool {
        matches!(self.peek(), Some(Tok::Num(_) | Tok::Var(_) | Tok::Const(_) | Tok::Func(_) | Tok::Op('(') | Tok::Built(_) | Tok::Log | Tok::LogBase)) || (self.bars == 0 && self.peek() == Some(&Tok::Op('|')))
    }
    fn sum(&mut self) -> Result<Expr, Diag> {
        let mut terms = vec![self.product()?];
        loop {
            match self.peek() {
                Some(Tok::Op('+')) => {
                    self.i += 1;
                    terms.push(self.product()?);
                }
                Some(Tok::Op('-')) => {
                    self.i += 1;
                    let t = self.product()?;
                    terms.push(expr::neg(t));
                }
                _ => break,
            }
        }
        Ok(if terms.len() == 1 { terms.pop().unwrap() } else { Expr::Add(terms) })
    }
    fn product(&mut self) -> Result<Expr, Diag> {
        let mut acc = self.unary()?;
        loop {
            match self.peek() {
                Some(Tok::Op('*')) => {
                    self.i += 1;
                    let r = self.unary()?;
                    acc = join_mul(acc, r);
                }
                Some(Tok::Op('/')) => {
                    self.i += 1;
                    let r = self.unary()?;
                    acc = expr::div(acc, r);
                }
                // 17 mod 5, 10 choose 3
                Some(Tok::Infix(f)) => {
                    let f = *f;
                    self.i += 1;
                    let r = self.unary()?;
                    acc = Expr::Call(f, vec![acc, r]);
                }
                _ if self.starts_primary() => {
                    if matches!(self.peek(), Some(Tok::Num(_))) {
                        let d = self.err("a number right after a factor");
                        return Err(if matches!(acc, Expr::Mul(ref v) if v.last() == Some(&expr::var("x"))) || acc == expr::var("x") {
                            d.hint("x is a letter here; for multiplication write * or times")
                        } else {
                            d.hint("put * or an operator between them")
                        });
                    }
                    let r = self.power()?;
                    acc = join_mul(acc, r);
                }
                _ => break,
            }
        }
        Ok(acc)
    }
    fn unary(&mut self) -> Result<Expr, Diag> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.i += 1;
                // -3 is a number; -x^2 is -(x^2)
                let e = self.unary()?;
                Ok(match e {
                    Expr::Num(q) => Expr::Num(q.neg()),
                    e => Expr::Neg(Box::new(e)),
                })
            }
            Some(Tok::Op('+')) => {
                self.i += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }
    fn power(&mut self) -> Result<Expr, Diag> {
        let base = self.postfix()?;
        if self.peek() == Some(&Tok::Op('^')) {
            self.i += 1;
            let e = self.unary()?;
            if base == Expr::Const(Konst::E) {
                return Ok(expr::func(Func::Exp, e));
            }
            return Ok(expr::pow(base, e));
        }
        Ok(base)
    }
    fn postfix(&mut self) -> Result<Expr, Diag> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::Squared) => e = expr::pow(e, expr::num(2)),
                Some(Tok::Cubed) => e = expr::pow(e, expr::num(3)),
                Some(Tok::Percent) => e = expr::div(e, expr::num(100)),
                Some(Tok::Bang) => e = Expr::Call(crate::calls::Named::Factorial, vec![e]),
                Some(Tok::Degrees) => e = join_mul(e, Expr::Const(Konst::Deg)),
                _ => break,
            }
            self.i += 1;
        }
        Ok(e)
    }
    fn primary(&mut self) -> Result<Expr, Diag> {
        let t = self.ts.get(self.i).ok_or_else(|| self.err("expected a number or a letter"))?;
        self.i += 1;
        match &t.tok {
            Tok::Num(q) => Ok(Expr::Num(*q)),
            Tok::Var(v) => Ok(Expr::Var(v.clone())),
            Tok::Const(k) => Ok(Expr::Const(*k)),
            Tok::Built(e) => Ok(e.clone()),
            Tok::Op('(') => {
                let e = self.sum()?;
                if self.peek() != Some(&Tok::Op(')')) {
                    return Err(self.err("missing )"));
                }
                self.i += 1;
                Ok(e)
            }
            Tok::Func(f) => {
                // sin^2 x = (sin x)^2, sin^-1 x = arcsin x (calculus and trig, agent B)
                let mut f = *f;
                let mut power = None;
                if self.peek() == Some(&Tok::Op('^')) {
                    let neg = matches!(self.ts.get(self.i + 1).map(|t| &t.tok), Some(Tok::Op('-')));
                    let at = self.i + 1 + usize::from(neg);
                    if let Some(Tok::Num(n)) = self.ts.get(at).map(|t| &t.tok) {
                        let inverse = match f {
                            Func::Sin => Some(Func::Asin),
                            Func::Cos => Some(Func::Acos),
                            Func::Tan => Some(Func::Atan),
                            _ => None,
                        };
                        match (neg, inverse) {
                            (true, Some(g)) if n.is_one() => f = g,
                            (false, _) if n.is_int() => power = Some(*n),
                            _ => return Err(self.err("can't read this power of a function")),
                        }
                        self.i = at + 1;
                    }
                }
                let wrap = |e: Expr| match power {
                    Some(n) => expr::pow(e, Expr::Num(n)),
                    None => e,
                };
                let arg = if self.peek() == Some(&Tok::Op('(')) {
                    self.postfix()?
                } else {
                    let mut a = self.power()?;
                    while self.starts_primary() && !matches!(self.peek(), Some(Tok::Func(_) | Tok::Num(_) | Tok::Log | Tok::LogBase)) {
                        let r = self.power()?;
                        a = join_mul(a, r);
                    }
                    a
                };
                Ok(wrap(expr::func(f, arg)))
            }
            // log_2 8, log_2(x), log base 2 of 8, log(100) (base 10)
            Tok::Log | Tok::LogBase => {
                let base = if t.tok == Tok::LogBase {
                    self.primary()?
                } else if self.peek() == Some(&Tok::Op('_')) {
                    self.i += 1;
                    self.primary()?
                } else {
                    expr::num(10)
                };
                let arg = if self.peek() == Some(&Tok::Op('(')) {
                    self.postfix()?
                } else {
                    let mut a = self.power()?;
                    // log_3 1/9: a written fraction is the argument
                    if let (Expr::Num(_), Some(Tok::Op('/')), Some(Tok::Num(d))) = (&a, self.peek(), self.ts.get(self.i + 1).map(|t| &t.tok)) {
                        a = expr::div(a.clone(), Expr::Num(*d));
                        self.i += 2;
                    }
                    while self.starts_primary() && !matches!(self.peek(), Some(Tok::Func(_) | Tok::Num(_) | Tok::Log | Tok::LogBase)) {
                        let r = self.power()?;
                        a = join_mul(a, r);
                    }
                    a
                };
                Ok(Expr::Log(Box::new(base), Box::new(arg)))
            }
            // |2x - 3|
            Tok::Op('|') => {
                self.bars += 1;
                let e = self.sum()?;
                self.bars -= 1;
                if self.peek() != Some(&Tok::Op('|')) {
                    return Err(self.err("missing closing |"));
                }
                self.i += 1;
                Ok(expr::func(Func::Abs, e))
            }
            _ => {
                self.i -= 1;
                Err(self.err("expected a number or a letter"))
            }
        }
    }
}

fn join_mul(a: Expr, b: Expr) -> Expr {
    let mut v = match a {
        Expr::Mul(v) => v,
        a => vec![a],
    };
    v.push(b);
    Expr::Mul(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::print::{self, Style};

    fn show(s: &str) -> String {
        let r = parse(s, &ParseOptions::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"));
        format!("{} | {} | {}", r.task.value.key(), print::math(&r.problem.value, Style::Ascii), r.var.value)
    }

    #[test]
    fn sentences_become_requests() {
        assert_eq!(show("solve 2x + 3 = 7"), "solve | 2x + 3 = 7 | x");
        assert_eq!(show("what is 15% of 80"), "evaluate | (15/100) * 80 | x");
        assert_eq!(show("differentiate x^3 sin x"), "differentiate | x^3 sin(x) | x");
        assert_eq!(show("expand (x+2)(x-3)"), "expand | (x + 2)(x - 3) | x");
        assert_eq!(show("x squared minus 4 equals 0"), "solve | x^2 - 4 = 0 | x");
        assert_eq!(show("simplify -x^2 + 3x"), "simplify | -x^2 + 3x | x");
        assert_eq!(show("d/dt sin 2t"), "differentiate | sin(2t) | t");
        assert_eq!(show("find x if 7 = 3x - 2"), "solve | 7 = 3x - 2 | x");
    }

    #[test]
    fn parses_algebra() {
        assert_eq!(show("solve 2x + 3 < 7"), "solve | 2x + 3 < 7 | x");
        assert_eq!(show("solve x is at least 4"), "solve | x >= 4 | x");
        assert_eq!(show("solve x + y = 3 and x - y = 1"), "solve | x + y = 3, x - y = 1 | x");
        assert_eq!(show("what is log base 2 of 8"), "evaluate | log_2(8) | x");
        assert_eq!(show("what is log(100)"), "evaluate | log(100) | x");
        assert_eq!(show("solve |2x - 3| = 5"), "solve | |2x - 3| = 5 | x");
        assert_eq!(show("divide x^3 - 1 by x - 1"), "divide | (x^3 - 1)/(x - 1) | x");
        let err = |s: &str| parse(s, &ParseOptions::default()).unwrap_err()[0].to_string();
        assert!(err("solve x^2 + y = 3 and x - y = 1").contains("isn't linear"));
        assert!(err("solve 1 < x < 3").contains("more than one sign"));
    }

    #[test]
    fn refuses_instead_of_guessing() {
        let err = |s: &str| parse(s, &ParseOptions::default()).unwrap_err()[0].to_string();
        assert!(err("solfe 2x = 4").contains("did you mean \"solve\""));
        assert!(err("what is 3 x 4").contains("x is a letter"));
        assert!(err("what is 2x").contains("has no value"));
        assert!(err("solve x^2 - 4").contains("isn't an equation"));
        assert!(err("solve x + y = 3").contains("which letter"));
        assert!(err("determinant of x").contains("matrix"));
        assert!(err("limit of 1/x").contains("needs a point"));
        assert!(err("tangent to y = x^2").contains("needs a point"));
    }

    #[test]
    fn calculus_and_trig_sentences() {
        let calc = |s: &str| {
            let r = parse(s, &ParseOptions::default()).unwrap_or_else(|d| panic!("{s}: {d:?}"));
            let p = r.calc.point.as_ref().map(|p| print::expr(&p.value, Style::Ascii)).unwrap_or_default();
            let b = r.calc.bounds.as_ref().map(|b| format!("{}..{}", print::expr(&b.value.0, Style::Ascii), print::expr(&b.value.1, Style::Ascii))).unwrap_or_default();
            format!("{} | {} | {} | {p} | {b} | {}", r.task.value.key(), print::math(&r.problem.value, Style::Ascii), r.var.value, r.calc.order())
        };
        assert_eq!(calc("integrate x^2 dx"), "integrate | x^2 | x |  |  | 1");
        assert_eq!(calc("integrate x^2 from 0 to 3"), "integrate | x^2 | x |  | 0..3 | 1");
        assert_eq!(calc("limit of (x^2 - 4)/(x - 2) as x approaches 2"), "limit | (x^2 - 4)/(x - 2) | x | 2 |  | 1");
        assert_eq!(calc("lim x->0 sin x / x"), "limit | sin(x)/x | x | 0 |  | 1");
        assert_eq!(calc("limit as x approaches infinity of 1/x"), "limit | 1/x | x | infinity |  | 1");
        assert_eq!(calc("second derivative of x^4"), "differentiate | x^4 | x |  |  | 2");
        assert_eq!(calc("tangent to y = x^2 at x = 1"), "tangent | y = x^2 | x | 1 |  | 1");
        assert_eq!(calc("what is cos(45 degrees)"), "evaluate | cos(45 deg) | x |  |  | 1");
        assert_eq!(calc("simplify sin^2 x + cos^2 x"), "simplify | sin(x)^2 + cos(x)^2 | x |  |  | 1");
        assert_eq!(calc("solve sin x = 1/2 for x between 0 and 2pi"), "solve | sin(x) = 1/2 | x |  | 0..2pi | 1");
    }
}
