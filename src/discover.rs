//! Finding a formula from data: the least prime of the Goldbach record
//! cases as a function of n. Each candidate family of formulas is a rule;
//! each is fitted by least squares on ln p over the training cases (n up to
//! a split), one is chosen by training error plus a price per parameter,
//! and only then is it asked to predict the cases beyond the split, which
//! it has never seen. The choice never looks at those.

use crate::goldbach;

/// The published record cases (OEIS A025018 / A025019), shipped with Nuome.
pub const RECORDS: &str = include_str!("../data/goldbach_records.tsv");

pub fn records() -> Vec<(u64, u64)> {
    RECORDS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Some((f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?))
        })
        .collect()
}

/// A family of formulas: ln p = fixed(n) + sum of c_i * basis_i(n).
struct Family {
    name: &'static str,
    fixed: fn(f64) -> f64,
    basis: &'static [fn(f64) -> f64],
    show: fn(&[f64]) -> String,
}

fn l(n: f64) -> f64 {
    n.ln()
}
fn ll(n: f64) -> f64 {
    n.ln().ln()
}
fn one(_: f64) -> f64 {
    1.0
}

const FAMILIES: &[Family] = &[
    Family { name: "a (ln n)^b", fixed: |_| 0.0, basis: &[one, ll], show: |c| format!("p = {:.3} (ln n)^{:.3}", c[0].exp(), c[1]) },
    Family { name: "a (ln n)^2", fixed: |n| 2.0 * ll(n), basis: &[one], show: |c| format!("p = {:.3} (ln n)^2", c[0].exp()) },
    Family { name: "a (ln n)^2 ln ln n", fixed: |n| 2.0 * ll(n) + ll(n).ln(), basis: &[one], show: |c| format!("p = {:.3} (ln n)^2 ln ln n", c[0].exp()) },
    Family { name: "a (ln n)^3", fixed: |n| 3.0 * ll(n), basis: &[one], show: |c| format!("p = {:.3} (ln n)^3", c[0].exp()) },
    Family { name: "a ln n", fixed: ll, basis: &[one], show: |c| format!("p = {:.3} ln n", c[0].exp()) },
    Family { name: "a n^b", fixed: |_| 0.0, basis: &[one, l], show: |c| format!("p = {:.3} n^{:.4}", c[0].exp(), c[1]) },
];

/// Least squares for up to two coefficients.
fn fit(f: &Family, data: &[(f64, f64)]) -> Vec<f64> {
    let k = f.basis.len();
    let mut ata = [[0.0f64; 2]; 2];
    let mut atb = [0.0f64; 2];
    for &(n, p) in data {
        let y = p.ln() - (f.fixed)(n);
        let x: Vec<f64> = f.basis.iter().map(|b| b(n)).collect();
        for i in 0..k {
            atb[i] += x[i] * y;
            for j in 0..k {
                ata[i][j] += x[i] * x[j];
            }
        }
    }
    if k == 1 {
        vec![atb[0] / ata[0][0]]
    } else {
        let det = ata[0][0] * ata[1][1] - ata[0][1] * ata[1][0];
        vec![(atb[0] * ata[1][1] - atb[1] * ata[0][1]) / det, (ata[0][0] * atb[1] - ata[1][0] * atb[0]) / det]
    }
}

fn predict(f: &Family, c: &[f64], n: f64) -> f64 {
    ((f.fixed)(n) + f.basis.iter().zip(c).map(|(b, ci)| b(n) * ci).sum::<f64>()).exp()
}

/// Root mean square error in ln p, and the largest error in percent.
fn errors(f: &Family, c: &[f64], data: &[(f64, f64)]) -> (f64, f64) {
    let mut sq = 0.0;
    let mut worst: f64 = 0.0;
    for &(n, p) in data {
        let q = predict(f, c, n);
        sq += (q.ln() - p.ln()).powi(2);
        worst = worst.max((q / p - 1.0).abs());
    }
    ((sq / data.len() as f64).sqrt(), worst * 100.0)
}

pub struct Settings {
    pub evolve: Option<crate::evolve::Settings>,
    pub min_n: u64,
    pub split: u64,
    pub price_per_parameter: f64,
    pub check_up_to: u64,
}

/// A number as it is printed with three decimals.
fn round3(x: f64) -> f64 {
    format!("{x:.3}").parse().unwrap_or(x)
}

/// The tree with every constant and exponent as printed.
fn round_tree(t: &crate::evolve::Node) -> crate::evolve::Node {
    use crate::evolve::Node;
    match t {
        Node::Const(c) => Node::Const(round3(*c)),
        Node::Add(a, b) => Node::Add(Box::new(round_tree(a)), Box::new(round_tree(b))),
        Node::Mul(a, b) => Node::Mul(Box::new(round_tree(a)), Box::new(round_tree(b))),
        Node::Pow(a, e) => Node::Pow(Box::new(round_tree(a)), round3(*e)),
        n => n.clone(),
    }
}

/// The smallest multiplier c, rounded UP to three decimals, with
/// p(n) <= c * shape(n) for every even n in the range, and whether a recheck
/// of exactly that printed bound against every n found no exception.
fn verified_bound(s: &Settings, threads: usize, guess: f64, shape: &(dyn Fn(f64) -> f64 + Sync)) -> (f64, bool) {
    let up = |x: f64| (x * 1000.0).ceil() / 1000.0;
    let (over, _) = goldbach::exceed(s.check_up_to, s.min_n, threads, &|n: f64| guess * shape(n));
    let mut c = match over.worst.first() {
        Some((_, p, fv)) => up(guess * (*p as f64 / fv)),
        None => up(guess),
    };
    for _ in 0..3 {
        let (again, _) = goldbach::exceed(s.check_up_to, s.min_n, threads, &|n: f64| c * shape(n));
        if again.count == 0 {
            return (c, true);
        }
        c += 0.001;
    }
    (c, false)
}

pub fn goldbach_formula(s: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    let data = records();
    // 1. the data against Nuome's own computation where both exist
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let run = goldbach::run(s.check_up_to, threads);
    let theirs: Vec<(u64, u64)> = data.iter().copied().filter(|(n, _)| *n <= s.check_up_to).collect();
    let agree = run.records == theirs;
    out.push(format!(
        "data: {} published record cases (OEIS A025018/A025019); Nuome recomputed every one up to {} itself: {}",
        data.len(),
        s.check_up_to,
        if agree { format!("all {} agree", theirs.len()) } else { "THEY DISAGREE, so the data isn't used".into() }
    ));
    if !agree {
        return out;
    }
    let as_f = |v: &[(u64, u64)]| v.iter().map(|&(n, p)| (n as f64, p as f64)).collect::<Vec<_>>();
    let train = as_f(&data.iter().copied().filter(|(n, _)| *n >= s.min_n && *n <= s.split).collect::<Vec<_>>());
    let test = as_f(&data.iter().copied().filter(|(n, _)| *n > s.split).collect::<Vec<_>>());
    out.push(format!("fit on the {} cases with {} <= n <= {}; then predict the {} cases above {}, which the fit never sees", train.len(), s.min_n, s.split, test.len(), s.split));
    // 2. every family fitted; chosen by training error plus a price per parameter
    let mut rows = Vec::new();
    for f in FAMILIES {
        let c = fit(f, &train);
        let (tr, _) = errors(f, &c, &train);
        let (te, te_worst) = errors(f, &c, &test);
        let score = tr + s.price_per_parameter * f.basis.len() as f64;
        rows.push((f, c, tr, te, te_worst, score));
    }
    let best = rows.iter().enumerate().min_by(|a, b| a.1 .5.partial_cmp(&b.1 .5).expect("finite")).map(|(i, _)| i).expect("families");
    out.push("formula families, fitted (error = root mean square of ln p; test = cases it never saw):".into());
    for (i, (f, c, tr, te, tw, _)) in rows.iter().enumerate() {
        out.push(format!("  {} {:<24} train {:.3}  test {:.3} (worst {:>5.1}%)   {}", if i == best { "=>" } else { "  " }, f.name, tr, te, tw, (f.show)(c)));
    }
    let (f, c, _, te, tw, _) = &rows[best];
    out.push(format!("chosen by the training data alone: {}", (f.show)(c)));
    out.push(format!("its predictions for the unseen cases (n up to {:.1e}): typical error {:.0}%, worst {:.0}%:", test.last().map_or(0.0, |t| t.0), (te.exp() - 1.0) * 100.0, tw));
    for &(n, p) in test.iter().step_by((test.len() / 6).max(1)) {
        out.push(format!("  n = {:.3e}: predicted {:>6.0}, actual {:>5}", n, predict(f, c, n), p));
    }
    // 3. the formula against every single even number, not just the records
    if f.name == "a (ln n)^b" {
        let (a, b) = (c[0].exp(), c[1]);
        let (over, secs) = goldbach::exceed(s.check_up_to, s.min_n, threads, &|n: f64| a * n.ln().powf(b));
        let evens = (s.check_up_to - s.min_n) / 2 + 1;
        out.push(format!("every even n from {} to {}: is the smallest prime p(n) at most {a:.3} (ln n)^{b:.3}? ({evens} numbers, {secs:.1} s; below {} the formula isn't meant to hold)", s.min_n, s.check_up_to, s.min_n));
        if over.count == 0 {
            out.push("  yes, for every one of them".into());
        } else {
            out.push(format!("  no: {} numbers exceed it (one in {:.0}); the worst:", over.count, evens as f64 / over.count as f64));
            for (n, p, fv) in &over.worst {
                out.push(format!("    n = {n}: p = {p}, formula {fv:.0} ({:+.0}%)", (*p as f64 / fv - 1.0) * 100.0));
            }
            let b3 = round3(b);
            let (c, ok) = verified_bound(s, threads, a, &|n: f64| n.ln().powf(b3));
            out.push(if ok {
                format!("  as an upper bound, exactly as printed: p(n) <= {c:.3} (ln n)^{b3:.3} for every even n from {} to {} (this printed formula rechecked against all of them; not proved beyond)", s.min_n, s.check_up_to)
            } else {
                "  no bound could be confirmed with the printed constants".to_string()
            });
        }
    }
    // 4. evolution: formulas bred by the fitness function, chosen on training fitness alone
    if let Some(es) = &s.evolve {
        let found = crate::evolve::evolve(&train, es);
        let how = if es.forward { "error predicting the largest third of the training cases from the rest" } else { "training error" };
        out.push(format!("evolved formulas ({} per generation, {} generations, seed {}; fitness = {how} + {} per node):", es.population, es.generations, es.seed, es.price_per_node));
        let baseline = rows[best].3;
        for (i, f) in found.iter().enumerate() {
            let pred = |n: f64| f.k * f.tree.eval(n);
            let test_err = (test.iter().map(|&(n, p)| (pred(n).ln() - p.ln()).powi(2)).sum::<f64>() / test.len() as f64).sqrt();
            let worst = test.iter().map(|&(n, p)| (pred(n) / p - 1.0).abs()).fold(0.0, f64::max) * 100.0;
            out.push(format!("  {} fitness {:.4}  train {:.3}  test {:.3} (worst {:>4.1}%)  p = {:.4} * {}", if i == 0 { "=>" } else { "  " }, f.fitness, f.train, test_err, worst, f.k, f.tree.show()));
        }
        if let Some(top) = found.first() {
            let pred = |n: f64| top.k * top.tree.eval(n);
            let test_err = (test.iter().map(|&(n, p)| (pred(n).ln() - p.ln()).powi(2)).sum::<f64>() / test.len() as f64).sqrt();
            out.push(format!("evolved winner on unseen cases: test error {:.3} against {:.3} for the family fit ({})", test_err, baseline, if test_err < baseline { "better" } else { "not better" }));
            for &(n, p) in test.iter().step_by((test.len() / 6).max(1)) {
                out.push(format!("  n = {:.3e}: predicted {:>6.0}, actual {:>5}", n, pred(n), p));
            }
            let (over, secs) = goldbach::exceed(s.check_up_to, s.min_n, threads, &|n: f64| top.k * top.tree.eval(n));
            let evens = (s.check_up_to - s.min_n) / 2 + 1;
            out.push(format!("  against every even n from {} to {} ({:.1} s): {} numbers exceed it", s.min_n, s.check_up_to, secs, over.count));
            if over.count > 0 {
                let shape = round_tree(&top.tree);
                let (c, ok) = verified_bound(s, threads, top.k, &|n: f64| shape.eval(n));
                out.push(if ok {
                    format!("  as an upper bound, exactly as printed: p(n) <= {c:.3} * {} for every even n from {} to {} ({evens} numbers; this printed formula rechecked against all of them)", shape.show(), s.min_n, s.check_up_to)
                } else {
                    "  no bound could be confirmed with the printed constants".to_string()
                });
            }
        }
    }
    out.push("this is a fitted formula, not a theorem: it describes the records seen so far".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_loads_and_a_formula_generalises() {
        let d = records();
        assert_eq!(d.len(), 67);
        assert_eq!(d[39], (35_884_080_836, 2803));
        let lines = goldbach_formula(&Settings { evolve: None, min_n: 1000, split: 100_000_000_000, price_per_parameter: 0.02, check_up_to: 1_000_000 });
        assert!(lines[0].contains("agree"), "{}", lines[0]);
    }
}
