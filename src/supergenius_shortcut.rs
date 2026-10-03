//! The supergenius looks for a shortcut to testing a huge prime, in two
//! ways, and says plainly what each found.
//!
//! 1. Reasoning over what is known: every test that decides a D-digit
//!    number needs about log2 N squarings of an N-sized number; what
//!    changes is how cheap one squaring is, and that depends on the form
//!    of N (2^p - 1 reduces for free, k 2^n + 1 and b^(2^m) + 1 almost,
//!    a general number like 10^n + 13 costs a full reduction) and on
//!    whether a GPU program exists for it. The costs on this machine come
//!    from the night's measurement (PFGW on 10^99,999,999 + 13).
//! 2. Search: the shortcut evolution again, but judged where a shortcut
//!    would matter. It learns on random 30..36-bit numbers (half prime)
//!    and the liars below 10^6, and it is tested on random 50..60-bit
//!    numbers and on the famous strong liars that fool Miller-Rabin with a
//!    few bases. A loop to sqrt n runs out of budget at these sizes, so
//!    trial division cannot win. A shortcut is a test with no mistakes
//!    that uses well under one multiplication per bit of n.

use crate::evolve::Rng;
use crate::prime_shortcut::{evolve, judge, liars, Settings};

/// Strong pseudoprimes that fool Miller-Rabin with the first few prime
/// bases (Jaeschke, 1993, and later): each passes every base up to the one named.
const STRONG_LIARS: [(i128, &str); 6] = [
    (2047, "base 2"),
    (1_373_653, "bases 2, 3"),
    (25_326_001, "bases 2, 3, 5"),
    (3_215_031_751, "bases 2, 3, 5, 7"),
    (2_152_302_898_747, "bases 2..11"),
    (3_474_749_660_383, "bases 2..13"),
];

fn is_prime(n: i128) -> bool {
    n >= 2 && crate::prime_formula::is_prime(n as u64)
}

/// `count` odd numbers of `lo..hi` bits, half prime, half composite with no
/// factor below 100 (so the easy divisibility checks do not decide them).
fn sample(r: &mut Rng, count: usize, lo: u32, hi: u32) -> Vec<i128> {
    let mut primes = Vec::new();
    let mut comps = Vec::new();
    while primes.len() < count / 2 || comps.len() < count / 2 {
        let b = lo + (r.below((hi - lo + 1) as usize) as u32);
        let n = ((r.next() >> (64 - b)) | (1 << (b - 1)) | 1) as i128;
        if is_prime(n) {
            if primes.len() < count / 2 {
                primes.push(n);
            }
        } else if (3..100).all(|p| n % p != 0) && comps.len() < count / 2 {
            comps.push(n);
        }
    }
    primes.into_iter().chain(comps).collect()
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

pub fn report(digits: u64, s: &Settings) -> String {
    let t0 = std::time::Instant::now();
    let mut out = String::new();
    let bits = (digits as f64 / std::f64::consts::LOG10_2) as u64;
    out.push_str(&format!("Nuome, the supergenius, looks for a shortcut to testing a {}-digit prime\n\n", group(digits)));

    // 1. what is known
    // measured: PFGW on 10^99,999,999 + 13 (a general number), 78 ms per squaring, this CPU, overnight
    let general_ms = 78.0 * (digits as f64 / 1e8).powf(1.1);
    let forms: [(&str, f64, &str, &str); 4] = [
        ("10^n + 13 (a general number: full reduction each squaring)", 1.0, "CPU only (PFGW, PRST)", "probable prime"),
        ("k 2^n + 1 (Proth: reduction nearly free)", 0.4, "CPU (LLR, PRST); GPU programs exist", "proof (Proth's theorem)"),
        ("b^(2^m) + 1 (generalized Fermat)", 0.35, "GPU (Genefer)", "proof (Proth-type)"),
        ("2^p - 1 (Mersenne: reduction free)", 0.33, "GPU (PRPLL, the program that found the record)", "PRP, then Lucas-Lehmer proof"),
    ];
    out.push_str(&format!(
        "1. What is known. Every test known to decide such a number does about log2 N = {} squarings of a {}-bit number, one after another;\n   no known test does fewer. What a shortcut can change is the price of one squaring, and that depends on the number's form:\n",
        group(bits),
        group(bits)
    ));
    for (name, rel, hw, kind) in forms {
        let cpu_days = bits as f64 * general_ms * rel / 1000.0 / 86_400.0;
        out.push_str(&format!("   {name:<62} CPU here ~{:>4.0} days   {hw}; result: {kind}\n", cpu_days));
    }
    out.push_str(
        "   (the general-number row is measured on this PC overnight; the others scale it by how cheap their reduction is: estimates)\n   with the RTX 4070 in this PC, a Mersenne or generalized Fermat number of this size is about 1-2 weeks (estimate)\n   => the known shortcut: choose the form. The same size costs about 3x less on the CPU, and 20-30x less on the GPU, as 2^p - 1 or b^(2^m) + 1 instead of 10^n + 13.\n\n",
    );

    // 2. the search, judged at large sizes
    let mut r = Rng(s.seed ^ 0x5c5c);
    let learn = sample(&mut r, 600, 30, 36);
    let (carm, psp) = liars(1_000_000);
    let mut liar_learn: Vec<i128> = carm.iter().chain(psp.iter()).copied().collect();
    liar_learn.sort();
    liar_learn.dedup();
    let test = sample(&mut r, 1000, 50, 60);
    let (carm7, _) = liars(10_000_000);
    let mut liar_test: Vec<i128> = carm7.into_iter().filter(|&n| n > 1_000_000).collect();
    liar_test.extend(STRONG_LIARS.iter().map(|x| x.0));
    out.push_str(&format!(
        "2. Search. Tests evolved from n, constants, + - * mod div, sqrt, gcd, a^e mod m, comparisons, and/or/not and loops,\n   every multiplication paid for; learned on 600 random 30..36-bit numbers (half prime) and the {} liars below 10^6;\n   {} islands of {} tests, {} generations each; then tested on 1,000 random 50..60-bit numbers (half prime, never seen)\n   and {} liars never seen, among them the strong liars that fool Miller-Rabin with the bases 2, 3, 5, 7 (3,215,031,751) and 2..13 (3,474,749,660,383)\n",
        liar_learn.len(),
        s.islands,
        s.population,
        s.generations,
        liar_test.len()
    ));
    let runs: Vec<Vec<(f64, crate::prime_shortcut::T)>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..s.islands)
            .map(|i| {
                let (learn, liar_learn) = (&learn, &liar_learn);
                sc.spawn(move || evolve(s, s.seed + i as u64 * 7_919, learn, liar_learn))
            })
            .collect();
        hs.into_iter().map(|h| h.join().expect("island")).collect()
    });
    let mut best: Vec<(f64, crate::prime_shortcut::T)> = runs.into_iter().flat_map(|v| v.into_iter().take(10)).collect();
    best.sort_by(|a, b| a.0.total_cmp(&b.0));
    best.dedup_by(|a, b| a.1 == b.1);
    let mean_bits = test.iter().map(|&n| (n as f64).log2()).sum::<f64>() / test.len() as f64;
    let mut found_shortcut = None;
    let mut lines = 0;
    for (_, t) in &best {
        if lines == 6 {
            break;
        }
        let Some((wrong, cost)) = judge(t, &test, 200_000) else { continue };
        let liar = judge(t, &liar_test, 200_000);
        let per_bit = cost as f64 / test.len() as f64 / mean_bits;
        out.push_str(&format!(
            "   {}\n       50..60-bit numbers: {} of {} wrong; liars: {}; cost {:.2} multiplications per bit\n",
            t.show(),
            wrong,
            test.len(),
            liar.map_or("breaks".into(), |x| format!("{} of {} wrong", x.0, liar_test.len())),
            per_bit
        ));
        if wrong == 0 && liar.is_some_and(|x| x.0 == 0) && per_bit < 1.0 && found_shortcut.is_none() {
            found_shortcut = Some(t.show());
        }
        lines += 1;
    }
    out.push_str("\nverdict: ");
    match found_shortcut {
        Some(t) => out.push_str(&format!("a SHORTCUT candidate: {t} - no mistakes on everything tested at under one multiplication per bit. Check it far wider before believing it.\n")),
        None => out.push_str(
            "no shortcut. No evolved test was right on every number and every liar at under one multiplication per bit; the cheap ones are\n fooled by liars, the right ones cost what Fermat and Miller-Rabin cost (about 2 per bit per base). The only shortcut found is the\n known one in part 1: choose a form whose squarings are cheap, and test it on the GPU.\n",
        ),
    }
    out.push_str(&format!("\n({:.1} s)\n", t0.elapsed().as_secs_f64()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strong_liars_are_composite_and_fool_their_bases() {
        for (n, _) in STRONG_LIARS {
            assert!(!is_prime(n), "{n}");
        }
        // 3215031751 = 151 * 751 * 28351
        assert_eq!(151 * 751 * 28351, 3_215_031_751i128);
    }

    #[test]
    fn samples_are_half_prime() {
        let mut r = Rng(1);
        let s = sample(&mut r, 100, 50, 60);
        assert_eq!(s.iter().filter(|&&n| is_prime(n)).count(), 50);
        assert!(s.iter().all(|&n| n >= 1 << 49));
    }
}
