//! Reading a statement about a group or a ring: hypotheses after "if",
//! the claim after "then", "for all a" after a hypothesis. Products keep
//! the brackets as written; abc is (ab)c.

use super::term::{self, Term};
use super::Kind;
use crate::lexicon::{self, Tok, Token};
use crate::parser::Diag;

/// One equation of a statement; `all` are the letters it holds for every
/// value of ("aa = e for all a").
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Eqn {
    pub l: Term,
    pub r: Term,
    pub all: Vec<String>,
    pub words: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Statement {
    pub hyps: Vec<Eqn>,
    pub goals: Vec<Eqn>,
}

/// What else the sentence said: the task and profile words.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    pub task: Option<Token>,
    pub modifiers: Vec<Token>,
}

fn words(ts: &[Token]) -> String {
    ts.iter().map(|t| t.words.as_str()).collect::<Vec<_>>().join(" ")
}

fn letter_word(t: &Token) -> Option<String> {
    match &t.tok {
        Tok::Var(v) => Some(v.clone()),
        Tok::Filler if t.words == "a" => Some("a".into()),
        _ => None,
    }
}

fn is_term(t: &Tok) -> bool {
    matches!(t, Tok::Var(_) | Tok::Num(_) | Tok::Const(crate::expr::Konst::E) | Tok::Squared | Tok::Cubed | Tok::InverseOf) || matches!(t, Tok::Op(c) if "+-*^()=".contains(*c))
}

/// Replace every named statement ("the group is abelian") by its math.
pub fn splice(toks: &[Token]) -> Result<Vec<Token>, Diag> {
    let mut out = Vec::new();
    for t in toks {
        match &t.tok {
            Tok::Statement(_, text) => {
                let inner = lexicon::lex(text).map_err(Diag::new)?;
                out.extend(inner);
            }
            _ => out.push(t.clone()),
        }
    }
    Ok(out)
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Hyp,
    Goal,
}

/// Split a sentence (statements already spliced) into hypotheses, claims
/// and the extra words.
pub fn read(toks: &[Token], kind: Kind) -> Result<(Statement, Extras), Vec<Diag>> {
    let mut diags = Vec::new();
    let mut ex = Extras::default();
    let mut spans: Vec<(Role, Vec<Token>, Vec<String>)> = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    let mut role = Role::Goal;
    let flush = |cur: &mut Vec<Token>, role: Role, spans: &mut Vec<(Role, Vec<Token>, Vec<String>)>| {
        if !cur.is_empty() {
            spans.push((role, std::mem::take(cur), Vec::new()));
        }
    };
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        let prev_math = cur.last().is_some();
        let next_math = toks.get(i + 1).is_some_and(|n| is_term(&n.tok));
        // "a" is an article unless it sits in math; "is" between two sides is "="
        let term_word = match &t.tok {
            Tok::Filler if t.words == "a" => prev_math || next_math,
            Tok::Filler if t.words == "is" => prev_math && (next_math || toks.get(i + 1).is_some_and(|n| letter_word(n).is_some())),
            tok => is_term(tok),
        };
        if term_word {
            let mut t = t.clone();
            if t.tok == Tok::Filler && t.words == "is" {
                t.tok = Tok::Op('=');
            } else if t.tok == Tok::Filler {
                t.tok = Tok::Var("a".into());
            }
            cur.push(t);
            i += 1;
            continue;
        }
        flush(&mut cur, role, &mut spans);
        match &t.tok {
            Tok::When => role = Role::Hyp,
            Tok::Sep if t.words == "then" => role = Role::Goal,
            Tok::Sep | Tok::Filler | Tok::Is | Tok::Structure(_) | Tok::Of => {}
            Tok::Task(crate::model::Task::Prove) => {
                ex.task.get_or_insert(t.clone());
                role = Role::Goal;
            }
            Tok::Mod(_) => ex.modifiers.push(t.clone()),
            // "for all a", "for every a and b"
            Tok::For if toks.get(i + 1).is_some_and(|n| (n.tok == Tok::Filler && n.words == "all") || matches!(n.tok, Tok::Per)) => {
                let mut j = i + 2;
                let mut vars = Vec::new();
                while let Some(v) = toks.get(j).and_then(letter_word) {
                    vars.push(v);
                    j += 1;
                    if toks.get(j).is_some_and(|s| s.tok == Tok::Sep && (s.words == "and" || s.words == ",")) && toks.get(j + 1).and_then(letter_word).is_some() && !toks.get(j + 2).is_some_and(|n| is_term(&n.tok)) {
                        j += 1;
                    } else {
                        break;
                    }
                }
                match spans.last_mut() {
                    Some(last) if !vars.is_empty() => last.2.extend(vars),
                    _ => diags.push(Diag::new(format!("\"{}\" needs an equation before it and a letter after it", words(&toks[i..j.min(toks.len())]))).hint("e.g. \"if a^2 = e for all a then ab = ba\"")),
                }
                i = j;
                continue;
            }
            Tok::Unknown => {
                let mut d = Diag::new(format!("unknown word \"{}\"", t.words));
                d.hint = Some(match lexicon::suggest(&t.words) {
                    Some(h) => format!("{h} (--vocabulary lists every word)"),
                    None => "--vocabulary lists every word".into(),
                });
                diags.push(d);
            }
            Tok::Task(_) => diags.push(Diag::new(format!("\"{}\": in a {}, Nuome proves statements", t.words, kind.name())).hint(format!("say \"prove\", e.g. \"in a {}, prove {}\"", kind.name(), kind.example()))),
            _ => diags.push(Diag::new(format!("\"{}\" doesn't fit a statement about a {}", t.words, kind.name())).hint(format!("e.g. \"in a {}, prove {}\"", kind.name(), kind.example()))),
        }
        i += 1;
    }
    flush(&mut cur, role, &mut spans);
    let mut st = Statement::default();
    for (role, span, all) in spans {
        match equation(&span, kind) {
            Ok(mut e) => {
                e.all = all;
                match role {
                    Role::Hyp => st.hyps.push(e),
                    Role::Goal => st.goals.push(e),
                }
            }
            Err(d) => diags.push(d),
        }
    }
    // "if ab = ac, b = c": with no "then", the last hypothesis is the claim
    if st.goals.is_empty() && st.hyps.len() > 1 {
        let g = st.hyps.pop().expect("two hypotheses");
        st.goals.push(g);
    }
    if diags.is_empty() {
        Ok((st, ex))
    } else {
        Err(diags)
    }
}

/// One equation: two terms and "=".
pub fn equation(ts: &[Token], kind: Kind) -> Result<Eqn, Diag> {
    let eqs: Vec<usize> = ts.iter().enumerate().filter(|(_, t)| t.tok == Tok::Op('=')).map(|(i, _)| i).collect();
    let w = words(ts);
    let [k] = eqs[..] else {
        return Err(Diag::new(format!("\"{w}\" isn't one equation")).hint(format!("state it as two sides and one \"=\", e.g. \"{}\"", kind.example())));
    };
    let (l, r) = (&ts[..k], &ts[k + 1..]);
    if l.is_empty() || r.is_empty() {
        return Err(Diag::new(format!("\"{w}\": one side of = is empty")));
    }
    Ok(Eqn { l: term_of(l, kind)?, r: term_of(r, kind)?, all: vec![], words: w })
}

pub fn term_of(ts: &[Token], kind: Kind) -> Result<Term, Diag> {
    let mut p = P { ts, i: 0, kind };
    let t = p.sum()?;
    if p.i < ts.len() {
        return Err(p.err("didn't expect this"));
    }
    Ok(t)
}

struct P<'a> {
    ts: &'a [Token],
    i: usize,
    kind: Kind,
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
    fn ring(&self) -> bool {
        self.kind == Kind::Ring
    }
    fn sum(&mut self) -> Result<Term, Diag> {
        let mut acc = if self.peek() == Some(&Tok::Op('-')) {
            self.need_ring("-")?;
            self.i += 1;
            term::neg(self.product()?)
        } else {
            self.product()?
        };
        loop {
            match self.peek() {
                Some(Tok::Op('+')) => {
                    self.need_ring("+")?;
                    self.i += 1;
                    let r = self.product()?;
                    acc = term::add(acc, r);
                }
                Some(Tok::Op('-')) => {
                    self.need_ring("-")?;
                    self.i += 1;
                    let r = self.product()?;
                    acc = term::add(acc, term::neg(r));
                }
                _ => return Ok(acc),
            }
        }
    }
    fn need_ring(&self, op: &str) -> Result<(), Diag> {
        if self.ring() {
            Ok(())
        } else {
            Err(self.err(&format!("a {} has one operation, so \"{op}\" means nothing", self.kind.name())).hint("write products as ab, inverses as a^-1 and the identity as e"))
        }
    }
    fn starts_factor(&self) -> bool {
        matches!(self.peek(), Some(Tok::Var(_) | Tok::Num(_) | Tok::Const(_) | Tok::Op('(') | Tok::InverseOf)) || self.ts.get(self.i).is_some_and(|t| letter_word(t).is_some())
    }
    fn product(&mut self) -> Result<Term, Diag> {
        let mut acc = self.power()?;
        loop {
            if self.peek() == Some(&Tok::Op('*')) {
                self.i += 1;
            } else if !self.starts_factor() {
                return Ok(acc);
            }
            let r = self.power()?;
            acc = term::mul(acc, r);
        }
    }
    fn power(&mut self) -> Result<Term, Diag> {
        let mut base = self.primary()?;
        loop {
            let n: i128 = match self.peek() {
                Some(Tok::Squared) => {
                    self.i += 1;
                    2
                }
                Some(Tok::Cubed) => {
                    self.i += 1;
                    3
                }
                Some(Tok::Op('^')) => {
                    self.i += 1;
                    let neg = self.peek() == Some(&Tok::Op('-'));
                    if neg {
                        self.i += 1;
                    }
                    let Some(Tok::Num(q)) = self.peek() else { return Err(self.err("expected a whole number as the power")) };
                    if !q.is_int() || q.num() > 4 || q.num() < 1 {
                        return Err(self.err("powers from 1 to 4 (or -1 for the inverse) only"));
                    }
                    let q = q.num();
                    self.i += 1;
                    if neg {
                        -q
                    } else {
                        q
                    }
                }
                _ => return Ok(base),
            };
            if n < 0 && self.ring() {
                return Err(self.err("a ring has no multiplicative inverses in Nuome's axioms"));
            }
            // a^3 = (aa)a, a^-2 = (aa)^-1
            let mut p = base.clone();
            for _ in 1..n.abs() {
                p = term::mul(p, base.clone());
            }
            base = if n < 0 { term::inv(p) } else { p };
        }
    }
    fn primary(&mut self) -> Result<Term, Diag> {
        let t = self.ts.get(self.i).ok_or_else(|| self.err("expected a letter"))?;
        self.i += 1;
        match &t.tok {
            // "ex" is read as two letters; in a group e is the identity
            Tok::Var(v) if v == "e" && !self.ring() => Ok(Term::E),
            Tok::Var(v) => Ok(term::el(v)),
            // the lexer reads a lone "a" as an article unless math is on both sides
            Tok::Filler if t.words == "a" => Ok(term::el("a")),
            Tok::Const(crate::expr::Konst::E) if !self.ring() => Ok(Term::E),
            Tok::Num(q) if q.is_zero() && self.ring() => Ok(Term::Zero),
            Tok::Num(q) if q.is_one() && !self.ring() => Ok(Term::E),
            Tok::InverseOf if !self.ring() => Ok(term::inv(self.power()?)),
            Tok::Op('(') => {
                let e = self.sum()?;
                if self.peek() != Some(&Tok::Op(')')) {
                    return Err(self.err("missing )"));
                }
                self.i += 1;
                Ok(e)
            }
            _ => {
                self.i -= 1;
                Err(self.err(&format!("expected a letter{}", if self.ring() { " or 0" } else { " or e" })))
            }
        }
    }
}
