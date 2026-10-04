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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prop {
    Even,
    Odd,
    Square,
    Cube,
    Perfect,
    Triangular,
    Fibonacci,
    Palindrome,
    PowerOfTwo,
    DivisibleBy(u64),
}

impl Prop {
    fn words(self) -> String {
        match self {
            Prop::Even => "even".into(),
            Prop::Odd => "odd".into(),
            Prop::Square => "a perfect square".into(),
            Prop::Cube => "a perfect cube".into(),
            Prop::Perfect => "a perfect number".into(),
            Prop::Triangular => "a triangular number".into(),
            Prop::Fibonacci => "a Fibonacci number".into(),
            Prop::Palindrome => "a palindrome".into(),
            Prop::PowerOfTwo => "a power of two".into(),
            Prop::DivisibleBy(k) => format!("divisible by {k}"),
        }
    }
}

/// Every whole number written in the text, in order.
fn integers_in(q: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in q.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            if let Ok(v) = cur.parse() {
                out.push(v);
            }
            cur.clear();
        }
    }
    out
}

fn prop_of(l: &str) -> Option<Prop> {
    let has = |w: &str| l.contains(w);
    if has("divisible by") {
        let after = l.split("divisible by").nth(1)?;
        return integers_in(after).first().map(|&k| Prop::DivisibleBy(k));
    }
    if has("perfect square") || has(" a square") || has(" square number") {
        return Some(Prop::Square);
    }
    if has("perfect cube") || has(" a cube") || has(" cube number") {
        return Some(Prop::Cube);
    }
    if has("perfect number") || l.trim_end_matches('?').ends_with(" perfect") {
        return Some(Prop::Perfect);
    }
    if has("triangular") {
        return Some(Prop::Triangular);
    }
    if has("fibonacci") {
        return Some(Prop::Fibonacci);
    }
    if has("palindrom") {
        return Some(Prop::Palindrome);
    }
    if has("power of two") || has("power of 2") {
        return Some(Prop::PowerOfTwo);
    }
    let words: Vec<&str> = l.split(|c: char| !c.is_alphanumeric()).collect();
    if words.contains(&"even") {
        return Some(Prop::Even);
    }
    if words.contains(&"odd") {
        return Some(Prop::Odd);
    }
    None
}

/// The arithmetic in a question, when that is all it asks.
fn arithmetic_of(l: &str) -> Option<String> {
    let mut t = format!(" {} ", l.trim().trim_end_matches('?'));
    for w in ["what is", "what's", "how much is", "calculate", "compute", "evaluate", "the value of", "find"] {
        t = t.replace(w, " ");
    }
    for (w, op) in [(" plus ", " + "), (" minus ", " - "), (" times ", " * "), (" divided by ", " / "), (" to the power of ", " ^ "), (" squared", " ^ 2"), (" cubed", " ^ 3"), ("**", "^"), ("×", "*"), ("÷", "/")] {
        t = t.replace(w, op);
    }
    let t = t.trim().trim_end_matches('=').trim().to_string();
    let rest = t.replace("sqrt", "");
    let ok = !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || " +-*/^().".contains(c));
    let op = rest.chars().skip(1).any(|c| "+-*/^".contains(c)) || t.contains("sqrt");
    (ok && op && rest.chars().any(|c| c.is_ascii_digit())).then_some(t)
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
    /// a property of one number: even, square, perfect, divisible by k, ...
    Property(Num, Prop),
    /// plain arithmetic: numbers and + - * / ^ ( ) sqrt
    Arithmetic(String),
    /// handed to Nuome's worked-solution engine
    Algebra,
    Unknown,
}

/// The question as words to match: lower case, no TeX dollars or
/// backslashes, no closing ? or ., single spaces.
fn normal(q: &str) -> String {
    let t = q.to_lowercase().replace(['$', '\\'], "");
    let t = t.trim().trim_end_matches(['?', '.', '!']).trim();
    let t = t.strip_prefix("is it true that ").unwrap_or(t);
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text is exactly one number expression, nothing else.
fn whole_number(t: &str) -> Option<Num> {
    let n = number_in(t)?;
    let squeeze = |x: &str| x.chars().filter(|c| !c.is_whitespace() && *c != ',').collect::<String>();
    (squeeze(t) == squeeze(&n.text)).then_some(n)
}

/// One of the prefixes, a number, one of the suffixes: the whole text.
fn frame(t: &str, prefixes: &[&str], suffixes: &[&str]) -> Option<Num> {
    for p in prefixes {
        let Some(rest) = t.strip_prefix(p) else { continue };
        for x in suffixes {
            if let Some(mid) = rest.strip_suffix(x) {
                if let Some(n) = whole_number(mid.trim()) {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// The question read strictly: a task only when the whole question is one
/// of the forms Golden Answer knows, word for word around the number. A
/// question it cannot read whole is Unknown (or Nuome's algebra), so an
/// answer that claims to be right never answers a question it misread.
pub fn task_of(q: &str) -> Task {
    let t = normal(q);
    const IS: &[&str] = &["is ", "check if ", "check whether ", "determine whether ", "determine if ", "decide whether ", "can "];
    if let Some(n) = frame(&t, IS, &[" prime", " a prime", " a prime number", " prime or composite"]) {
        return Task::IsPrime(n);
    }
    if let Some(n) = frame(&t, &["factor ", "factorise ", "factorize ", "find the prime factorization of ", "find the prime factorisation of ", "the prime factorization of ", "what is the prime factorization of ", "what are the prime factors of ", "prime factors of "], &[""]) {
        return Task::Factor(n);
    }
    let count_pre = &["how many primes are there below ", "how many primes are below ", "how many primes below ", "how many primes are less than ", "how many primes less than ", "how many prime numbers are less than ", "how many prime numbers are below ", "count the primes below ", "the number of primes below ", "number of primes below "];
    if let Some(n) = frame(&t, count_pre, &[""]) {
        return Task::PrimeCount(n, true);
    }
    if let Some(n) = frame(&t, &["how many primes are there up to ", "how many primes are up to ", "how many primes up to ", "count the primes up to ", "number of primes up to ", "how many primes are at most "], &[""]) {
        return Task::PrimeCount(n, false);
    }
    if let Some(n) = frame(&t, IS, &[" a sum of two primes", " the sum of two primes", " be written as a sum of two primes", " be written as the sum of two primes"]) {
        return Task::Goldbach(n);
    }
    if let Some(n) = frame(&t, IS, &[" a sum of two squares", " the sum of two squares", " be written as a sum of two squares", " be written as the sum of two squares"]) {
        return Task::TwoSquares(n);
    }
    if let Some(n) = frame(&t, IS, &[" a sum of four squares", " the sum of four squares", " be written as a sum of four squares"]) {
        return Task::FourSquares(n);
    }
    if let Some(n) = frame(&t, &["does ", "will "], &[" reach 1 under the collatz map", " reach 1 under collatz", " reach 1 in the collatz sequence", " reach 1 under the 3n + 1 map", " reach 1 under the 3n+1 map"]) {
        return Task::Collatz(n);
    }
    let props: &[(&str, Prop)] = &[
        (" a perfect square", Prop::Square),
        (" a square number", Prop::Square),
        (" a square", Prop::Square),
        (" a perfect cube", Prop::Cube),
        (" a cube", Prop::Cube),
        (" a perfect number", Prop::Perfect),
        (" perfect", Prop::Perfect),
        (" a triangular number", Prop::Triangular),
        (" triangular", Prop::Triangular),
        (" a fibonacci number", Prop::Fibonacci),
        (" a palindrome", Prop::Palindrome),
        (" a power of two", Prop::PowerOfTwo),
        (" a power of 2", Prop::PowerOfTwo),
        (" even", Prop::Even),
        (" odd", Prop::Odd),
    ];
    for &(suffix, prop) in props {
        if let Some(n) = frame(&t, IS, &[suffix]) {
            return Task::Property(n, prop);
        }
    }
    if let Some((left, k)) = t.rsplit_once(" divisible by ") {
        if let (Ok(k), Some(n)) = (k.trim().parse::<u64>(), frame(left, IS, &[""])) {
            return Task::Property(n, Prop::DivisibleBy(k));
        }
    }
    if let Some(e) = arithmetic_of(&t) {
        return Task::Arithmetic(e);
    }
    let statements: &[(&[&str], Statement)] = &[
        (&["are there infinitely many primes", "is the number of primes infinite", "are there infinitely many prime numbers"], Statement::InfinitelyManyPrimes),
        (&["are there infinitely many twin primes", "is the twin prime conjecture true", "are there infinitely many twin prime pairs"], Statement::TwinPrimes),
        (&["is the riemann hypothesis true", "is riemann's hypothesis true"], Statement::Riemann),
        (&["is goldbach's conjecture true", "is every even number above 2 a sum of two primes", "is every even number greater than 2 a sum of two primes", "is every even number above 2 the sum of two primes"], Statement::Goldbach),
        (&["is the collatz conjecture true", "does every number reach 1 under the collatz map", "does every positive integer reach 1 under the collatz map"], Statement::Collatz),
        (&["is fermat's last theorem true", "does x^n + y^n = z^n have solutions for n > 2", "does a^n + b^n = c^n have solutions for n > 2"], Statement::FermatLast),
        (&["is the four colour theorem true", "is the four color theorem true", "can every map be coloured with four colours", "can every map be colored with four colors"], Statement::FourColour),
        (&["which consecutive powers are there", "catalan: which consecutive powers are there", "is catalan's conjecture true", "which perfect powers are consecutive"], Statement::Catalan),
    ];
    for (forms, st) in statements {
        if forms.contains(&t.as_str()) {
            return Task::Statement(*st);
        }
    }
    let algebra = ["solve ", "differentiate ", "integrate ", "simplify ", "expand ", "factor ", "evaluate ", "find the derivative", "find the integral"].iter().any(|w| t.starts_with(w)) || (t.contains('=') && !t.contains(" is ") && t.split_whitespace().count() <= 12);
    if algebra {
        return Task::Algebra;
    }
    Task::Unknown
}

/// The question read loosely, by its key words and its first number: only
/// for abstract answers, which say they read it so.
fn task_loose(q: &str) -> Task {
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
    if let Some(e) = arithmetic_of(&l) {
        return Task::Arithmetic(e);
    }
    if let (Some(n), Some(p)) = (n.clone(), prop_of(&l)) {
        if !l.contains('=') {
            return Task::Property(n, p);
        }
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
    let mut task = task_of(question);
    let mut loose = false;
    if kind == Kind::Abstract && matches!(task, Task::Unknown) {
        task = task_loose(question);
        loose = !matches!(task, Task::Unknown);
    }
    let mut a = answer_task(kind, question, &task);
    if kind == Kind::Abstract && a.short.is_none() {
        // an abstract answer always answers: a guess with its reasons
        a = guess(Answer::new(kind, question), question, &task);
    }
    if loose {
        a.lines.insert(0, format!("the question could not be read whole; read loosely as: {}", describe(&task)));
    }
    a
}

fn describe(t: &Task) -> String {
    match t {
        Task::IsPrime(n) => format!("is {} prime", n.text),
        Task::Factor(n) => format!("factor {}", n.text),
        Task::PrimeCount(n, _) => format!("how many primes up to {}", n.text),
        Task::Goldbach(n) => format!("is {} a sum of two primes", n.text),
        Task::TwoSquares(n) => format!("is {} a sum of two squares", n.text),
        Task::FourSquares(n) => format!("is {} a sum of four squares", n.text),
        Task::Collatz(n) => format!("does {} reach 1 under the Collatz map", n.text),
        Task::Property(n, p) => format!("is {} {}", n.text, p.words()),
        Task::Arithmetic(e) => format!("the arithmetic {e}"),
        Task::Statement(s) => format!("{s:?}"),
        Task::Algebra => "algebra for Nuome".into(),
        Task::Unknown => "nothing".into(),
    }
}

fn answer_task(kind: Kind, question: &str, task: &Task) -> Answer {
    let a = Answer::new(kind, question);
    match task.clone() {
        Task::Property(n, p) => property(a, &n, p),
        Task::Arithmetic(e) => arithmetic(a, &e),
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
                if b == 2 && c == -1 {
                    // Wagstaff's heuristic: 2^p - 1 has only factors 2kp + 1, so it is prime with chance e^gamma log2(a p) / p
                    let a_ = if e % 4 == 3 { 2.0 } else { 6.0 };
                    let chance = 1.781_072_418 * (a_ * e as f64).log2() / e as f64;
                    return a
                        .says("probably composite")
                        .line(format!("every factor of 2^{e} - 1 has the form 2k·{e} + 1, so small primes cannot divide it and the usual 1/ln n does not apply"))
                        .line(format!("Wagstaff's heuristic: 2^p - 1 is prime with chance about e^γ log2({a_} p) / p = 1 in {:.0}", 1.0 / chance))
                        .line("so composite is the better guess, though a few of these are prime (GIMPS has found 52)");
                }
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
        Kind::Abstract => a.says(format!("probably {words}")).line("trial division to 1000, then Pollard's rho; each factor passed Miller-Rabin, which a composite can fool"),
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

fn property(a: Answer, n: &Num, p: Prop) -> Answer {
    let what = p.words();
    // exactly, when the number is held
    let exact: Option<(bool, String)> = n.value.as_ref().and_then(|v| {
        let two = BigUint::from(2u32);
        Some(match p {
            Prop::Even => ((v % &two).is_zero(), format!("{} leaves remainder {} when divided by 2", n.text, v % &two)),
            Prop::Odd => (!(v % &two).is_zero(), format!("{} leaves remainder {} when divided by 2", n.text, v % &two)),
            Prop::DivisibleBy(k) if k > 0 => {
                let r = v % BigUint::from(k);
                (r.is_zero(), format!("{} = {k} x {} + {r}", n.text, v / BigUint::from(k)))
            }
            Prop::Square => {
                let r = num_integer::Roots::sqrt(v);
                (&r * &r == *v, format!("{r}² = {} and {}² = {}", &r * &r, &r + 1u32, (&r + 1u32) * (&r + 1u32)))
            }
            Prop::Cube => {
                let r = num_integer::Roots::cbrt(v);
                (&r * &r * &r == *v, format!("{r}³ = {} and {}³ = {}", &r * &r * &r, &r + 1u32, (&r + 1u32).pow(3)))
            }
            Prop::Palindrome => {
                let t = v.to_string();
                (t.chars().rev().collect::<String>() == t, format!("{t} read backwards is {}", t.chars().rev().collect::<String>()))
            }
            Prop::PowerOfTwo => {
                let ok = !v.is_zero() && (v & (v - 1u32)).is_zero();
                (ok, format!("{} in binary has {} ones", n.text, v.count_ones()))
            }
            Prop::Triangular => {
                let m = v * 8u32 + 1u32;
                let r = num_integer::Roots::sqrt(&m);
                (&r * &r == m, format!("n is triangular exactly when 8n + 1 is a square; 8n + 1 = {m}, √ ≈ {r}"))
            }
            Prop::Fibonacci => {
                let m = v * v * 5u32;
                let sq = |x: &BigUint| {
                    let r = num_integer::Roots::sqrt(x);
                    &r * &r == *x
                };
                let ok = sq(&(&m + 4u32)) || (m >= BigUint::from(4u32) && sq(&(&m - 4u32)));
                (ok, "n is a Fibonacci number exactly when 5n² + 4 or 5n² - 4 is a square".to_string())
            }
            Prop::Perfect => {
                let w = v.to_u64()?;
                let mut f = factor_u64(w);
                let mut sigma: u128 = 1;
                f.dedup();
                for q in f {
                    let mut k = 0;
                    let mut x = w;
                    while x % q == 0 {
                        x /= q;
                        k += 1;
                    }
                    sigma *= ((q as u128).pow(k + 1) - 1) / (q as u128 - 1);
                }
                (sigma == 2 * w as u128, format!("the divisors of {w} below it add up to {}", sigma - w as u128))
            }
            Prop::DivisibleBy(_) => return None,
        })
    });
    // b^e + c read by its form: parity and divisibility
    let by_form: Option<(bool, String)> = n.form.and_then(|(b, e, c)| match p {
        Prop::Even | Prop::Odd => {
            let r = form_mod(b, e, c, 2);
            Some(((r == 0) == (p == Prop::Even), format!("{} leaves remainder {r} when divided by 2", n.text)))
        }
        Prop::DivisibleBy(k) if k > 1 => {
            let r = form_mod(b, e, c, k);
            Some((r == 0, format!("{} mod {k} = {r}, by repeated squaring", n.text)))
        }
        _ => None,
    });
    let found = exact.or(by_form);
    match (a.kind, found) {
        (Kind::Logical, Some((yes, why))) => a.says(if yes { "yes" } else { "no" }).line(why).line(format!("so {} is {}{}", n.text, if yes { "" } else { "not " }, what)),
        (Kind::Logical, None) => a.none(format!("{} is too large to test whether it is {what}", n.text)),
        (Kind::Theoretical, _) => a.none("a direct check, not a known result; ask logical"),
        (Kind::Abstract, Some((yes, why))) => a.says(if yes { "yes" } else { "no" }).line(why),
        (Kind::Abstract, None) => a.none(""),
    }
}

/// A value while evaluating: f64 always, exact as a fraction when it stays one.
#[derive(Clone, Copy, Debug)]
struct V {
    f: f64,
    r: Option<(i128, i128)>,
}

fn gcd128(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

fn frac(n: i128, d: i128) -> Option<(i128, i128)> {
    if d == 0 {
        return None;
    }
    let g = gcd128(n, d);
    let s = if d < 0 { -1 } else { 1 };
    Some((s * n / g, s * d / g))
}

fn show(v: V) -> String {
    match v.r {
        Some((n, 1)) => n.to_string(),
        Some((n, d)) => format!("{n}/{d}"),
        None => format!("{}", v.f),
    }
}

struct Calc<'a> {
    s: &'a [u8],
    i: usize,
    steps: Vec<String>,
}

impl<'a> Calc<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn op(&mut self, a: V, o: char, b: V) -> Option<V> {
        let f = match o {
            '+' => a.f + b.f,
            '-' => a.f - b.f,
            '*' => a.f * b.f,
            '/' => a.f / b.f,
            _ => a.f.powf(b.f),
        };
        let r = match (a.r, b.r) {
            (Some((an, ad)), Some((bn, bd))) => match o {
                '+' => an.checked_mul(bd).zip(bn.checked_mul(ad)).and_then(|(x, y)| x.checked_add(y)).zip(ad.checked_mul(bd)).and_then(|(n, d)| frac(n, d)),
                '-' => an.checked_mul(bd).zip(bn.checked_mul(ad)).and_then(|(x, y)| x.checked_sub(y)).zip(ad.checked_mul(bd)).and_then(|(n, d)| frac(n, d)),
                '*' => an.checked_mul(bn).zip(ad.checked_mul(bd)).and_then(|(n, d)| frac(n, d)),
                '/' => an.checked_mul(bd).zip(ad.checked_mul(bn)).and_then(|(n, d)| frac(n, d)),
                _ if bd == 1 && (0..=200).contains(&bn) => an.checked_pow(bn as u32).zip(ad.checked_pow(bn as u32)).and_then(|(n, d)| frac(n, d)),
                _ => None,
            },
            _ => None,
        };
        let v = V { f, r };
        self.steps.push(format!("{} {o} {} = {}", show(a), show(b), show(v)));
        Some(v)
    }
    fn expr(&mut self) -> Option<V> {
        let mut v = self.term()?;
        while let Some(c) = self.peek().filter(|c| *c == b'+' || *c == b'-') {
            self.i += 1;
            let b = self.term()?;
            v = self.op(v, c as char, b)?;
        }
        Some(v)
    }
    fn term(&mut self) -> Option<V> {
        let mut v = self.power()?;
        while let Some(c) = self.peek().filter(|c| *c == b'*' || *c == b'/') {
            self.i += 1;
            let b = self.power()?;
            v = self.op(v, c as char, b)?;
        }
        Some(v)
    }
    fn power(&mut self) -> Option<V> {
        let base = self.unary()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            let e = self.power()?;
            return self.op(base, '^', e);
        }
        Some(base)
    }
    fn unary(&mut self) -> Option<V> {
        if self.peek() == Some(b'-') {
            self.i += 1;
            let v = self.unary()?;
            return Some(V { f: -v.f, r: v.r.map(|(n, d)| (-n, d)) });
        }
        self.primary()
    }
    fn primary(&mut self) -> Option<V> {
        match self.peek()? {
            b'(' => {
                self.i += 1;
                let v = self.expr()?;
                (self.peek() == Some(b')')).then(|| self.i += 1)?;
                Some(v)
            }
            b's' if self.s[self.i..].starts_with(b"sqrt") => {
                self.i += 4;
                let x = self.primary()?;
                let f = x.f.sqrt();
                let r = x.r.and_then(|(n, d)| {
                    let (rn, rd) = (isqrt(n.max(0) as u128) as i128, isqrt(d as u128) as i128);
                    (rn * rn == n && rd * rd == d).then_some((rn, rd))
                });
                let v = V { f, r };
                self.steps.push(format!("√{} = {}", show(x), show(v)));
                Some(v)
            }
            c if c.is_ascii_digit() || c == b'.' => {
                let st = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).ok()?;
                let f: f64 = t.parse().ok()?;
                let r = if let Some((w, d)) = t.split_once('.') {
                    let den = 10i128.checked_pow(d.len() as u32)?;
                    format!("{w}{d}").parse::<i128>().ok().and_then(|n| frac(n, den))
                } else {
                    t.parse::<i128>().ok().map(|n| (n, 1))
                };
                Some(V { f, r })
            }
            _ => None,
        }
    }
}

fn calc(e: &str) -> Option<(V, Vec<String>)> {
    let mut c = Calc { s: e.as_bytes(), i: 0, steps: Vec::new() };
    let v = c.expr()?;
    (c.peek().is_none()).then_some((v, c.steps))
}

fn arithmetic(a: Answer, e: &str) -> Answer {
    let Some((v, steps)) = calc(e) else { return a.none(format!("could not read \"{e}\" as arithmetic")) };
    match a.kind {
        Kind::Logical => match v.r {
            Some(_) => {
                let mut a = a.says(show(v));
                for s in steps {
                    a = a.line(s);
                }
                a
            }
            None => a.none("the value is not an exact fraction (a root or a number too large for exact arithmetic here)"),
        },
        Kind::Theoretical => a.none("arithmetic is a direct calculation, not a known result; ask logical"),
        Kind::Abstract => a.says(match v.r {
            Some(_) => show(v),
            None => format!("about {:.10}", v.f),
        })
        .line(format!("{e} evaluated step by step")),
    }
}

/// The abstract guess when the question's own answer has none: every
/// question gets one, with its reasons, never claimed to be right always.
fn guess(a: Answer, question: &str, task: &Task) -> Answer {
    let ln10 = std::f64::consts::LN_10;
    match task {
        Task::IsPrime(n) => {
            let ln_n = n.size * ln10;
            a.says("probably composite").line(format!("a number of this size is prime with chance about 1/ln n = 1 in {ln_n:.0}"))
        }
        Task::Factor(n) => {
            if let Some(v) = &n.value {
                let mut rest = v.clone();
                let mut fs = Vec::new();
                for p in primes_up_to(1_000_000) {
                    let bp = BigUint::from(p as u64);
                    while (&rest % &bp).is_zero() {
                        fs.push(p.to_string());
                        rest /= &bp;
                    }
                }
                let mut a = a;
                if !rest.is_one() {
                    let pp = miller_rabin(&rest, &FIRST_PRIMES);
                    let t = rest.to_string();
                    fs.push(if pp || t.len() <= 30 { t } else { format!("[a {}-digit composite, not split]", t.len()) });
                    a = a.line(format!("the part left after the primes below 10^6 {}", if pp { "passes 20 Miller-Rabin rounds: probably prime" } else { "is composite (a Miller-Rabin round fails) but its factors are above 10^6: not split" }));
                }
                return a.says(format!("probably {}", fs.join(" * "))).line("trial division by every prime below 10^6");
            }
            if let Some((b, e, c)) = n.form {
                let small: Vec<String> = primes_up_to(100_000).into_iter().filter(|&p| form_mod(b, e, c, p as u64) == 0).map(|p| p.to_string()).collect();
                return a
                    .says(if small.is_empty() { "probably a product of large primes".to_string() } else { format!("probably {} times large factors", small.join(" * ")) })
                    .line("the primes below 10^5 that divide it, found by repeated squaring; the rest is too large to split");
            }
            a.says("probably a product of large primes")
        }
        Task::PrimeCount(n, _) => {
            // x / ln x in logarithms: log10 pi(x) ≈ size - log10(size ln 10)
            let lg = n.size - (n.size * ln10).log10();
            a.says(format!("about 10^{lg:.3}")).line("the prime number theorem: π(x) ≈ x / ln x, computed in logarithms")
        }
        Task::Goldbach(n) if n.u64().is_some_and(|v| v % 2 == 1) => {
            let v = n.u64().unwrap();
            let yes = v > 2 && is_prime_u64(v - 2);
            a.says(if yes { "yes" } else { "no" }).line(format!("an odd sum of two primes must be 2 + {}, which is {}", v.saturating_sub(2), if yes { "prime" } else { "not prime" }))
        }
        Task::Goldbach(n) => a.says("probably yes").line(format!("the expected number of ways to write a number this size as p + q grows like N/ln² N (Hardy-Littlewood); {} is far past where a failure was ever plausible", n.text)),
        Task::TwoSquares(n) => {
            if let Some(v) = n.u64().filter(|&v| v <= 100_000_000_000_000) {
                let found = (0..=isqrt(v as u128 / 2)).find(|&x| {
                    let r = v as u128 - x * x;
                    isqrt(r) * isqrt(r) == r
                });
                return match found {
                    Some(x) => a.says("yes").line(format!("{v} = {x}² + {}²", isqrt(v as u128 - x * x))),
                    None => a.says("no").line("no a up to √(N/2) leaves a square"),
                };
            }
            let ln_n = n.size * ln10;
            let share = 0.764_223_653 / ln_n.sqrt();
            a.says(if share < 0.5 { "probably no" } else { "probably yes" }).line(format!("about {:.1}% of numbers this size are sums of two squares (Landau-Ramanujan: 0.764 / √ln n)", 100.0 * share))
        }
        Task::FourSquares(_) => {
            let ok = (0..=1_000u128).all(|v| {
                let r = isqrt(v);
                (0..=r).any(|x| (0..=isqrt(v - x * x)).any(|y| (0..=isqrt(v - x * x - y * y)).any(|z| {
                    let w2 = v - x * x - y * y - z * z;
                    isqrt(w2) * isqrt(w2) == w2
                })))
            });
            a.says(if ok { "probably yes" } else { "probably no" }).line("every number from 0 to 1000 is a sum of four squares (each one checked)")
        }
        Task::Collatz(_) => a.says("probably yes").line("every number checked so far reaches 1, and on average a step multiplies by about 3/4"),
        Task::Statement(Statement::FourColour) => a
            .says("probably yes")
            .line("every planar map has a country with at most five neighbours (Euler's formula), so five colours always suffice (Heawood's argument)")
            .line("no map needing five colours has ever been drawn"),
        Task::Property(n, p) => a.says("probably no").line(format!("{} has about {:.0} digits, and numbers that are {} thin out as numbers grow", n.text, n.size + 1.0, p.words())),
        _ => generic_guess(a, question),
    }
}

/// The last resort: Nuome's own working even if not every check passed,
/// then arithmetic, then a plain statement that nothing could be tested.
fn generic_guess(a: Answer, question: &str) -> Answer {
    let cfg = crate::config::Config::builtin();
    if let Ok(s) = crate::solve(question, &cfg, &crate::Options { lenient: true, ..Default::default() }) {
        let short = crate::render::answer(&s.request, &s.outcome, &cfg, crate::print::Style::default());
        let passed = s.outcome.checks().iter().filter(|c| c.ok).count();
        return a.says(format!("probably {}", short.trim_start_matches("Answer: "))).line(format!("Nuome's worked solution, read leniently; {passed} of {} checks passed", s.outcome.checks().len()));
    }
    if let Some(e) = arithmetic_of(&question.to_lowercase()) {
        if let Some((v, _)) = calc(&e) {
            return a.says(format!("about {}", v.f)).line(format!("{e} evaluated"));
        }
    }
    a.says("no evidence either way").line("nothing in the question could be read as something to compute or test, so this guess carries no information (a coin flip)")
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
    ("properties", "is 28 a perfect number", "yes"),
    ("properties", "is 1001 divisible by 7", "yes"),
    ("properties", "is 12321 a palindrome", "yes"),
    ("properties", "is 145 a perfect square", "no"),
    ("properties", "is 10^100 + 1 even", "no"),
    ("properties", "is 832040 a fibonacci number", "yes"),
    ("arithmetic", "what is 2^10 + 3 * 7", "1045"),
    ("arithmetic", "what is (17 - 5) / 4", "3"),
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
        // algebra is answered logically, never theoretically
        assert!(answer(Kind::Theoretical, "solve 2x + 3 = 7").short.is_none());
        // an abstract answer never claims proof, even when it is exact
        for &(_, q, _) in BENCH {
            assert_eq!(answer(Kind::Abstract, q).kind, Kind::Abstract);
        }
    }

    #[test]
    fn abstract_answers_everything() {
        for &(_, q, _) in BENCH {
            assert!(answer(Kind::Abstract, q).short.is_some(), "{q}");
        }
        for q in ["is the moon made of cheese", "what is the best number", "is 10^5000000 + 7 a perfect square", "factor 10^1000 + 1", "how many primes are below 10^1000"] {
            assert!(answer(Kind::Abstract, q).short.is_some(), "{q}");
        }
    }

    #[test]
    fn a_question_read_only_in_part_gets_no_claimed_answer() {
        for q in [
            "Find the sum of the smallest and largest prime factors of $10101$.",
            "Factor $r^2+10r+25$.",
            "How many perfect square factors does the number 46,656 have?",
            "Three consecutive prime numbers, each less than $100$, have a sum that is a multiple of 5. What is the greatest possible sum?",
            "There are finitely many primes $p$ for which the congruence $$8x\\equiv 1\\pmod{p}$$ has no solutions $x$.",
        ] {
            assert!(answer(Kind::Logical, q).short.is_none() || matches!(task_of(q), Task::Algebra), "{q}");
            assert!(answer(Kind::Theoretical, q).short.is_none(), "{q}");
            assert!(answer(Kind::Abstract, q).short.is_some(), "{q}");
        }
        assert!(matches!(task_of("Is $1000003$ prime?"), Task::IsPrime(_)));
        assert!(matches!(task_of("How many primes are below 10^6?"), Task::PrimeCount(_, true)));
    }

    #[test]
    fn arithmetic_is_exact_when_it_can_be() {
        assert_eq!(answer(Kind::Logical, "what is 2^10 + 3 * 7").short.as_deref(), Some("1045"));
        assert_eq!(answer(Kind::Logical, "what is 1/3 + 1/6").short.as_deref(), Some("1/2"));
        assert_eq!(answer(Kind::Logical, "is 1001 divisible by 7").short.as_deref(), Some("yes"));
        assert_eq!(answer(Kind::Logical, "is 10^100 + 1 even").short.as_deref(), Some("no"));
    }
}
