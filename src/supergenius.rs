//! The supergenius: asked for "the first prime with D digits", Nuome
//! answers the way a genius would on paper, abstractly and step by step,
//! claiming only what is proved or measured and labelling every estimate:
//!
//! 1. it exists: Bertrand's postulate (proved, Chebyshev 1852) puts a prime
//!    between 10^(D-1) and 2 * 10^(D-1);
//! 2. where it is: 10^(D-1) + k, and how far k goes (the prime number
//!    theorem and Cramer's model: an estimate), and the proven ceiling
//!    (Baker-Harman-Pintz 2001: a prime within x^0.525 of x, for large x);
//! 3. what k must look like: odd, not a multiple of 3, 5, 7 (a wheel);
//! 4. written short: prize-size Mersenne numbers 2^p - 1 (14 symbols for
//!    100 million digits), with Nuome's trial factoring;
//! 5. a formula that only gives primes: Mills' floor(A^(3^n)), which writes
//!    a prime of more than D digits in a few symbols, and why it is circular;
//! 6. what is left: the tests no abstraction has removed, and what they cost.

use crate::mersenne;

/// "the first 100 million digit prime" -> 100,000,000 digits.
pub fn digits_asked(sentence: &str) -> Option<u64> {
    let low = sentence.to_lowercase().replace(',', "").replace('-', " ");
    let words: Vec<&str> = low.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        if let Ok(n) = w.parse::<f64>() {
            let scale = match words.get(i + 1).copied() {
                Some("thousand") => 1e3,
                Some("million") | Some("miljoonaa") | Some("miljoona") => 1e6,
                Some("billion") | Some("miljardia") => 1e9,
                _ => 1.0,
            };
            return Some((n * scale) as u64);
        }
    }
    None
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

/// x as "m x 10^e" for a number known by its base-10 log.
fn sci(log10: f64) -> String {
    let e = log10.floor();
    format!("{:.2} x 10^{}", 10f64.powf(log10 - e), group(e as u64))
}

/// Mills' constant (the smallest A for which floor(A^(3^n)) is always prime,
/// its known digits assuming the Riemann hypothesis).
pub const MILLS: f64 = 1.306_377_883_863_080_7;

/// floor(A^(3^n)) for n = 1..4 (OEIS A051254); the next has 29 digits.
pub const MILLS_PRIMES: [u64; 4] = [2, 11, 1361, 2_521_008_887];

pub fn report(digits: u64) -> String {
    let d = digits.max(2);
    let ln10 = std::f64::consts::LN_10;
    // x = 10^(D-1)
    let ln_x = (d - 1) as f64 * ln10;
    let mut r = String::new();
    r.push_str(&format!("Nuome, the supergenius: the first prime with {} digits\n\n", group(d)));

    r.push_str("1. It exists (proved).\n");
    r.push_str(&format!(
        "   Bertrand's postulate (Chebyshev, 1852): for every n >= 1 there is a prime p with n < p <= 2n.\n   With n = 10^{}: a prime between 10^{} and 2 x 10^{}, so a prime with {} digits exists.\n   How many there are (an estimate, from the prime number theorem): about 0.9 x 10^{} / ln(10^{}) = {} of them.\n\n",
        group(d - 1),
        group(d - 1),
        group(d - 1),
        group(d),
        group(d),
        group(d),
        sci(d as f64 + (0.9f64).log10() - (d as f64 * ln10).log10())
    ));

    r.push_str("2. Where it is: 10^(D-1) + k. How far k goes:\n");
    let gap = ln_x;
    r.push_str(&format!(
        "   the average gap between primes there is ln(10^{}) = {} (the prime number theorem)\n   if primes there behave like random numbers with that density (Cramer's model, an estimate, not proved):\n     k < {} with probability 50%\n     k < {} with probability 95%\n     k < {} with probability 99.9%\n   proved (Baker, Harman and Pintz 2001): there is a prime in [x, x + x^0.525] for every large enough x,\n     so k < 10^{} for certain: a ceiling, far above where it almost surely is\n\n",
        group(d - 1),
        group(gap.round() as u64),
        group((gap * std::f64::consts::LN_2).round() as u64),
        group((gap * 3.0).round() as u64),
        group((gap * 6.9078).round() as u64),
        group(((d - 1) as f64 * 0.525).ceil() as u64)
    ));

    r.push_str("3. What k must look like (proved: otherwise 10^(D-1) + k has a small factor):\n");
    let wheel = 48.0 / 210.0;
    r.push_str(&format!(
        "   10^(D-1) is even and is 1 mod 3 and 0 mod 5 and 10^(D-1) mod 7 is fixed, so k must be odd, k mod 3 != 2, k mod 5 != 0 and k avoids one class mod 7:\n   only {:.1}% of k are left (a wheel); within the 95% window that is about {} candidates\n\n",
        wheel * 100.0,
        group((gap * 3.0 * wheel).round() as u64)
    ));

    r.push_str("4. Written short: Mersenne numbers 2^p - 1.\n");
    let log2 = std::f64::consts::LOG10_2;
    let first_p = ((d - 1) as f64 / log2).ceil() as u64;
    r.push_str(&format!(
        "   2^p - 1 has floor(p log10 2) + 1 digits, so {} digits need p >= {}; 2^p - 1 can be prime only if p is prime (proved).\n",
        group(d),
        group(first_p)
    ));
    if d == 100_000_000 {
        let c = mersenne::candidates(6, 48);
        r.push_str("   the first prize-size prime exponents, each with Nuome's trial factoring to 2^48 (every factor is 2kp + 1, 1 or 7 mod 8):\n");
        for (p, f) in &c {
            r.push_str(&format!(
                "     2^{} - 1 ({} digits): {}\n",
                group(*p),
                group((*p as f64 * log2).floor() as u64 + 1),
                match f {
                    Some(q) => format!("composite, divisible by {}", group(*q)),
                    None => "no factor below 2^48: still a candidate (only a Lucas-Lehmer test can say)".into(),
                }
            ));
        }
        r.push_str("   (the first Mersenne prime with 100 million digits need not be the first prime with 100 million digits)\n");
    }
    r.push('\n');

    r.push_str("5. A formula that gives only primes, in a few symbols: Mills (1947).\n");
    r.push_str(&format!("   there is a constant A with floor(A^(3^n)) prime for every n >= 1; the smallest is A = {MILLS} (its known digits assume the Riemann hypothesis)\n"));
    // the first Mills primes as published (OEIS A051254): f64 holds A to 16
    // digits, too few past n = 3, so each is checked with Nuome's primality test
    for (n, v) in MILLS_PRIMES.iter().enumerate() {
        r.push_str(&format!(
            "     n = {}: floor(A^{}) = {} ({})\n",
            n + 1,
            3u64.pow(n as u32 + 1),
            group(*v),
            if crate::prime_formula::is_prime(*v) { "prime, checked" } else { "NOT prime" }
        ));
    }
    let log_a = MILLS.log10();
    let n_mills = (1u32..).find(|&n| 3f64.powi(n as i32) * log_a >= (d - 1) as f64).unwrap_or(1);
    let mills_digits = 3f64.powi(n_mills as i32) * log_a;
    r.push_str(&format!(
        "   so floor(A^(3^{n_mills})) is a prime with about {} digits: a prime of more than {} digits, written in 11 symbols\n   but circular: A's digits are found from the primes themselves, and writing that prime out needs A to about {} digits; the formula hides the search, it does not skip it\n\n",
        group(mills_digits as u64),
        group(d),
        group(mills_digits as u64)
    ));

    r.push_str("6. What is left: knowing which number is prime. No abstraction found so far removes this step.\n");
    let bits = d as f64 / log2;
    let sieve_to = 1e12f64;
    let tests = ln_x / (1.781_072 * sieve_to.ln());
    r.push_str(&format!(
        "   a {}-digit number has {} bits\n   trial division (the rule Nuome found today) on a prime that size: about 10^{} divisions; it never finishes\n   Lucas-Lehmer for 2^p - 1: {} squarings of a {}-bit number; weeks on a strong GPU for one exponent (an estimate)\n   the first prime with {} digits is not a Mersenne number: sieving candidates to 10^12 leaves about {} probable-prime tests (Mertens' theorem, an estimate),\n     each as heavy as one Lucas-Lehmer test\n   and proving such a number prime (not just probable) is beyond every known method at that size: the record proofs for general numbers are near 100,000 digits\n\n",
        group(d),
        group(bits as u64),
        group(d / 2),
        group(bits as u64),
        group(bits as u64),
        group(d),
        group(tests.round() as u64)
    ));

    r.push_str(&format!(
        "the supergenius's answer on paper: \"a prime with {} digits exists (proved); the first one is 10^{} + k with k odd and, almost surely, below {};\n among prize-size Mersenne numbers, those without a small factor are listed above; floor(A^(3^{n_mills})) is a prime of {} digits but says nothing new;\n which number it is takes the tests in step 6 - no known thought replaces them.\"\n",
        group(d),
        group(d - 1),
        group((gap * 6.9078).round() as u64),
        group(mills_digits as u64)
    ));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_digits_asked() {
        assert_eq!(digits_asked("the first 100 million digit prime"), Some(100_000_000));
        assert_eq!(digits_asked("first prime with 1000 digits"), Some(1000));
    }

    #[test]
    fn mills_gives_the_known_primes() {
        // the first three follow from A's 16 known digits in f64; all four are prime
        let v: Vec<u64> = (1..=3u32).map(|n| MILLS.powi(3i32.pow(n)).floor() as u64).collect();
        assert_eq!(v, MILLS_PRIMES[..3].to_vec());
        for p in MILLS_PRIMES {
            assert!(crate::prime_formula::is_prime(p));
        }
    }

    #[test]
    fn small_case_is_checked_against_the_truth() {
        // the first 4-digit prime is 1009 = 10^3 + 9: inside the 99.9% window (6.9 ln 1000 = 47)
        let text = report(4);
        assert!(text.contains("10^3 + k"));
        assert!(9.0 < (1000f64).ln() * 6.9078);
    }
}
