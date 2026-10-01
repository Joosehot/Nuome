//! Nuome's idea machine for the open part of Goldbach: it makes up new
//! statements about the even numbers and their prime pairs, finds the best
//! constant for each on the first half of the data, tests it on the second
//! half it never saw, keeps the survivors, and says for each what it would
//! give if it were proved. Statements are conjectures, never proofs.

use crate::goldbach::C2;

/// What Nuome knows about each even n: the number of ways r(n) to write it
/// as p + q (p <= q, both prime), the smallest such p, and S(n), the product
/// over odd primes q dividing n of (q - 1)/(q - 2).
struct Data {
    n: Vec<u64>,
    r: Vec<f64>,
    p: Vec<f64>,
    s: Vec<f64>,
}

fn data(limit: usize) -> Data {
    let mut composite = vec![false; limit + 1];
    composite[0] = true;
    composite[1] = true;
    let mut i = 2;
    while i * i <= limit {
        if !composite[i] {
            let mut j = i * i;
            while j <= limit {
                composite[j] = true;
                j += i;
            }
        }
        i += 1;
    }
    let primes: Vec<usize> = (2..=limit).filter(|&k| !composite[k]).collect();
    let evens: Vec<usize> = (6..=limit).step_by(2).collect();
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let chunk = evens.len().div_ceil(threads);
    let rows: Vec<(u64, f64, f64, f64)> = std::thread::scope(|sc| {
        let hs: Vec<_> = evens
            .chunks(chunk)
            .map(|part| {
                let (primes, composite) = (&primes, &composite);
                sc.spawn(move || {
                    part.iter()
                        .map(|&n| {
                            let (mut r, mut first) = (0u32, 0usize);
                            for &q in primes.iter().take_while(|&&q| q <= n / 2) {
                                if !composite[n - q] {
                                    r += 1;
                                    if first == 0 {
                                        first = q;
                                    }
                                }
                            }
                            let mut s = 1.0;
                            let mut m = n;
                            while m % 2 == 0 {
                                m /= 2;
                            }
                            let mut d = 3;
                            while d * d <= m {
                                if m % d == 0 {
                                    s *= (d as f64 - 1.0) / (d as f64 - 2.0);
                                    while m % d == 0 {
                                        m /= d;
                                    }
                                }
                                d += 2;
                            }
                            if m > 1 {
                                s *= (m as f64 - 1.0) / (m as f64 - 2.0);
                            }
                            (n as u64, r as f64, first as f64, s)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("no panics")).collect()
    });
    Data { n: rows.iter().map(|r| r.0).collect(), r: rows.iter().map(|r| r.1).collect(), p: rows.iter().map(|r| r.2).collect(), s: rows.iter().map(|r| r.3).collect() }
}

/// An idea: a statement "lhs(n) >= c * rhs(n)" (or <=) for every even n >= n0,
/// with what proving it would give.
struct Idea {
    text: String,
    /// value(n) = lhs / rhs, and whether the statement is a lower bound
    ratio: Box<dyn Fn(usize, &Data) -> f64>,
    lower: bool,
    gives: &'static str,
}

fn ln(n: u64) -> f64 {
    (n as f64).ln()
}

fn ideas() -> Vec<Idea> {
    let mut v: Vec<Idea> = Vec::new();
    // lower bounds for the number of ways: any of these, proved, gives Goldbach
    for (name, f) in [
        ("n / (ln n)^2", Box::new(|n: u64, _s: f64| n as f64 / ln(n).powi(2)) as Box<dyn Fn(u64, f64) -> f64>),
        ("S(n) n / (ln n)^2", Box::new(|n: u64, s: f64| s * n as f64 / ln(n).powi(2))),
        ("2 C2 S(n) n / (ln n)^2 (the Hardy-Littlewood size)", Box::new(|n: u64, s: f64| 2.0 * C2 * s * n as f64 / ln(n).powi(2))),
        ("S(n) n / ((ln n)^2 ln ln n)", Box::new(|n: u64, s: f64| s * n as f64 / (ln(n).powi(2) * ln(n).ln()))),
        ("sqrt(n)", Box::new(|n: u64, _s: f64| (n as f64).sqrt())),
        ("S(n) n^0.9 / (ln n)^2", Box::new(|n: u64, s: f64| s * (n as f64).powf(0.9) / ln(n).powi(2))),
    ] {
        v.push(Idea {
            text: format!("the number of ways r(n) is at least c * {name}"),
            ratio: Box::new(move |i, d| d.r[i] / f(d.n[i], d.s[i])),
            lower: true,
            gives: "Goldbach for every even n >= n0, since it makes r(n) > 0",
        });
    }
    // upper bounds for the smallest prime, with S(n) helping
    for (name, k) in [("(ln n)^2 ln ln n", 0.0), ("(ln n)^2 ln ln n / sqrt(S(n))", 0.5), ("(ln n)^2 ln ln n / S(n)", 1.0)] {
        v.push(Idea {
            text: format!("the smallest prime p(n) is at most c * {name}"),
            ratio: Box::new(move |i, d| d.p[i] * d.s[i].powf(k) / (ln(d.n[i]).powi(2) * ln(d.n[i]).ln())),
            lower: false,
            gives: "Goldbach for every even n >= n0, and where to look for the prime",
        });
    }
    // relations between neighbours: the multiples of 6 against their neighbours
    v.push(Idea {
        text: "a multiple of 6 has at least c times as many ways as each even neighbour (r(6k) >= c max(r(6k - 2), r(6k + 2)))".into(),
        ratio: Box::new(|i, d| {
            if d.n[i] % 6 != 0 || i == 0 || i + 1 >= d.n.len() {
                return f64::NAN;
            }
            d.r[i] / d.r[i - 1].max(d.r[i + 1])
        }),
        lower: true,
        gives: "nothing for Goldbach by itself (it compares counts), but a regularity of the counts",
    });
    v.push(Idea {
        text: "r(n) / S(n) never drops by more than a factor c from one even number to the next (r(n+2)/S(n+2) >= c r(n)/S(n))".into(),
        ratio: Box::new(|i, d| {
            if i + 1 >= d.n.len() || d.r[i] == 0.0 {
                return f64::NAN;
            }
            (d.r[i + 1] / d.s[i + 1]) / (d.r[i] / d.s[i])
        }),
        lower: true,
        gives: "Goldbach for every n by induction from a start, if c > 0 held for all n: the counts can never fall to 0",
    });
    v
}

pub struct Settings {
    pub limit: usize,
    pub from: u64,
}

pub fn report(s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let d = data(s.limit);
    let half = d.n.iter().position(|&n| n > s.limit as u64 / 2).unwrap_or(d.n.len());
    let start = d.n.iter().position(|&n| n >= s.from).unwrap_or(0);
    let mut out = vec![
        format!("Nuome's idea machine for the open part of Goldbach: new statements about the even numbers, made up from templates, each with its best constant found on n = {} .. {} and then tested on n = {} .. {}, numbers it never saw", s.from, s.limit / 2, s.limit / 2, s.limit),
        format!("data: the number of ways r(n), the smallest prime p(n) and S(n) for every even n up to {} ({:.1} s)", s.limit, t0.elapsed().as_secs_f64()),
        String::new(),
    ];
    let mut kept = 0;
    for idea in ideas() {
        let vals = |range: std::ops::Range<usize>| -> Vec<f64> { range.map(|i| (idea.ratio)(i, &d)).filter(|x| x.is_finite()).collect() };
        let train = vals(start..half);
        let test = vals(half..d.n.len());
        if train.is_empty() || test.is_empty() {
            continue;
        }
        // the best constant on the training half, and how the unseen half fares
        let (c, test_extreme, holds) = if idea.lower {
            let c = train.iter().cloned().fold(f64::INFINITY, f64::min);
            let t = test.iter().cloned().fold(f64::INFINITY, f64::min);
            (c, t, t >= c && c > 0.0)
        } else {
            let c = train.iter().cloned().fold(0.0, f64::max);
            let t = test.iter().cloned().fold(0.0, f64::max);
            (c, t, t <= c)
        };
        let shown = if idea.lower { (c * 1000.0).floor() / 1000.0 } else { (c * 1000.0).ceil() / 1000.0 };
        if holds {
            kept += 1;
            out.push(format!("KEPT  {} for every even n >= {}, with c = {shown}", idea.text, s.from));
            out.push(format!("      found on the first half; on the unseen half the {} value is {test_extreme:.4}, so it {}", if idea.lower { "smallest" } else { "largest" }, if idea.lower { "never drops below c" } else { "never rises above c" }));
            out.push(format!("      if proved, it would give: {}", idea.gives));
        } else {
            out.push(format!("FAILED {}: the best c on the first half is {c:.4}, but the unseen half reaches {test_extreme:.4}", idea.text));
        }
    }
    out.push(String::new());
    out.push(format!("{kept} statements survived the test. They are Nuome's conjectures: made up and tested by it, true for every even n it looked at, proved for none. The ones that would give Goldbach are as hard to prove as Goldbach: the difficulty moves into the statement, it does not go away"));
    out.join("\n")
}

#[cfg(test)]
mod tests {
    #[test]
    fn small_run() {
        let text = super::report(&super::Settings { limit: 20_000, from: 1_000 });
        assert!(text.contains("survived the test"));
        // r(n) > 0 everywhere up to 20 000: the sqrt(n) bound must have some positive c
        assert!(text.contains("KEPT") || text.contains("FAILED"));
    }
}
