//! P versus NP as an experiment (it cannot be settled by one: the question
//! is about every possible algorithm).
//!
//! - Checking is easy: a proposed assignment is checked clause by clause.
//! - Finding: random 3-SAT at 4.26 clauses per variable (where it is
//!   hardest) solved by DPLL (unit propagation, branching on the busiest
//!   variable), counting the branch points.
//! - For contrast 2-SAT, which is in P, solved through its implication graph
//!   (strongly connected components), counting the steps.
//! - The growth is judged the way the Goldbach formula was: an exponential
//!   and a polynomial model are fitted on the smaller sizes and scored on
//!   how well they predict the larger ones.

use crate::evolve::Rng;

/// A literal: variable v (from 1) as +v or -v.
type Clause = Vec<i32>;

pub fn random_ksat(n: usize, m: usize, k: usize, r: &mut Rng) -> Vec<Clause> {
    (0..m)
        .map(|_| {
            let mut vars: Vec<i32> = Vec::new();
            while vars.len() < k {
                let v = 1 + r.below(n) as i32;
                if !vars.contains(&v) {
                    vars.push(v);
                }
            }
            vars.into_iter().map(|v| if r.below(2) == 0 { v } else { -v }).collect()
        })
        .collect()
}

/// Check an assignment (index v holds variable v's value); also returns
/// the number of literals looked at.
pub fn check(clauses: &[Clause], value: &[bool]) -> (bool, usize) {
    let mut looked = 0;
    for c in clauses {
        let mut sat = false;
        for &l in c {
            looked += 1;
            if value[l.unsigned_abs() as usize] == (l > 0) {
                sat = true;
                break;
            }
        }
        if !sat {
            return (false, looked);
        }
    }
    (true, looked)
}

/// DPLL; returns (a satisfying assignment if any, branch points used).
pub fn dpll(n: usize, clauses: &[Clause]) -> (Option<Vec<bool>>, u64) {
    // 0 = unset, 1 = true, 2 = false
    fn value_of(assign: &[u8], l: i32) -> Option<bool> {
        match assign[l.unsigned_abs() as usize] {
            0 => None,
            a => Some((a == 1) == (l > 0)),
        }
    }
    fn go(clauses: &[Clause], assign: &mut Vec<u8>, nodes: &mut u64) -> bool {
        // unit propagation
        let mut set: Vec<usize> = Vec::new();
        loop {
            let mut unit = None;
            for c in clauses {
                let mut open = None;
                let mut open_count = 0;
                let mut sat = false;
                for &l in c {
                    match value_of(assign, l) {
                        Some(true) => {
                            sat = true;
                            break;
                        }
                        Some(false) => {}
                        None => {
                            open_count += 1;
                            open = Some(l);
                        }
                    }
                }
                if sat {
                    continue;
                }
                if open_count == 0 {
                    for v in set {
                        assign[v] = 0;
                    }
                    return false;
                }
                if open_count == 1 {
                    unit = open;
                    break;
                }
            }
            match unit {
                Some(l) => {
                    let v = l.unsigned_abs() as usize;
                    assign[v] = if l > 0 { 1 } else { 2 };
                    set.push(v);
                }
                None => break,
            }
        }
        // the busiest unset variable in open clauses
        let mut count = vec![0u32; assign.len()];
        for c in clauses {
            if c.iter().any(|&l| value_of(assign, l) == Some(true)) {
                continue;
            }
            for &l in c {
                if assign[l.unsigned_abs() as usize] == 0 {
                    count[l.unsigned_abs() as usize] += 1;
                }
            }
        }
        let Some(v) = (1..assign.len()).filter(|&v| assign[v] == 0 && count[v] > 0).max_by_key(|&v| (count[v], std::cmp::Reverse(v))) else {
            return true; // every clause is satisfied
        };
        *nodes += 1;
        for choice in [1u8, 2] {
            assign[v] = choice;
            if go(clauses, assign, nodes) {
                return true;
            }
        }
        assign[v] = 0;
        for x in set {
            assign[x] = 0;
        }
        false
    }
    let mut assign = vec![0u8; n + 1];
    let mut nodes = 0;
    if go(clauses, &mut assign, &mut nodes) {
        (Some(assign.iter().map(|&a| a == 1).collect()), nodes)
    } else {
        (None, nodes)
    }
}

/// 2-SAT through the implication graph: satisfiable exactly when no
/// variable shares a strongly connected component with its negation.
/// Returns (satisfiable, steps of the graph walk).
pub fn two_sat(n: usize, clauses: &[Clause]) -> (bool, u64) {
    let node = |l: i32| (l.unsigned_abs() as usize - 1) * 2 + usize::from(l < 0);
    let size = 2 * n;
    let mut adj = vec![Vec::new(); size];
    for c in clauses {
        // (a or b): not a -> b, not b -> a
        adj[node(-c[0])].push(node(c[1]));
        adj[node(-c[1])].push(node(c[0]));
    }
    // Tarjan's algorithm, iteratively
    let (mut index, mut steps) = (0usize, 0u64);
    let mut idx = vec![usize::MAX; size];
    let mut low = vec![0usize; size];
    let mut on = vec![false; size];
    let mut comp = vec![usize::MAX; size];
    let mut stack = Vec::new();
    let mut ncomp = 0;
    for s in 0..size {
        if idx[s] != usize::MAX {
            continue;
        }
        let mut work: Vec<(usize, usize)> = vec![(s, 0)];
        idx[s] = index;
        low[s] = index;
        index += 1;
        stack.push(s);
        on[s] = true;
        while let Some(&mut (v, ref mut i)) = work.last_mut() {
            steps += 1;
            if *i < adj[v].len() {
                let w = adj[v][*i];
                *i += 1;
                if idx[w] == usize::MAX {
                    idx[w] = index;
                    low[w] = index;
                    index += 1;
                    stack.push(w);
                    on[w] = true;
                    work.push((w, 0));
                } else if on[w] {
                    low[v] = low[v].min(idx[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[v]);
                }
                if low[v] == idx[v] {
                    loop {
                        let w = stack.pop().expect("on the stack");
                        on[w] = false;
                        comp[w] = ncomp;
                        if w == v {
                            break;
                        }
                    }
                    ncomp += 1;
                }
            }
        }
    }
    ((0..n).all(|v| comp[2 * v] != comp[2 * v + 1]), steps)
}

/// splitmix64 of (a, b): seeds that differ in any bit give unrelated streams.
fn mix(a: u64, b: u64) -> u64 {
    let mut z = a.wrapping_add(b.wrapping_mul(0x9E37_79B9_7F4A_7C15)).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub struct Settings {
    pub sizes: Vec<usize>,
    pub instances: usize,
    pub ratio: f64,
    pub seed: u64,
}

/// Least squares y = a + b x.
fn line(xs: &[f64], ys: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let b = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum::<f64>() / xs.iter().map(|x| (x - mx).powi(2)).sum::<f64>();
    (my - b * mx, b)
}

pub fn report(s: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    // 3-SAT: median branch points per size
    let mut medians = Vec::new();
    let mut check_cost = Vec::new();
    for &n in &s.sizes {
        let m = (s.ratio * n as f64).round() as usize;
        let runs: Vec<(u64, bool, usize)> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..s.instances)
                .map(|i| {
                    sc.spawn(move || {
                        let mut r = Rng(mix(mix(s.seed, n as u64), i as u64).max(1));
                        let f = random_ksat(n, m, 3, &mut r);
                        let (sol, nodes) = dpll(n, &f);
                        let (ok, looked) = sol.as_ref().map_or((true, 0), |v| check(&f, v));
                        (nodes, sol.is_some() && ok, looked)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().expect("no panics")).collect()
        });
        let mut nodes: Vec<u64> = runs.iter().map(|x| x.0).collect();
        nodes.sort();
        let median = nodes[nodes.len() / 2] as f64;
        let sat = runs.iter().filter(|x| x.1).count();
        let looked = runs.iter().filter(|x| x.1).map(|x| x.2).max().unwrap_or(0);
        medians.push((n, median.max(1.0), sat));
        check_cost.push((n, m, looked));
    }
    out.push(format!(
        "random 3-SAT at {} clauses per variable (the hardest ratio), {} formulas per size, solved by DPLL (unit propagation, branching on the busiest variable); every answer 'satisfiable' checked by plugging the assignment in",
        s.ratio, s.instances
    ));
    let rows: Vec<String> = medians.iter().map(|(n, med, sat)| format!("n={n}: median {med:.0} branch points ({sat}/{} satisfiable)", s.instances)).collect();
    out.push(format!("  finding: {}", rows.join("; ")));
    let checks: Vec<String> = check_cost.iter().map(|(n, m, l)| format!("n={n}: at most {l} literals for {m} clauses")).collect();
    out.push(format!("  checking a found assignment: {} (never more than 3 per clause: linear)", checks.join("; ")));
    // models, fitted on the first two thirds, judged on the rest (the Goldbach way)
    let cut = (medians.len() * 2).div_ceil(3).max(2).min(medians.len() - 1);
    let ns: Vec<f64> = medians.iter().map(|x| x.0 as f64).collect();
    let logs: Vec<f64> = medians.iter().map(|x| x.1.ln()).collect();
    let (ea, eb) = line(&ns[..cut], &logs[..cut]);
    let lns: Vec<f64> = ns.iter().map(|n| n.ln()).collect();
    let (pa, pb) = line(&lns[..cut], &logs[..cut]);
    let ahead = |f: &dyn Fn(usize) -> f64| ((cut..ns.len()).map(|i| (f(i) - logs[i]).powi(2)).sum::<f64>() / (ns.len() - cut) as f64).sqrt();
    let exp_err = ahead(&|i| ea + eb * ns[i]);
    let pol_err = ahead(&|i| pa + pb * lns[i]);
    out.push(format!(
        "  growth, fitted on n <= {} and judged on the larger sizes: exponential 2^({:.3} n) misses them by a factor {:.2}; polynomial n^{:.1} misses by a factor {:.2}: {}",
        medians[cut - 1].0,
        eb / std::f64::consts::LN_2,
        exp_err.exp(),
        pb,
        pol_err.exp(),
        if exp_err < pol_err { "the exponential predicts better" } else { "the polynomial predicts better" }
    ));
    // 2-SAT for contrast
    let mut rows2 = Vec::new();
    for n in [1_000usize, 10_000, 100_000] {
        let mut r = Rng(mix(s.seed, n as u64).max(1));
        let f = random_ksat(n, n, 2, &mut r);
        let (sat, steps) = two_sat(n, &f);
        rows2.push(format!("n={n}: {steps} steps ({})", if sat { "satisfiable" } else { "not" }));
    }
    out.push(format!("  2-SAT (in P) for contrast, by strongly connected components: {}: the work grows linearly", rows2.join("; ")));
    out.push("this shows the gap between checking and finding for one algorithm on random formulas; P vs NP asks whether ANY algorithm closes it on EVERY formula, and relativization, natural proofs and algebrization show why experiments like this cannot decide it".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpll_agrees_with_trying_everything() {
        let mut r = Rng(11);
        for _ in 0..30 {
            let n = 8;
            let f = random_ksat(n, 36, 3, &mut r);
            let brute = (0..1u32 << n).any(|bits| {
                let v: Vec<bool> = (0..=n).map(|i| i > 0 && bits >> (i - 1) & 1 == 1).collect();
                check(&f, &v).0
            });
            let (sol, _) = dpll(n, &f);
            assert_eq!(sol.is_some(), brute);
            if let Some(v) = sol {
                assert!(check(&f, &v).0);
            }
        }
    }

    #[test]
    fn two_sat_agrees_with_trying_everything() {
        let mut r = Rng(5);
        for _ in 0..50 {
            let n = 7;
            let f = random_ksat(n, 9, 2, &mut r);
            let brute = (0..1u32 << n).any(|bits| {
                let v: Vec<bool> = (0..=n).map(|i| i > 0 && bits >> (i - 1) & 1 == 1).collect();
                check(&f, &v).0
            });
            assert_eq!(two_sat(n, &f).0, brute);
        }
    }
}
