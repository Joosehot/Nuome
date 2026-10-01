//! General proofs for the open and prize problems: what can be proved for
//! every case, with no number checked, set apart from the computations and
//! from what stays open. Where Nuome's own rules can do a step (an equation,
//! a factorisation, a step of logic), they do it and their checks are shown.
//! Theorems proved by others are named as such, not passed off as Nuome's.

use crate::config::Config;

/// Run one of Nuome's rules and give its answer with how many checks passed.
fn rule(cfg: &Config, sentence: &str) -> String {
    let opts = crate::Options { lenient: false, style: crate::print::Style::Ascii };
    match crate::solve(sentence, cfg, &opts) {
        Ok(s) => {
            let answer = s.text.lines().find(|l| l.starts_with("Answer:")).map_or("?", |l| l.trim_start_matches("Answer:").trim()).to_string();
            let checks = s.text.lines().filter(|l| l.trim_start().starts_with("ok ")).count();
            format!("Nuome's rules: \"{sentence}\" gives {answer} ({checks} checks passed)")
        }
        Err(_) => format!("Nuome's rules could not do \"{sentence}\", so this step is not proved here"),
    }
}

/// The general proofs for a problem, or nothing when it has none here.
pub fn lines(key: &str, cfg: &Config) -> Vec<String> {
    let mut o: Vec<String> = Vec::new();
    match key {
        "eff_prime" => {
            o.push("lemma 1 (only prime exponents): if 2^p - 1 is prime, then p is prime. Proof: if p = ab with 1 < a, b < p, put x = 2^a; then x - 1 divides x^b - 1 = 2^p - 1, because x^b - 1 = (x - 1)(x^(b-1) + ... + x + 1), and 1 < 2^a - 1 < 2^p - 1, so 2^p - 1 is not prime. QED".into());
            o.push(format!("  the factor x - 1, for b = 3: {}", rule(cfg, "factor x^3 - 1")));
            o.push("lemma 2 (the digit count): 2^p - 1 has floor(p log10 2) + 1 digits. Proof: 2^p is never a power of 10 (5 does not divide it), so 2^p - 1 has as many digits as 2^p, which is floor(p log10 2) + 1. QED".into());
            o.push("theorem 3 (such a prime exists): there is a prime with at least 100 million digits. Proof: by Bertrand's postulate (Chebyshev 1852) there is a prime between N and 2N for every N >= 1; take N = 10^(10^8 - 1). QED. So the prize is not about existence, which is proved, but about finding and certifying one".into());
            o.push("theorem used, proved by others: Lucas-Lehmer (Lucas 1878, Lehmer 1930): for an odd prime p, 2^p - 1 is prime exactly when s_(p-2) = 0 mod 2^p - 1, with s_0 = 4 and s_(k+1) = s_k^2 - 2".into());
        }
        "conway99" => {
            o.push("lemma 1 (the size): a graph where every point has k neighbours, joined points share 1 neighbour and others share 2, has 1 + k + k(k - 2)/2 points. Proof: fix a point; its k neighbours each have k - 2 neighbours outside the point and its own triangle partner; each other point is reached exactly 2 times; so there are k(k - 2)/2 others. For k = 14: 1 + 14 + 84 = 99. QED".into());
            o.push("lemma 2 (the eigenvalues): with A the adjacency matrix and J all ones, counting common neighbours gives A^2 = 14 I + A + 2 (J - I - A), so A^2 + A - 12 I = 2 J. On vectors with sum 0 (where J gives 0), every eigenvalue r has r^2 + r - 12 = 0:".into());
            o.push(format!("  {}", rule(cfg, "solve x^2 + x - 12 = 0")));
            o.push("lemma 3 (the multiplicities): the trace of A is 0 (no loops) and A has 99 eigenvalues: 14 once (the all-ones vector), 3 f times and -4 g times, so 14 + 3f - 4g = 0 and f + g = 98:".into());
            o.push(format!("  {}", rule(cfg, "solve 3f - 4g = -14 and f + g = 98")));
            o.push("  both are whole numbers, so the eigenvalue test cannot rule the graph out (the same computation rules out 19 points, where the eigenvalues are irrational and the trace cannot be 0, and 33 points, where the multiplicity comes out as 14.4). QED".into());
            o.push("lemma 4 (local structure): the 14 neighbours of any point form 7 disjoint pairs (triangles through the point). Proof: two joined points share exactly 1 neighbour, so each edge lies in exactly one triangle; the neighbours of a point are therefore matched in pairs. QED".into());
        }
        "riemann" => {
            o.push("lemma 1 (a sign change is a zero on the line): Z(t) = e^(i theta(t)) zeta(1/2 + it) with theta real is real for real t and |Z(t)| = |zeta(1/2 + it)|; Z is continuous, so between two points where Z has opposite signs it is 0 somewhere (intermediate value theorem), and there zeta(1/2 + it) = 0, exactly on the line. QED".into());
            o.push("theorem used, proved by others: the functional equation (Riemann 1859): the zeros in the strip are symmetric about the line Re(s) = 1/2, so a zero off the line would come with a mirror zero; and Turing's method (Turing 1953, Brent 1979) bounds ALL zeros up to a Gram point by the sign changes counted, which is how the computation below closes the count".into());
            o.push("what this gives: the line count equals the strip count up to the height checked, which is a proof for that height; nothing about larger heights".into());
        }
        "p_vs_np" => {
            o.push("lemma 1 (checking is fast): a proposed assignment is checked by reading each clause once, at most (number of literals) steps, linear in the size of the formula. QED".into());
            o.push("lemma 2 (2-SAT is in P): a clause (a or b) is the two implications not a -> b and not b -> a; the formula is unsatisfiable exactly when some variable and its negation lie in one strongly connected component of these implications (Aspvall, Plass and Tarjan 1979), which a graph search finds in linear time. QED (as cited)".into());
            o.push("theorem used, proved by others: Cook-Levin (1971): SAT is NP-complete, so P = NP exactly when SAT has a polynomial algorithm; with q = 'SAT is in P' and e = 'P = NP', Nuome's logic rules check the equivalence goes both ways:".into());
            o.push(format!("  {}", rule(cfg, "prove that ((q implies e) and (e implies q)) implies (q iff e)")));
            o.push("what stays open: whether SAT has a polynomial algorithm. Relativization, natural proofs and algebrization show whole kinds of proof cannot settle it".into());
        }
        "navier_stokes" => {
            o.push("lemma 1 (energy only falls): for a smooth solution on the periodic box, take the dot product of the equation with u and integrate: the nonlinear term gives the integral of (u . grad)(|u|^2/2) = -(div u)|u|^2/2 = 0 and the pressure term the integral of -(div u) p = 0, by incompressibility; what is left is dE/dt = -nu times the integral of |grad u|^2 = -2 nu Omega <= 0 (for divergence-free periodic fields the integrals of |grad u|^2 and |omega|^2 agree). QED. The computation below checks exactly this identity".into());
            o.push("theorems used, proved by others: in two dimensions smooth solutions stay smooth forever (Ladyzhenskaya 1959); in three, a smooth solution can only break down at a time T if the integral of max |omega| up to T is infinite (Beale, Kato and Majda 1984); weak solutions exist (Leray 1934)".into());
            o.push("what stays open: whether every smooth start in three dimensions stays smooth for all time".into());
        }
        "bsd" => {
            o.push("theorems used, proved by others: Hasse (1933): |a_p| <= 2 sqrt(p) for every good prime; the modularity theorem (Wiles, Breuil, Conrad, Diamond, Taylor 1995-2001) gives L(E, s) its functional equation with sign w = +1 or -1, so the order of vanishing at s = 1 is even when w = +1 and odd when w = -1 (the computation below finds w and starts at that parity); Gross-Zagier (1986) and Kolyvagin (1989): when the order of vanishing is 0 or 1, it equals the rank".into());
            o.push("what this gives: for ranks 0 and 1 the conjecture's rank statement is a theorem; for analytic rank 2 or more it is open, and there a computation shows lower derivatives are small, not exactly zero".into());
        }
        "hodge" => {
            o.push("theorems used, proved by others: Lefschetz (1,1) (1924): every Hodge class in degree 2 is algebraic, so the conjecture holds for p = 1; hard Lefschetz then gives p = dim X - 1, so every variety of dimension at most 3 satisfies it; Shioda and Ran: on a Fermat variety, a Hodge class whose index pairs up (a_i + a_j = 0 mod d) is spanned by linear subspaces, hence algebraic".into());
            o.push("lemma (what the computation proves): if every Hodge class of a Fermat fourfold pairs up, every Hodge class is algebraic, so the conjecture holds for that variety. QED from the theorems above; the computation below finds which degrees have this property".into());
        }
        "yang_mills" => {
            o.push("lemma 1 (the plaquette is between -1 and 1): an SU(2) matrix is a unit quaternion (a0, a1, a2, a3), and Tr U / 2 = a0 with a0^2 + a1^2 + a2^2 + a3^2 = 1, so -1 <= Tr U / 2 <= 1 for every plaquette. QED".into());
            o.push("lemma 2 (small coupling): with the weight exp(beta a0) and the Haar measure sqrt(1 - a0^2) da0, the average of a0 is I_2(beta) / I_1(beta) = beta/4 - beta^3/96 + ...; the computation below matches this series at small beta. (Expansion of Bessel functions, standard)".into());
            o.push("what stays open: building the quantum theory in the continuum with all the axioms and proving its mass gap; lattice results are evidence for it".into());
        }
        _ => {}
    }
    if o.is_empty() {
        return o;
    }
    let mut out = vec!["general proofs (for every case; no number is checked):".to_string()];
    out.extend(o.into_iter().map(|l| format!("  {l}")));
    out.push(String::new());
    out.push("computed (not a general proof):".into());
    out
}
