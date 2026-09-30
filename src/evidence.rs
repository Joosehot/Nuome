//! Attempts at open problems that can be computed on, never proved: every
//! case up to a limit (from rules.toml) is worked out and the result is
//! reported as evidence for those cases only. A finite check can refute a
//! conjecture (a counterexample) but never prove it for all numbers.

/// Every even number 4..=limit as a sum of two primes. Returns report lines.
pub fn goldbach(limit: u64) -> Vec<String> {
    let n = limit as usize;
    let mut composite = vec![false; n + 1];
    let mut primes = Vec::new();
    for i in 2..=n {
        if !composite[i] {
            primes.push(i);
            let mut j = i * i;
            while j <= n {
                composite[j] = true;
                j += i;
            }
        }
    }
    let is_prime = |k: usize| k >= 2 && !composite[k];
    // the hardest case: the even number whose smallest usable prime is largest
    let (mut worst_n, mut worst_p) = (4, 2);
    let mut e = 4;
    while e <= n {
        match primes.iter().find(|&&p| is_prime(e - p)) {
            Some(&p) => {
                if p > worst_p {
                    (worst_n, worst_p) = (e, p);
                }
            }
            None => return vec![format!("counterexample: {e} is not a sum of two primes, so the conjecture is false")],
        }
        e += 2;
    }
    vec![
        format!("checked every even number from 4 to {limit}: each is a sum of two primes ({} primes used)", primes.len()),
        format!("hardest case in that range: {worst_n} = {worst_p} + {}, where no smaller prime works", worst_n - worst_p),
        "this covers those numbers only; it is evidence, not a proof for all even numbers".into(),
    ]
}

/// Every starting number 1..=limit run until it reaches 1. Returns report lines.
pub fn collatz(limit: u64) -> Vec<String> {
    let n = limit as usize;
    let mut steps = vec![0u32; n + 1];
    let (mut longest_n, mut longest, mut peak_n, mut peak) = (1usize, 0u32, 1usize, 1u128);
    for start in 2..=n {
        let mut x = start as u128;
        let mut count = 0u32;
        let mut top = x;
        // run until the orbit drops to a number already known to reach 1
        while x >= start as u128 {
            x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
            count += 1;
            top = top.max(x);
            if count > 100_000 {
                return vec![format!("{start} did not reach a smaller number within 100000 steps: a possible counterexample, needs a closer look")];
            }
        }
        steps[start] = count + steps[x as usize];
        if steps[start] > longest {
            (longest_n, longest) = (start, steps[start]);
        }
        if top > peak {
            (peak_n, peak) = (start, top);
        }
    }
    vec![
        format!("checked every starting number from 1 to {limit}: each reaches 1"),
        format!("longest run in that range: {longest_n} takes {longest} steps; highest point: {peak_n} climbs to {peak}"),
        "this covers those numbers only; it is evidence, not a proof for all numbers".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_ranges() {
        assert!(goldbach(1000)[0].starts_with("checked every even number from 4 to 1000"));
        assert!(collatz(30)[1].contains("27 takes 111 steps"));
    }
}
