//! The exact calculator Golden Answer reads written mathematics with:
//! LaTeX (\frac, \sqrt, \sqrt[n], \lfloor, \lceil, \log_b, ^{...}, \cdot,
//! base literals 218_9, factorials, |x|) turned into one expression and
//! evaluated with exact fractions of big integers, or modulo m when the
//! number itself is too large to write. Nothing is approximated: a value
//! that is not an exact fraction (sqrt 2, log_2 3) is no value.

use num_bigint::{BigInt, Sign};
use num_integer::{Integer, Roots};
use num_traits::{One, Signed, ToPrimitive, Zero};

/// An exact fraction n/d, d > 0, in lowest terms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rat {
    pub n: BigInt,
    pub d: BigInt,
}

impl Rat {
    pub fn int(n: impl Into<BigInt>) -> Rat {
        Rat { n: n.into(), d: BigInt::one() }
    }
    pub fn new(n: BigInt, d: BigInt) -> Option<Rat> {
        if d.is_zero() {
            return None;
        }
        let g = n.gcd(&d);
        let g = if g.is_zero() { BigInt::one() } else { g };
        let (mut n, mut d) = (n / &g, d / &g);
        if d.is_negative() {
            n = -n;
            d = -d;
        }
        Some(Rat { n, d })
    }
    pub fn is_int(&self) -> bool {
        self.d.is_one()
    }
    fn add(&self, o: &Rat) -> Option<Rat> {
        Rat::new(&self.n * &o.d + &o.n * &self.d, &self.d * &o.d)
    }
    fn sub(&self, o: &Rat) -> Option<Rat> {
        Rat::new(&self.n * &o.d - &o.n * &self.d, &self.d * &o.d)
    }
    fn mul(&self, o: &Rat) -> Option<Rat> {
        Rat::new(&self.n * &o.n, &self.d * &o.d)
    }
    fn div(&self, o: &Rat) -> Option<Rat> {
        Rat::new(&self.n * &o.d, &self.d * &o.n)
    }
    fn floor(&self) -> BigInt {
        self.n.div_floor(&self.d)
    }
    /// The exact k-th root, when there is one.
    fn root(&self, k: u32) -> Option<Rat> {
        if k == 0 {
            return None;
        }
        if self.n.is_negative() {
            if k % 2 == 0 {
                return None;
            }
            let r = Rat { n: -self.n.clone(), d: self.d.clone() }.root(k)?;
            return Some(Rat { n: -r.n, d: r.d });
        }
        let (a, b) = (self.n.nth_root(k), self.d.nth_root(k));
        (a.pow(k) == self.n && b.pow(k) == self.d).then(|| Rat { n: a, d: b })
    }
    fn pow_int(&self, k: i64) -> Option<Rat> {
        // keep results writable: at most a few million bits
        let bits = self.n.bits().max(self.d.bits()) as f64;
        if bits * (k.unsigned_abs() as f64) > 4.0e6 {
            return None;
        }
        let e = k.unsigned_abs() as u32;
        let r = Rat { n: self.n.pow(e), d: self.d.pow(e) };
        if k < 0 {
            Rat::new(r.d, r.n)
        } else {
            Some(r)
        }
    }
    pub fn show(&self) -> String {
        if self.is_int() {
            self.n.to_string()
        } else {
            format!("{}/{}", self.n, self.d)
        }
    }
}

#[derive(Clone, Debug)]
pub enum E {
    Num(Rat),
    Neg(Box<E>),
    Add(Box<E>, Box<E>),
    Sub(Box<E>, Box<E>),
    Mul(Box<E>, Box<E>),
    Div(Box<E>, Box<E>),
    Pow(Box<E>, Box<E>),
    Root(u32, Box<E>),
    Floor(Box<E>),
    Ceil(Box<E>),
    Abs(Box<E>),
    Fact(Box<E>),
    Log(Box<E>, Box<E>),
}

impl E {
    pub fn eval(&self) -> Option<Rat> {
        Some(match self {
            E::Num(r) => r.clone(),
            E::Neg(a) => {
                let r = a.eval()?;
                Rat { n: -r.n, d: r.d }
            }
            E::Add(a, b) => a.eval()?.add(&b.eval()?)?,
            E::Sub(a, b) => a.eval()?.sub(&b.eval()?)?,
            E::Mul(a, b) => a.eval()?.mul(&b.eval()?)?,
            E::Div(a, b) => a.eval()?.div(&b.eval()?)?,
            E::Pow(a, b) => {
                let (x, y) = (a.eval()?, b.eval()?);
                let q = y.d.to_u32()?;
                let p = y.n.to_i64()?;
                x.root(q)?.pow_int(p)?
            }
            E::Root(k, a) => a.eval()?.root(*k)?,
            E::Floor(a) => Rat::int(a.eval()?.floor()),
            E::Ceil(a) => {
                let r = a.eval()?;
                Rat::int(-((-r.n).div_floor(&r.d)))
            }
            E::Abs(a) => {
                let r = a.eval()?;
                Rat { n: r.n.abs(), d: r.d }
            }
            E::Fact(a) => {
                let r = a.eval()?;
                let n = r.n.to_u64().filter(|&n| r.is_int() && n <= 5000)?;
                Rat::int((1..=n).fold(BigInt::one(), |acc, k| acc * k))
            }
            E::Log(b, x) => {
                let (b, x) = (b.eval()?, x.eval()?);
                if b.n <= b.d || x.n.sign() != Sign::Plus {
                    return None;
                }
                // log_b x = p/q exactly when b^p = x^q, small p, q
                for q in 1..=12u32 {
                    for p in -64..=64i64 {
                        if b.pow_int(p).zip(x.pow_int(q as i64)).is_some_and(|(l, r)| l == r) {
                            return Rat::new(BigInt::from(p), BigInt::from(q));
                        }
                    }
                }
                return None;
            }
        })
    }

    /// The value mod m for integer expressions too large to write
    /// (+, -, *, powers with whole exponents, factorials).
    pub fn eval_mod(&self, m: &BigInt) -> Option<BigInt> {
        if let Some(r) = self.small_eval() {
            return r.is_int().then(|| r.n.mod_floor(m));
        }
        Some(match self {
            E::Num(r) => r.is_int().then(|| r.n.mod_floor(m))?,
            E::Neg(a) => (-a.eval_mod(m)?).mod_floor(m),
            E::Add(a, b) => (a.eval_mod(m)? + b.eval_mod(m)?).mod_floor(m),
            E::Sub(a, b) => (a.eval_mod(m)? - b.eval_mod(m)?).mod_floor(m),
            E::Mul(a, b) => (a.eval_mod(m)? * b.eval_mod(m)?).mod_floor(m),
            E::Pow(a, b) => {
                let e = b.eval()?;
                if !e.is_int() || e.n.is_negative() {
                    return None;
                }
                a.eval_mod(m)?.modpow(&e.n, m)
            }
            E::Fact(a) => {
                let n = a.eval()?.n.to_u64()?;
                if BigInt::from(n) >= *m {
                    BigInt::zero()
                } else {
                    (1..=n).fold(BigInt::one(), |acc, k| (acc * k).mod_floor(m))
                }
            }
            _ => return None,
        })
    }

    /// The exact value when it stays small enough to write cheaply.
    fn small_eval(&self) -> Option<Rat> {
        if self.size_bits()? > 2.0e5 {
            return None;
        }
        self.eval()
    }

    /// A bound on the bits of the value (None when unknown).
    fn size_bits(&self) -> Option<f64> {
        Some(match self {
            E::Num(r) => r.n.bits().max(r.d.bits()) as f64,
            E::Neg(a) | E::Floor(a) | E::Ceil(a) | E::Abs(a) => a.size_bits()?,
            E::Add(a, b) | E::Sub(a, b) => a.size_bits()?.max(b.size_bits()?) + 1.0,
            E::Mul(a, b) | E::Div(a, b) => a.size_bits()? + b.size_bits()?,
            E::Pow(a, b) => {
                let e = b.eval()?;
                a.size_bits()? * e.n.to_f64()?.abs() / e.d.to_f64()?
            }
            E::Root(_, a) => a.size_bits()?,
            E::Fact(a) => {
                let n = a.eval()?.n.to_f64()?;
                n * n.max(2.0).log2()
            }
            E::Log(_, _) => 64.0,
        })
    }
}

// ───────────────────────── LaTeX to plain ─────────────────────────

/// The text inside the braces opening at `i` (s[i] == '{'), and the index after.
fn braced(s: &[char], i: usize) -> Option<(String, usize)> {
    if s.get(i) != Some(&'{') {
        return None;
    }
    let mut depth = 0;
    for j in i..s.len() {
        match s[j] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((s[i + 1..j].iter().collect(), j + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// One argument after a command: {...} or a single character.
fn arg(s: &[char], i: usize) -> Option<(String, usize)> {
    let mut i = i;
    while s.get(i) == Some(&' ') {
        i += 1;
    }
    if s.get(i) == Some(&'{') {
        return braced(s, i);
    }
    s.get(i).map(|c| (c.to_string(), i + 1))
}

/// LaTeX math to the calculator's plain syntax.
pub fn plain(tex: &str) -> String {
    let mut t = tex.to_string();
    for (a, b) in [
        ("\\left", ""), ("\\right", ""), ("\\!", ""), ("\\,", ""), ("\\;", ""), ("\\ ", " "), ("\\quad", " "), ("\\displaystyle", ""),
        ("\\cdot", "*"), ("\\times", "*"), ("\\div", "/"), ("\\dfrac", "\\frac"), ("\\tfrac", "\\frac"), ("{,}", ""), ("\\%", ""), ("$", ""),
    ] {
        t = t.replace(a, b);
    }
    let s: Vec<char> = t.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        let rest: String = s[i..s.len().min(i + 8)].iter().collect();
        if rest.starts_with("\\frac") {
            let Some((a, j)) = arg(&s, i + 5) else { break };
            let Some((b, k)) = arg(&s, j) else { break };
            out.push_str(&format!("(({})/({}))", plain(&a), plain(&b)));
            i = k;
        } else if rest.starts_with("\\sqrt") {
            let mut j = i + 5;
            let mut n = String::from("2");
            if s.get(j) == Some(&'[') {
                let close = (j..s.len()).find(|&k| s[k] == ']').unwrap_or(s.len() - 1);
                n = s[j + 1..close].iter().collect();
                j = close + 1;
            }
            let Some((a, k)) = arg(&s, j) else { break };
            out.push_str(&format!("root({},({}))", plain(&n), plain(&a)));
            i = k;
        } else if rest.starts_with("\\lfloor") {
            out.push_str("floor(");
            i += 7;
        } else if rest.starts_with("\\rfloor") {
            out.push(')');
            i += 7;
        } else if rest.starts_with("\\lceil") {
            out.push_str("ceil(");
            i += 6;
        } else if rest.starts_with("\\rceil") {
            out.push(')');
            i += 6;
        } else if rest.starts_with("\\log_") {
            let Some((b, j)) = arg(&s, i + 5) else { break };
            out.push_str(&format!("log[{}]", plain(&b)));
            i = j;
        } else if s[i] == '^' {
            let Some((a, j)) = arg(&s, i + 1) else { break };
            out.push_str(&format!("^({})", plain(&a)));
            i = j;
        } else if s[i] == '_' && out.chars().last().is_some_and(|c| c.is_ascii_alphanumeric()) {
            // a base literal: digits_{b}
            let Some((b, j)) = arg(&s, i + 1) else { break };
            let digits: String = out.chars().rev().take_while(|c| c.is_ascii_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
            let base: u32 = b.trim().parse().unwrap_or(0);
            match (2..=36).contains(&base).then(|| BigInt::parse_bytes(digits.to_ascii_lowercase().as_bytes(), base)).flatten() {
                Some(v) => {
                    out.truncate(out.len() - digits.len());
                    out.push_str(&v.to_string());
                }
                None => out.push('?'),
            }
            i = j;
        } else if s[i] == '{' {
            out.push('(');
            i += 1;
        } else if s[i] == '}' {
            out.push(')');
            i += 1;
        } else if s[i] == '\\' {
            // an unknown command: the expression cannot be read
            out.push('?');
            i += 1;
            while i < s.len() && s[i].is_ascii_alphabetic() {
                i += 1;
            }
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

// ───────────────────────── parsing ─────────────────────────

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn eat(&mut self, w: &str) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            true
        } else {
            false
        }
    }
    fn expr(&mut self) -> Option<E> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.i += 1;
                    v = E::Add(Box::new(v), Box::new(self.term()?));
                }
                Some(b'-') => {
                    self.i += 1;
                    v = E::Sub(Box::new(v), Box::new(self.term()?));
                }
                _ => return Some(v),
            }
        }
    }
    fn term(&mut self) -> Option<E> {
        let mut v = self.unary()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.i += 1;
                    v = E::Mul(Box::new(v), Box::new(self.unary()?));
                }
                Some(b'/') => {
                    self.i += 1;
                    v = E::Div(Box::new(v), Box::new(self.unary()?));
                }
                // implicit product: 2(3), (2)(3), 2root(...)
                Some(c) if c == b'(' || c.is_ascii_alphabetic() => v = E::Mul(Box::new(v), Box::new(self.unary()?)),
                _ => return Some(v),
            }
        }
    }
    fn unary(&mut self) -> Option<E> {
        if self.peek() == Some(b'-') {
            self.i += 1;
            return Some(E::Neg(Box::new(self.unary()?)));
        }
        if self.peek() == Some(b'+') {
            self.i += 1;
        }
        self.power()
    }
    fn power(&mut self) -> Option<E> {
        let mut base = self.postfix()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            let e = self.unary()?;
            base = E::Pow(Box::new(base), Box::new(e));
        }
        Some(base)
    }
    fn postfix(&mut self) -> Option<E> {
        let mut v = self.primary()?;
        while self.peek() == Some(b'!') {
            self.i += 1;
            v = E::Fact(Box::new(v));
        }
        Some(v)
    }
    fn primary(&mut self) -> Option<E> {
        let c = self.peek()?;
        if c == b'(' {
            self.i += 1;
            let v = self.expr()?;
            self.eat(")").then_some(v)
        } else if c == b'|' {
            self.i += 1;
            let v = self.expr()?;
            self.eat("|").then(|| E::Abs(Box::new(v)))
        } else if c.is_ascii_digit() || c == b'.' {
            let st = self.i;
            while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.' || (self.s[self.i] == b',' && self.s.get(self.i + 1..self.i + 4).is_some_and(|w| w.iter().all(u8::is_ascii_digit)) && !self.s.get(self.i + 4).is_some_and(u8::is_ascii_digit))) {
                self.i += 1;
            }
            let t: String = std::str::from_utf8(&self.s[st..self.i]).ok()?.replace(',', "");
            Some(E::Num(match t.split_once('.') {
                Some((w, f)) => Rat::new(format!("{w}{f}").parse().ok()?, BigInt::from(10).pow(f.len() as u32))?,
                None => Rat::int(t.parse::<BigInt>().ok()?),
            }))
        } else if self.eat("root(") {
            let k = self.expr()?.eval()?;
            self.eat(",").then_some(())?;
            let a = self.expr()?;
            self.eat(")").then_some(())?;
            Some(E::Root(k.n.to_u32().filter(|_| k.is_int())?, Box::new(a)))
        } else if self.eat("floor(") {
            let a = self.expr()?;
            self.eat(")").then(|| E::Floor(Box::new(a)))
        } else if self.eat("ceil(") {
            let a = self.expr()?;
            self.eat(")").then(|| E::Ceil(Box::new(a)))
        } else if self.eat("log[") {
            let b = self.expr()?;
            self.eat("]").then_some(())?;
            let x = self.power()?;
            Some(E::Log(Box::new(b), Box::new(x)))
        } else {
            None
        }
    }
}

/// A whole text as one expression (LaTeX allowed), or None.
pub fn parse(tex: &str) -> Option<E> {
    // a sentence's full stop may sit inside the math: "$\sqrt{9}.$"
    let p = plain(tex).trim().trim_end_matches(['.', ',']).to_string();
    if p.contains('?') {
        return None;
    }
    let mut parser = P { s: p.as_bytes(), i: 0 };
    let e = parser.expr()?;
    (parser.peek().is_none()).then_some(e)
}

// ───────────────────────── the questions it reads ─────────────────────────

#[derive(Clone, Debug)]
pub enum Q {
    Value(E),
    Remainder(E, BigInt),
    UnitsDigit(E),
    Gcd(Vec<BigInt>),
    Lcm(Vec<BigInt>),
    Inverse(BigInt, BigInt),
    Base(E, u32),
}

/// The question without its answer-format instructions ("Express your
/// answer as a common fraction.", "(Give an answer between 0 and 184.)").
pub fn strip_instructions(q: &str) -> String {
    let mut t = q.trim().to_string();
    loop {
        let low = t.to_lowercase();
        let cut = ["express your answer", "give your answer", "(give an answer", "(give your answer", "your answer should", "write your answer", "enter your answer", "(express your answer", "(your answer"]
            .iter()
            .filter_map(|w| low.rfind(w))
            .filter(|&k| k > 0)
            .min();
        match cut {
            Some(k) => t = t[..k].trim().to_string(),
            None => break,
        }
    }
    let low = t.to_lowercase();
    if let Some(k) = low.find(" as a residue modulo") {
        t = t[..k].to_string();
    }
    t.trim().trim_end_matches(['?', '.', '!', ':', ',']).trim().to_string()
}

fn int_of(tex: &str) -> Option<BigInt> {
    let r = parse(tex)?.eval()?;
    r.is_int().then_some(r.n)
}

fn after<'a>(t: &'a str, low: &str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find(|p| low.starts_with(*p)).map(|p| &t[p.len()..])
}

const ASK: &[&str] = &["what is ", "find ", "compute ", "calculate ", "determine ", "evaluate ", ""];

/// Read the whole question as one of the exact forms, or None. "Express
/// your answer in base b" asks for the value written in base b.
pub fn read(q: &str) -> Option<Q> {
    let low_q = q.to_lowercase().replace('$', "");
    if let Some(k) = low_q.find("answer in base ") {
        let b: u32 = low_q[k + 15..].trim_start().chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok()?;
        return match read_whole(q)? {
            Q::Value(e) => (2..=36).contains(&b).then(|| Q::Base(e, b)),
            other @ Q::Base(..) => Some(other),
            _ => None,
        };
    }
    read_whole(q)
}

fn read_whole(q: &str) -> Option<Q> {
    let t = strip_instructions(q);
    let t = t.trim();
    let low = t.to_lowercase();
    for ask in ASK {
        let Some(rest_low) = low.strip_prefix(ask) else { continue };
        let rest = &t[ask.len()..];
        // the remainder when E is divided by M
        for pre in ["the remainder when ", "the remainder of ", "the remainder after "] {
            if let Some(body) = rest_low.strip_prefix(pre) {
                let body_t = &rest[pre.len()..];
                for mid in [" is divided by ", " divided by ", " when divided by "] {
                    if let Some(k) = body.rfind(mid) {
                        let e = parse(&body_t[..k])?;
                        let m = int_of(&body_t[k + mid.len()..])?;
                        return (m > BigInt::zero()).then(|| Q::Remainder(e, m));
                    }
                }
            }
        }
        for pre in ["the units digit of ", "the ones digit of ", "the unit digit of ", "the last digit of "] {
            if let Some(body) = after(rest, rest_low, &[pre]) {
                return parse(body).map(Q::UnitsDigit);
            }
        }
        for pre in ["the residue of "] {
            if let Some(body) = after(rest, rest_low, &[pre]) {
                let bl = body.to_lowercase();
                let k = bl.rfind("modulo ")?;
                let e = parse(body[..k].trim().trim_end_matches(','))?;
                return Some(Q::Remainder(e, int_of(&body[k + 7..])?));
            }
        }
        for (pre, gcd) in [("the greatest common divisor of ", true), ("the greatest common factor of ", true), ("the gcd of ", true), ("the least common multiple of ", false), ("the lcm of ", false)] {
            if let Some(body) = after(rest, rest_low, &[pre]) {
                let parts: Vec<&str> = body.split(|c| c == ',').flat_map(|p| p.split(" and ")).map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
                let nums: Option<Vec<BigInt>> = parts.iter().map(|p| int_of(p)).collect();
                let nums = nums?;
                return (nums.len() >= 2).then(|| if gcd { Q::Gcd(nums) } else { Q::Lcm(nums) });
            }
        }
        for pre in ["the multiplicative inverse of ", "the modular inverse of ", "the inverse of "] {
            if let Some(body) = after(rest, rest_low, &[pre]) {
                let bl = body.to_lowercase();
                let k = bl.rfind("modulo ")?;
                let a = int_of(body[..k].trim().trim_end_matches(','))?;
                return Some(Q::Inverse(a, int_of(&body[k + 7..])?));
            }
        }
        // E \pmod{m}: a value or an inverse a^{-1}
        if let Some(k) = rest.find("\\pmod") {
            let (left, right) = (rest[..k].trim(), &rest[k + 5..]);
            let m = int_of(right.trim())?;
            let e = parse(left)?;
            if let E::Pow(a, b) = &e {
                if b.eval().is_some_and(|x| x == Rat::int(-1)) {
                    return Some(Q::Inverse(a.eval().filter(Rat::is_int)?.n, m));
                }
            }
            return Some(Q::Remainder(e, m));
        }
        // E in base b / E to base b
        for mid in [" in base ", " to base ", " as a base "] {
            if let Some(k) = rest_low.rfind(mid) {
                let b: u32 = rest_low[k + mid.len()..].trim().trim_end_matches(" number").trim_start_matches('-').parse().ok()?;
                let body = rest[..k].trim();
                let body = ["express ", "convert ", "write "].iter().find_map(|p| body.to_lowercase().starts_with(p).then(|| &body[p.len()..])).unwrap_or(body);
                return (2..=36).contains(&b).then(|| parse(body).map(|e| Q::Base(e, b))).flatten();
            }
        }
        if rest_low.starts_with("express ") || rest_low.starts_with("convert ") {
            continue;
        }
        // the value of E
        let body = after(rest, rest_low, &["the value of ", "the exact value of "]).unwrap_or(rest);
        let body = ["simplify ", "evaluate ", "compute "].iter().find_map(|p| body.to_lowercase().starts_with(p).then(|| &body[p.len()..])).unwrap_or(body);
        if !ask.is_empty() || ["simplify ", "evaluate ", "compute "].iter().any(|p| low.starts_with(p)) {
            if let Some(e) = parse(body) {
                return Some(Q::Value(e));
            }
        }
    }
    let low = low.as_str();
    for p in ["simplify ", "evaluate ", "compute "] {
        if let Some(body) = low.strip_prefix(p) {
            return parse(&t[p.len()..p.len() + body.len()]).map(Q::Value);
        }
    }
    None
}

fn to_base(mut v: BigInt, b: u32) -> String {
    if v.is_zero() {
        return "0".into();
    }
    let neg = v.is_negative();
    v = v.abs();
    let mut digits = Vec::new();
    let bb = BigInt::from(b);
    while !v.is_zero() {
        let d = (&v % &bb).to_u32().unwrap();
        digits.push(std::char::from_digit(d, b).unwrap().to_ascii_uppercase());
        v /= &bb;
    }
    let s: String = digits.into_iter().rev().collect();
    if neg {
        format!("-{s}")
    } else {
        s
    }
}

/// The answer to a read question: the short answer and the working, every
/// line exact.
pub fn solve(q: &Q) -> Option<(String, Vec<String>)> {
    match q {
        Q::Value(e) => {
            let v = e.eval()?;
            Some((v.show(), vec![format!("evaluated exactly: {}", v.show())]))
        }
        Q::Remainder(e, m) => {
            let r = e.eval_mod(m)?;
            let mut lines = vec![format!("the value mod {m} = {r}")];
            if let Some(v) = e.small_eval().filter(Rat::is_int) {
                let qt = (&v.n - &r) / m;
                lines.push(format!("check: {} = {m} x {qt} + {r}", v.n));
            } else {
                lines.push(format!("computed by repeated squaring mod {m}"));
            }
            Some((r.to_string(), lines))
        }
        Q::UnitsDigit(e) => {
            let r = e.eval_mod(&BigInt::from(10))?;
            Some((r.to_string(), vec![format!("the value mod 10 = {r}")]))
        }
        Q::Gcd(ns) => {
            let g = ns.iter().skip(1).fold(ns[0].abs(), |g, n| g.gcd(n));
            let lines = ns.iter().map(|n| format!("{n} = {g} x {}", n / &g)).chain(std::iter::once(format!("and the quotients have no common factor: gcd = {g}"))).collect();
            Some((g.to_string(), lines))
        }
        Q::Lcm(ns) => {
            let l = ns.iter().skip(1).fold(ns[0].abs(), |l, n| l.lcm(n));
            let lines = ns.iter().map(|n| format!("{l} = {n} x {}", &l / n)).collect();
            Some((l.to_string(), lines))
        }
        Q::Inverse(a, m) => {
            let e = a.extended_gcd(m);
            if !e.gcd.is_one() {
                return None;
            }
            let x = e.x.mod_floor(m);
            let check = (a * &x).mod_floor(m);
            Some((x.to_string(), vec![format!("by the extended Euclidean algorithm, {a} x {x} = {} ≡ {check} (mod {m})", a * &x)]))
        }
        Q::Base(e, b) => {
            let v = e.eval()?;
            if !v.is_int() {
                return None;
            }
            let s = to_base(v.n.clone(), *b);
            Some((if *b == 10 { s.clone() } else { format!("{s}_{b}") }, vec![format!("{} in base {b} is {s}", v.n)]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ans(q: &str) -> Option<String> {
        read(q).and_then(|x| solve(&x)).map(|x| x.0)
    }

    #[test]
    fn latex_values() {
        assert_eq!(parse("\\frac{1}{3}-\\frac{1}{9}").unwrap().eval().unwrap().show(), "2/9");
        assert_eq!(parse("\\sqrt[4]{12960000}").unwrap().eval().unwrap().show(), "60");
        assert_eq!(parse("\\log_2 (4^2)").unwrap().eval().unwrap().show(), "4");
        assert_eq!(parse("\\log_3\\frac{1}{27}").unwrap().eval().unwrap().show(), "-3");
        // log_3 (1/sqrt 3) = -1/2, but 1/sqrt 3 is no exact fraction: no value
        assert!(parse("\\log_3\\frac{1}{\\sqrt3}").unwrap().eval().is_none());
        assert_eq!(parse("\\lfloor-2.54\\rfloor+\\lceil25.4\\rceil").unwrap().eval().unwrap().show(), "23");
        assert_eq!(parse("(-125)^{4/3}").unwrap().eval().unwrap().show(), "625");
        assert_eq!(parse("10!").unwrap().eval().unwrap().show(), "3628800");
        assert_eq!(parse("218_9").unwrap().eval().unwrap().show(), "179");
        assert!(parse("\\sqrt{2}").unwrap().eval().is_none());
    }

    #[test]
    fn questions() {
        assert_eq!(ans("What is the remainder when 1,234,567,890 is divided by 99?").as_deref(), Some("72"));
        assert_eq!(ans("What is the remainder when $2^{19}$ is divided by $7$?").as_deref(), Some("2"));
        assert_eq!(ans("What is the units digit of $3^{2004}$?").as_deref(), Some("1"));
        assert_eq!(ans("What is the greatest common divisor of 1407 and 903?").as_deref(), Some("21"));
        assert_eq!(ans("What is the least common multiple of 135 and 468?").as_deref(), Some("7020"));
        assert_eq!(ans("Find $2^{-1} \\pmod{185}$, as a residue modulo 185. (Give an answer between 0 and 184, inclusive.)").as_deref(), Some("93"));
        assert_eq!(ans("Compute the multiplicative inverse of $201$ modulo $299$. Express your answer as an integer from $0$ to $298$.").as_deref(), Some("180"));
        assert_eq!(ans("What is the residue of $9^{2010}$, modulo 17?").as_deref(), Some("13"));
        assert_eq!(ans("Express $43210_{6}-3210_{7}$ in base 10.").as_deref(), Some("4776"));
        assert_eq!(ans("Evaluate $\\log_\\frac{1}{3}9$."), None);
        assert_eq!(ans("Compute $\\sqrt[4]{12960000}.$").as_deref(), Some("60"));
        assert_eq!(ans("What is the value of $252^2 - 248^2$?").as_deref(), Some("2000"));
        // not read whole: no answer
        assert!(read("Find the units digit of $n$ given that $mn = 21^6$ and $m$ has a units digit of 7.").is_none());
        assert!(read("What is the remainder when the sum of the first 100 primes is divided by 7?").is_none());
    }
}
