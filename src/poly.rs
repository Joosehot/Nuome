//! Polynomials in one variable with exact rational coefficients. The rules
//! use them to read a side as "ax² + bx + c"; the checks use them to count
//! an equation's real roots independently of the steps that solved it.

use crate::expr::{self, Expr, Func};
use crate::q::{gcd, lcm, Q};

/// Coefficients, lowest power first, no trailing zeros.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Poly(pub Vec<Q>);

impl Poly {
    pub fn zero() -> Poly {
        Poly(vec![])
    }
    pub fn constant(c: Q) -> Poly {
        Poly(vec![c]).trim()
    }
    pub fn x() -> Poly {
        Poly(vec![Q::ZERO, Q::ONE])
    }
    fn trim(mut self) -> Poly {
        while self.0.last().is_some_and(|c| c.is_zero()) {
            self.0.pop();
        }
        self
    }
    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    /// Degree; the zero polynomial has none.
    pub fn deg(&self) -> Option<usize> {
        self.0.len().checked_sub(1)
    }
    pub fn coef(&self, i: usize) -> Q {
        self.0.get(i).copied().unwrap_or(Q::ZERO)
    }
    pub fn lead(&self) -> Q {
        self.0.last().copied().unwrap_or(Q::ZERO)
    }
    pub fn add(&self, o: &Poly) -> Option<Poly> {
        let n = self.0.len().max(o.0.len());
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(self.coef(i).add(&o.coef(i))?);
        }
        Some(Poly(v).trim())
    }
    pub fn scale(&self, c: &Q) -> Option<Poly> {
        Some(Poly(self.0.iter().map(|a| a.mul(c)).collect::<Option<_>>()?).trim())
    }
    pub fn sub(&self, o: &Poly) -> Option<Poly> {
        self.add(&o.scale(&Q::int(-1))?)
    }
    pub fn mul(&self, o: &Poly) -> Option<Poly> {
        if self.is_zero() || o.is_zero() {
            return Some(Poly::zero());
        }
        let mut v = vec![Q::ZERO; self.0.len() + o.0.len() - 1];
        for (i, a) in self.0.iter().enumerate() {
            for (j, b) in o.0.iter().enumerate() {
                v[i + j] = v[i + j].add(&a.mul(b)?)?;
            }
        }
        Some(Poly(v).trim())
    }
    pub fn pow(&self, n: u32) -> Option<Poly> {
        if n > 64 {
            return None;
        }
        let mut r = Poly::constant(Q::ONE);
        for _ in 0..n {
            r = r.mul(self)?;
        }
        Some(r)
    }
    pub fn eval(&self, x: &Q) -> Option<Q> {
        self.0.iter().rev().try_fold(Q::ZERO, |acc, c| acc.mul(x)?.add(c))
    }
    pub fn eval_f(&self, x: f64) -> f64 {
        self.0.iter().rev().fold(0.0, |acc, c| acc * x + c.to_f64())
    }
    /// Divide by (x - r), when r is a root.
    pub fn deflate(&self, r: &Q) -> Option<Poly> {
        let n = self.0.len();
        if n < 2 {
            return None;
        }
        let mut out = vec![Q::ZERO; n - 1];
        let mut carry = Q::ZERO;
        for i in (1..n).rev() {
            carry = carry.mul(r)?.add(&self.0[i])?;
            out[i - 1] = carry;
        }
        let rem = carry.mul(r)?.add(&self.0[0])?;
        rem.is_zero().then(|| Poly(out).trim())
    }
    /// Same roots, integer coefficients with no common factor.
    pub fn primitive(&self) -> Option<Vec<i128>> {
        let mut l = 1i128;
        for c in &self.0 {
            l = lcm(l, c.den())?;
        }
        let ints: Vec<i128> = self.0.iter().map(|c| c.num().checked_mul(l / c.den())).collect::<Option<_>>()?;
        let g = ints.iter().fold(0, |g, &a| gcd(g, a)).max(1);
        Some(ints.iter().map(|a| a / g).collect())
    }
    /// Every distinct rational root (rational root theorem).
    pub fn rational_roots(&self) -> Vec<Q> {
        let mut out: Vec<Q> = Vec::new();
        let Some(mut ints) = self.primitive() else { return out };
        // strip x factors: 0 is a root
        if ints.len() > 1 && ints[0] == 0 {
            out.push(Q::ZERO);
            while ints.len() > 1 && ints[0] == 0 {
                ints.remove(0);
            }
        }
        if ints.len() < 2 {
            return out;
        }
        let p = Poly(ints.iter().map(|&a| Q::int(a)).collect());
        let (a0, an) = (ints[0].abs(), ints[ints.len() - 1].abs());
        let (Some(ps), Some(qs)) = (divisors(a0), divisors(an)) else { return out };
        for &pp in &ps {
            for &qq in &qs {
                for sign in [1, -1] {
                    let Some(r) = Q::new(sign * pp, qq) else { continue };
                    if !out.contains(&r) && p.eval(&r).is_some_and(|v| v.is_zero()) {
                        out.push(r);
                    }
                }
            }
        }
        out.sort();
        out
    }
    /// How many distinct real roots, split into (rational roots, count of
    /// irrational real roots). None when the degree is too high to be sure.
    pub fn real_roots(&self) -> Option<(Vec<Q>, usize)> {
        let rational = self.rational_roots();
        let mut rest = self.clone();
        for r in &rational {
            while let Some(d) = rest.deflate(r) {
                rest = d;
            }
        }
        let irrational = match rest.deg()? {
            0 => 0,
            1 => return None, // a linear factor has a rational root: arithmetic went wrong
            2 => {
                let (a, b, c) = (rest.coef(2), rest.coef(1), rest.coef(0));
                let d = b.mul(&b)?.sub(&Q::int(4).mul(&a)?.mul(&c)?)?;
                if d.is_neg() {
                    0
                } else {
                    2
                }
            }
            3 => {
                let (a, b, c, d) = (rest.coef(3), rest.coef(2), rest.coef(1), rest.coef(0));
                let f = |v: f64| v;
                let (a, b, c, d) = (f(a.to_f64()), f(b.to_f64()), f(c.to_f64()), f(d.to_f64()));
                let disc = 18.0 * a * b * c * d - 4.0 * b.powi(3) * d + b * b * c * c - 4.0 * a * c.powi(3) - 27.0 * a * a * d * d;
                if disc > 0.0 {
                    3
                } else {
                    1
                }
            }
            _ => return None,
        };
        Some((rational, irrational))
    }
    /// Descending-power expression: 2x^2 - 3x + 1.
    pub fn to_expr(&self, v: &str) -> Expr {
        let mut terms = Vec::new();
        for i in (0..self.0.len()).rev() {
            let c = self.0[i];
            if c.is_zero() {
                continue;
            }
            let m = match i {
                0 => expr::num(1),
                1 => expr::var(v),
                _ => expr::pow(expr::var(v), expr::num(i as i128)),
            };
            terms.push(expr::with_coeff(c, m));
        }
        expr::add(terms)
    }
}

fn divisors(n: i128) -> Option<Vec<i128>> {
    if n == 0 {
        return Some(vec![1]);
    }
    let mut out = Vec::new();
    let mut d = 1i128;
    while d * d <= n {
        if d > 1_000_000 {
            return None;
        }
        if n % d == 0 {
            out.push(d);
            if d != n / d {
                out.push(n / d);
            }
        }
        d += 1;
    }
    out.sort();
    Some(out)
}

/// Read an expression as a polynomial in `v`. Other letters, functions and
/// division by anything but a number make it not one.
pub fn from_expr(e: &Expr, v: &str) -> Option<Poly> {
    match e {
        Expr::Num(q) => Some(Poly::constant(*q)),
        Expr::Var(x) if x == v => Some(Poly::x()),
        Expr::Add(t) => t.iter().try_fold(Poly::zero(), |acc, x| acc.add(&from_expr(x, v)?)),
        Expr::Mul(t) => t.iter().try_fold(Poly::constant(Q::ONE), |acc, x| acc.mul(&from_expr(x, v)?)),
        Expr::Neg(a) => from_expr(a, v)?.scale(&Q::int(-1)),
        Expr::Div(a, b) => {
            let d = b.eval_q(&|_| None)?;
            from_expr(a, v)?.scale(&d.recip()?)
        }
        Expr::Pow(a, b) => {
            let n = b.as_num().filter(|q| q.is_int() && !q.is_neg())?;
            from_expr(a, v)?.pow(u32::try_from(n.num()).ok()?)
        }
        Expr::Func(Func::Sqrt, _) | Expr::Const(_) => {
            // a constant like sqrt(2) is fine only if exact
            Some(Poly::constant(e.eval_q(&|_| None)?))
        }
        _ => None,
    }
}

/// Read an expression as a quotient of polynomials in `v`: (numerator, denominator).
pub fn rational_from_expr(e: &Expr, v: &str) -> Option<(Poly, Poly)> {
    let one = || Poly::constant(Q::ONE);
    match e {
        Expr::Add(t) => {
            let mut acc = (Poly::zero(), one());
            for x in t {
                let (n, d) = rational_from_expr(x, v)?;
                acc = (acc.0.mul(&d)?.add(&n.mul(&acc.1)?)?, acc.1.mul(&d)?);
            }
            Some(acc)
        }
        Expr::Mul(t) => {
            let mut acc = (one(), one());
            for x in t {
                let (n, d) = rational_from_expr(x, v)?;
                acc = (acc.0.mul(&n)?, acc.1.mul(&d)?);
            }
            Some(acc)
        }
        Expr::Neg(a) => {
            let (n, d) = rational_from_expr(a, v)?;
            Some((n.scale(&Q::int(-1))?, d))
        }
        Expr::Div(a, b) => {
            let (n1, d1) = rational_from_expr(a, v)?;
            let (n2, d2) = rational_from_expr(b, v)?;
            if n2.is_zero() {
                return None;
            }
            Some((n1.mul(&d2)?, d1.mul(&n2)?))
        }
        Expr::Pow(a, b) => {
            let n = b.as_num().filter(|q| q.is_int())?;
            let (pn, pd) = rational_from_expr(a, v)?;
            let k = u32::try_from(n.num().abs()).ok()?;
            if n.is_neg() {
                Some((pd.pow(k)?, pn.pow(k)?))
            } else {
                Some((pn.pow(k)?, pd.pow(k)?))
            }
        }
        _ => Some((from_expr(e, v)?, one())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::*;

    #[test]
    fn roots_are_counted_independently() {
        // x^2 - 5x + 6
        let e = add(vec![pow(var("x"), num(2)), mul(vec![num(-5), var("x")]), num(6)]);
        let p = from_expr(&e, "x").unwrap();
        assert_eq!(p.rational_roots(), vec![Q::int(2), Q::int(3)]);
        assert_eq!(p.real_roots().unwrap(), (vec![Q::int(2), Q::int(3)], 0));
        // x^2 - 2: two irrational roots
        let p = Poly(vec![Q::int(-2), Q::ZERO, Q::ONE]);
        assert_eq!(p.real_roots().unwrap(), (vec![], 2));
        // x^2 + 1: none
        let p = Poly(vec![Q::ONE, Q::ZERO, Q::ONE]);
        assert_eq!(p.real_roots().unwrap(), (vec![], 0));
        assert_eq!(p.to_expr("x"), add(vec![pow(var("x"), num(2)), num(1)]));
    }
}
