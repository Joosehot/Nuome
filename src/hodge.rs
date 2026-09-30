//! The Hodge conjecture on Fermat varieties X: x_0^d + ... + x_{n+1}^d = 0
//! in projective space (dimension n).
//!
//! 1. Hodge numbers of a smooth degree-d hypersurface (Griffiths): the
//!    primitive h^{n-p,p} is the number of monomials x^a with every
//!    exponent 0..d-2 and total degree (p+1) d - (n+2), a basis of the
//!    Jacobian ring of the Fermat polynomial.
//! 2. Hodge classes of the Fermat variety (Shioda): the primitive middle
//!    cohomology splits into lines indexed by a = (a_0..a_{n+1}) with
//!    1 <= a_i <= d-1 and sum a_i = 0 mod d; the line is a Hodge class
//!    (type (n/2, n/2)) exactly when sum_i (t a_i mod d) = d (n/2 + 1) for
//!    every t prime to d.
//! 3. Algebraic ones: when the indices of a pair up with a_i + a_j = 0 mod d,
//!    the class is spanned by linear subspaces lying on X (Shioda, Ran),
//!    so it is algebraic. If every Hodge class pairs up this way, the Hodge
//!    conjecture holds for that X. For surfaces it is a theorem anyway
//!    (Lefschetz (1,1)), which makes them a check of the counting.

/// Primitive Hodge numbers h^{n-p,p} (p = 0..=n) of a degree-d hypersurface of dimension n.
pub fn hodge_numbers(n: usize, d: usize) -> Vec<u64> {
    let vars = n + 2;
    let top = vars * (d - 2);
    // ways[s] = number of exponent vectors with entries 0..d-2 summing to s
    let mut ways = vec![0u64; top + 1];
    ways[0] = 1;
    for _ in 0..vars {
        let mut next = vec![0u64; top + 1];
        for (s, &w) in ways.iter().enumerate() {
            if w == 0 {
                continue;
            }
            for e in 0..=d - 2 {
                if s + e <= top {
                    next[s + e] += w;
                }
            }
        }
        ways = next;
    }
    (0..=n)
        .map(|p| {
            let target = (p + 1) * d;
            if target < vars {
                return 0;
            }
            ways.get(target - vars).copied().unwrap_or(0)
        })
        .collect()
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Can the entries be paired so that each pair sums to 0 mod d?
fn pairs_up(a: &mut Vec<usize>, d: usize) -> bool {
    let Some(first) = a.pop() else { return true };
    for i in 0..a.len() {
        if (first + a[i]) % d == 0 {
            let other = a.remove(i);
            if pairs_up(a, d) {
                a.insert(i, other);
                a.push(first);
                return true;
            }
            a.insert(i, other);
        }
    }
    a.push(first);
    false
}

pub struct Fermat {
    pub n: usize,
    pub d: usize,
    /// primitive Hodge classes
    pub hodge: u64,
    /// of them, the ones spanned by linear subspaces
    pub linear: u64,
}

/// Count the primitive Hodge classes of the Fermat variety of dimension n
/// (even) and degree d, and how many pair up.
pub fn fermat(n: usize, d: usize) -> Fermat {
    let k = n + 2;
    let units: Vec<usize> = (1..d).filter(|&t| gcd(t, d) == 1).collect();
    let target = d * (n / 2 + 1);
    let mut a = vec![1usize; k];
    let (mut hodge, mut linear) = (0u64, 0u64);
    loop {
        if a.iter().sum::<usize>() % d == 0 && units.iter().all(|&t| a.iter().map(|&x| t * x % d).sum::<usize>() == target) {
            hodge += 1;
            if pairs_up(&mut a.clone(), d) {
                linear += 1;
            }
        }
        // next vector in {1..d-1}^k
        let mut i = 0;
        loop {
            if i == k {
                return Fermat { n, d, hodge, linear };
            }
            if a[i] < d - 1 {
                a[i] += 1;
                break;
            }
            a[i] = 1;
            i += 1;
        }
    }
}

pub struct Settings {
    pub surface_degrees: Vec<usize>,
    pub fourfold_degrees: Vec<usize>,
}

fn is_prime(d: usize) -> bool {
    d >= 2 && (2..d).take_while(|x| x * x <= d).all(|x| d % x != 0)
}

pub fn report(s: &Settings) -> Vec<String> {
    let mut out = Vec::new();
    // 1. Hodge numbers, checked against known varieties
    let known = [
        ("K3 surface (quartic in P^3)", 2, 4, vec![1u64, 19, 1]),
        ("cubic surface", 2, 3, vec![0, 6, 0]),
        ("quintic threefold (the Calabi-Yau of string theory)", 3, 5, vec![1, 101, 101, 1]),
        ("cubic fourfold", 4, 3, vec![0, 1, 20, 1, 0]),
    ];
    let checks: Vec<String> = known
        .iter()
        .map(|(name, n, d, want)| {
            let got = hodge_numbers(*n, *d);
            format!("{name}: primitive {:?} {}", got, if &got == want { "= known" } else { "DIFFERS from the known values" })
        })
        .collect();
    out.push(format!("Hodge numbers of hypersurfaces by Griffiths' Jacobian ring (one line per variety, h^(n,0) .. h^(0,n) of the primitive part): {}", checks.join("; ")));
    // 2. surfaces: Lefschetz makes it a theorem, and the count must agree
    let mut rows = Vec::new();
    for &d in &s.surface_degrees {
        let f = fermat(2, d);
        let rho = f.hodge + 1; // plus the hyperplane class
        let formula = if is_prime(d) { format!(", formula 3(d-1)(d-2)+1 = {}", 3 * (d - 1) * (d - 2) + 1) } else { String::new() };
        rows.push(format!("d={d}: Picard number {rho}{formula}"));
    }
    out.push(format!("Fermat surfaces (the Hodge conjecture holds there by the Lefschetz (1,1) theorem, so this checks the counting): {}", rows.join("; ")));
    // 3. fourfolds: the real test
    let mut settled = Vec::new();
    let mut open = Vec::new();
    for &d in &s.fourfold_degrees {
        let f = fermat(4, d);
        let h22 = hodge_numbers(4, d)[2];
        let line = format!("d={d}: {} of the {h22} primitive (2,2) classes are Hodge classes, {} of them spanned by linear subspaces", f.hodge, f.linear);
        if f.hodge == f.linear {
            settled.push(line);
        } else {
            open.push(format!("{line} ({} not reached by linear subspaces)", f.hodge - f.linear));
        }
    }
    out.push(format!("Fermat fourfolds, every Hodge class checked: the Hodge conjecture HOLDS for these, each Hodge class being a combination of linear subspaces lying on X: {}", if settled.is_empty() { "none".into() } else { settled.join("; ") }));
    if !open.is_empty() {
        out.push(format!("  and where linear subspaces are not enough (other algebraic cycles may still give the rest; this method does not settle them): {}", open.join("; ")));
    }
    out.push("these are particular varieties, where the conjecture is checked (or known) one by one; the Millennium problem asks for every smooth projective variety".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hodge_numbers() {
        assert_eq!(hodge_numbers(2, 4), vec![1, 19, 1]); // K3: h11 = 19 + 1
        assert_eq!(hodge_numbers(3, 5), vec![1, 101, 101, 1]); // quintic threefold
        assert_eq!(hodge_numbers(4, 3), vec![0, 1, 20, 1, 0]); // cubic fourfold
    }

    #[test]
    fn fermat_picard_numbers() {
        assert_eq!(fermat(2, 4).hodge + 1, 20); // the Fermat quartic has Picard number 20
        assert_eq!(fermat(2, 5).hodge + 1, 37); // prime degree: 3(d-1)(d-2) + 1
        let cubic = fermat(4, 3);
        assert_eq!((cubic.hodge, cubic.linear), (20, 20));
    }
}
