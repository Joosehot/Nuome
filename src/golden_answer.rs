//! Golden Answer: a mathematical question answered in the kind the asker
//! chooses.
//!
//! - logical: derived from the question itself, every step checkable (a
//!   divisor shown and multiplied back, a witness found, a count made, an
//!   equation solved by Nuome's rules and substituted back, a proof written
//!   out). It claims to be right because it shows why.
//! - theoretical: a claim from results already known, each one named with
//!   who proved or verified it. It is right as far as those results are.
//! - abstract: an answer that is probably right, with the reasoning behind
//!   it (densities, estimates, searches, tests that can be fooled). It
//!   never claims to be right always.
//!
//! When the chosen kind has no answer, Golden Answer says so and why; it
//! never slips into another kind.

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Logical,
    Theoretical,
    Abstract,
}

pub const KINDS: [Kind; 3] = [Kind::Logical, Kind::Theoretical, Kind::Abstract];

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim().to_lowercase().as_str() {
            "logical" | "looginen" => Some(Kind::Logical),
            "theoretical" | "teoreettinen" => Some(Kind::Theoretical),
            "abstract" | "abstrakti" => Some(Kind::Abstract),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Logical => "logical",
            Kind::Theoretical => "theoretical",
            Kind::Abstract => "abstract",
        }
    }
    /// What an answer of this kind claims.
    pub fn claim(self) -> &'static str {
        match self {
            Kind::Logical => "proved: every step below can be checked",
            Kind::Theoretical => "from known results: right as far as the results named below are",
            Kind::Abstract => "a guess with its reasons: probably right, not claimed to be right always",
        }
    }
}

pub struct Answer {
    pub kind: Kind,
    pub question: String,
    /// The answer in a few words ("prime", "168", "yes", "open"); None when
    /// this kind has no answer to the question.
    pub short: Option<String>,
    /// The reasoning, line by line (or why there is no answer).
    pub lines: Vec<String>,
    /// Theoretical answers: the known results the claim rests on.
    pub sources: Vec<String>,
}

impl Answer {
    fn new(kind: Kind, question: &str) -> Answer {
        Answer { kind, question: question.to_string(), short: None, lines: Vec::new(), sources: Vec::new() }
    }
    fn says(mut self, short: impl Into<String>) -> Answer {
        self.short = Some(short.into());
        self
    }
    fn line(mut self, l: impl Into<String>) -> Answer {
        self.lines.push(l.into());
        self
    }
    fn source(mut self, s: impl Into<String>) -> Answer {
        self.sources.push(s.into());
        self
    }
    fn none(self, why: impl Into<String>) -> Answer {
        let mut a = self;
        a.short = None;
        a.lines = vec![why.into()];
        a.sources.clear();
        a
    }

    pub fn render(&self) -> String {
        let mut out = format!("Golden Answer ({}) to \"{}\"\n", self.kind.name(), self.question);
        match &self.short {
            None => {
                out.push_str(&format!("no {} answer\n", self.kind.name()));
                for l in &self.lines {
                    out.push_str(&format!("  {l}\n"));
                }
            }
            Some(s) => {
                out.push_str(&format!("answer: {s}\n"));
                out.push_str(&format!("claim: {}\n", self.kind.claim()));
                if !self.lines.is_empty() {
                    out.push_str("why:\n");
                    for (i, l) in self.lines.iter().enumerate() {
                        out.push_str(&format!("  {}. {l}\n", i + 1));
                    }
                }
                if !self.sources.is_empty() {
                    out.push_str("known results:\n");
                    for s in &self.sources {
                        out.push_str(&format!("  - {s}\n"));
                    }
                }
            }
        }
        out
    }
}

// ───────────────────────── the questions ─────────────────────────

/// A number in a question: its value when it is small enough to hold, and
/// its form b^e + c when it was written that way.
#[derive(Clone, Debug)]
pub struct Num {
    pub text: String,
    pub value: Option<BigUint>,
    pub form: Option<(u64, u64, i64)>,
    /// log10 of the number
    pub size: f64,
}

impl Num {
    fn small(&self) -> Option<u128> {
        self.value.as_ref().and_then(|v| v.to_u128())
    }
    fn u64(&self) -> Option<u64> {
        self.value.as_ref().and_then(|v| v.to_u64())
    }
}

/// Values above this many digits are kept by their form only.
const HOLD_DIGITS: f64 = 5000.0;

fn digits_at(s: &[char], i: usize) -> (String, usize) {
    let mut j = i;
    let mut t = String::new();
    while j < s.len() && (s[j].is_ascii_digit() || (s[j] == ',' && j + 1 < s.len() && s[j + 1].is_ascii_digit() && !t.is_empty())) {
        if s[j] != ',' {
            t.push(s[j]);
        }
        j += 1;
    }
    (t, j)
}

fn skip_spaces(s: &[char], mut i: usize) -> usize {
    while i < s.len() && s[i] == ' ' {
        i += 1;
    }
    i
}

/// The first number written in the question: digits, a power b^e, b^e + c
/// or b^e - c (also "10**6" and "2 * 10^9").
pub fn number_in(q: &str) -> Option<Num> {
    let q = q.replace("**", "^").replace('×', "*");
    let s: Vec<char> = q.chars().collect();
    let mut i = 0;
    while i < s.len() {
        if s[i].is_ascii_digit() && (i == 0 || !s[i - 1].is_ascii_alphanumeric()) {
            let (a, mut j) = digits_at(&s, i);
            let mut mult: Option<BigUint> = None;
            // a * b^e
            let k = skip_spaces(&s, j);
            if k < s.len() && s[k] == '*' {
                let k2 = skip_spaces(&s, k + 1);
                if k2 < s.len() && s[k2].is_ascii_digit() {
                    let (b2, j2) = digits_at(&s, k2);
                    if j2 < s.len() && s[j2] == '^' {
                        mult = a.parse::<BigUint>().ok();
                        let (a2, _) = (b2, 0);
                        return power_from(&s, mult, &a2, j2);
                    }
                }
            }
            if j < s.len() && s[j] == '^' {
                return power_from(&s, mult, &a, j);
            }
            j = j.max(i);
            let v: BigUint = a.parse().ok()?;
            let size = (a.len() as f64 - 1.0) + a[..a.len().min(15)].parse::<f64>().ok().map_or(0.0, |x| (x / 10f64.powi(a.len().min(15) as i32 - 1)).log10());
            let _ = j;
            return Some(Num { text: a, value: Some(v), form: None, size });
        }
        i += 1;
    }
    None
}

fn power_from(s: &[char], mult: Option<BigUint>, base: &str, caret: usize) -> Option<Num> {
    let (e, mut j) = digits_at(s, caret + 1);
    let b: u64 = base.parse().ok()?;
    let e: u64 = e.parse().ok()?;
    let mut add: i64 = 0;
    let k = skip_spaces(s, j);
    if k < s.len() && (s[k] == '+' || s[k] == '-') {
        let k2 = skip_spaces(s, k + 1);
        if k2 < s.len() && s[k2].is_ascii_digit() {
            let (c, j2) = digits_at(s, k2);
            // "2^p - 1" but not "2^10 - 1 = x"-like algebra: a plain integer
            if let Ok(c) = c.parse::<i64>() {
                add = if s[k] == '+' { c } else { -c };
                j = j2;
            }
        }
    }
    let _ = j;
    let m = mult.clone().unwrap_or_else(BigUint::one);
    let size = (b as f64).log10() * e as f64 + m.to_f64().unwrap_or(1.0).log10();
    let value = if size <= HOLD_DIGITS {
        let p = m * BigUint::from(b).pow(e as u32);
        Some(if add >= 0 { p + BigUint::from(add as u64) } else { p - BigUint::from((-add) as u64) })
    } else {
        None
    };
    let text = format!("{}{b}^{e}{}", mult.map_or(String::new(), |m| format!("{m} * ")), if add > 0 { format!(" + {add}") } else if add < 0 { format!(" - {}", -add) } else { String::new() });
    let form = if text.contains('*') { None } else { Some((b, e, add)) };
    Some(Num { text, value, form, size })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Statement {
    InfinitelyManyPrimes,
    FermatLast,
    TwinPrimes,
    Goldbach,
    Collatz,
    Riemann,
    Catalan,
    FourColour,
}

#[derive(Clone, Debug)]
pub enum Task {
    IsPrime(Num),
    Factor(Num),
    /// how many primes up to x (inclusive)
    PrimeCount(Num, bool),
    Goldbach(Num),
    TwoSquares(Num),
    FourSquares(Num),
    Collatz(Num),
    Statement(Statement),
    /// handed to Nuome's worked-solution engine
    Algebra,
    Unknown,
}

pub fn task_of(q: &str) -> Task {
    let l = q.to_lowercase();
    let n = number_in(&l);
    let has = |w: &str| l.contains(w);
    if has("riemann") {
        return Task::Statement(Statement::Riemann);
    }
    if has("twin prime") {
        return Task::Statement(Statement::TwinPrimes);
    }
    if has("four colo") {
        return Task::Statement(Statement::FourColour);
    }
    if has("catalan") || has("consecutive powers") || has("consecutive perfect powers") {
        return Task::Statement(Statement::Catalan);
    }
    if has("fermat's last") || has("fermats last") || has("x^n + y^n") || has("a^n + b^n") {
        return Task::Statement(Statement::FermatLast);
    }
    if has("infinitely many primes") {
        return Task::Statement(Statement::InfinitelyManyPrimes);
    }
    if has("collatz") || has("3n + 1") || has("3n+1") {
        return match n {
            Some(n) if !has("every") && !has("all ") => Task::Collatz(n),
            _ => Task::Statement(Statement::Collatz),
        };
    }
    if has("sum of two primes") || has("goldbach") {
        return match n {
            Some(n) if !has("every") && !has("all ") => Task::Goldbach(n),
            _ => Task::Statement(Statement::Goldbach),
        };
    }
    if has("sum of two squares") {
        return n.map_or(Task::Unknown, Task::TwoSquares);
    }
    if has("sum of four squares") {
        return n.map_or(Task::Unknown, Task::FourSquares);
    }
    let algebra = l.contains('=') || ["solve", "differentiate", "derivative", "integrate", "simplify", "expand", "evaluate", "calculate"].iter().any(|w| has(w));
    if (has("how many primes") || has("number of primes") || has("count the primes") || has("primes below") || has("primes up to") || has("primes less than")) && n.is_some() {
        let below = has("below") || has("less than") || has("under");
        return Task::PrimeCount(n.unwrap(), below);
    }
    if (has("factor") || has("divisors")) && !algebra && n.is_some() && !l.chars().any(|c| c == 'x' || c == 'y') {
        return Task::Factor(n.unwrap());
    }
    if has("prime") && n.is_some() && !algebra {
        return Task::IsPrime(n.unwrap());
    }
    if algebra || has("factor") {
        return Task::Algebra;
    }
    Task::Unknown
}

// ───────────────────────── arithmetic ─────────────────────────

fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn powmod(mut a: u64, mut e: u64, m: u64) -> u64 {
    let mut r = 1 % m;
    a %= m;
    while e > 0 {
        if e & 1 == 1 {
            r = mulmod(r, a, m);
        }
        a = mulmod(a, a, m);
        e >>= 1;
    }
    r
}

const FIRST_PRIMES: [u64; 20] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71];

/// Miller-Rabin on n with these bases: false = a base proves n composite.
fn miller_rabin(n: &BigUint, bases: &[u64]) -> bool {
    let two = BigUint::from(2u32);
    if *n < two {
        return false;
    }
    for &p in bases {
        let bp = BigUint::from(p);
        if *n == bp {
            return true;
        }
        if (n % &bp).is_zero() {
            return false;
        }
    }
    let one = BigUint::one();
    let nm1 = n - &one;
    let mut d = nm1.clone();
    let mut s = 0;
    while (&d % &two).is_zero() {
        d /= &two;
        s += 1;
    }
    'bases: for &a in bases {
        let mut x = BigUint::from(a).modpow(&d, n);
        if x == one || x == nm1 {
            continue;
        }
        for _ in 1..s {
            x = x.modpow(&two, n);
            if x == nm1 {
                continue 'bases;
            }
        }
        return false;
    }
    true
}

/// Deterministic for every u64 (the first 12 primes as bases; Jaeschke / Sorenson-Webster).
fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for &p in &FIRST_PRIMES[..12] {
        if n % p == 0 {
            return n == p;
        }
    }
    let (mut d, mut s) = (n - 1, 0);
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'b: for &a in &FIRST_PRIMES[..12] {
        let mut x = powmod(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'b;
            }
        }
        return false;
    }
    true
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn rho(n: u64) -> u64 {
    if n % 2 == 0 {
        return 2;
    }
    let mut c = 1u64;
    loop {
        let f = |x: u64| (mulmod(x, x, n) + c) % n;
        let (mut x, mut y, mut d) = (2u64, 2u64, 1u64);
        while d == 1 {
            x = f(x);
            y = f(f(y));
            d = gcd(x.abs_diff(y), n);
        }
        if d != n {
            return d;
        }
        c += 1;
    }
}

/// The prime factors of n (with repeats, sorted).
fn factor_u64(n: u64) -> Vec<u64> {
    let mut out = Vec::new();
    let mut m = n;
    for p in 2..1000u64 {
        while m % p == 0 {
            out.push(p);
            m /= p;
        }
    }
    let mut stack = vec![m];
    while let Some(x) = stack.pop() {
        if x == 1 {
            continue;
        }
        if is_prime_u64(x) {
            out.push(x);
            continue;
        }
        let d = rho(x);
        stack.push(d);
        stack.push(x / d);
    }
    out.sort();
    out
}

/// The smallest divisor of n in 2..=limit (and below √n), if any.
fn trial_divisor(n: u128, limit: u128) -> Option<u128> {
    if n % 2 == 0 && n > 2 {
        return Some(2);
    }
    let mut d = 3u128;
    while d <= limit && d * d <= n {
        if n % d == 0 {
            return Some(d);
        }
        d += 2;
    }
    None
}

fn isqrt(n: u128) -> u128 {
    if n < 2 {
        return n;
    }
    let mut x = (n as f64).sqrt() as u128;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

fn primes_up_to(n: usize) -> Vec<usize> {
    let mut sieve = vec![true; n + 1];
    let mut out = Vec::new();
    for i in 2..=n {
        if sieve[i] {
            out.push(i);
            let mut j = i * i;
            while j <= n {
                sieve[j] = false;
                j += i;
            }
        }
    }
    out
}

/// Count the primes up to x by the sieve of Eratosthenes (odd numbers only).
fn sieve_count(x: u64) -> u64 {
    if x < 2 {
        return 0;
    }
    let half = (x as usize + 1) / 2; // index i stands for 2i+1
    let mut composite = vec![false; half];
    let mut i = 1;
    while (2 * i + 1) * (2 * i + 1) <= x as usize {
        if !composite[i] {
            let p = 2 * i + 1;
            let mut j = p * p / 2;
            while j < half {
                composite[j] = true;
                j += p;
            }
        }
        i += 1;
    }
    1 + (1..half).filter(|&i| !composite[i]).count() as u64
}

/// The logarithmic integral li(x), by Ramanujan's series.
fn li(x: f64) -> f64 {
    if x <= 1.0 {
        return 0.0;
    }
    let l = x.ln();
    let mut sum = 0.0;
    let mut term = 1.0; // (ln x)^k / k!
    let mut inner = 0.0;
    for k in 1..400 {
        term *= l / k as f64;
        if (k - 1) % 2 == 0 {
            // inner = sum_{j=0}^{floor((k-1)/2)} 1/(2j+1)
            inner += 1.0 / (2 * ((k - 1) / 2) + 1) as f64;
        }
        let sign = if (k - 1) % 2 == 0 { 1.0 } else { -1.0 };
        let add = sign * term / 2f64.powi(k as i32 - 1) * inner;
        sum += add;
        if add.abs() < 1e-17 * sum.abs() && k > 2 * l as usize {
            break;
        }
    }
    0.577_215_664_901_532_9 + l.ln() + x.sqrt() * sum
}

fn mobius(n: u64) -> i64 {
    let f = factor_u64(n);
    for w in f.windows(2) {
        if w[0] == w[1] {
            return 0;
        }
    }
    if f.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

/// Riemann's R(x) = sum mu(n)/n li(x^(1/n)): his estimate of pi(x).
fn riemann_r(x: f64) -> f64 {
    let mut r = 0.0;
    for n in 1..100u64 {
        let y = x.powf(1.0 / n as f64);
        if y < 2.0 {
            break;
        }
        let mu = mobius(n);
        if mu != 0 {
            r += mu as f64 / n as f64 * li(y);
        }
    }
    r
}

/// pi(10^k), k = 1..=25 (OEIS A006880: Deleglise-Rivat 1996, Gourdon 2000,
/// Oliveira e Silva 2008, Platt 2012, Buethe-Franke-Jost-Kleinjung 2014).
const PI_POWERS_OF_TEN: [&str; 25] = [
    "4", "25", "168", "1229", "9592", "78498", "664579", "5761455", "50847534", "455052511", "4118054813", "37607912018", "346065536839",
    "3204941750802", "29844570422669", "279238341033925", "2623557157654233", "24739954287740860", "234057667276344607",
    "2220819602560918840", "21127269486018731928", "201467286689315906290", "1925320391606803968923", "18435599767349200867866",
    "176846309399143769411680",
];

/// The exponents p of the 52 known Mersenne primes 2^p - 1 (GIMPS and before, to M136279841, 2024).
const MERSENNE_EXPONENTS: [u64; 52] = [
    2, 3, 5, 7, 13, 17, 19, 31, 61, 89, 107, 127, 521, 607, 1279, 2203, 2281, 3217, 4253, 4423, 9689, 9941, 11213, 19937, 21701, 23209, 44497, 86243,
    110503, 132049, 216091, 756839, 859433, 1257787, 1398269, 2976221, 3021377, 6972593, 13466917, 20996011, 24036583, 25964951, 30402457, 32582657,
    37156667, 42643801, 43112609, 57885161, 74207281, 77232917, 82589933, 136279841,
];

/// The largest n for which the first 13 primes as Miller-Rabin bases decide primality (Sorenson & Webster 2015).
const MR13_LIMIT: f64 = 3.317_044_064_679_887e24;

/// (b^e + c) mod p.
fn form_mod(b: u64, e: u64, c: i64, p: u64) -> u64 {
    let v = powmod(b, e, p) as i128 + c as i128;
    v.rem_euclid(p as i128) as u64
}

/// A prime p <= limit dividing b^e + c (and smaller than it), if any.
fn form_divisor(b: u64, e: u64, c: i64, limit: usize) -> Option<u64> {
    primes_up_to(limit).into_iter().map(|p| p as u64).find(|&p| form_mod(b, e, c, p) == 0)
}

// ───────────────────────── answering ─────────────────────────

pub fn answer(kind: Kind, question: &str) -> Answer {
    let a = Answer::new(kind, question);
    match task_of(question) {
        Task::IsPrime(n) => is_prime(a, &n),
        Task::Factor(n) => factor(a, &n),
        Task::PrimeCount(n, below) => prime_count(a, &n, below),
        Task::Goldbach(n) => goldbach(a, &n),
        Task::TwoSquares(n) => two_squares(a, &n),
        Task::FourSquares(n) => four_squares(a, &n),
        Task::Collatz(n) => collatz(a, &n),
        Task::Statement(s) => statement(a, s),
        Task::Algebra => algebra(a, question),
        Task::Unknown => a.none("Golden Answer does not know this kind of question yet (it knows primality, factoring, counting primes, Goldbach, sums of squares, Collatz, the famous theorems and open problems, and Nuome's algebra)"),
    }
}

fn is_prime(a: Answer, n: &Num) -> Answer {
    match a.kind {
        Kind::Logical => {
            if let Some(v) = n.small() {
                if v < 2 {
                    return a.says("not prime").line(format!("{v} is less than 2, and a prime is by definition a whole number above 1 whose only divisors are 1 and itself"));
                }
                if let Some(d) = trial_divisor(v, 10_000_000) {
                    return a.says("composite").line(format!("{v} = {d} x {}", v / d)).line(format!("check: {d} x {} = {}", v / d, d * (v / d)));
                }
                let r = isqrt(v);
                if r <= 10_000_000 {
                    return a.says("prime").line(format!("√{v} < {}", r + 1)).line(format!("no number from 2 to {r} divides {v} (each one tried)")).line("a composite number has a divisor no larger than its square root, so it is prime");
                }
                if let Some(w) = n.u64() {
                    if !is_prime_u64(w) {
                        let d = rho(w);
                        return a.says("composite").line(format!("{w} = {d} x {}", w / d)).line(format!("check: {d} x {} = {}", w / d, d as u128 * (w / d) as u128));
                    }
                }
                return a.none(format!("a logical proof needs every divisor up to √{v} ≈ {r:.3e} ruled out; trial division stopped at 10^7 with none found (a primality certificate is not written yet)"));
            }
            if let Some((b, e, c)) = n.form {
                if let Some(p) = form_divisor(b, e, c, 1_000_000) {
                    return a.says("composite").line(format!("({b}^{e} {}) mod {p} = 0, computed by repeated squaring mod {p}", sign(c))).line(format!("so {p} divides {}, and {p} is smaller than it", n.text));
                }
                return a.none(format!("{} has about {:.0} digits; no prime below 10^6 divides it, and a logical proof of primality at this size needs a test whose correctness rests on theorems", n.text, n.size + 1.0));
            }
            a.none("the number is too large to hold")
        }
        Kind::Theoretical => {
            if let Some((2, p, -1)) = n.form {
                if MERSENNE_EXPONENTS.contains(&p) {
                    let i = MERSENNE_EXPONENTS.iter().position(|&x| x == p).unwrap() + 1;
                    return a.says("prime").line(format!("2^{p} - 1 is the {i}th known Mersenne prime")).source("the list of known Mersenne primes, each proved prime by the Lucas-Lehmer test (Lucas 1876, Lehmer 1930); GIMPS for those found since 1996");
                }
                if p > 1 {
                    let f = factor_u64(p);
                    if f.len() > 1 {
                        return a.says("composite").line(format!("{p} = {} x {}, and 2^ab - 1 is divisible by 2^a - 1", f[0], p / f[0])).line(format!("so 2^{} - 1 divides 2^{p} - 1", f[0])).source("2^a - 1 divides 2^ab - 1 (the factorisation x^b - 1 = (x - 1)(x^(b-1) + ... + 1) with x = 2^a)");
                    }
                }
            }
            if let Some(v) = &n.value {
                if n.size < MR13_LIMIT.log10() && v.to_f64().is_some_and(|f| f < MR13_LIMIT) {
                    let prime = miller_rabin(v, &FIRST_PRIMES[..13]);
                    return a.says(if prime { "prime" } else { "composite" }).line(format!("the Miller-Rabin test with the bases 2, 3, 5, ..., 41 says {}", if prime { "prime" } else { "composite" })).source("no composite number below 3.317 x 10^24 passes Miller-Rabin with the first 13 primes as bases (Sorenson & Webster 2015)");
                }
            }
            a.none(format!("no known result decides whether {} is prime", n.text))
        }
        Kind::Abstract => {
            if let Some(v) = &n.value {
                if n.size <= 3000.0 {
                    let prime = miller_rabin(v, &FIRST_PRIMES);
                    return if prime {
                        a.says("probably prime").line("it passes 20 rounds of the Miller-Rabin test").line("a composite number passes one round with chance at most 1/4, so a composite passing all 20 is very unlikely, but not impossible")
                    } else {
                        a.says("composite").line("a Miller-Rabin round fails, which a prime never does")
                    };
                }
            }
            if let Some((b, e, c)) = n.form {
                let bound = 1_000_000usize;
                if let Some(p) = form_divisor(b, e, c, bound) {
                    return a.says("composite").line(format!("{p} divides it"));
                }
                let ln_n = n.size * std::f64::consts::LN_10;
                // Mertens: no factor below B makes a prime e^gamma ln B times more likely than 1/ln n
                let chance = 1.781_072_418 * (bound as f64).ln() / ln_n;
                return a
                    .says("probably composite")
                    .line(format!("a random number of this size is prime with chance about 1/ln n = 1 in {:.0}", ln_n))
                    .line(format!("no prime below 10^6 divides it, which makes it e^γ ln(10^6) ≈ {:.1} times likelier (Mertens)", 1.781_072_418 * (bound as f64).ln()))
                    .line(format!("so it is prime with chance about 1 in {:.0}", 1.0 / chance));
            }
            a.none("the number is too large to hold")
        }
    }
}

fn sign(c: i64) -> String {
    if c >= 0 {
        format!("+ {c}")
    } else {
        format!("- {}", -c)
    }
}

fn factor(a: Answer, n: &Num) -> Answer {
    let Some(v) = n.u64() else {
        return a.none("factoring is written for numbers below 2^64 so far");
    };
    if v < 2 {
        return a.none(format!("{v} has no prime factors"));
    }
    let f = factor_u64(v);
    let words = f.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" * ");
    match a.kind {
        Kind::Logical => {
            let product: u128 = f.iter().map(|&p| p as u128).product();
            let mut a = a.says(words.clone()).line(format!("check: {words} = {product}"));
            for &p in f.iter().filter(|&&p| p > 1) {
                if (p as u128) <= 1u128 << 62 && isqrt(p as u128) <= 10_000_000 {
                    a = a.line(format!("{p} is prime: no number from 2 to {} divides it", isqrt(p as u128)));
                } else {
                    return a.none(format!("the factor {p} is too large to prove prime by trial division"));
                }
            }
            a
        }
        Kind::Theoretical => a.says(words).line("each factor is prime by the Miller-Rabin test with the first 12 primes as bases").source("no composite below 3.18 x 10^23 passes Miller-Rabin with the bases 2..37 (Sorenson & Webster 2015); this covers every 64-bit number"),
        Kind::Abstract => a.none("factoring has an exact answer; an abstract guess adds nothing (ask logical or theoretical)"),
    }
}

fn prime_count(a: Answer, n: &Num, below: bool) -> Answer {
    // "below x" counts up to x - 1; for x = 10^k the same
    let x = n.value.clone();
    match a.kind {
        Kind::Logical => {
            let Some(x) = x.and_then(|v| v.to_u64()) else { return a.none("too large to count by a sieve") };
            let upto = if below { x.saturating_sub(1) } else { x };
            if upto > 1_000_000_000 {
                return a.none(format!("counting the primes up to {upto} by the sieve of Eratosthenes is beyond this answer's budget (10^9); a logical count at this size needs the Meissel-Lehmer method, not written yet"));
            }
            let c = sieve_count(upto);
            a.says(c.to_string()).line(format!("the sieve of Eratosthenes on 2..{upto}: every multiple of every prime up to √{upto} crossed out")).line(format!("{c} numbers are left, and those are the primes"))
        }
        Kind::Theoretical => {
            if let Some((10, k, 0)) = n.form.or_else(|| power_of_ten(n)) {
                if (1..=25).contains(&k) {
                    return a.says(PI_POWERS_OF_TEN[k as usize - 1]).line(format!("π(10^{k}) = {}", PI_POWERS_OF_TEN[k as usize - 1])).source("π(10^k) as computed by Deléglise-Rivat 1996, Gourdon 2000, Oliveira e Silva 2008, Platt 2012, Büthe-Franke-Jost-Kleinjung 2014 (OEIS A006880)");
                }
            }
            let xf = n.size;
            let x = 10f64.powf(xf);
            if x >= 599.0 && xf < 300.0 {
                let l = x.ln();
                let lo = x / l * (1.0 + 1.0 / l);
                let hi = x / l * (1.0 + 1.0 / l + 2.51 / (l * l));
                let (lo, hi) = (lo.floor() as u128, hi.ceil() as u128);
                return a.says(format!("between {lo} and {hi}")).line(format!("x/ln x (1 + 1/ln x) <= π(x) <= x/ln x (1 + 1/ln x + 2.51/ln² x) at x = {}", n.text)).source("Dusart 2010: the lower bound for x >= 599, the upper bound for x >= 355991");
            }
            a.none("no known bound in the table covers this x")
        }
        Kind::Abstract => {
            let x = 10f64.powf(n.size);
            if n.size > 300.0 {
                return a.none("too large for the estimate in floating point");
            }
            let r = riemann_r(x).round();
            a.says(format!("about {r:.0}")).line(format!("Riemann's R(x) = Σ μ(n)/n li(x^(1/n)) at x = {} is {r:.0}", n.text)).line("R(x) follows π(x) closely but is not exact; the error changes sign infinitely often")
        }
    }
}

fn power_of_ten(n: &Num) -> Option<(u64, u64, i64)> {
    let t = n.value.as_ref()?.to_string();
    (t.starts_with('1') && t[1..].chars().all(|c| c == '0') && t.len() > 1).then(|| (10, t.len() as u64 - 1, 0))
}

fn goldbach(a: Answer, n: &Num) -> Answer {
    let Some(v) = n.u64() else {
        return match a.kind {
            Kind::Theoretical => a.says("open").line("above 4 x 10^18 no one has checked or proved it").source("Goldbach's conjecture is unproved; verified for every even number up to 4 x 10^18 (Oliveira e Silva, Herzog & Pardi 2014)"),
            Kind::Abstract => a.says("probably yes").line("the expected number of ways grows like N/ln² N, so a failure this high would be astonishing"),
            Kind::Logical => a.none("the number is too large to search for the two primes"),
        };
    };
    if v < 4 {
        return a.says("no").line(format!("the smallest sum of two primes is 2 + 2 = 4 > {v}"));
    }
    if v % 2 == 1 {
        // odd: one of the two primes must be 2
        let rest = v - 2;
        return match a.kind {
            Kind::Logical => {
                if (rest as u128) > 100_000_000_000_000 {
                    return a.none("an odd sum of two primes must be 2 + (N - 2); N - 2 is too large to prove prime by trial division");
                }
                let d = trial_divisor(rest as u128, 10_000_000);
                let base = a.line("two odd primes add up to an even number, so an odd sum of two primes must use 2");
                match d {
                    Some(d) if rest > 2 => base.says("no").line(format!("{v} - 2 = {rest} = {d} x {}, not prime", rest as u128 / d)),
                    _ if rest < 2 => base.says("no").line(format!("{v} - 2 = {rest} is not prime")),
                    _ => base.says("yes").line(format!("{v} = 2 + {rest}, and no number from 2 to {} divides {rest}", isqrt(rest as u128))),
                }
            }
            Kind::Theoretical => {
                let p = is_prime_u64(rest);
                a.says(if p { "yes" } else { "no" }).line(format!("an odd sum of two primes is 2 + (N - 2); {rest} is {}", if p { "prime" } else { "composite" })).source("Miller-Rabin with the bases 2..37 decides every 64-bit number (Sorenson & Webster 2015)")
            }
            Kind::Abstract => a.none("for an odd number the question is exact (is N - 2 prime); ask logical or theoretical"),
        };
    }
    match a.kind {
        Kind::Logical => {
            if (v as u128) > 100_000_000_000_000 {
                return a.none("the number is too large to prove both primes by trial division");
            }
            for p in primes_up_to(1_000_000).into_iter().map(|p| p as u64) {
                if p > v / 2 {
                    break;
                }
                let q = v - p;
                if trial_divisor(q as u128, 10_000_000).is_none() && q > 1 {
                    return a.says("yes").line(format!("{v} = {p} + {q}")).line(format!("{p} is prime, and no number from 2 to {} divides {q}", isqrt(q as u128)));
                }
            }
            a.none("no pair found with the smaller prime below 10^6")
        }
        Kind::Theoretical => {
            if (v as f64) <= 4e18 {
                a.says("yes").line(format!("{v} is even and at most 4 x 10^18")).source("every even number from 4 to 4 x 10^18 is a sum of two primes (Oliveira e Silva, Herzog & Pardi 2014)")
            } else {
                a.says("open").source("Goldbach's conjecture is unproved above 4 x 10^18 (Oliveira e Silva, Herzog & Pardi 2014)")
            }
        }
        Kind::Abstract => {
            let x = v as f64;
            let c2 = 0.660_161_815_846_869_6;
            let mut corr = 1.0;
            let mut f = factor_u64(v);
            f.dedup();
            for p in f.into_iter().filter(|&p| p > 2) {
                corr *= (p as f64 - 1.0) / (p as f64 - 2.0);
            }
            let ways = 2.0 * c2 * corr * x / (x.ln() * x.ln());
            a.says("probably yes").line(format!("the Hardy-Littlewood estimate expects about {ways:.0} ways to write {v} as p + q")).line("an estimate of how many ways, not a proof that one exists")
        }
    }
}

fn two_squares(a: Answer, n: &Num) -> Answer {
    let Some(v) = n.u64() else { return a.none("written for numbers below 2^64 so far") };
    match a.kind {
        Kind::Logical => {
            if v > 100_000_000_000_000 {
                return a.none("too large to search every a up to √(N/2)");
            }
            let lim = isqrt(v as u128 / 2);
            for x in 0..=lim {
                let r = v as u128 - x * x;
                let y = isqrt(r);
                if y * y == r {
                    return a.says("yes").line(format!("{v} = {x}² + {y}²")).line(format!("check: {} + {} = {v}", x * x, y * y));
                }
            }
            a.says("no").line(format!("for every a from 0 to √({v}/2) = {lim}, {v} - a² is not a square (each one tried)")).line("if v = a² + b² with a <= b, then a <= √(v/2), so every case is covered")
        }
        Kind::Theoretical => {
            let f = factor_u64(v);
            let bad: Vec<u64> = {
                let mut ps = f.clone();
                ps.dedup();
                ps.into_iter().filter(|&p| p % 4 == 3 && f.iter().filter(|&&q| q == p).count() % 2 == 1).collect()
            };
            let words = f.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(" * ");
            let a = a.line(format!("{v} = {words}")).source("a whole number is a sum of two squares exactly when every prime ≡ 3 (mod 4) divides it an even number of times (Fermat, proved by Euler 1749)");
            if bad.is_empty() {
                a.says("yes").line("no prime ≡ 3 (mod 4) appears an odd number of times")
            } else {
                a.says("no").line(format!("{} ≡ 3 (mod 4) appears an odd number of times", bad[0]))
            }
        }
        Kind::Abstract => a.none("the question has an exact answer; an abstract guess adds nothing (ask logical or theoretical)"),
    }
}

fn four_squares(a: Answer, n: &Num) -> Answer {
    match a.kind {
        Kind::Theoretical => a.says("yes").source("every natural number is a sum of four squares (Lagrange 1770)"),
        Kind::Abstract => a.none("the question has an exact answer; ask logical or theoretical"),
        Kind::Logical => {
            let Some(v) = n.u64().filter(|&v| v <= 1_000_000_000_000) else { return a.none("too large to search for the four squares") };
            let v = v as u128;
            let mut x = isqrt(v);
            loop {
                let r1 = v - x * x;
                let mut y = isqrt(r1).min(x);
                loop {
                    let r2 = r1 - y * y;
                    let mut z = isqrt(r2).min(y);
                    loop {
                        let r3 = r2 - z * z;
                        let w = isqrt(r3);
                        if w * w == r3 && w <= z {
                            return a.says("yes").line(format!("{v} = {x}² + {y}² + {z}² + {w}²")).line(format!("check: {} + {} + {} + {} = {v}", x * x, y * y, z * z, w * w));
                        }
                        if z == 0 || z * z * 3 < r2 {
                            break;
                        }
                        z -= 1;
                    }
                    if y == 0 || y * y * 4 < r1 {
                        break;
                    }
                    y -= 1;
                }
                if x == 0 {
                    break;
                }
                x -= 1;
            }
            a.none("no four squares found")
        }
    }
}

fn collatz(a: Answer, n: &Num) -> Answer {
    let Some(v) = n.small() else {
        return match a.kind {
            Kind::Abstract => a.says("probably yes").line("every number checked so far reaches 1, and on average a step multiplies by about 3/4"),
            Kind::Theoretical => a.says("open").source("the Collatz conjecture is unproved; verified below 2^68 (Barina 2021)"),
            Kind::Logical => a.none("too large to run"),
        };
    };
    if v == 0 {
        return a.none("the Collatz map is for positive whole numbers");
    }
    match a.kind {
        Kind::Logical => {
            let mut x = v;
            let mut path = vec![x];
            for steps in 0..10_000_000u64 {
                if x == 1 {
                    let shown = path.iter().take(12).map(|n| n.to_string()).collect::<Vec<_>>().join(" -> ");
                    return a.says("yes").line(format!("{v} reaches 1 after {steps} steps")).line(format!("{shown}{}", if path.len() > 12 { " -> ..." } else { "" })).line("every step is n/2 for even n and 3n + 1 for odd n");
                }
                x = if x % 2 == 0 { x / 2 } else { x.checked_mul(3).and_then(|y| y.checked_add(1)).unwrap_or(0) };
                if x == 0 {
                    return a.none("the path grew past 2^128");
                }
                if path.len() < 13 {
                    path.push(x);
                }
            }
            a.none("did not reach 1 within 10^7 steps")
        }
        Kind::Theoretical => {
            if v < 1u128 << 68 {
                a.says("yes").line(format!("{v} < 2^68")).source("every number below 2^68 reaches 1 (Barina 2021)")
            } else {
                a.says("open").source("the Collatz conjecture is unproved; verified below 2^68 (Barina 2021)")
            }
        }
        Kind::Abstract => a.says("probably yes").line("every number checked so far reaches 1, and on average a step multiplies by about 3/4"),
    }
}

fn statement(a: Answer, s: Statement) -> Answer {
    use Statement::*;
    match (s, a.kind) {
        (InfinitelyManyPrimes, Kind::Logical) => a
            .says("yes")
            .line("suppose there were only finitely many primes p1, ..., pk")
            .line("let N = p1 · p2 · ... · pk + 1")
            .line("N > 1, so some prime p divides N")
            .line("p is one of p1, ..., pk, so p divides p1 · ... · pk, and so p divides N - p1 · ... · pk = 1")
            .line("no prime divides 1: the supposition is false, there are infinitely many primes"),
        (InfinitelyManyPrimes, Kind::Theoretical) => a.says("yes").source("Euclid, Elements IX.20 (about 300 BC)"),
        (InfinitelyManyPrimes, Kind::Abstract) => a.says("probably yes").line("the primes keep coming: 4 below 10, 168 below 1000, 78498 below 10^6, with no sign of stopping"),
        (FermatLast, Kind::Theoretical) => a.says("no solutions").line("x^n + y^n = z^n has no solution in positive whole numbers for n > 2").source("Wiles 1995 (with Taylor-Wiles 1995): Fermat's Last Theorem"),
        (FermatLast, Kind::Abstract) => {
            let mut found = None;
            'search: for n in 3..=8u32 {
                let pw: Vec<u128> = (0..=60u128).map(|x| x.pow(n)).collect();
                for x in 1..=60 {
                    for y in x..=60 {
                        if let Ok(z) = pw.binary_search(&(pw[x] + pw[y])) {
                            found = Some((x, y, z, n));
                            break 'search;
                        }
                    }
                }
            }
            match found {
                Some((x, y, z, n)) => a.says("there is one").line(format!("{x}^{n} + {y}^{n} = {z}^{n}")),
                None => a.says("probably no solutions").line("no x, y <= 60 with n from 3 to 8 gives x^n + y^n a perfect n-th power (each one tried)"),
            }
        }
        (FermatLast, Kind::Logical) => a.none("no proof from the question alone is known that fits here; the only proof (Wiles) runs through elliptic curves and modular forms"),
        (TwinPrimes, Kind::Theoretical) => a.says("open").source("infinitely many twin primes is unproved; infinitely many prime pairs at most 246 apart is proved (Zhang 2013, Maynard 2015, Polymath 2014)"),
        (TwinPrimes, Kind::Abstract) => {
            let ps = primes_up_to(10_000_000);
            let twins = ps.windows(2).filter(|w| w[1] - w[0] == 2).count();
            a.says("probably yes").line(format!("there are {twins} twin prime pairs below 10^7, and the Hardy-Litttlewood estimate 2 C2 x/ln² x keeps growing without bound"))
        }
        (Goldbach, Kind::Theoretical) => a.says("open").source("unproved; verified for every even number up to 4 x 10^18 (Oliveira e Silva, Herzog & Pardi 2014); every odd number above 5 is a sum of three primes (Helfgott 2013)"),
        (Goldbach, Kind::Abstract) => {
            let lim = 1_000_000usize;
            let ps = primes_up_to(lim);
            let mut is = vec![false; lim + 1];
            for &p in &ps {
                is[p] = true;
            }
            let ok = (4..=lim).step_by(2).all(|n| ps.iter().take_while(|&&p| p <= n / 2).any(|&p| is[n - p]));
            if ok {
                a.says("probably yes").line("every even number from 4 to 10^6 is a sum of two primes (each one checked)").line("the number of ways grows like N/ln² N, so a failure gets ever less likely")
            } else {
                a.says("no").line("a counterexample below 10^6")
            }
        }
        (Collatz, Kind::Theoretical) => a.says("open").source("unproved; verified below 2^68 (Barina 2021); almost all orbits reach almost bounded values (Tao 2019)"),
        (Collatz, Kind::Abstract) => a.says("probably yes").line("every number checked so far reaches 1, and on average a step multiplies by about 3/4"),
        (Riemann, Kind::Theoretical) => a.says("open").source("the Riemann hypothesis is unproved; the first 10^13 nontrivial zeros lie on the critical line (Gourdon 2004)"),
        (Riemann, Kind::Abstract) => a.says("probably true").line("every zero computed so far lies on the line, and many results that follow from it hold where they can be checked"),
        (Catalan, Kind::Theoretical) => a.says("only 8 and 9").line("3² - 2³ = 1 is the only pair of consecutive perfect powers").source("Mihăilescu 2002 (Catalan's conjecture)"),
        (Catalan, Kind::Abstract) => {
            let lim: u128 = 1_000_000_000_000;
            let mut powers = std::collections::BTreeSet::new();
            let mut b = 2u128;
            while b * b <= lim {
                let mut p = b * b;
                while p <= lim {
                    powers.insert(p);
                    p *= b;
                }
                b += 1;
            }
            let pairs: Vec<u128> = powers.iter().filter(|&&p| powers.contains(&(p + 1))).cloned().collect();
            a.says(if pairs == vec![8] { "probably only 8 and 9" } else { "more than 8 and 9" }).line(format!("among the perfect powers up to 10^12, the consecutive pairs start at {:?}", pairs))
        }
        (FourColour, Kind::Theoretical) => a.says("yes").line("every planar map can be coloured with four colours").source("Appel & Haken 1976; Robertson, Sanders, Seymour & Thomas 1997; checked in Coq by Gonthier 2005"),
        (FourColour, Kind::Abstract) => a.none("no abstract argument written for the four colour theorem"),
        (s, Kind::Logical) => a.none(format!("{s:?}: open or with no proof short enough to derive here")),
    }
}

fn algebra(a: Answer, question: &str) -> Answer {
    if a.kind != Kind::Logical {
        return a.none("algebra questions are answered logically (Nuome's worked solution with every answer checked)");
    }
    let cfg = crate::config::Config::builtin();
    match crate::solve(question, &cfg, &crate::Options::default()) {
        Err(d) => a.none(format!("Nuome could not read or solve it: {}", d.first().map_or(String::new(), |d| format!("{d:?}")))),
        Ok(s) => {
            let checks = s.outcome.checks();
            if checks.iter().any(|c| !c.ok) {
                return a.none("Nuome's working did not pass every check");
            }
            let short = crate::render::answer(&s.request, &s.outcome, &cfg, crate::print::Style::default());
            let mut a = a.says(short.trim_start_matches("Answer: ").to_string());
            for l in s.text.lines().filter(|l| !l.trim().is_empty()) {
                a = a.line(l.trim_end().to_string());
            }
            a
        }
    }
}

// ───────────────────────── the fixed test set ─────────────────────────

/// The fixed test set: (category, question, the true answer). "open" marks
/// a question nobody can answer yet. Fixed once; never re-drawn.
pub const BENCH: &[(&str, &str, &str)] = &[
    ("primality", "is 97 prime", "prime"),
    ("primality", "is 561 prime", "composite"),
    ("primality", "is 1000003 prime", "prime"),
    ("primality", "is 1000000007 prime", "prime"),
    ("primality", "is 4294967297 prime", "composite"),
    ("primality", "is 2^61 - 1 prime", "prime"),
    ("primality", "is 2^67 - 1 prime", "composite"),
    ("primality", "is 2^89 - 1 prime", "prime"),
    ("primality", "is 2^82589933 - 1 prime", "prime"),
    ("primality", "is 2^82589931 - 1 prime", "composite"),
    ("primality", "is 10^99999999 + 13 prime", "open"),
    ("factoring", "factor 1001", "7 * 11 * 13"),
    ("factoring", "factor 4294967297", "641 * 6700417"),
    ("factoring", "factor 600851475143", "71 * 839 * 1471 * 6857"),
    ("prime counting", "how many primes are below 1000", "168"),
    ("prime counting", "how many primes are below 10^6", "78498"),
    ("prime counting", "how many primes are below 10^9", "50847534"),
    ("prime counting", "how many primes are below 10^12", "37607912018"),
    ("prime counting", "how many primes are below 10^20", "2220819602560918840"),
    ("goldbach", "is 100 a sum of two primes", "yes"),
    ("goldbach", "is 1000000 a sum of two primes", "yes"),
    ("goldbach", "is 27 a sum of two primes", "no"),
    ("goldbach", "is every even number above 2 a sum of two primes", "open"),
    ("sums of squares", "is 65 a sum of two squares", "yes"),
    ("sums of squares", "is 21 a sum of two squares", "no"),
    ("sums of squares", "is 1000001 a sum of two squares", "yes"),
    ("sums of squares", "is 7 a sum of four squares", "yes"),
    ("collatz", "does 27 reach 1 under the collatz map", "yes"),
    ("collatz", "does every number reach 1 under the collatz map", "open"),
    ("theorems", "are there infinitely many primes", "yes"),
    ("theorems", "does x^n + y^n = z^n have solutions for n > 2", "no solutions"),
    ("theorems", "are there infinitely many twin primes", "open"),
    ("theorems", "is the riemann hypothesis true", "open"),
    ("theorems", "catalan: which consecutive powers are there", "only 8 and 9"),
    ("algebra", "solve 2x + 3 = 7", "x = 2"),
    ("algebra", "solve x^2 - 5x + 6 = 0", "x = 2 or x = 3"),
];

/// Whether an answer agrees with the truth: exactly; "about N" within 1%;
/// "between L and U" when the truth lies inside; "probably X" as X.
pub fn agrees(short: &str, truth: &str) -> bool {
    let s = short.trim();
    if let Some(r) = s.strip_prefix("about ") {
        let (Ok(g), Ok(t)) = (r.parse::<f64>(), truth.parse::<f64>()) else { return false };
        return (g - t).abs() <= 0.01 * t;
    }
    if let Some(r) = s.strip_prefix("between ") {
        let mut it = r.split(" and ");
        let (Some(lo), Some(hi)) = (it.next(), it.next()) else { return false };
        let (Ok(lo), Ok(hi), Ok(t)) = (lo.parse::<u128>(), hi.parse::<u128>(), truth.parse::<u128>()) else { return false };
        return lo <= t && t <= hi;
    }
    let s = s.strip_prefix("probably ").unwrap_or(s);
    let norm = |x: &str| x.to_lowercase().replace(['(', ')'], "").split_whitespace().collect::<Vec<_>>().join(" ");
    let (s, t) = (norm(s), norm(truth));
    s == t || (t == "x = 2 or x = 3" && (s == "x = 2 or x = 3" || s == "x = 3 or x = 2" || s == "x = 2, x = 3"))
}

pub struct BenchRow {
    pub kind: Kind,
    pub category: &'static str,
    pub question: &'static str,
    pub truth: &'static str,
    pub short: Option<String>,
    pub right: Option<bool>,
}

/// Every question of the fixed set in every kind. `right` is None for open
/// questions and for no answer.
pub fn run_bench() -> Vec<BenchRow> {
    let mut rows = Vec::new();
    for kind in KINDS {
        for &(category, question, truth) in BENCH {
            let a = answer(kind, question);
            let right = match (&a.short, truth) {
                (None, _) => None,
                (Some(s), "open") => Some(s == "open" || kind == Kind::Abstract && s.starts_with("probably")).filter(|_| kind != Kind::Abstract),
                (Some(s), t) => Some(agrees(s, t)),
            };
            rows.push(BenchRow { kind, category, question, truth, short: a.short, right });
        }
    }
    rows
}

pub fn bench_report() -> String {
    let rows = run_bench();
    let mut out = format!("Golden Answer on the fixed test set ({} questions, split: bench, fixed in src/golden_answer.rs)\n", BENCH.len());
    for kind in KINDS {
        out.push_str(&format!("\n{} ({}):\n", kind.name(), kind.claim()));
        out.push_str(&format!("  {:<16} {:>4} {:>9} {:>12}\n", "category", "n", "answered", "right"));
        let mut cats: Vec<&str> = Vec::new();
        for r in rows.iter().filter(|r| r.kind == kind) {
            if !cats.contains(&r.category) {
                cats.push(r.category);
            }
        }
        let (mut tn, mut ta, mut tr, mut tj) = (0, 0, 0, 0);
        for c in cats {
            let rs: Vec<&BenchRow> = rows.iter().filter(|r| r.kind == kind && r.category == c).collect();
            let answered = rs.iter().filter(|r| r.short.is_some()).count();
            let judged = rs.iter().filter(|r| r.right.is_some()).count();
            let right = rs.iter().filter(|r| r.right == Some(true)).count();
            out.push_str(&format!("  {:<16} {:>4} {:>9} {:>12}\n", c, rs.len(), answered, format!("{right}/{judged}")));
            tn += rs.len();
            ta += answered;
            tr += right;
            tj += judged;
        }
        out.push_str(&format!("  {:<16} {:>4} {:>9} {:>12}\n", "all", tn, ta, format!("{tr}/{tj}")));
        for r in rows.iter().filter(|r| r.kind == kind && r.right == Some(false)) {
            out.push_str(&format!("  WRONG: \"{}\" answered {:?}, truth {}\n", r.question, r.short.as_deref().unwrap_or("-"), r.truth));
        }
    }
    out.push_str("\nanswered = the kind gave an answer; right = agrees with the truth, of the answers that can be judged (open questions are judged only for logical and theoretical: the right answer there is \"open\" or none)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_read_with_their_form() {
        let n = number_in("is 2^61 - 1 prime").unwrap();
        assert_eq!(n.form, Some((2, 61, -1)));
        assert_eq!(n.small(), Some((1u128 << 61) - 1));
        let big = number_in("is 10^99999999 + 13 prime").unwrap();
        assert!(big.value.is_none());
        assert_eq!(big.form, Some((10, 99_999_999, 13)));
        assert_eq!(number_in("how many primes below 1000").unwrap().small(), Some(1000));
    }

    #[test]
    fn every_answer_is_of_the_kind_asked() {
        for kind in KINDS {
            for &(_, q, _) in BENCH {
                assert_eq!(answer(kind, q).kind, kind, "{q}");
            }
        }
    }

    #[test]
    fn logical_and_theoretical_answers_are_never_wrong_on_the_set() {
        for r in run_bench() {
            if r.kind != Kind::Abstract {
                assert_ne!(r.right, Some(false), "{} {}: {:?} (truth {})", r.kind.name(), r.question, r.short, r.truth);
            }
        }
    }

    #[test]
    fn logical_answers_never_answer_an_open_question() {
        for &(_, q, truth) in BENCH {
            if truth == "open" {
                assert!(answer(Kind::Logical, q).short.is_none(), "{q}");
            }
        }
    }

    #[test]
    fn theoretical_answers_name_their_results() {
        for &(_, q, _) in BENCH {
            let a = answer(Kind::Theoretical, q);
            if a.short.is_some() && a.short.as_deref() != Some("open") {
                assert!(!a.sources.is_empty(), "{q}");
            }
        }
    }

    #[test]
    fn abstract_answers_never_claim_proof() {
        assert!(!Kind::Abstract.claim().contains("proved"));
        let a = answer(Kind::Abstract, "is 10^99999999 + 13 prime");
        assert_eq!(a.short.as_deref(), Some("probably composite"));
    }

    #[test]
    fn a_composite_proof_multiplies_back() {
        let a = answer(Kind::Logical, "is 4294967297 prime");
        assert_eq!(a.short.as_deref(), Some("composite"));
        assert!(a.lines.iter().any(|l| l.contains("641 x 6700417")));
    }

    #[test]
    fn the_estimates_are_close() {
        assert!((riemann_r(1000.0) - 168.36).abs() < 0.1);
        assert!((li(1e6) - 78627.5).abs() < 1.0);
        assert_eq!(sieve_count(1000), 168);
        assert_eq!(sieve_count(1_000_000), 78498);
    }

    #[test]
    fn no_kind_slips_into_another() {
        // an exact question asked abstractly gives no answer rather than a logical one
        assert!(answer(Kind::Abstract, "is 21 a sum of two squares").short.is_none());
        // algebra is answered only logically
        assert!(answer(Kind::Theoretical, "solve 2x + 3 = 7").short.is_none());
    }
}
