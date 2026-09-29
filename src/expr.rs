//! Expressions and the states a solution passes through.
//!
//! The tree keeps the notation a person wrote: subtraction is a sum with a
//! negated term and division stays a quotient, so every intermediate line
//! prints the way it would on paper. `tidy` only flattens structure; turning
//! 3 + 4 into 7 is a rule's job, because it is a step worth showing.

use crate::q::Q;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Exp,
    Ln,
    Sqrt,
}

impl Func {
    pub fn name(self) -> &'static str {
        match self {
            Func::Sin => "sin",
            Func::Cos => "cos",
            Func::Tan => "tan",
            Func::Exp => "exp",
            Func::Ln => "ln",
            Func::Sqrt => "sqrt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Konst {
    Pi,
    E,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Expr {
    Num(Q),
    Var(String),
    Const(Konst),
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Neg(Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Func(Func, Box<Expr>),
    /// d/d(var) of the inner expression, not yet worked out.
    Deriv(Box<Expr>, String),
}

use Expr::*;

pub fn num(n: i128) -> Expr {
    Num(Q::int(n))
}
pub fn q(v: Q) -> Expr {
    Num(v)
}
pub fn var(v: &str) -> Expr {
    Var(v.to_string())
}
pub fn add(v: Vec<Expr>) -> Expr {
    tidy(Add(v))
}
pub fn mul(v: Vec<Expr>) -> Expr {
    tidy(Mul(v))
}
pub fn neg(e: Expr) -> Expr {
    tidy(Neg(Box::new(e)))
}
pub fn div(a: Expr, b: Expr) -> Expr {
    Div(Box::new(a), Box::new(b))
}
pub fn pow(a: Expr, b: Expr) -> Expr {
    Pow(Box::new(a), Box::new(b))
}
pub fn func(f: Func, a: Expr) -> Expr {
    Func(f, Box::new(a))
}
pub fn sqrt(a: Expr) -> Expr {
    func(Func::Sqrt, a)
}

impl Expr {
    pub fn as_num(&self) -> Option<Q> {
        match self {
            Num(q) => Some(*q),
            _ => None,
        }
    }
    pub fn is_num(&self, v: i128) -> bool {
        matches!(self, Num(q) if *q == Q::int(v))
    }
    pub fn children(&self) -> Vec<&Expr> {
        match self {
            Add(v) | Mul(v) => v.iter().collect(),
            Neg(a) | Func(_, a) | Deriv(a, _) => vec![a],
            Div(a, b) | Pow(a, b) => vec![a, b],
            Num(_) | Var(_) | Const(_) => vec![],
        }
    }
    fn child_mut(&mut self, i: usize) -> &mut Expr {
        match self {
            Add(v) | Mul(v) => &mut v[i],
            Neg(a) | Func(_, a) | Deriv(a, _) => a,
            Div(a, b) | Pow(a, b) => {
                if i == 0 {
                    a
                } else {
                    b
                }
            }
            Num(_) | Var(_) | Const(_) => unreachable!("leaf has no children"),
        }
    }
    pub fn get(&self, path: &[usize]) -> &Expr {
        match path.split_first() {
            None => self,
            Some((i, rest)) => self.children()[*i].get(rest),
        }
    }
    /// Replace the node at `path`, then tidy the whole tree.
    pub fn replace(&self, path: &[usize], new: Expr) -> Expr {
        tidy(self.replace_raw(path, new))
    }
    /// Replace the node at `path`; nothing else changes.
    pub fn replace_raw(&self, path: &[usize], new: Expr) -> Expr {
        let mut out = self.clone();
        let mut cur = &mut out;
        for &i in path {
            cur = cur.child_mut(i);
        }
        *cur = new;
        out
    }
    /// Every node with its path, parents before children, left to right.
    pub fn walk(&self) -> Vec<(Vec<usize>, &Expr)> {
        let mut out = Vec::new();
        fn go<'a>(e: &'a Expr, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, &'a Expr)>) {
            out.push((path.clone(), e));
            for (i, c) in e.children().into_iter().enumerate() {
                path.push(i);
                go(c, path, out);
                path.pop();
            }
        }
        go(self, &mut Vec::new(), &mut out);
        out
    }
    pub fn size(&self) -> usize {
        1 + self.children().iter().map(|c| c.size()).sum::<usize>()
    }
    pub fn has_var(&self, v: &str) -> bool {
        match self {
            Var(x) => x == v,
            Deriv(a, _) => a.has_var(v),
            _ => self.children().iter().any(|c| c.has_var(v)),
        }
    }
    pub fn vars(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for (_, e) in self.walk() {
            if let Var(x) = e {
                out.insert(x.clone());
            }
        }
        out
    }
    pub fn has_deriv(&self) -> bool {
        self.walk().iter().any(|(_, e)| matches!(e, Deriv(..)))
    }
    pub fn subst(&self, v: &str, val: &Expr) -> Expr {
        match self {
            Var(x) if x == v => val.clone(),
            Deriv(..) => self.clone(),
            _ => {
                let mut out = self.clone();
                for i in 0..self.children().len() {
                    *out.child_mut(i) = self.children()[i].subst(v, val);
                }
                out
            }
        }
    }
    /// Exact value, when it is rational.
    pub fn eval_q(&self, env: &dyn Fn(&str) -> Option<Q>) -> Option<Q> {
        match self {
            Num(q) => Some(*q),
            Var(v) => env(v),
            Const(_) | Deriv(..) => None,
            Add(v) => v.iter().try_fold(Q::ZERO, |a, e| a.add(&e.eval_q(env)?)),
            Mul(v) => v.iter().try_fold(Q::ONE, |a, e| a.mul(&e.eval_q(env)?)),
            Neg(a) => Some(a.eval_q(env)?.neg()),
            Div(a, b) => a.eval_q(env)?.div(&b.eval_q(env)?),
            Pow(a, b) => {
                let e = b.eval_q(env)?;
                let base = a.eval_q(env)?;
                if e.is_int() {
                    if e.is_neg() && base.is_zero() {
                        return None;
                    }
                    base.pow(e.num() as i64)
                } else if e.den() == 2 {
                    base.sqrt()?.pow(e.num() as i64)
                } else {
                    None
                }
            }
            Func(crate::expr::Func::Sqrt, a) => a.eval_q(env)?.sqrt(),
            Func(..) => None,
        }
    }
    pub fn eval_f(&self, env: &dyn Fn(&str) -> f64) -> f64 {
        match self {
            Num(q) => q.to_f64(),
            Var(v) => env(v),
            Const(Konst::Pi) => std::f64::consts::PI,
            Const(Konst::E) => std::f64::consts::E,
            Add(v) => v.iter().map(|e| e.eval_f(env)).sum(),
            Mul(v) => v.iter().map(|e| e.eval_f(env)).product(),
            Neg(a) => -a.eval_f(env),
            Div(a, b) => a.eval_f(env) / b.eval_f(env),
            Pow(a, b) => {
                let (x, y) = (a.eval_f(env), b.eval_f(env));
                // odd roots of negatives stay real: (-8)^(1/3) = -2
                if x < 0.0 && y.fract() != 0.0 {
                    if let Some(qy) = b.eval_q(&|_| None) {
                        if qy.den() % 2 == 1 {
                            let r = (-x).powf(y);
                            return if qy.num() % 2 == 0 { r } else { -r };
                        }
                    }
                }
                x.powf(y)
            }
            Func(f, a) => {
                let x = a.eval_f(env);
                match f {
                    crate::expr::Func::Sin => x.sin(),
                    crate::expr::Func::Cos => x.cos(),
                    crate::expr::Func::Tan => x.tan(),
                    crate::expr::Func::Exp => x.exp(),
                    crate::expr::Func::Ln => x.ln(),
                    crate::expr::Func::Sqrt => x.sqrt(),
                }
            }
            Deriv(a, v) => {
                // numeric derivative, for checks only
                let h = 1e-5;
                let at = |d: f64| {
                    let env2 = |n: &str| if n == v { env(n) + d } else { env(n) };
                    a.eval_f(&env2)
                };
                (at(h) - at(-h)) / (2.0 * h)
            }
        }
    }
}

/// Structural cleanup only: flatten nested sums and products, unwrap
/// one-element ones, fold a sign into a number, cancel a double negation.
pub fn tidy(e: Expr) -> Expr {
    match e {
        Add(v) => {
            let mut out = Vec::new();
            for x in v.into_iter().map(tidy) {
                match x {
                    Add(inner) => out.extend(inner),
                    x => out.push(x),
                }
            }
            match out.len() {
                0 => num(0),
                1 => out.pop().unwrap(),
                _ => Add(out),
            }
        }
        Mul(v) => {
            let mut out = Vec::new();
            for x in v.into_iter().map(tidy) {
                match x {
                    Mul(inner) => out.extend(inner),
                    x => out.push(x),
                }
            }
            match out.len() {
                0 => num(1),
                1 => out.pop().unwrap(),
                _ => Mul(out),
            }
        }
        Neg(a) => match tidy(*a) {
            Num(q) => Num(q.neg()),
            Neg(b) => *b,
            Mul(mut v) if matches!(v.first(), Some(Num(_))) => {
                if let Num(c) = v[0] {
                    v[0] = Num(c.neg());
                }
                if v[0].is_num(1) {
                    v.remove(0);
                    tidy(Mul(v))
                } else {
                    Mul(v)
                }
            }
            x => Neg(Box::new(x)),
        },
        Div(a, b) => match (tidy(*a), tidy(*b)) {
            // 3/4 in lowest terms is how the number is written, not a division to do
            (Num(x), Num(y)) if x.is_int() && y.is_int() && y.num() > 1 && crate::q::gcd(x.num(), y.num()) == 1 => Num(Q::new(x.num(), y.num()).expect("nonzero denominator")),
            (a, b) => Div(Box::new(a), Box::new(b)),
        },
        Pow(a, b) => Pow(Box::new(tidy(*a)), Box::new(tidy(*b))),
        Func(f, a) => Func(f, Box::new(tidy(*a))),
        Deriv(a, v) => Deriv(Box::new(tidy(*a)), v),
        x => x,
    }
}

/// The terms of a sum (a lone expression is a one-term sum).
pub fn terms(e: &Expr) -> Vec<Expr> {
    match e {
        Add(v) => v.clone(),
        x => vec![x.clone()],
    }
}

/// Split a term into its numeric coefficient and the rest: -3x -> (-3, x),
/// x/2 -> (1/2, x), 5 -> (5, 1).
pub fn coeff(term: &Expr) -> (Q, Expr) {
    match term {
        Num(q) => (*q, num(1)),
        Neg(a) => {
            let (c, r) = coeff(a);
            (c.neg(), r)
        }
        Mul(v) => {
            let mut c = Q::ONE;
            let mut rest = Vec::new();
            for x in v {
                match x {
                    Num(q) => match c.mul(q) {
                        Some(p) => c = p,
                        None => return (Q::ONE, term.clone()),
                    },
                    x => rest.push(x.clone()),
                }
            }
            (c, mul(rest))
        }
        Div(a, b) => match b.as_num() {
            Some(d) if !d.is_zero() => {
                let (c, r) = coeff(a);
                match c.div(&d) {
                    Some(c) => (c, r),
                    None => (Q::ONE, term.clone()),
                }
            }
            _ => (Q::ONE, term.clone()),
        },
        x => (Q::ONE, x.clone()),
    }
}

/// The inverse of `coeff`: (-3, x) -> -3x, (1, x) -> x, (-1, x) -> -x.
pub fn with_coeff(c: Q, rest: Expr) -> Expr {
    if c.is_zero() {
        return num(0);
    }
    if rest.is_num(1) {
        return Num(c);
    }
    if c.is_one() {
        return rest;
    }
    if c == Q::int(-1) {
        return Neg(Box::new(rest));
    }
    let mut v = vec![Num(c)];
    match rest {
        Mul(r) => v.extend(r),
        r => v.push(r),
    }
    Mul(v)
}

/// What a solution is at each moment: an expression being worked on, an
/// equation, a set of alternatives ("x = 2 or x = 3"), or a verdict.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Math {
    Expr(Expr),
    Eq(Expr, Expr),
    Or(Vec<(Expr, Expr)>),
    NoSolution,
    AllReals,
}

impl Math {
    /// The expressions a rule may rewrite, in a fixed order: the expression;
    /// left then right side; each alternative's left then right side.
    pub fn slots(&self) -> Vec<&Expr> {
        match self {
            Math::Expr(e) => vec![e],
            Math::Eq(l, r) => vec![l, r],
            Math::Or(v) => v.iter().flat_map(|(l, r)| [l, r]).collect(),
            Math::NoSolution | Math::AllReals => vec![],
        }
    }
    pub fn with_slot(&self, i: usize, e: Expr) -> Math {
        match self {
            Math::Expr(_) => Math::Expr(e),
            Math::Eq(l, r) => {
                if i == 0 {
                    Math::Eq(e, r.clone())
                } else {
                    Math::Eq(l.clone(), e)
                }
            }
            Math::Or(v) => {
                let mut v = v.clone();
                if i % 2 == 0 {
                    v[i / 2].0 = e;
                } else {
                    v[i / 2].1 = e;
                }
                Math::Or(v)
            }
            m => m.clone(),
        }
    }
    pub fn size(&self) -> usize {
        self.slots().iter().map(|e| e.size()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_flattens_and_coeff_splits() {
        let e = tidy(Add(vec![Add(vec![var("x"), num(1)]), Neg(Box::new(num(2)))]));
        assert_eq!(e, Add(vec![var("x"), num(1), num(-2)]));
        let t = Mul(vec![num(-3), var("x")]);
        assert_eq!(coeff(&t), (Q::int(-3), var("x")));
        assert_eq!(with_coeff(Q::int(-1), var("x")), Neg(Box::new(var("x"))));
        let p = Add(vec![var("x"), num(2)]);
        assert_eq!(p.replace(&[1], num(5)), Add(vec![var("x"), num(5)]));
    }
}
