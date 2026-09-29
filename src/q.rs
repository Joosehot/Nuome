//! Exact rational numbers. Every number the engine computes with is one of
//! these; a result that would overflow is refused, never rounded.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Q {
    n: i128,
    d: i128,
}

pub fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

pub fn lcm(a: i128, b: i128) -> Option<i128> {
    if a == 0 || b == 0 {
        return Some(0);
    }
    (a / gcd(a, b)).checked_mul(b).map(i128::abs)
}

impl Q {
    pub const ZERO: Q = Q { n: 0, d: 1 };
    pub const ONE: Q = Q { n: 1, d: 1 };

    pub fn new(n: i128, d: i128) -> Option<Q> {
        if d == 0 {
            return None;
        }
        let g = gcd(n, d).max(1);
        let (mut n, mut d) = (n / g, d / g);
        if d < 0 {
            n = n.checked_neg()?;
            d = d.checked_neg()?;
        }
        Some(Q { n, d })
    }
    pub const fn int(n: i128) -> Q {
        Q { n, d: 1 }
    }
    pub fn num(&self) -> i128 {
        self.n
    }
    pub fn den(&self) -> i128 {
        self.d
    }
    pub fn is_int(&self) -> bool {
        self.d == 1
    }
    pub fn is_zero(&self) -> bool {
        self.n == 0
    }
    pub fn is_one(&self) -> bool {
        self.n == 1 && self.d == 1
    }
    pub fn is_neg(&self) -> bool {
        self.n < 0
    }
    pub fn abs(&self) -> Q {
        Q { n: self.n.abs(), d: self.d }
    }
    pub fn neg(&self) -> Q {
        Q { n: -self.n, d: self.d }
    }
    pub fn add(&self, o: &Q) -> Option<Q> {
        let l = lcm(self.d, o.d)?;
        let a = self.n.checked_mul(l / self.d)?;
        let b = o.n.checked_mul(l / o.d)?;
        Q::new(a.checked_add(b)?, l)
    }
    pub fn sub(&self, o: &Q) -> Option<Q> {
        self.add(&o.neg())
    }
    pub fn mul(&self, o: &Q) -> Option<Q> {
        let g1 = gcd(self.n, o.d).max(1);
        let g2 = gcd(o.n, self.d).max(1);
        Q::new((self.n / g1).checked_mul(o.n / g2)?, (self.d / g2).checked_mul(o.d / g1)?)
    }
    pub fn div(&self, o: &Q) -> Option<Q> {
        if o.is_zero() {
            return None;
        }
        self.mul(&Q::new(o.d, o.n)?)
    }
    pub fn recip(&self) -> Option<Q> {
        Q::new(self.d, self.n)
    }
    pub fn pow(&self, e: i64) -> Option<Q> {
        if e < 0 {
            return self.recip()?.pow(-e);
        }
        if e > 256 {
            return if self.n.abs() <= 1 && self.d == 1 { Some(if self.n == -1 && e % 2 == 1 { *self } else { self.abs() }) } else { None };
        }
        let mut r = Q::ONE;
        for _ in 0..e {
            r = r.mul(self)?;
        }
        Some(r)
    }
    /// Exact square root, when there is one.
    pub fn sqrt(&self) -> Option<Q> {
        if self.n < 0 {
            return None;
        }
        Some(Q { n: isqrt(self.n)?, d: isqrt(self.d)? })
    }
    pub fn to_f64(&self) -> f64 {
        self.n as f64 / self.d as f64
    }
    /// Parse "12", "0.25", "3.5".
    pub fn parse(s: &str) -> Option<Q> {
        match s.split_once('.') {
            None => s.parse().ok().map(Q::int),
            Some((a, b)) => {
                let digits = format!("{a}{b}");
                let n: i128 = digits.parse().ok()?;
                Q::new(n, 10i128.checked_pow(b.len() as u32)?)
            }
        }
    }
    /// Round to `places` decimals, as text.
    pub fn decimal(&self, places: u32) -> String {
        decimal(self.to_f64(), places)
    }
}

pub fn decimal(x: f64, places: u32) -> String {
    let s = format!("{:.*}", places as usize, x);
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_string()
    } else {
        s
    }
}

/// Exact integer square root, when `n` is a perfect square.
pub fn isqrt(n: i128) -> Option<i128> {
    if n < 0 {
        return None;
    }
    let mut r = (n as f64).sqrt() as i128;
    while r > 0 && r.checked_mul(r).map_or(true, |s| s > n) {
        r -= 1;
    }
    while (r + 1).checked_mul(r + 1).is_some_and(|s| s <= n) {
        r += 1;
    }
    (r * r == n).then_some(r)
}

/// n = k² · m with m square-free: returns (k, m).
pub fn split_square(n: i128) -> (i128, i128) {
    let (mut k, mut m) = (1i128, n);
    let mut p = 2i128;
    while p * p <= m {
        while m % (p * p) == 0 {
            m /= p * p;
            k *= p;
        }
        p += 1;
    }
    (k, m)
}

impl PartialOrd for Q {
    fn partial_cmp(&self, o: &Q) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for Q {
    fn cmp(&self, o: &Q) -> Ordering {
        match (self.n.checked_mul(o.d), o.n.checked_mul(self.d)) {
            (Some(a), Some(b)) => a.cmp(&b),
            _ => self.to_f64().partial_cmp(&o.to_f64()).unwrap_or(Ordering::Equal),
        }
    }
}

impl fmt::Display for Q {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.d == 1 {
            write!(f, "{}", self.n)
        } else {
            write!(f, "{}/{}", self.n, self.d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_is_exact() {
        let a = Q::new(3, 4).unwrap();
        let b = Q::new(1, 6).unwrap();
        assert_eq!(a.add(&b).unwrap(), Q::new(11, 12).unwrap());
        assert_eq!(Q::parse("0.25").unwrap(), Q::new(1, 4).unwrap());
        assert_eq!(Q::new(9, 4).unwrap().sqrt(), Some(Q::new(3, 2).unwrap()));
        assert_eq!(Q::int(8).sqrt(), None);
        assert_eq!(split_square(72), (6, 2));
        assert!(Q::int(1).div(&Q::ZERO).is_none());
    }
}
