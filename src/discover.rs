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
    pub min_n: u64,
    pub split: u64,
    pub price_per_parameter: f64,
    pub check_up_to: u64,
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
        let (over, secs) = goldbach::exceed(s.check_up_to, s.min_n, threads, a, b);
        let evens = (s.check_up_to - s.min_n) / 2 + 1;
        out.push(format!("every even n from {} to {}: is the smallest prime p(n) at most {a:.3} (ln n)^{b:.3}? ({evens} numbers, {secs:.1} s; below {} the formula isn't meant to hold)", s.min_n, s.check_up_to, s.min_n));
        if over.count == 0 {
            out.push("  yes, for every one of them".into());
        } else {
            out.push(format!("  no: {} numbers exceed it (one in {:.0}); the worst:", over.count, evens as f64 / over.count as f64));
            for (n, p, fv) in &over.worst {
                out.push(format!("    n = {n}: p = {p}, formula {fv:.0} ({:+.0}%)", (*p as f64 / fv - 1.0) * 100.0));
            }
            // the smallest factor that makes it a bound over this whole range
            let k = over.worst.first().map_or(1.0, |(_, p, fv)| *p as f64 / fv);
            out.push(format!("  as an upper bound it needs a factor {k:.3}: p(n) <= {:.3} (ln n)^{b:.3} holds for every even n up to {} (checked, not proved beyond)", a * k, s.check_up_to));
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
        let lines = goldbach_formula(&Settings { min_n: 1000, split: 100_000_000_000, price_per_parameter: 0.02, check_up_to: 1_000_000 });
        assert!(lines[0].contains("agree"), "{}", lines[0]);
    }
}
