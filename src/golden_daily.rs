//! Nuome Daily's problems: what Golden invents (golden_problem), asked so that the whole answer is known.
//!
//! A problem Nuome did not solve for every n still has a part it did: every n from 3 up to the first one left undecided.
//! So the question asked is "for which n from 3 to B", with B that far, and the answer is exact on all of it: each n
//! built and checked, or impossible because the search ran to its end. What lies above B is said to be open, or closed
//! if the parts ran out. Each seed grows other problems; which day gets which is tools/daily_generate.py's.
//!
//! Whether anyone has asked or settled the same question is not checked, so nothing here calls a problem new.
use crate::golden_problem::{fingerprint, holds, invent_from, Class, Problem, Verdict, A, COMMON_IN, COMMON_OUT, E, OUT_X, OUT_Y, PATHS, SIZE};
use serde_json::{json, Value};

/// shorter than this a question is hardly asked
const LEAST_B: usize = 6;

fn what(class: Class) -> &'static str {
    match class {
        Class::Tournament => "tournament on n players (every two players play once, and one of them wins)",
        _ => "graph on n points (any two points are joined or not)",
    }
}

fn word(class: Class) -> &'static str {
    if class == Class::Tournament { "players" } else { "points" }
}

/// what F = 0 asks, in the order it is written: a product is 0 when a factor is, a difference when its sides are equal
fn zero(e: &E, class: Class) -> String {
    match e {
        E::Mul(a, b) => match (&**a, &**b) {
            (E::Num(k), f) | (f, E::Num(k)) if *k != 0 => zero(f, class),
            _ => format!("{} or {}", zero(a, class), zero(b, class)),
        },
        E::Abs(a) => zero(a, class),
        E::Sub(a, b) => match &**a {
            E::Num(_) => format!("{} = {}", b.show(class), a.show(class)),
            _ => format!("{} = {}", a.show(class), b.show(class)),
        },
        E::Add(a, b) => match (&**a, &**b) {
            (E::Num(k), f) | (f, E::Num(k)) => format!("{} = {}", f.show(class), -k),
            _ => format!("{} = 0", e.show(class)),
        },
        _ => format!("{} = 0", e.show(class)),
    }
}

/// the words of the readings F uses, and no others
fn readings_used(e: &E, class: Class) -> Vec<String> {
    fn walk(e: &E, seen: &mut Vec<u8>) {
        if let E::Read(i) = e {
            if !seen.contains(i) {
                seen.push(*i);
            }
        }
        for k in e.kids() {
            walk(k, seen);
        }
    }
    let mut seen = Vec::new();
    walk(e, &mut seen);
    seen.sort_unstable();
    let t = class == Class::Tournament;
    seen.iter()
        .filter_map(|&r| {
            Some(match r {
                A if t => "beats(x,y) is 1 when x beats y, otherwise 0",
                A => "adj(x,y) is 1 when x and y are joined, otherwise 0",
                OUT_X if t => "out(x) is how many players x beats",
                OUT_X => "deg(x) is how many points x is joined to",
                OUT_Y if t => "out(y) is how many players y beats",
                OUT_Y => "deg(y) is how many points y is joined to",
                COMMON_OUT if t => "both_beat(x,y) is how many players both x and y beat",
                COMMON_OUT => "common(x,y) is how many points are joined to both x and y",
                COMMON_IN => "beat_both(x,y) is how many players beat both x and y",
                PATHS => "paths(x,y) is how many players z have x beating z and z beating y",
                SIZE => "n is the number of points",
                _ => return None,
            })
        })
        .map(String::from)
        .collect()
}

/// every n from 3 to here is decided
fn bound(p: &Problem) -> usize {
    let mut b = 2;
    while p.spectrum.iter().any(|(n, v)| *n == b + 1 && matches!(v, Verdict::Built(_) | Verdict::Impossible(_))) {
        b += 1;
    }
    b
}

fn tree(e: &E) -> Value {
    let two = |o: &str, a: &E, b: &E| json!([o, tree(a), tree(b)]);
    match e {
        E::Num(k) => json!(["num", k]),
        E::Read(i) => json!(["read", i]),
        E::Add(a, b) => two("add", a, b),
        E::Sub(a, b) => two("sub", a, b),
        E::Mul(a, b) => two("mul", a, b),
        E::Min(a, b) => two("min", a, b),
        E::Max(a, b) => two("max", a, b),
        E::If(c, a, b) => json!(["if", tree(c), tree(a), tree(b)]),
        E::Abs(a) => json!(["abs", tree(a)]),
    }
}

fn drawn(rows: &[u32], class: Class) -> String {
    let n = rows.len();
    let mut out = Vec::new();
    for x in 0..n {
        for y in 0..n {
            if rows[x] >> y & 1 == 1 && (class != Class::Graph || x < y) {
                out.push(format!("{x}{}{y}", if class == Class::Graph { "-" } else { ">" }));
            }
        }
    }
    out.join(" ")
}

/// a puzzle of a problem, or none if its decided part asks nothing
fn puzzle(p: &Problem, seed: u64) -> Option<Value> {
    let b = bound(p);
    let here: Vec<&(usize, Verdict)> = p.spectrum.iter().filter(|(n, _)| (3..=b).contains(n)).collect();
    let built: Vec<(usize, &Vec<u32>)> = here.iter().filter_map(|(n, v)| if let Verdict::Built(r) = v { Some((*n, r)) } else { None }).collect();
    let none: Vec<usize> = here.iter().filter(|(_, v)| matches!(v, Verdict::Impossible(_))).map(|x| x.0).collect();
    if b < LEAST_B || built.is_empty() || none.is_empty() {
        return None;
    }
    for (n, rows) in &built {
        assert!(holds(&p.f, p.class, rows), "built at n = {n} and F is not 0 on it: a bug");
    }
    let question = format!("For which n from 3 to {b} is there a {} such that, for every ordered pair (x, y) of different {}: {}?", what(p.class), word(p.class), zero(&p.f, p.class));
    let mut lines = Vec::new();
    for (i, (n, v)) in here.iter().enumerate() {
        lines.push(match v {
            Verdict::Built(rows) => format!("{:>3}.  n = {n}: here is one: {}   [built]", i + 1, drawn(rows, p.class)),
            _ => format!("{:>3}.  n = {n}: none exists; the search ran to its end.   [ran_to_end]", i + 1),
        });
    }
    let step = here.len() + 1;
    lines.push(match p.none_from {
        Some(d) => format!(
            "{step:>3}.  Above {b}: the parts such a {} can hold, grown point by point, run out at {d} points (how many of each size from 1: {}). None on {d} or more.{}   [parts]",
            p.class.name(),
            p.part_sizes.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
            if d > b + 1 { format!(" From {} to {} is left open here.", b + 1, d - 1) } else { String::new() }
        ),
        None => format!("{step:>3}.  Above {b}: not every n is decided. Left open here.   [open]"),
    });
    let answer: Vec<usize> = built.iter().map(|x| x.0).collect();
    let list = answer.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ");
    Some(json!({
        "key": format!("{}-{:016x}", p.class.name(), fingerprint(&p.f, p.class)),
        "seed": seed,
        "question": question,
        "definitions": readings_used(&p.f, p.class),
        "bound": b,
        "answer": answer,
        "impossible": none,
        "steps": lines,
        "answer_line": format!("Answer: n = {list}"),
        "checks": [
            format!("ok built: the condition holds on every ordered pair of each of the {} structures, checked again from scratch", built.len()),
            format!("ok tried again: at {} of the n's every structure was tried one by one, with no search, and the verdict held", p.tried_again()),
        ],
        "class": p.class.name(),
        "tree": tree(&p.f),
        "built": built.iter().map(|(n, r)| (n.to_string(), json!(r))).collect::<serde_json::Map<_, _>>(),
    }))
}

/// `nuome --ideas golden-daily SEED OUT`: the puzzles of one seed, written to the file OUT
pub fn run(seed: u64, out: &str) -> Result<(), String> {
    let kept = invent_from(seed, &mut |l| eprintln!("{l}"))?;
    let all: Vec<Value> = kept.iter().filter_map(|p| puzzle(p, seed)).collect();
    eprintln!("seed {seed}: {} kept, {} can be asked", kept.len(), all.len());
    std::fs::write(out, serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden_problem::Verdict::{Built, Impossible, Undecided};
    use E::*;

    fn p(spectrum: Vec<(usize, Verdict)>) -> Problem {
        Problem { class: Class::Tournament, f: Num(0), spectrum, none_from: None, part_sizes: Vec::new() }
    }

    #[test]
    fn a_product_is_read_factor_by_factor() {
        let minus_one = |r| Add(Box::new(Num(-1)), Box::new(Read(r)));
        let f = Mul(Box::new(minus_one(COMMON_OUT)), Box::new(minus_one(COMMON_IN)));
        assert_eq!(zero(&f, Class::Tournament), "both_beat(x,y) = 1 or beat_both(x,y) = 1");
        let scaled = Mul(Box::new(Num(2)), Box::new(Sub(Box::new(Num(1)), Box::new(Read(COMMON_OUT)))));
        assert_eq!(zero(&scaled, Class::Tournament), "both_beat(x,y) = 1");
    }

    #[test]
    fn only_the_readings_it_uses_are_explained() {
        let f = Sub(Box::new(Read(COMMON_IN)), Box::new(Num(1)));
        let words = readings_used(&f, Class::Tournament);
        assert_eq!(words.len(), 1);
        assert!(words[0].starts_with("beat_both"));
    }

    #[test]
    fn the_bound_stops_at_the_first_undecided() {
        let b = bound(&p(vec![(3, Impossible(0)), (4, Built(vec![])), (5, Undecided(9)), (6, Built(vec![]))]));
        assert_eq!(b, 4);
    }

    #[test]
    fn a_short_decided_part_is_no_puzzle() {
        assert!(puzzle(&p(vec![(3, Impossible(0)), (4, Built(vec![0, 0, 0, 0]))]), 0).is_none());
    }
}
