//! Golden writes problems that could move a question mathematics has left open: a bridge.
//!
//! No problem can be known beforehand to advance anything, but there is a way one can, and it can be checked: F is written
//! so that every object of an open question solves it. Then "impossible" for F at some n says the object does not exist
//! on n points, and what F does allow is the near kin of the object.
//!
//! The bridges, each from a family whose readings are fixed by n and whether x beats (or is joined to) y:
//! - doubly regular tournaments, n = 3 mod 4: out (n-1)/2, both_beat and beat_both (n-3)/4. One on n points exists exactly
//!   when a skew Hadamard matrix of order n + 1 does (Reid and Brown 1972), and whether that is every multiple of 4 is open.
//! - conference graphs, n = 1 mod 4: degree (n-1)/2, common (n-5)/4 when joined, (n-1)/4 when not.
//!
//! Golden starts from the family's own conditions with parts taken away, and may change F only while the family still
//! solves it. The answers come from Nuome (golden_problem's sieve and parts); what is built is checked from scratch. A
//! problem is kept only for its kin: a structure that is neither the family nor the family with one point added that
//! loses or wins everything. Each comes with its symmetries, since exceptional objects tend to have many.
//!
//! Two more sieves come from working the first results by hand. A factor that the degree alone brings to 0 makes any
//! regular object a solution (`empty_condition`, dropped). And kin found on every n of a residue are usually generic, a
//! climb's random regular objects; the ones to look at are sporadic, kin at n with n - 4 and n + 4 impossible.
//!
//! The limit, plainly: every object of these families is known to exist as far as Nuome can search (`known_to`), so no
//! "impossible" can move the open question within reach. What can is a pattern in the kin, as a conjecture for every n.
//! How fruitful a problem is, and whether anyone has asked it, is not measured.
use crate::golden_problem::E::*;
use crate::golden_problem::{child_up_to, decide, fingerprint, grow, holds, look, Class, Problem, Rng, Verdict, COMMON_IN, COMMON_OUT, IN_X, IN_Y, A, B, E, OUT_X, OUT_Y, PATHS, READS, SIZE};
use std::collections::{BTreeMap, BTreeSet};

/// a family of objects whose readings n fixes
pub struct Bridge {
    pub name: &'static str,
    pub class: Class,
    /// n mod 4 where the family can live
    pub residue: usize,
    /// the smallest n it is defined at
    pub least: usize,
    /// up to here every n the family can live at is settled: objects known, or (conference 21) known not to exist
    pub known_to: usize,
    /// what an answer of "impossible" would say, with the known results it rests on
    pub says: &'static str,
    pub sources: &'static [&'static str],
}

pub const BRIDGES: [Bridge; 2] = [
    Bridge {
        name: "doubly regular tournament",
        class: Class::Tournament,
        residue: 3,
        least: 3,
        // Paley's at 3, 7, 11, 19, 23, 27, 31 with doubling: skew Hadamard orders 4 to 32
        known_to: 31,
        says: "no doubly regular tournament on n points, so no skew Hadamard matrix of order n + 1",
        sources: &["Reid and Brown 1972: a doubly regular tournament on n points exists exactly when a skew Hadamard matrix of order n + 1 does", "the skew Hadamard conjecture, open: one exists for every multiple of 4"],
    },
    Bridge {
        name: "conference graph",
        class: Class::Graph,
        residue: 1,
        least: 5,
        known_to: 49,
        says: "no conference graph on n points",
        sources: &["A. E. Brouwer's table of strongly regular graphs: conference parameters at n = 21 and 33 do not exist"],
    },
];

impl Bridge {
    /// the readings of any object of the family: `joined` is whether x beats (or is joined to) y
    fn readings(&self, n: i64, joined: bool) -> [i64; READS] {
        let mut r = [0; READS];
        let a = joined as i64;
        r[A as usize] = a;
        r[SIZE as usize] = n;
        for i in [OUT_X, OUT_Y, IN_X, IN_Y] {
            r[i as usize] = (n - 1) / 2;
        }
        match self.class {
            Class::Tournament => {
                r[B as usize] = 1 - a;
                r[COMMON_OUT as usize] = (n - 3) / 4;
                r[COMMON_IN as usize] = (n - 3) / 4;
                r[PATHS as usize] = if joined { (n - 3) / 4 } else { (n + 1) / 4 };
            }
            _ => {
                r[B as usize] = a;
                let c = if joined { (n - 5) / 4 } else { (n - 1) / 4 };
                r[COMMON_OUT as usize] = c;
                r[COMMON_IN as usize] = c;
                r[PATHS as usize] = c;
            }
        }
        r
    }

    /// does every object of the family solve F, as far as n = `to`?
    pub fn solved_by_family(&self, f: &E, to: usize) -> bool {
        (self.least..=to).filter(|n| n % 4 == self.residue).all(|n| [false, true].iter().all(|&j| f.val(&self.readings(n as i64, j)) == 0))
    }

    /// the family's own conditions
    fn exact(&self) -> E {
        let parts = self.conditions();
        Add(Box::new(parts[0].clone()), Box::new(parts[1].clone()))
    }

    /// its conditions, one by one and with parts taken away: where Golden starts
    fn seeds(&self) -> Vec<E> {
        let p = self.conditions();
        let mul = |a: &E, b: &E| Mul(Box::new(a.clone()), Box::new(b.clone()));
        let add = |a: &E, b: &E| Add(Box::new(a.clone()), Box::new(b.clone()));
        let min = |a: &E, b: &E| Min(Box::new(a.clone()), Box::new(b.clone()));
        vec![p[0].clone(), p[1].clone(), p[2].clone(), p[3].clone(), mul(&p[2], &p[3]), add(&p[0], &mul(&p[2], &p[3])), min(&p[0], &p[1]), mul(&p[0], &p[1])]
    }

    /// 0: the degree; 1: the whole pair condition; 2, 3: its two halves, each taken alone
    fn conditions(&self) -> [E; 4] {
        // |m * read - (n + c)|
        let off = |m: i64, read: u8, c: i64| Abs(Box::new(Sub(Box::new(Mul(Box::new(Num(m)), Box::new(Read(read)))), Box::new(Add(Box::new(Read(SIZE)), Box::new(Num(c)))))));
        let degree = off(2, OUT_X, -1);
        match self.class {
            Class::Tournament => {
                let (c, b) = (off(4, COMMON_OUT, -3), off(4, COMMON_IN, -3));
                [degree, Add(Box::new(c.clone()), Box::new(b.clone())), c, b]
            }
            _ => {
                let (j, not) = (off(4, COMMON_OUT, -5), off(4, COMMON_OUT, -1));
                [degree, If(Box::new(Read(A)), Box::new(j.clone()), Box::new(not.clone())), Mul(Box::new(j.clone()), Box::new(not.clone())), Min(Box::new(Read(A)), Box::new(j))]
            }
        }
    }

    /// a real object of the family: Paley's on a prime q
    fn paley(&self, q: usize) -> Vec<u32> {
        let residues: BTreeSet<usize> = (1..q).map(|x| x * x % q).collect();
        (0..q).map(|x| (0..q).fold(0u32, |r, y| r | ((x != y && residues.contains(&((y + q - x) % q))) as u32) << y)).collect()
    }

    fn paley_sizes(&self) -> &'static [usize] {
        match self.class {
            Class::Tournament => &[7, 11, 19, 23],
            _ => &[5, 13, 17, 29],
        }
    }

    fn lives_at(&self, n: usize) -> bool {
        n >= self.least && n % 4 == self.residue
    }
}

const GROW_TO: usize = 13;
const GROW_STEPS: u64 = 300;
const ISLANDS: usize = 8;
const POP: usize = 40;
const GENERATIONS: usize = 15;
const MAX_NODES: usize = 40;
/// how far the family is asked while growing, and at the end
const FAMILY_GROW: usize = 403;
const FAMILY_END: usize = 4003;
const FINALISTS: usize = 24;

/// what a short look says of a bridge: structures where the family cannot live, n's that turn, n's undecided
fn score(f: &E, b: &Bridge) -> f64 {
    if !b.solved_by_family(f, FAMILY_GROW) {
        return -100.0;
    }
    let (mut new, mut none, mut open, mut plain, mut turns) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut last = None;
    for n in 3..=GROW_TO {
        let now = match decide(f, b.class, n, GROW_STEPS) {
            Verdict::Plain(_) => {
                plain += 1;
                continue;
            }
            Verdict::Built(_) => {
                if !b.lives_at(n) {
                    new += 1;
                }
                true
            }
            Verdict::Impossible(_) => {
                none += 1;
                false
            }
            Verdict::Undecided(_) => {
                open += 1;
                continue;
            }
        };
        if last.is_some_and(|l| l != now) {
            turns += 1;
        }
        last = Some(now);
    }
    2.0 * new.min(4) as f64 + 2.0 * turns as f64 + 3.0 * open.min(2) as f64 + none.min(4) as f64 - 5.0 * plain.saturating_sub(1) as f64 - f.size() as f64 * 0.02
}

fn grown(b: &'static Bridge) -> Vec<(f64, E)> {
    let seeds = b.seeds();
    let halls: Vec<BTreeMap<String, (f64, E)>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..ISLANDS)
            .map(|i| {
                let seeds = &seeds;
                sc.spawn(move || {
                    let mut r = Rng(0xA0761D6478BD642F ^ ((b.class as u64 * 131 + i as u64 + 1) << 32));
                    let mut pop: Vec<(f64, E)> = (0..POP)
                        .map(|k| {
                            let s = &seeds[k % seeds.len()];
                            // a seed, or a seed that any zero of another function also answers
                            let e = if k < seeds.len() { s.clone() } else { Mul(Box::new(s.clone()), Box::new(grow(b.class, &mut r, 2))) };
                            (score(&e, b), e)
                        })
                        .collect();
                    let mut hall = BTreeMap::new();
                    for _ in 0..GENERATIONS {
                        for (s, e) in &pop {
                            if *s > 0.0 {
                                hall.insert(e.show(b.class), (*s, e.clone()));
                            }
                        }
                        let top = pop.iter().cloned().fold((f64::MIN, Num(0)), |x, y| if y.0 > x.0 { y } else { x });
                        let mut next = vec![top];
                        while next.len() < POP {
                            let e = child_up_to(b.class, &pop, &mut r, MAX_NODES);
                            next.push((score(&e, b), e));
                        }
                        pop = next;
                    }
                    hall
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut all = BTreeMap::new();
    for h in halls {
        all.extend(h);
    }
    let mut out: Vec<(f64, E)> = all.into_values().collect();
    out.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.size().cmp(&y.1.size())));
    out
}

/// known cases first: the family's own conditions and real objects must come out as known
fn validated() -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for b in &BRIDGES {
        let exact = b.exact();
        if !b.solved_by_family(&exact, FAMILY_END) {
            return Err(format!("{}: its conditions do not hold on its own readings", b.name));
        }
        for &q in b.paley_sizes() {
            if !holds(&exact, b.class, &b.paley(q)) {
                return Err(format!("{}: Paley's on {q} points fails its conditions", b.name));
            }
            for s in b.seeds() {
                if !holds(&s, b.class, &b.paley(q)) {
                    return Err(format!("{}: Paley's on {q} points fails a seed: {}", b.name, s.show(b.class)));
                }
            }
        }
        lines.push(format!("known case {}: its conditions hold on its readings to n = {FAMILY_END} and on Paley's at n = {:?}", b.name, b.paley_sizes()));
    }
    let conference = &BRIDGES[1];
    if matches!(decide(&conference.exact(), Class::Graph, 21, 200_000), Verdict::Built(_)) {
        return Err("a conference graph on 21 points was built, and none exists".into());
    }
    lines.push("known case: no conference graph is built on 21 points, where none exists".into());
    Ok(lines)
}

/// without the point that loses to all or beats all (a graph: joined to none or to all): is what is left in the family?
fn family_plus_a_point(rows: &[u32], exact: &E, class: Class) -> bool {
    let n = rows.len();
    (0..n).any(|p| {
        let out = rows[p].count_ones() as usize;
        (out == 0 || out == n - 1) && {
            let keep = |r: u32| (0..n).filter(|&j| j != p).enumerate().fold(0u32, |x, (b, j)| x | ((r >> j & 1) << b));
            let rest: Vec<u32> = (0..n).filter(|&i| i != p).map(|i| keep(rows[i])).collect();
            holds(exact, class, &rest)
        }
    })
}

/// how many renamings leave a structure as it is
fn symmetries(rows: &[u32]) -> u64 {
    fn go(rows: &[u32], image: &mut Vec<usize>, used: u32) -> u64 {
        let k = image.len();
        if k == rows.len() {
            return 1;
        }
        let mut c = 0;
        for v in 0..rows.len() {
            if used >> v & 1 == 1 {
                continue;
            }
            let same = (0..k).all(|i| rows[i] >> k & 1 == rows[image[i]] >> v & 1 && rows[k] >> i & 1 == rows[v] >> image[i] & 1);
            if same && rows[k].count_ones() == rows[v].count_ones() {
                image.push(v);
                c += go(rows, image, used | 1 << v);
                image.pop();
            }
        }
        c
    }
    go(rows, &mut Vec::new(), 0)
}

/// does a plain regular object solve F at several n? Then F asks nothing beyond the degree
fn empty_condition(f: &E, b: &Bridge) -> bool {
    let circulant = |n: usize| -> Vec<u32> {
        let joined = |i: usize, j: usize| {
            let d = (j + n - i) % n;
            match b.class {
                Class::Tournament => d != 0 && d <= n / 2,
                _ => d != 0 && (d <= n / 4 || n - d <= n / 4),
            }
        };
        (0..n).map(|i| (0..n).fold(0u32, |r, j| r | (joined(i, j) as u32) << j)).collect()
    };
    let sizes: &[usize] = if b.class == Class::Tournament { &[7, 9, 11] } else { &[9, 13, 17] };
    sizes.iter().all(|&n| holds(f, b.class, &circulant(n)))
}

/// a problem with its bridge
pub struct Span {
    pub bridge: &'static Bridge,
    pub problem: Problem,
}

impl Span {
    /// structures that are not the family and not the family with one point added: the kin
    fn kin(&self) -> Vec<(usize, &Vec<u32>)> {
        let exact = self.bridge.exact();
        self.problem
            .spectrum
            .iter()
            .filter_map(|(n, v)| match v {
                Verdict::Built(rows) if !holds(&exact, self.bridge.class, rows) && !family_plus_a_point(rows, &exact, self.bridge.class) => Some((*n, rows)),
                _ => None,
            })
            .collect()
    }

    /// kin at an n with n - 4 and n + 4 both impossible (or too small to ask): the exceptions
    fn sporadic(&self) -> Vec<usize> {
        let none = |n: usize| n < 3 || self.problem.spectrum.iter().any(|(m, v)| *m == n && matches!(v, Verdict::Impossible(_)));
        self.kin().iter().map(|k| k.0).filter(|&n| none(n.wrapping_sub(4)) && none(n + 4)).collect()
    }

    /// n's undecided where the family lives and is not settled: only here could "impossible" say anything new
    fn at_stake(&self) -> Vec<usize> {
        self.problem.spectrum.iter().filter(|(n, v)| self.bridge.lives_at(*n) && *n > self.bridge.known_to && matches!(v, Verdict::Undecided(_))).map(|x| x.0).collect()
    }

    pub fn lines(&self) -> Vec<String> {
        let b = self.bridge;
        let mut out = self.problem.lines();
        let kin = self.kin();
        out.insert(1, format!("  bridge: every {} solves this (its readings to n = {FAMILY_END}; Paley's objects at n = {:?} checked whole)", b.name, b.paley_sizes()));
        out.insert(2, format!("  kin, neither the family nor the family with one point added: {}", kin.iter().map(|(n, r)| format!("n = {n} with {} symmetries", symmetries(r))).collect::<Vec<_>>().join(", ")));
        out.insert(3, format!("  sporadic, kin with n - 4 and n + 4 impossible: n = {:?}", self.sporadic()));
        out.insert(4, format!("  open question moved within reach: {}", if self.at_stake().is_empty() { format!("none; the family is settled to n = {}", b.known_to) } else { format!("{:?}", self.at_stake()) }));
        if let Some((n, rows)) = kin.first() {
            let degrees: Vec<u32> = rows.iter().map(|r| r.count_ones()).collect();
            out.insert(5, format!("  the smallest kin, n = {n}: degrees {degrees:?}; rows {rows:?}"));
        }
        out
    }
}

/// Golden writes the bridges, Nuome answers them
pub fn invent(log: &mut dyn FnMut(String)) -> Result<Vec<Span>, String> {
    for l in validated()? {
        log(l);
    }
    let mut kept = Vec::new();
    for b in &BRIDGES {
        let t0 = std::time::Instant::now();
        let hall = grown(b);
        let mut seen = BTreeSet::from([fingerprint(&b.exact(), b.class)]);
        let finalists: Vec<E> = hall.iter().map(|(_, e)| e.tidy()).filter(|e| b.solved_by_family(e, FAMILY_END) && seen.insert(fingerprint(e, b.class))).take(FINALISTS).collect();
        let looked: Vec<Vec<(usize, Verdict)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = finalists.chunks(FINALISTS.div_ceil(ISLANDS)).map(|c| sc.spawn(move || c.iter().map(|f| look(f, b.class)).collect::<Vec<_>>())).collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let (mut same, mut flat, mut here, mut no_kin, mut empty) = (0, 0, 0, 0, 0);
        let mut answers = BTreeSet::new();
        let looked_at = finalists.len();
        for (f, spectrum) in finalists.into_iter().zip(looked) {
            let count = |p: &dyn Fn(&Verdict) -> bool| spectrum.iter().filter(|(_, v)| p(v)).count();
            let (plain, built, none) = (count(&|v| matches!(v, Verdict::Plain(_))), count(&|v| matches!(v, Verdict::Built(_))), count(&|v| matches!(v, Verdict::Impossible(_))));
            if plain > 1 || built == 0 || none == 0 {
                flat += 1;
                continue;
            }
            for &q in b.paley_sizes() {
                assert!(holds(&f, b.class, &b.paley(q)), "a kept bridge fails on Paley's at {q}: a bug");
            }
            for (n, v) in &spectrum {
                if b.paley_sizes().contains(n) {
                    assert!(!matches!(v, Verdict::Impossible(_)), "the search calls n = {n} impossible and Paley's object solves it: a bug");
                }
            }
            let answer: String = spectrum.iter().map(|(_, v)| match v {
                Verdict::Built(_) | Verdict::Plain(_) => 'y',
                Verdict::Impossible(_) => 'n',
                _ => '?',
            }).collect();
            if !answers.insert(answer) {
                same += 1;
                continue;
            }
            if empty_condition(&f, b) {
                empty += 1;
                continue;
            }
            let mut problem = Problem { class: b.class, f, spectrum, none_from: None, part_sizes: Vec::new() };
            problem.solve();
            let span = Span { bridge: b, problem };
            if span.kin().is_empty() {
                no_kin += 1;
                continue;
            }
            here += 1;
            kept.push(span);
        }
        log(format!("{}: {} questions grown, {looked_at} looked at: {here} kept; dropped: {flat} a plain structure answers or nothing built or nothing impossible, {same} with the same answers as another, {empty} asking nothing beyond the degree, {no_kin} with no kin ({:.0} s)", b.name, hall.len(), t0.elapsed().as_secs_f64()));
    }
    kept.sort_by_key(|s| (std::cmp::Reverse(s.sporadic().len()), s.problem.f.size()));
    Ok(kept)
}

/// `nuome --ideas golden-bridges [HOW_MANY]`: the first shown, all written to bench/golden_problem/bridges.md
pub fn run(how_many: usize) -> Result<(), String> {
    let kept = invent(&mut |l| println!("{l}"))?;
    let mut text = String::from("# Problems Golden invented that bridge to an open question\n\nEvery object of the family solves the problem, so \"impossible\" at n would say the object does not exist on n points. Open means Nuome did not decide it. Whether anyone has asked the same question, and how fruitful it is, is not checked.\n");
    println!("\n{} bridges kept; the first {}:", kept.len(), how_many.min(kept.len()));
    for (i, s) in kept.iter().enumerate() {
        let lines = s.lines();
        if i < how_many {
            println!("\n{}. {}", i + 1, lines.join("\n"));
        }
        text.push_str(&format!("\n## {}\n\n{}\n", i + 1, lines.join("\n\n")));
    }
    std::fs::create_dir_all("bench/golden_problem").map_err(|e| e.to_string())?;
    std::fs::write("bench/golden_problem/bridges.md", text).map_err(|e| e.to_string())
}

/// The question, read whole.
pub const QUESTION: &str = "Invent a problem whose answer could advance a question mathematics has left open.";

pub fn read(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// Logical: the problems and what is checked of them. Theoretical: what an answer would say, from the known results named.
/// Abstract: the questions alone, claiming nothing.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    let mut lines = Vec::new();
    let kept = invent(&mut |l| lines.push(l))?;
    if !kind_logical {
        lines.clear();
    }
    let mut sources = Vec::new();
    for s in kept.iter().take(5) {
        if kind_logical {
            lines.extend(s.lines());
        } else {
            lines.push(s.problem.statement());
            if !kind_abstract {
                lines.push(format!("  impossible at n would say: {}", s.bridge.says));
                for src in s.bridge.sources {
                    if !sources.iter().any(|x: &String| x == src) {
                        sources.push(src.to_string());
                    }
                }
            }
        }
    }
    if kind_abstract {
        lines.push("this is an answer that claims nothing".into());
    } else if kind_logical {
        lines.push("whether anyone has asked the same question, and how fruitful it is, is not checked".into());
    }
    Ok((format!("{} problems that bridge to an open question", kept.len()), lines, sources))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_cases_come_out_as_known() {
        validated().unwrap();
    }

    #[test]
    fn the_readings_are_those_of_real_objects() {
        // Paley's objects read pair by pair give exactly the family's readings
        for b in &BRIDGES {
            for &q in b.paley_sizes() {
                let rows = b.paley(q);
                for x in 0..q {
                    for y in 0..q {
                        if x != y {
                            let joined = rows[x] >> y & 1 == 1;
                            assert_eq!(crate::golden_problem::readings(&rows, x, y), b.readings(q as i64, joined), "{} q = {q}", b.name);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_degree_factor_asks_nothing() {
        for b in &BRIDGES {
            let c = b.conditions();
            let exempt = Mul(Box::new(c[0].clone()), Box::new(Num(5)));
            assert!(empty_condition(&exempt, b));
            assert!(!empty_condition(&b.exact(), b));
        }
    }

    #[test]
    fn a_function_the_family_does_not_solve_is_refused() {
        for b in &BRIDGES {
            assert!(!b.solved_by_family(&Num(1), 100));
            assert!(score(&Num(1), b) < 0.0);
            assert!(b.solved_by_family(&Num(0), 100));
        }
    }

    #[test]
    fn a_family_with_a_point_added_is_no_kin() {
        let b = &BRIDGES[0];
        let exact = b.exact();
        // Paley's on 7 with an eighth point that loses to all
        let mut rows = b.paley(7);
        for r in rows.iter_mut() {
            *r |= 1 << 7;
        }
        rows.push(0);
        assert!(!holds(&exact, b.class, &rows));
        assert!(family_plus_a_point(&rows, &exact, b.class));
        assert!(!family_plus_a_point(&b.paley(7), &Num(1), b.class));
    }

    /// is there a renaming that takes one structure to the other?
    fn same(x: &[u32], y: &[u32]) -> bool {
        fn go(x: &[u32], y: &[u32], image: &mut Vec<usize>, used: u32) -> bool {
            let k = image.len();
            if k == x.len() {
                return true;
            }
            (0..x.len()).any(|v| {
                used >> v & 1 == 0 && (0..k).all(|i| x[i] >> k & 1 == y[image[i]] >> v & 1 && x[k] >> i & 1 == y[v] >> image[i] & 1) && {
                    image.push(v);
                    let found = go(x, y, image, used | 1 << v);
                    image.pop();
                    found
                }
            })
        }
        go(x, y, &mut Vec::new(), 0)
    }

    /// the core of the sporadic problem, with no exemption: on every two players exactly one beat both and exactly one both
    /// beat, or two beat both and at least two both beat. Solved for every n by the parts: n = 7, 9, 11 and no other
    #[test]
    fn the_core_of_the_nine() {
        let core = Abs(Box::new(Sub(Box::new(Min(Box::new(Read(COMMON_OUT)), Box::new(Num(2)))), Box::new(Max(Box::new(Read(A)), Box::new(Read(COMMON_IN)))))));
        let p = crate::golden_problem::parts(&core, Class::Tournament, 2_000_000);
        assert_eq!(p.dies_at, Some(12));
        assert_eq!(p.decided.iter().filter(|x| x.1.is_some()).map(|x| x.0).collect::<Vec<_>>(), [7, 9, 11]);
        let whole = |n: usize| -> Vec<Vec<u32>> {
            let mut classes: Vec<Vec<u32>> = Vec::new();
            for r in crate::golden_problem::all_parts(&core, Class::Tournament, n, 2_000_000) {
                if holds(&core, Class::Tournament, &r) && !classes.iter().any(|c| same(c, &r)) {
                    classes.push(r);
                }
            }
            classes
        };
        // 7 and 11: one each, and Paley's
        for q in [7, 11] {
            let c = whole(q);
            assert_eq!(c.len(), 1);
            assert!(same(&c[0], &BRIDGES[0].paley(q)));
        }
        // 9: not doubly regular, since 9 is not 3 mod 4
        let nine = whole(9);
        let mut symmetric: Vec<u64> = nine.iter().map(|r| symmetries(r)).collect();
        symmetric.sort_unstable();
        println!("structures at 9: {}, symmetries {symmetric:?}", nine.len());
        assert!(!nine.is_empty() && nine.iter().all(|r| r.iter().all(|x| x.count_ones() == 4)));
        // the other search, with no parts in it, reaches the same: 8, 10, 12, 13 have none
        for n in [8, 10, 12, 13] {
            assert!(matches!(decide(&core, Class::Tournament, n, 5_000_000), Verdict::Impossible(_)), "n = {n}");
        }
    }

    #[test]
    fn symmetries_are_counted() {
        // Paley's tournament on 7 has 21 (x -> a x + b, a a square), the pentagon 10, the transitive tournament 1
        assert_eq!(symmetries(&BRIDGES[0].paley(7)), 21);
        assert_eq!(symmetries(&BRIDGES[1].paley(5)), 10);
        assert_eq!(symmetries(&[0b110, 0b100, 0]), 1);
    }

    #[test]
    fn the_question_is_read_whole() {
        assert!(read("invent a problem whose answer could advance a question mathematics has left open"));
        assert!(!read("invent a problem"));
    }
}
