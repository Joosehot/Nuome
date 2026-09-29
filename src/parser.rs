//! Tokens -> a structured request. The math itself is a small Pratt parser
//! with the conventions people use on paper: 2x is 2*x, sin 2x is sin(2x),
//! -x^2 is -(x^2), 15% of 80 is 15/100 * 80.

use crate::expr::{self, Expr, Func, Konst, Math};
use crate::lexicon::{self, Tok, Token};
use crate::model::{Request, Said, Task};
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
    matches!(t, Tok::Num(_) | Tok::Var(_) | Tok::Const(_) | Tok::Func(_) | Tok::Op(_) | Tok::Squared | Tok::Cubed | Tok::Percent | Tok::Infix(_) | Tok::Bang | Tok::Built(_))
}

fn words(ts: &[Token]) -> String {
    ts.iter().map(|t| t.words.as_str()).collect::<Vec<_>>().join(" ")
}

pub fn parse(sentence: &str, opts: &ParseOptions) -> Result<Request, Vec<Diag>> {
    parse_with(sentence, opts, &crate::config::Config::builtin())
}

pub fn parse_with(sentence: &str, opts: &ParseOptions, cfg: &crate::config::Config) -> Result<Request, Vec<Diag>> {
    let toks = lexicon::lex(sentence).map_err(|e| vec![Diag::new(e)])?;
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
            Tok::Sep | Tok::Filler | Tok::Is | Tok::To | Tok::Of => {}
            Tok::Unsupported(what) => diags.push(Diag::new(format!("\"{}\": {what} is not in Nuome v0", t.words)).hint("v0 evaluates, simplifies, expands, factors, solves one equation in one unknown, and differentiates")),
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
            Tok::List(_) | Tok::Ordinal(_) | Tok::SumOf | Tok::First | Tok::TermsOf | Tok::TermOf | Tok::InfSum | Tok::From | Tok::Pick(_) | Tok::Remainder | Tok::Grow | Tok::Per | Tok::Period(_) | Tok::Every(_) | Tok::Change(_) | Tok::By | Tok::WhatPct | Tok::IsWhatPct | Tok::PctChange => {
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
        _ => Default::default(),
    };
    let task = task.unwrap_or_else(|| match &problem.value {
        Math::Eq(..) => Said::new(Task::Solve, "(an equation: solve)"),
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
                _ if task.value == Task::Solve || task.value == Task::Differentiate => {
                    let names: Vec<&str> = free.iter().map(|s| s.as_str()).collect();
                    return Err(vec![Diag::new(format!("which letter? {} has {}", problem.words, names.join(", "))).hint(format!("say \"for {}\" or \"with respect to {}\"", names[0], names[0]))]);
                }
                _ => Said::new(free[0].clone(), "(first letter)"),
            }
        }
    };
    let shown = crate::print::math(&problem.value, crate::print::Style::Ascii);
    let math_ok = match (task.value, &problem.value) {
        (Task::Solve, Math::Eq(..)) => Ok(()),
        (Task::Solve, _) => Err(Diag::new(format!("{shown} isn't an equation")).hint(format!("to find where it is zero, write \"solve {shown} = 0\""))),
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
        if matches!(task.value, Task::Factor | Task::Expand | Task::Differentiate) && given.is_empty() {
            return Err(vec![Diag::new(format!("\"{}\": a {} has no single value to round", d.words, task.value.key()))]);
        }
    }
    Ok(Request { sentence: sentence.trim().to_string(), task, problem, var, given, method, modifiers, decimals, notes })
}

/// Parse one math span: an expression, or an equation with one "=".
pub fn parse_math(ts: &[Token]) -> Result<Math, Diag> {
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
    let mut p = P { ts, i: 0 };
    let e = p.sum()?;
    if p.i < ts.len() {
        return Err(Diag::new(format!("didn't expect \"{}\" in \"{}\"", ts[p.i].words, words(ts))));
    }
    Ok(expr::tidy(e))
}

struct P<'a> {
    ts: &'a [Token],
    i: usize,
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
        matches!(self.peek(), Some(Tok::Num(_) | Tok::Var(_) | Tok::Const(_) | Tok::Func(_) | Tok::Op('(') | Tok::Built(_)))
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
                // sin(x); sin 2x = sin(2x); stops before the next function
                let arg = if self.peek() == Some(&Tok::Op('(')) {
                    self.postfix()?
                } else {
                    let mut a = self.power()?;
                    while self.starts_primary() && !matches!(self.peek(), Some(Tok::Func(_) | Tok::Num(_))) {
                        let r = self.power()?;
                        a = join_mul(a, r);
                    }
                    a
                };
                Ok(expr::func(*f, arg))
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
    fn refuses_instead_of_guessing() {
        let err = |s: &str| parse(s, &ParseOptions::default()).unwrap_err()[0].to_string();
        assert!(err("solfe 2x = 4").contains("did you mean \"solve\""));
        assert!(err("what is 3 x 4").contains("x is a letter"));
        assert!(err("what is 2x").contains("has no value"));
        assert!(err("solve x^2 - 4").contains("isn't an equation"));
        assert!(err("solve x + y = 3").contains("which letter"));
        assert!(err("integrate x^2").contains("integration"));
    }
}
