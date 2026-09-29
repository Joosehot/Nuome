//! The word grammar for problems that are phrased rather than written:
//! "gcd of 48 and 18", "the 20th term of 3, 7, 11", "increase 80 by 15%",
//! "1000 invested at 5% per year for 10 years". Each construction becomes
//! one finished expression (a `Built` token) before the math parser runs,
//! so the rest of the pipeline sees plain math.
//!
//! Conventions a sentence relies on but doesn't say ("a month is 30 days")
//! come from rules.toml and are returned as notes, printed with the answer.

use crate::calls::Named;
use crate::config::Config;
use crate::expr::{self, Expr};
use crate::lexicon::{Period, Tok, Token};
use crate::model::Said;
use crate::parser::{parse_expr, Diag};
use crate::q::Q;

fn is_math(t: &Tok) -> bool {
    matches!(t, Tok::Num(_) | Tok::Var(_) | Tok::Const(_) | Tok::Func(_) | Tok::Op(_) | Tok::Squared | Tok::Cubed | Tok::Percent | Tok::Infix(_) | Tok::Bang | Tok::Built(_))
}

fn words(ts: &[Token]) -> String {
    ts.iter().map(|t| t.words.as_str()).collect::<Vec<_>>().join(" ")
}

fn built(e: Expr, ts: &[Token]) -> Token {
    Token { tok: Tok::Built(e), words: words(ts) }
}

/// A run of math tokens starting at `i`: (the expression, index after it).
fn span(ts: &[Token], i: usize) -> Result<(Expr, usize), Diag> {
    let mut j = i;
    while j < ts.len() && is_math(&ts[j].tok) {
        j += 1;
    }
    if j == i {
        let at = ts.get(i).map_or("the end".to_string(), |t| format!("\"{}\"", t.words));
        return Err(Diag::new(format!("expected a number at {at}")));
    }
    Ok((parse_expr(&ts[i..j])?, j))
}

/// A list "3, 7, 11 and 15" (a trailing "..." allowed): (items, index after).
fn list(ts: &[Token], mut i: usize) -> Result<(Vec<Expr>, usize), Diag> {
    let mut items = Vec::new();
    loop {
        let (e, j) = span(ts, i)?;
        items.push(e);
        i = j;
        // separators, then another item?
        let mut k = i;
        while k < ts.len() && ts[k].tok == Tok::Sep && (ts[k].words == "," || ts[k].words == "and" || ts[k].words == ".") {
            k += 1;
        }
        if k > i && k < ts.len() && is_math(&ts[k].tok) {
            i = k;
            continue;
        }
        // swallow a trailing "..." or ", ..."
        while i < ts.len() && ts[i].tok == Tok::Sep && (ts[i].words == "," || ts[i].words == ".") {
            i += 1;
        }
        return Ok((items, i));
    }
}

fn skip_filler(ts: &[Token], mut i: usize) -> usize {
    while i < ts.len() && ts[i].tok == Tok::Filler {
        i += 1;
    }
    i
}

/// Rewrite phrased constructions into `Built` tokens. Returns the new
/// tokens and any conventions the answer relies on.
pub fn rewrite(ts: Vec<Token>, cfg: &Config) -> Result<(Vec<Token>, Vec<Said<String>>), Diag> {
    // "compound ...", or a rate with a period: "5% per year", "10% a day"
    let rate_with_period = ts.windows(3).any(|w| w[0].tok == Tok::Percent && matches!(w[1].tok, Tok::Per | Tok::Filler | Tok::Period(_) | Tok::Every(_)) && matches!(w[2].tok, Tok::Period(_) | Tok::Every(_)))
        || ts.windows(2).any(|w| w[0].tok == Tok::Percent && matches!(w[1].tok, Tok::Period(_) | Tok::Every(_)));
    if ts.iter().any(|t| t.tok == Tok::Grow) || rate_with_period {
        return growth(ts, cfg);
    }
    let ts = series(ts)?;
    let ts = divisibility(ts)?;
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < ts.len() {
        let t = &ts[i];
        match &t.tok {
            Tok::List(f) => {
                let (items, j) = list(&ts, skip_filler(&ts, i + 1))?;
                let min = if *f == Named::Factorial { 1 } else { 2 };
                if items.len() < min || (*f == Named::Factorial && items.len() != 1) {
                    return Err(Diag::new(format!("\"{}\" needs {}", t.words, if *f == Named::Factorial { "one number" } else { "at least two numbers" })));
                }
                out.push(built(Expr::Call(*f, items), &ts[i..j]));
                i = j;
            }
            Tok::SumOf => {
                let k = skip_filler(&ts, i + 1);
                match ts.get(k).map(|t| &t.tok) {
                    // sum of the first 20 terms of 3, 7, 11
                    Some(Tok::First) => {
                        let (n, j) = span(&ts, k + 1)?;
                        let j = skip_filler(&ts, j);
                        if ts.get(j).map(|t| &t.tok) != Some(&Tok::TermsOf) {
                            return Err(Diag::new("\"sum of the first N\" needs \"terms of\" and the sequence").hint("e.g. \"sum of the first 20 terms of 3, 7, 11\""));
                        }
                        let (items, end) = list(&ts, skip_filler(&ts, j + 1))?;
                        let mut args = vec![n];
                        args.extend(items);
                        out.push(built(Expr::Call(Named::SeriesSum, args), &ts[i..end]));
                        i = end;
                    }
                    // sum of the numbers from 1 to 100 / sum of 1 to 100
                    _ => {
                        let k = if ts.get(k).map(|t| &t.tok) == Some(&Tok::From) { k + 1 } else { k };
                        let (a, j) = span(&ts, k)?;
                        if ts.get(j).map(|t| &t.tok) == Some(&Tok::To) {
                            let (b, end) = span(&ts, j + 1)?;
                            out.push(built(Expr::Call(Named::SumTo, vec![a, b]), &ts[i..end]));
                            i = end;
                        } else {
                            // sum of 3, 5 and 7: an ordinary sum
                            let (items, end) = list(&ts, k)?;
                            out.push(built(expr::add(items), &ts[i..end]));
                            i = end;
                        }
                    }
                }
            }
            Tok::Ordinal(n) => {
                let k = skip_filler(&ts, i + 1);
                if ts.get(k).map(|t| &t.tok) != Some(&Tok::TermOf) {
                    return Err(Diag::new(format!("\"{}\": the {} what?", t.words, t.words)).hint("e.g. \"the 20th term of 3, 7, 11\""));
                }
                let (items, end) = list(&ts, skip_filler(&ts, k + 1))?;
                let mut args = vec![expr::num(*n)];
                args.extend(items);
                out.push(built(Expr::Call(Named::NthTerm, args), &ts[i..end]));
                i = end;
            }
            Tok::InfSum => {
                let (items, end) = list(&ts, skip_filler(&ts, i + 1))?;
                out.push(built(Expr::Call(Named::InfiniteSum, items), &ts[i..end]));
                i = end;
            }
            // combinations of 3 from 10
            Tok::Pick(f) => {
                let (k, j) = span(&ts, skip_filler(&ts, i + 1))?;
                if ts.get(j).map(|t| &t.tok) != Some(&Tok::From) {
                    return Err(Diag::new(format!("\"{}\" needs \"from\"", t.words)).hint("e.g. \"combinations of 3 from 10\""));
                }
                let (n, end) = span(&ts, j + 1)?;
                out.push(built(Expr::Call(*f, vec![n, k]), &ts[i..end]));
                i = end;
            }
            // remainder when 17 is divided by 5
            Tok::Remainder => {
                // "17 is divided by 5": the filler "is" sits inside the division
                let mut end = i + 1;
                while end < ts.len() && (is_math(&ts[end].tok) || ts[end].tok == Tok::Filler) {
                    end += 1;
                }
                let inner: Vec<Token> = ts[i + 1..end].iter().filter(|t| t.tok != Tok::Filler).cloned().collect();
                // split at the division itself (17/5 would otherwise read as the number 17/5)
                let Some(k) = inner.iter().position(|t| t.tok == Tok::Op('/')) else {
                    return Err(Diag::new("\"remainder\" needs a division").hint("e.g. \"the remainder when 17 is divided by 5\""));
                };
                let (a, b) = (parse_expr(&inner[..k])?, parse_expr(&inner[k + 1..])?);
                out.push(built(Expr::Call(Named::Mod, vec![a, b]), &ts[i..end]));
                i = end;
            }
            // increase 80 by 15%
            Tok::Change(sign) => {
                let (a, j) = span(&ts, skip_filler(&ts, i + 1))?;
                if ts.get(j).map(|t| &t.tok) != Some(&Tok::By) {
                    return Err(Diag::new(format!("\"{}\" needs \"by\" and a percentage", t.words)).hint("e.g. \"increase 80 by 15%\""));
                }
                let (p, end) = percent(&ts, j + 1)?;
                let p = if *sign < 0 { expr::q(p.neg()) } else { expr::q(p) };
                out.push(built(Expr::Call(Named::Raise, vec![a, p]), &ts[i..end]));
                i = end;
            }
            // what percent of 80 is 12
            Tok::WhatPct => {
                let (whole, j) = span(&ts, skip_filler(&ts, i + 1))?;
                let (part, end) = span(&ts, skip_filler(&ts, j))?;
                out.push(built(Expr::Call(Named::WhatPercent, vec![part, whole]), &ts[i..end]));
                i = end;
            }
            // percentage change from 80 to 100
            Tok::PctChange => {
                let (a, j) = span(&ts, i + 1)?;
                if ts.get(j).map(|t| &t.tok) != Some(&Tok::To) {
                    return Err(Diag::new("a change needs \"from ... to ...\"").hint("e.g. \"percentage change from 80 to 100\""));
                }
                let (b, end) = span(&ts, j + 1)?;
                out.push(built(Expr::Call(Named::PercentChange, vec![a, b]), &ts[i..end]));
                i = end;
            }
            _ => {
                out.push(t.clone());
                i += 1;
            }
        }
        // 12 is what percent of 80: the part came just before
        if i < ts.len() && ts[i].tok == Tok::IsWhatPct {
            let mut start = out.len();
            while start > 0 && is_math(&out[start - 1].tok) {
                start -= 1;
            }
            if start == out.len() {
                return Err(Diag::new(format!("\"{}\": what number?", ts[i].words)).hint("e.g. \"12 is what percent of 80\""));
            }
            let part_toks: Vec<Token> = out.drain(start..).collect();
            let part = parse_expr(&part_toks)?;
            let (whole, end) = span(&ts, i + 1)?;
            let mut all = part_toks.clone();
            all.extend(ts[i..end].iter().cloned());
            out.push(built(Expr::Call(Named::WhatPercent, vec![part, whole]), &all));
            i = end;
        }
    }
    Ok((out, vec![]))
}

/// "15%" -> 15 (the number of percent).
fn percent(ts: &[Token], i: usize) -> Result<(Q, usize), Diag> {
    match (ts.get(i).map(|t| &t.tok), ts.get(i + 1).map(|t| &t.tok)) {
        (Some(Tok::Num(q)), Some(Tok::Percent)) => Ok((*q, i + 2)),
        _ => Err(Diag::new("expected a percentage like 15%")),
    }
}

/// How many `to` periods one `from` period is, and the note that says so.
fn periods_per(from: Period, to: Period, cfg: &Config) -> Option<(Q, Option<String>)> {
    if from == to {
        return Some((Q::ONE, None));
    }
    let f = &cfg.finance;
    let months = |p: Period| match p {
        Period::Month => Some(Q::ONE),
        Period::Year => Some(Q::int(f.months_per_year as i128)),
        _ => None,
    };
    if let (Some(a), Some(b)) = (months(from), months(to)) {
        return Some((a.div(&b)?, Some(format!("A year is {} months.", f.months_per_year))));
    }
    let days = |p: Period| match p {
        Period::Day => Q::ONE,
        Period::Week => Q::int(f.days_per_week as i128),
        Period::Month => Q::int(f.days_per_month as i128),
        Period::Year => Q::int(f.days_per_year as i128),
    };
    let note = [from, to]
        .iter()
        .filter(|p| **p != Period::Day)
        .map(|p| format!("a {} is taken as {} days", p.key(), days(*p)))
        .collect::<Vec<_>>()
        .join(", ");
    Some((days(from).div(&days(to))?, Some(format!("{}{} ([finance] in rules.toml).", note[..1].to_uppercase(), &note[1..]))))
}

/// "calculate the compounding at 10% per day for a full month",
/// "1000 invested at 5% per year for 10 years compounded monthly".
fn growth(ts: Vec<Token>, cfg: &Config) -> Result<(Vec<Token>, Vec<Said<String>>), Diag> {
    let mut used = vec![false; ts.len()];
    let mut notes = Vec::new();
    // the rate: a number with %
    let Some(ri) = (0..ts.len().saturating_sub(1)).find(|&k| matches!(ts[k].tok, Tok::Num(_)) && ts[k + 1].tok == Tok::Percent) else {
        return Err(Diag::new("a growth problem needs a rate").hint("e.g. \"1000 at 5% per year for 10 years\""));
    };
    let Tok::Num(rate) = ts[ri].tok else { unreachable!() };
    used[ri] = true;
    used[ri + 1] = true;
    // the duration: "for 10 years", "for a month"
    let fi = ts.iter().position(|t| t.tok == Tok::For);
    let mut duration: Option<(Q, Period, String)> = None;
    if let Some(fi) = fi {
        used[fi] = true;
        let mut k = fi + 1;
        let mut count = Q::ONE;
        while k < ts.len() && ts[k].tok == Tok::Filler {
            used[k] = true;
            k += 1;
        }
        if let Some(Tok::Num(q)) = ts.get(k).map(|t| &t.tok) {
            count = *q;
            used[k] = true;
            k += 1;
        }
        match ts.get(k).map(|t| &t.tok) {
            Some(Tok::Period(p)) => {
                used[k] = true;
                duration = Some((count, *p, words(&ts[fi..=k])));
            }
            _ => return Err(Diag::new("for how long?").hint("say it with a unit: \"for 10 years\", \"for a month\"")),
        }
    }
    let Some((count, dur_unit, dur_words)) = duration else {
        return Err(Diag::new("a growth problem needs a duration").hint("e.g. \"for 10 years\", \"for a month\""));
    };
    // the rate's period: "per day", "a year", "daily" right after the rate
    let mut rate_unit = None;
    for k in ri + 2..ts.len() {
        match &ts[k].tok {
            Tok::Per | Tok::Filler => continue,
            Tok::Period(p) | Tok::Every(p) if !used[k] => {
                rate_unit = Some(*p);
                used[k] = true;
                if k > 0 && ts[k - 1].tok == Tok::Per {
                    used[k - 1] = true;
                }
                break;
            }
            _ => break,
        }
    }
    // compounding frequency: "compounded monthly"
    let every = (0..ts.len()).find(|&k| !used[k] && matches!(ts[k].tok, Tok::Every(_)));
    let every = every.map(|k| {
        used[k] = true;
        let Tok::Every(p) = ts[k].tok else { unreachable!() };
        p
    });
    let rate_unit = rate_unit.unwrap_or_else(|| {
        notes.push(Said::new(format!("The rate is taken per {}, the unit of the duration.", dur_unit.key()), "(no unit on the rate)".to_string()));
        dur_unit
    });
    // the principal: the first other number, else 1 (the growth factor)
    let pi = (0..ts.len()).find(|&k| !used[k] && matches!(ts[k].tok, Tok::Num(_)));
    let principal = match pi {
        Some(k) => {
            used[k] = true;
            let Tok::Num(q) = ts[k].tok else { unreachable!() };
            q
        }
        None => {
            notes.push(Said::new("No starting amount is given, so this is the growth of 1.".to_string(), "(no amount)".to_string()));
            Q::ONE
        }
    };
    // periods: compound in `every` units if given, else in the rate's units
    let step = every.unwrap_or(rate_unit);
    let Some(r) = rate.div(&Q::int(100)) else { return Err(Diag::new("rate too large")) };
    let (per_rate, n) = {
        let Some((k, note)) = periods_per(rate_unit, step, cfg) else { return Err(Diag::new("can't convert those periods")) };
        // a nominal yearly rate compounded monthly: 12 compoundings share the rate
        let per = r.div(&k).ok_or_else(|| Diag::new("rate too large"))?;
        if let Some(n) = note {
            notes.push(Said::new(n, dur_words.clone()));
        }
        let Some((d, note)) = periods_per(dur_unit, step, cfg) else { return Err(Diag::new("can't convert those periods")) };
        if let Some(n) = note {
            if !notes.iter().any(|x| x.value == n) {
                notes.push(Said::new(n, dur_words.clone()));
            }
        }
        let n = count.mul(&d).ok_or_else(|| Diag::new("too many periods"))?;
        (per, n)
    };
    if !n.is_int() || n.is_neg() {
        return Err(Diag::new(format!("{dur_words} is {n} {}s: not a whole number of compounding periods", step.key())).hint(format!("give the duration in {}s", step.key())));
    }
    let call = Expr::Call(Named::Compound, vec![expr::q(principal), expr::q(per_rate), expr::q(n)]);
    let mut out: Vec<Token> = Vec::new();
    let mut placed = false;
    for (k, t) in ts.iter().enumerate() {
        if used[k] || matches!(t.tok, Tok::Grow | Tok::Per | Tok::Period(_) | Tok::Every(_) | Tok::When) {
            if !placed {
                out.push(Token { tok: Tok::Built(call.clone()), words: words(&ts) });
                placed = true;
            }
            continue;
        }
        out.push(t.clone());
    }
    Ok((out, notes))
}

// ---- proofs (main) ---------------------------------------------------------------

fn dot(t: &Token) -> bool {
    t.tok == Tok::Sep && t.words == "."
}

/// "1 + 2 + ... + n", "1^2 + 2^2 + ... + n^2": the last term, written in the
/// letter, is the pattern; the first terms must follow it (checked here, so
/// a sum whose pattern Nuome would have to guess is refused).
fn series(ts: Vec<Token>) -> Result<Vec<Token>, Diag> {
    let Some(k) = (1..ts.len().saturating_sub(3)).find(|&k| dot(&ts[k]) && dot(&ts[k + 1]) && dot(&ts[k + 2]) && ts[k - 1].tok == Tok::Op('+') && ts.get(k + 3).map(|t| &t.tok) == Some(&Tok::Op('+'))) else {
        return Ok(ts);
    };
    let mut start = k - 1;
    while start > 0 && is_math(&ts[start - 1].tok) && ts[start - 1].tok != Tok::Op('=') {
        start -= 1;
    }
    let mut end = k + 4;
    while end < ts.len() && is_math(&ts[end].tok) && ts[end].tok != Tok::Op('=') && !matches!(ts[end].tok, Tok::Rel(_)) {
        end += 1;
    }
    let first = parse_expr(&ts[start..k - 1])?;
    let last = parse_expr(&ts[k + 4..end])?;
    let letters = last.vars();
    let Some(n) = letters.iter().next().filter(|_| letters.len() == 1).cloned() else {
        return Err(Diag::new("the last term of a \"...\" sum must be written in one letter").hint("e.g. \"1 + 2 + ... + n\""));
    };
    let firsts: Vec<Q> = crate::expr::terms(&first).iter().map(|t| t.eval_q(&|_| None)).collect::<Option<_>>().ok_or_else(|| Diag::new("the first terms of a \"...\" sum must be numbers"))?;
    let term = |i: i128| last.subst(&n, &Expr::Num(Q::int(i))).eval_q(&|_| None);
    let from = [1i128, 0].into_iter().find(|&s| firsts.iter().enumerate().all(|(i, q)| term(s + i as i128) == Some(*q)));
    let Some(from) = from else {
        let shown = crate::print::expr(&last, crate::print::Style::Ascii);
        return Err(Diag::new(format!("the first terms don't follow the pattern {shown}")).hint(format!("with {n} = 1, 2, ... the pattern gives {}, {}, ...", term(1).map_or("?".into(), |q| q.to_string()), term(2).map_or("?".into(), |q| q.to_string()))));
    };
    let call = Expr::Call(Named::Series, vec![last, expr::num(from), Expr::Var(n)]);
    let mut out: Vec<Token> = ts[..start].to_vec();
    out.push(built(call, &ts[start..end]));
    out.extend(ts[end..].iter().cloned());
    Ok(out)
}

/// "6 divides n^3 - n", "n^3 - n is divisible by 6", "n^2 + n is even".
fn divisibility(ts: Vec<Token>) -> Result<Vec<Token>, Diag> {
    let Some(k) = ts.iter().position(|t| matches!(t.tok, Tok::DividesW | Tok::DivisibleBy | Tok::Even)) else { return Ok(ts) };
    let mut start = k;
    while start > 0 && is_math(&ts[start - 1].tok) {
        start -= 1;
    }
    if start == k {
        return Err(Diag::new(format!("\"{}\": what is?", ts[k].words)).hint("e.g. \"prove n^3 - n is divisible by 6\""));
    }
    let before = parse_expr(&ts[start..k])?;
    let (d, e, end) = match ts[k].tok {
        Tok::Even => (expr::num(2), before, k + 1),
        _ => {
            let (after, end) = span(&ts, k + 1)?;
            if ts[k].tok == Tok::DividesW {
                (before, after, end)
            } else {
                (after, before, end)
            }
        }
    };
    if !d.as_num().is_some_and(|q| q.is_int() && q.num() > 1) {
        return Err(Diag::new("divisibility by a whole number greater than 1 only").hint("e.g. \"6 divides n^3 - n\""));
    }
    let mut out: Vec<Token> = ts[..start].to_vec();
    out.push(built(Expr::Call(Named::Divides, vec![d, e]), &ts[start..end]));
    out.extend(ts[end..].iter().cloned());
    Ok(out)
}
