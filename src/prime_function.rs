//! The idea machine for a prime function: where is the n-th prime? Nuome
//! builds formulas p(n) ~ n * (sum of terms in ln n and ln ln n) itself:
//! every combination of up to `max_terms` terms of the shape
//! (ln n)^i (ln ln n)^j, its coefficients fitted to the first half of the
//! primes, judged on the second half it never saw, the price of each term
//! paid (Occam). Then the abstract step: the best formulas' coefficients
//! are read as simple fractions, and a formula whose fractions do as well
//! as its fitted numbers is recognised as a law, not a fit. With the
//! `neural` feature a Neuras network learns what the formula leaves over,
//! and is judged the same way. Last, what the function can and cannot do:
//! it says where the n-th prime is to within a band; which number in the
//! band is prime it cannot say, and each must still be tested.

/// The search's settings.
pub struct Settings {
    /// primes to learn from and judge on (the first half learns)
    pub count: usize,
    /// a far check: the formula at the `far`-th prime
    pub far: usize,
    pub max_terms: usize,
    /// what each term costs, in relative error
    pub price: f64,
}

/// Every prime up to `limit` (sieve of Eratosthenes).
pub fn primes_to(limit: usize) -> Vec<u64> {
    let mut comp = vec![false; limit + 1];
    let mut out = Vec::new();
    for i in 2..=limit {
        if !comp[i] {
            out.push(i as u64);
            let mut j = i * i;
            while j <= limit {
                comp[j] = true;
                j += i;
            }
        }
    }
    out
}

/// A term (ln n)^i (ln ln n)^j.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Term {
    i: i32,
    j: i32,
}

impl Term {
    fn at(self, n: f64) -> f64 {
        let l = n.ln();
        l.powi(self.i) * l.ln().powi(self.j)
    }
    fn show(self) -> String {
        let l = |p: i32| match p {
            1 => "ln n".to_string(),
            p => format!("(ln n)^{p}"),
        };
        let ll = |p: i32| match p {
            1 => "ln ln n".to_string(),
            p => format!("(ln ln n)^{p}"),
        };
        match (self.i, self.j) {
            (0, 0) => "1".into(),
            (i, 0) if i > 0 => l(i),
            (0, j) => ll(j),
            (i, j) if i > 0 => format!("{} {}", l(i), ll(j)),
            (i, 0) => format!("1/{}", l(-i)),
            (i, j) => format!("{}/{}", ll(j), l(-i)),
        }
    }
}

fn terms() -> Vec<Term> {
    let mut v = Vec::new();
    for i in [1, 0, -1, -2] {
        for j in 0..=2 {
            v.push(Term { i, j });
        }
    }
    v
}

/// A formula: p(n) ~ n * sum c_k t_k(n).
#[derive(Clone, Debug)]
pub struct Formula {
    terms: Vec<Term>,
    coef: Vec<f64>,
    /// n ln n added with coefficient exactly 1 (the prime number theorem)
    anchored: bool,
}

impl Formula {
    pub fn at(&self, n: f64) -> f64 {
        let base = if self.anchored { n * n.ln() } else { 0.0 };
        base + n * self.terms.iter().zip(&self.coef).map(|(t, c)| c * t.at(n)).sum::<f64>()
    }
    pub fn show(&self) -> String {
        let mut s = String::new();
        if self.anchored {
            s.push_str("ln n");
        }
        for (k, (t, c)) in self.terms.iter().zip(&self.coef).enumerate() {
            let (sign, a) = if *c < 0.0 { ("-", -c) } else { ("+", *c) };
            let num = fraction(a).unwrap_or_else(|| format!("{a:.4}"));
            let body = match (num.as_str(), t.show().as_str()) {
                ("1", b) => b.to_string(),
                (n, "1") => n.to_string(),
                (n, b) => format!("{n} {b}"),
            };
            if k == 0 && !self.anchored {
                s.push_str(&if sign == "-" { format!("-{body}") } else { body });
            } else {
                s.push_str(&format!(" {sign} {body}"));
            }
        }
        format!("n ({s})")
    }
}

/// a as a simple fraction, if it is one (to 1e-9).
fn fraction(a: f64) -> Option<String> {
    for d in [1i64, 2, 3, 4, 6] {
        let k = (a * d as f64).round();
        if (k / d as f64 - a).abs() < 1e-9 {
            return Some(if d == 1 { format!("{}", k as i64) } else { format!("{}/{}", k as i64, d) });
        }
    }
    None
}

/// Least squares on relative error: the coefficients of `ts` that make
/// n * sum c t(n) closest to p, over the learning primes.
fn fit(ts: &[Term], learn: &[(f64, f64)], anchored: bool) -> Option<Vec<f64>> {
    let k = ts.len();
    let mut a = vec![vec![0.0f64; k + 1]; k];
    for &(n, p) in learn {
        let row: Vec<f64> = ts.iter().map(|t| n * t.at(n) / p).collect();
        // what is left to fit after a fixed n ln n
        let rhs = if anchored { 1.0 - n * n.ln() / p } else { 1.0 };
        for r in 0..k {
            for c in 0..k {
                a[r][c] += row[r] * row[c];
            }
            a[r][k] += row[r] * rhs;
        }
    }
    // Gaussian elimination with partial pivoting
    for col in 0..k {
        let piv = (col..k).max_by(|&x, &y| a[x][col].abs().total_cmp(&a[y][col].abs()))?;
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        for r in 0..k {
            if r != col {
                let f = a[r][col] / a[col][col];
                for c in col..=k {
                    a[r][c] -= f * a[col][c];
                }
            }
        }
    }
    Some((0..k).map(|r| a[r][k] / a[r][r]).collect())
}

/// Largest and mean relative error of `f` over `judge`.
fn errors(f: &dyn Fn(f64) -> f64, judge: &[(f64, f64)]) -> (f64, f64) {
    let mut max = 0.0f64;
    let mut sum = 0.0;
    for &(n, p) in judge {
        let e = ((f(n) - p) / p).abs();
        max = max.max(e);
        sum += e;
    }
    (max, sum / judge.len().max(1) as f64)
}

fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for size in 1..=k {
        let mut idx: Vec<usize> = (0..size).collect();
        loop {
            out.push(idx.clone());
            let mut i = size;
            while i > 0 && idx[i - 1] == n - size + i - 1 {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            idx[i - 1] += 1;
            for j in i..size {
                idx[j] = idx[j - 1] + 1;
            }
        }
    }
    out
}

/// The nearest simple fraction (denominator up to 6), if close.
fn simple(c: f64) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for d in [1.0, 2.0, 3.0, 4.0, 6.0] {
        let r = (c * d).round() / d;
        let e = (r - c).abs();
        if e < 0.2 && best.is_none_or(|b| e < b.1) {
            best = Some((r, e));
        }
    }
    best.map(|b| b.0)
}

fn group(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let mut r = String::new();
    // enough primes for the far check
    let far = s.far.max(s.count);
    let lf = (far as f64).ln();
    let limit = (far as f64 * (lf + lf.ln())) as usize + 1000;
    let ps = primes_to(limit);
    let p = |n: usize| ps[n - 1] as f64;
    let half = s.count / 2;
    // learn on n = 100..half (every 5th), judge on half+1..count (every one)
    let learn: Vec<(f64, f64)> = (100..=half).step_by(5).map(|n| (n as f64, p(n))).collect();
    let judge: Vec<(f64, f64)> = (half + 1..=s.count).map(|n| (n as f64, p(n))).collect();
    let far_judge: Vec<(f64, f64)> = (s.count + 1..=far).step_by(97).map(|n| (n as f64, p(n))).collect();
    r.push_str(&format!(
        "Nuome invents a prime function: where is the n-th prime?\n\nthe primes: the first {} (the {}th is {}), and for a far check up to the {}th ({})\nlearned on n = 100..{}, judged on n = {}..{} which the search never saw, then far: n = {}..{}\n",
        group(s.count as u64),
        group(s.count as u64),
        group(p(s.count) as u64),
        group(far as u64),
        group(p(far) as u64),
        group(half as u64),
        group(half as u64 + 1),
        group(s.count as u64),
        group(s.count as u64 + 1),
        group(far as u64)
    ));

    // the search, twice: free, and anchored to the prime number theorem
    // (p(n) ~ n ln n is proved, so a law for all n must start with exactly
    // n ln n and add only terms that grow slower)
    let all = terms();
    let judge_quick: Vec<(f64, f64)> = judge.iter().step_by(7).copied().collect();
    let search = |anchored: bool| -> (usize, Vec<(f64, f64, Formula)>, Vec<(f64, Formula, f64)>) {
        let pool: Vec<Term> = all.iter().copied().filter(|t| !anchored || t.i <= 0).collect();
        let combos = subsets(pool.len(), s.max_terms);
        let mut found: Vec<(f64, f64, Formula)> = Vec::new();
        for idx in &combos {
            let ts: Vec<Term> = idx.iter().map(|&i| pool[i]).collect();
            let Some(coef) = fit(&ts, &learn, anchored) else { continue };
            let f = Formula { terms: ts, coef, anchored };
            let (max, _) = errors(&|n| f.at(n), &judge_quick);
            if max.is_finite() {
                found.push((max + s.price * f.terms.len() as f64, max, f));
            }
        }
        found.sort_by(|a, b| a.0.total_cmp(&b.0));
        // the abstract step: read the coefficients as simple fractions
        let mut laws: Vec<(f64, Formula, f64)> = Vec::new();
        for (_, fitted_max, f) in found.iter().take(60) {
            let Some(coef) = f.coef.iter().map(|&c| simple(c)).collect::<Option<Vec<f64>>>() else { continue };
            if coef.iter().any(|&c| c == 0.0) {
                continue;
            }
            let g = Formula { terms: f.terms.clone(), coef, anchored };
            let (max, _) = errors(&|n| g.at(n), &judge);
            // a law if its fractions do (nearly) as well as the fitted numbers
            if max <= fitted_max * 1.5 {
                laws.push((max + s.price * g.terms.len() as f64, g, *fitted_max));
            }
        }
        laws.sort_by(|a, b| a.0.total_cmp(&b.0));
        (combos.len(), found, laws)
    };
    let (n_free, found, laws) = search(false);
    let (n_anch, found_a, laws_a) = search(true);
    r.push_str(&format!(
        "\nthe search: {} terms (ln n)^i (ln ln n)^j, i = 1, 0, -1, -2 and j = 0, 1, 2; every combination of up to {} of them: {} formulas free, and {} anchored to the prime number theorem (exactly n ln n, plus terms that grow slower); each fitted on the learning primes and judged on the unseen ones, {:.2}% error charged per term\n",
        all.len(),
        s.max_terms,
        group(n_free as u64),
        group(n_anch as u64),
        s.price * 100.0
    ));

    let known: Vec<(&str, Formula)> = vec![
        ("n ln n (the prime number theorem)", Formula { terms: vec![Term { i: 1, j: 0 }], coef: vec![1.0], anchored: false }),
        ("n (ln n + ln ln n - 1) (Cesaro)", Formula { terms: vec![Term { i: 1, j: 0 }, Term { i: 0, j: 1 }, Term { i: 0, j: 0 }], coef: vec![1.0, 1.0, -1.0], anchored: false }),
        // Cipolla's asymptotic series to second order, from theory, not fitted:
        // n (ln n + ln ln n - 1 + (ln ln n - 2)/ln n - ((ln ln n)^2 - 6 ln ln n + 11)/(2 (ln n)^2))
        (
            "Cipolla, 2nd order (from theory)",
            Formula {
                terms: vec![
                    Term { i: 1, j: 0 },
                    Term { i: 0, j: 1 },
                    Term { i: 0, j: 0 },
                    Term { i: -1, j: 1 },
                    Term { i: -1, j: 0 },
                    Term { i: -2, j: 2 },
                    Term { i: -2, j: 1 },
                    Term { i: -2, j: 0 },
                ],
                coef: vec![1.0, 1.0, -1.0, 1.0, -2.0, -0.5, 3.0, -5.5],
                anchored: false,
            },
        ),
    ];
    // two far primes whose values are known (the billionth and trillionth)
    let distant = [(1e9, 22_801_763_489.0), (1e12, 29_996_224_275_833.0)];
    let line = |name: &str, f: &dyn Fn(f64) -> f64| {
        let (m, a) = errors(f, &judge);
        let (fm, _) = errors(f, &far_judge);
        let d: Vec<String> = distant.iter().map(|&(n, p)| format!("{:+.3}%", (f(n) - p) / p * 100.0)).collect();
        format!("  {name:<72} unseen max {:>7.4}% (mean {:.4}%), to 10^6 max {:>7.4}%, at n = 10^9 {}, at 10^12 {}\n", m * 100.0, a * 100.0, fm * 100.0, d[0], d[1])
    };
    r.push_str("\nknown formulas, for comparison:\n");
    for (name, f) in &known {
        r.push_str(&line(name, &|n| f.at(n)));
    }
    r.push_str("\nNuome's best free formulas (coefficients as fitted):\n");
    for (_, _, f) in found.iter().take(3) {
        r.push_str(&line(&f.show(), &|n| f.at(n)));
    }
    let _ = &laws;
    r.push_str("\nNuome's best formulas anchored to the prime number theorem (coefficients as fitted):\n");
    for (_, _, f) in found_a.iter().take(3) {
        r.push_str(&line(&f.show(), &|n| f.at(n)));
    }
    r.push_str("\nanchored formulas recognised as laws (every coefficient a simple fraction, as good as the fit):\n");
    if laws_a.is_empty() {
        r.push_str("  none: no top formula's coefficients read as simple fractions without losing accuracy\n");
    }
    for (_, g, _) in laws_a.iter().take(3) {
        r.push_str(&line(&g.show(), &|n| g.at(n)));
    }
    r.push_str("  (a free formula can fit the first million primes better, but its leading coefficient is not 1, so far out it drifts: see n = 10^12)\n");
    let best = found_a[0].2.clone();

    // the network: what the formula leaves over
    #[cfg(feature = "neural")]
    r.push_str(&neural(&best, &ps, half, s.count, &judge, &far_judge));
    #[cfg(not(feature = "neural"))]
    r.push_str("\n(the Neuras network that learns what the formula leaves over runs with --features neural)\n");

    // what the function does and does not do
    let (bmax, _) = errors(&|n| best.at(n), &judge);
    r.push_str(&format!("\nwhat the function does, with {}:\n", best.show()));
    for n in [s.count, far] {
        let guess = best.at(n as f64);
        let actual = p(n);
        let width = guess * bmax;
        let (a, b) = ((guess - width).max(2.0) as u64, (guess + width) as u64);
        let in_band = ps.iter().filter(|&&q| q >= a && q <= b).count();
        r.push_str(&format!(
            "  the {}th prime: the function says {}, it is {} (off by {}); within the unseen-range band of +-{:.3}% lie {} numbers, {} of them prime: it says where, not which\n",
            group(n as u64),
            group(guess.round() as u64),
            group(actual as u64),
            group((guess - actual).abs().round() as u64),
            bmax * 100.0,
            group(b - a + 1),
            group(in_band as u64)
        ));
    }
    r.push_str(
        "\nfor a prime of 100 million digits the same holds, only larger: near 10^(10^8) one number in about 230 million is prime (1/ln x),\n\
         so the function can place the n-th such prime within a band, but the band holds astronomically many numbers and the function\n\
         cannot say which of them is prime. Each candidate must still be tested; for a 100-million-digit number one test is a\n\
         332-million-bit computation (Lucas-Lehmer or a probable-prime test).\n",
    );
    r.push_str(&format!("\n({:.1} s)\n", t0.elapsed().as_secs_f64()));
    r
}

/// A Neuras network learns the formula's relative error from ln n and
/// ln ln n on the learning primes; judged on the unseen and far ones.
#[cfg(feature = "neural")]
fn neural(best: &Formula, ps: &[u64], half: usize, count: usize, judge: &[(f64, f64)], far_judge: &[(f64, f64)]) -> String {
    use neuras::model::{Layer, Net, Shape};
    use neuras::train::net::{Model, Rng};
    let feats = |n: f64| -> Vec<f32> {
        let l = n.ln();
        vec![(l / 15.0) as f32, (l.ln() / 3.0) as f32, (1.0 / l) as f32, (l.ln() / l) as f32]
    };
    // the target: the formula's relative error, in thousandths
    let target = |n: f64, p: f64| ((p - best.at(n)) / best.at(n) * 1000.0) as f32;
    let learn: Vec<(Vec<f32>, f32)> = (100..=half).map(|n| (feats(n as f64), target(n as f64, ps[n - 1] as f64))).collect();
    let net = Net::build(Shape::Vector(4), vec![Layer::Dense { out: 32 }, Layer::Relu, Layer::Dense { out: 32 }, Layer::Relu, Layer::Dense { out: 1 }]).expect("an mlp");
    let mut model = Model::new(&net, 7);
    let mut order: Vec<usize> = (0..learn.len()).collect();
    for e in 0..20 {
        let mut r = Rng::new(50 + e);
        for i in (1..order.len()).rev() {
            order.swap(i, r.below(i + 1));
        }
        for b in order.chunks(64) {
            let mut g = model.zero_grads();
            for &i in b {
                let t = model.forward(&learn[i].0);
                model.backward_mse(&t, &[learn[i].1], &mut g);
            }
            let k = 1.0 / b.len() as f32;
            for a in g.iter_mut() {
                for v in a.iter_mut() {
                    *v *= k;
                }
            }
            model.adam(&g, 0.001);
        }
    }
    let with_net = |n: f64| best.at(n) * (1.0 + model.outputs(&feats(n))[0] as f64 / 1000.0);
    let (m0, a0) = errors(&|n| best.at(n), judge);
    let (m1, a1) = errors(&with_net, judge);
    let (fm0, fa0) = errors(&|n| best.at(n), far_judge);
    let (fm1, fa1) = errors(&with_net, far_judge);
    let _ = count;
    format!(
        "\nthe Neuras network (4 -> 32 -> 32 -> 1, {} parameters) learns what the formula leaves over, on the learning primes:\n  formula alone        unseen: max {:.4}%, mean {:.4}%   far: max {:.4}%, mean {:.4}%\n  formula + network    unseen: max {:.4}%, mean {:.4}%   far: max {:.4}%, mean {:.4}%\n  {}\n",
        net.params,
        m0 * 100.0,
        a0 * 100.0,
        fm0 * 100.0,
        fa0 * 100.0,
        m1 * 100.0,
        a1 * 100.0,
        fm1 * 100.0,
        fa1 * 100.0,
        if a1 < a0 && fa1 < fa0 { "the network helps, here and far" } else if a1 < a0 { "the network helps where it learned, not far beyond" } else { "the network does not beat the formula: what is left over is the primes' own irregularity" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sieve_counts_primes() {
        assert_eq!(primes_to(100).len(), 25);
        assert_eq!(primes_to(1_000_000).len(), 78_498);
    }

    #[test]
    fn the_known_law_fits_and_reads_as_fractions() {
        let ps = primes_to(1_400_000);
        let learn: Vec<(f64, f64)> = (100..=50_000).step_by(5).map(|n| (n as f64, ps[n - 1] as f64)).collect();
        let ts = vec![Term { i: 1, j: 0 }, Term { i: 0, j: 1 }, Term { i: 0, j: 0 }];
        let c = fit(&ts, &learn, false).unwrap();
        // close to Cesaro's 1, 1, -1
        assert!((c[0] - 1.0).abs() < 0.2 && (c[1] - 1.0).abs() < 0.3, "{c:?}");
        assert_eq!(fraction(-0.5), Some("1/2".into()));
        assert_eq!(subsets(4, 2).len(), 10);
    }
}
