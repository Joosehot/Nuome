//! Golden's conjectures: every number path Golden Tree keeps is a question of Collatz's kind, and Golden asks it.
//!
//! For a rule f on the natural numbers, Golden looks at where the orbits of the small numbers end (1 to 10^4) and
//! writes the conjecture: every positive integer reaches one of these cycles. Then, in Nuome's way for an open
//! problem (CLAUDE.md):
//!
//! 1. Forcing. Every residue class x = N·m + r (N = 2^K times the rule's moduli) is followed symbolically as an
//!    affine map A·m + B, step by step, for as long as the step is decided by the class (a condition x ≡ r (mod m)
//!    is decided when m divides A, a shift by k when 2^k divides A). A class whose coefficient falls below N descends:
//!    from some m on, its orbit goes below where it started. If every class descends, every x above a bound X does.
//! 2. The choice left: the numbers up to X, and the classes that did not descend.
//! 3. A short exact search: every n up to 10^7 is followed until it falls below n or meets a cycle; a cycle not
//!    among the conjecture's refutes it, and an orbit past 2^120 or longer than the limit is reported.
//! 4. Which of three: proved (every class descends and every n up to X reaches the cycles), refuted (a new cycle,
//!    with the number that finds it), or open (how far the search went, what share of the classes descends).
//!
//! The drift is reported too: the mean of log2(A_after / A_before) over the classes after one iteration, the rate
//! an orbit grows or shrinks if its residues were evenly spread. It is a heuristic and is called one.
//! Open means Nuome did not decide it; whether anyone has asked the same question is not checked.

use crate::golden_tree::{self as gt, Rule, Step, World};
use std::collections::BTreeSet;

/// the numbers whose orbits make the conjecture
pub const ASK_TO: u64 = 10_000;
/// the numbers the search follows
pub const CHECK_TO: u64 = 10_000_000;
/// the longest an orbit is followed before it is reported
const ORBIT_LIMIT: usize = 100_000;
/// the most residue classes followed symbolically
const MAX_CLASSES: u64 = 1 << 22;
/// iterations a class is followed
const SYM_ITER: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// every class descends above `bound`, and every n up to `bound` reaches the cycles
    Proved { bound: u64 },
    /// `start`'s orbit meets a cycle the conjecture does not name, whose least member is `cycle_min`
    Refuted { start: u64, cycle_min: u128 },
    /// undecided: the search reached `checked`; `descends` of `classes` classes mod `modulus` descend
    Open { checked: u64, descends: u64, classes: u64, modulus: u64, trouble: Option<(u64, &'static str)> },
}

#[derive(Clone, Debug)]
pub struct Conjecture {
    pub rule: Rule,
    /// the cycles, each written from its least member
    pub cycles: Vec<Vec<u128>>,
    pub drift: f64,
    pub status: Status,
}

/// The cycle an orbit from n falls into (Brent), written from its least member; None if it blows up or runs past
/// the limit.
fn cycle_of(codes: &[u32], n: u128) -> Option<Vec<u128>> {
    let (mut tort, mut power, mut lam) = (n, 1usize, 1usize);
    let mut x = gt::step_number(codes, n)?;
    let mut steps = 0;
    while x != tort {
        if power == lam {
            tort = x;
            power *= 2;
            lam = 0;
        }
        x = gt::step_number(codes, x)?;
        lam += 1;
        steps += 1;
        if steps > ORBIT_LIMIT {
            return None;
        }
    }
    let mut cyc = vec![x];
    let mut y = gt::step_number(codes, x)?;
    while y != x {
        cyc.push(y);
        y = gt::step_number(codes, y)?;
    }
    let k = (0..cyc.len()).min_by_key(|&i| cyc[i]).unwrap();
    cyc.rotate_left(k);
    Some(cyc)
}

/// The moduli the rule's steps read: conditions and mods.
fn moduli(rule: &Rule) -> u64 {
    let mut l = 1u64;
    let gcd = |mut a: u64, mut b: u64| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    for s in &rule.steps {
        let m = match *s {
            Step::If { m, .. } | Step::Mod(m) => m as u64,
            _ => 1,
        };
        if m > 1 {
            l = l / gcd(l, m) * m;
        }
    }
    l
}

/// One step on the class A·m + B, if the class decides it.
fn sym_step(s: &Step, a: i128, b: i128) -> Option<(i128, i128)> {
    let basic = |op: u8, v: u8, a: i128, b: i128| -> Option<(i128, i128)> {
        let v = v as i128;
        Some(match op {
            1 => (a, b + v),
            2 => (a * v, b * v),
            3 => {
                let d = 1i128 << v;
                if a % d != 0 {
                    return None;
                }
                (a / d, b.div_euclid(d))
            }
            4 => (a << v, b << v),
            5 => {
                let bits = 128 - (v as u128).leading_zeros() as i128;
                let d = 1i128 << bits;
                if a % d != 0 {
                    return None;
                }
                let low = b.rem_euclid(d);
                (a, b - low + (low ^ v))
            }
            6 => {
                if v == 0 {
                    (a, b)
                } else if a % v != 0 {
                    return None;
                } else {
                    (0, b.rem_euclid(v))
                }
            }
            _ => return None,
        })
    };
    match *s {
        Step::Add(v) => basic(1, v, a, b),
        Step::Mul(v) => basic(2, v, a, b),
        Step::Shr(v) => basic(3, v, a, b),
        Step::Shl(v) => basic(4, v, a, b),
        Step::Xor(v) => basic(5, v, a, b),
        Step::Mod(v) => basic(6, v, a, b),
        Step::If { m, r, op, arg } => {
            let m = m as i128;
            if a % m != 0 {
                return None;
            }
            if b.rem_euclid(m) == r as i128 {
                basic(op, arg, a, b)
            } else {
                Some((a, b))
            }
        }
        _ => None,
    }
}

/// The forcing: every class mod N followed until it descends or the class no longer decides the step. Returns
/// (N, classes, how many descend, the bound X above which every class has descended, the drift).
fn force(rule: &Rule) -> (u64, u64, u64, u64, f64) {
    let l = moduli(rule);
    let mut n = l;
    while n * 2 <= MAX_CLASSES {
        n *= 2;
    }
    let threads = std::thread::available_parallelism().map(|t| t.get()).unwrap_or(4) as u64;
    let chunk = n.div_ceil(threads);
    let parts: Vec<(u64, u64, f64)> = std::thread::scope(|sc| {
        (0..threads)
            .map(|k| {
                let rule = &rule;
                sc.spawn(move || {
                    let (mut down, mut bound, mut drift) = (0u64, 0u64, 0f64);
                    for r in (k * chunk)..((k + 1) * chunk).min(n) {
                        let (mut a, mut b) = (n as i128, r as i128);
                        let mut first: Option<f64> = None;
                        'iter: for it in 0..SYM_ITER {
                            for s in &rule.steps {
                                match sym_step(s, a, b) {
                                    Some((a2, b2)) => {
                                        a = a2;
                                        b = b2;
                                    }
                                    None => break 'iter,
                                }
                                if a > 1i128 << 100 {
                                    break 'iter;
                                }
                            }
                            if it == 0 {
                                first = Some(((a.max(1)) as f64 / n as f64).log2());
                            }
                            if a < n as i128 {
                                // A·m + B < N·m + r once (N − A)·m > B − r
                                let gap = n as i128 - a;
                                let m0 = if b - (r as i128) < 0 { 1 } else { (b - r as i128) / gap + 1 };
                                let x = (n as i128) * m0.max(1) + r as i128;
                                bound = bound.max(x.min(u64::MAX as i128) as u64);
                                down += 1;
                                break;
                            }
                        }
                        drift += first.unwrap_or(0.0);
                    }
                    (down, bound, drift)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });
    let down = parts.iter().map(|p| p.0).sum();
    let bound = parts.iter().map(|p| p.1).max().unwrap_or(0);
    let drift = parts.iter().map(|p| p.2).sum::<f64>() / n as f64;
    (n, n, down, bound, drift)
}

/// The search: every n from 1 to `to`, followed until it falls below n or meets a cycle. Returns the first trouble:
/// (n, what) for a new cycle (Some(cycle)) or an orbit that blows up or runs past the limit (None).
fn search(codes: &[u32], cycles: &BTreeSet<u128>, to: u64) -> Option<(u64, Option<Vec<u128>>)> {
    let threads = std::thread::available_parallelism().map(|t| t.get()).unwrap_or(4) as u64;
    let chunk = to.div_ceil(threads);
    let found: Vec<(u64, Option<Vec<u128>>)> = std::thread::scope(|sc| {
        (0..threads)
            .map(|k| {
                sc.spawn(move || {
                    for n in (k * chunk + 1)..=((k + 1) * chunk).min(to) {
                        let mut x = n as u128;
                        let mut ok = false;
                        for _ in 0..ORBIT_LIMIT {
                            match gt::step_number(codes, x) {
                                Some(y) => x = y,
                                None => return Some((n, None)),
                            }
                            if x < n as u128 || cycles.contains(&x) {
                                ok = true;
                                break;
                            }
                            if x == n as u128 {
                                // n lies on a cycle the conjecture does not name
                                return Some((n, cycle_of(codes, n as u128)));
                            }
                        }
                        if !ok {
                            return Some((n, cycle_of(codes, n as u128)));
                        }
                    }
                    None
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|h| h.join().unwrap())
            .collect()
    });
    found.into_iter().min_by_key(|f| f.0)
}

/// Golden's conjecture for a number rule, decided as far as Nuome can.
pub fn conjecture(rule: &Rule) -> Option<Conjecture> {
    if rule.world() != World::Number {
        return None;
    }
    let codes = rule.codes();
    // the cycles the small numbers fall into
    let mut cycles: Vec<Vec<u128>> = Vec::new();
    for n in 1..=ASK_TO {
        let c = cycle_of(&codes, n as u128)?;
        if !cycles.contains(&c) {
            cycles.push(c);
        }
    }
    cycles.sort();
    let members: BTreeSet<u128> = cycles.iter().flatten().copied().collect();
    let (modulus, classes, descends, bound, drift) = force(rule);
    let status = match search(&codes, &members, CHECK_TO) {
        Some((n, Some(c))) => Status::Refuted { start: n, cycle_min: c[0] },
        Some((n, None)) => Status::Open { checked: n - 1, descends, classes, modulus, trouble: Some((n, "its orbit passes 2^120 or 100 000 steps: a candidate for an orbit that never comes down")) },
        None if descends == classes && bound <= CHECK_TO => Status::Proved { bound },
        None => Status::Open { checked: CHECK_TO, descends, classes, modulus, trouble: None },
    };
    Some(Conjecture { rule: rule.clone(), cycles, drift, status })
}

impl Conjecture {
    pub fn statement(&self) -> String {
        // a long cycle is written by its least members and its length
        let cyc: Vec<String> = self
            .cycles
            .iter()
            .map(|c| {
                if c.len() <= 12 {
                    format!("({})", c.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" → "))
                } else {
                    format!("({} → … , {} members)", c[..4].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" → "), c.len())
                }
            })
            .collect();
        let f = self.rule.text().replacen("x ↦ ", "", 1);
        format!("Under f(x) = [{f}], every positive integer reaches {}.", if cyc.len() == 1 { format!("the cycle {}", cyc[0]) } else { format!("one of the cycles {}", cyc.join(", ")) })
    }

    pub fn verdict(&self) -> String {
        match &self.status {
            Status::Proved { bound } => format!("proved: every residue class descends above {bound}, and every n up to {bound} was followed into the cycles"),
            Status::Refuted { start, cycle_min } => format!("refuted: {start} falls into another cycle, whose least member is {cycle_min}"),
            Status::Open { checked, descends, classes, modulus, trouble } => {
                let mut s = format!(
                    "open: true for every n up to {checked}; descent forced for {descends} of {classes} residue classes mod {modulus} ({:.2}%)",
                    100.0 * *descends as f64 / *classes as f64
                );
                if let Some((n, why)) = trouble {
                    s.push_str(&format!("; {n}: {why}"));
                }
                s
            }
        }
    }
}

/// The question, read whole.
pub const QUESTION: &str = "Make conjectures from golden tree paths.";

pub fn read_question(q: &str) -> bool {
    let norm = |s: &str| s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    norm(q).trim_end_matches(['.', '?', '!']) == norm(QUESTION).trim_end_matches(['.', '?', '!'])
}

/// The number rules of seeds 1 to `seeds`, none a shorter one lengthened, best first.
pub fn candidates(seeds: u64, gpu: Option<&gt::Gpu>) -> Vec<Rule> {
    let mut all: Vec<gt::Entry> = Vec::new();
    for seed in 1..=seeds {
        all.extend(gt::best(&gt::grow(300, gt::RENDER_DISTANCE, seed, gpu), 40).into_iter().filter(|e| e.world == World::Number));
    }
    all.sort_by(|a, b| a.codes.len().cmp(&b.codes.len()).then(b.score.partial_cmp(&a.score).unwrap()));
    let mut distinct: Vec<gt::Entry> = Vec::new();
    for e in all {
        if !distinct.iter().any(|d| e.codes.starts_with(&d.codes)) {
            distinct.push(e);
        }
    }
    distinct.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap().then(a.id.cmp(&b.id)));
    distinct.into_iter().map(|e| Rule::from_codes(&e.codes)).collect()
}

/// Golden Answer's kinds. Logical: the conjectures with how each was decided. Abstract: the statements alone.
pub fn answer(kind_logical: bool, kind_abstract: bool) -> Result<(String, Vec<String>, Vec<String>), String> {
    if !kind_logical && !kind_abstract {
        return Err("a conjecture made here rests on no known result to name; ask logical".into());
    }
    let gpu = gt::Gpu::new().ok();
    let mut lines = Vec::new();
    let mut n = 0;
    for rule in candidates(10, gpu.as_ref()) {
        let Some(c) = conjecture(&rule) else { continue };
        lines.push(c.statement());
        if kind_logical {
            lines.push(format!("  {}; drift (heuristic) {:+.3} bits an iteration", c.verdict(), c.drift));
        }
        n += 1;
        if n == 5 {
            break;
        }
    }
    if kind_abstract {
        lines.retain(|l| !l.starts_with("  "));
        lines.push("this is an answer that claims nothing".into());
    } else {
        lines.push("open means Nuome did not decide it; whether anyone has asked the same question is not checked".into());
    }
    Ok((format!("{n} conjectures"), lines, vec![]))
}
