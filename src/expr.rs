//! Expressions and the states a solution passes through.
//!
//! The tree keeps the notation a person wrote: subtraction is a sum with a
//! negated term and division stays a quotient, so every intermediate line
//! prints the way it would on paper. `tidy` only flattens structure; turning
//! 3 + 4 into 7 is a rule's job, because it is a step worth showing.

use crate::calls::{self, Named};
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
    // calculus and trig (agent B)
    Abs,
    Sec,
    Csc,
    Cot,
    Asin,
    Acos,
    Atan,
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
            Func::Abs => "abs",
            Func::Sec => "sec",
            Func::Csc => "csc",
            Func::Cot => "cot",
            Func::Asin => "arcsin",
            Func::Acos => "arccos",
            Func::Atan => "arctan",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Konst {
    Pi,
    E,
    /// Infinity: only the point a limit approaches, or a limit's value.
    Inf,
    /// One degree, pi/180: 45 degrees is 45 * Deg.
    Deg,
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
    /// A named operation on numbers: gcd(48, 18), 10 choose 3, mean(...).
    Call(Named, Vec<Expr>),
    /// log to a base: Log(base, argument). log(100) is Log(10, 100).
    Log(Box<Expr>, Box<Expr>),
    /// The indefinite integral of the inner expression d(var), not yet worked out.
    Integral(Box<Expr>, String),
    /// [F]_a^b = F(b) - F(a), with (var) bound. Around an integral it is the
    /// definite integral from a to b.
    Bounds(Box<Expr>, String, Box<Expr>, Box<Expr>),
    /// The limit of the inner expression as (var) approaches the point.
    Limit(Box<Expr>, String, Box<Expr>),
    /// The inner expression with (var) = the point: f(1), f'(1).
    At(Box<Expr>, String, Box<Expr>),
    // logic and sets (agent L)
    /// A statement built with a connective: not (one part), and, or (two or
    /// more, flat), implies, iff (two).
    Logic(Conn, Vec<Expr>),
    /// true (T) or false (F).
    Truth(bool),
    /// A set built with an operation: complement (one part), union,
    /// intersection (two or more, flat), difference (two).
    Set(SetOp, Vec<Expr>),
    /// The universal set U (true) or the empty set (false).
    SetConst(bool),
    /// "x is in A": the element's letter and the set.
    Member(String, Box<Expr>),
    /// "for all x" (true) or "there exists x" (false), the letter it binds,
    /// and the statement about it.
    Quant(bool, String, Box<Expr>),
    /// A one-place predicate of a letter: P(x).
    Pred(String, String),
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
            Add(v) | Mul(v) | Call(_, v) => v.iter().collect(),
            Neg(a) | Func(_, a) | Deriv(a, _) => vec![a],
            Div(a, b) | Pow(a, b) => vec![a, b],
            Log(b, a) => vec![b, a],
            Num(_) | Var(_) | Const(_) => vec![],
            Integral(a, _) => vec![a],
            Bounds(f, _, a, b) => vec![f, a, b],
            Limit(a, _, p) | At(a, _, p) => vec![a, p],
            // logic and sets (agent L)
            Logic(_, v) | Set(_, v) => v.iter().collect(),
            Truth(_) | SetConst(_) | Pred(..) => vec![],
            Member(_, a) | Quant(_, _, a) => vec![a],
        }
    }
    fn child_mut(&mut self, i: usize) -> &mut Expr {
        match self {
            Add(v) | Mul(v) | Call(_, v) => &mut v[i],
            Neg(a) | Func(_, a) | Deriv(a, _) => a,
            Div(a, b) | Pow(a, b) | Log(a, b) => {
                if i == 0 {
                    a
                } else {
                    b
                }
            }
            Num(_) | Var(_) | Const(_) => unreachable!("leaf has no children"),
            Integral(a, _) => a,
            Bounds(f, _, a, b) => match i {
                0 => f,
                1 => a,
                _ => b,
            },
            Limit(a, _, p) | At(a, _, p) => {
                if i == 0 {
                    a
                } else {
                    p
                }
            }
            // logic and sets (agent L)
            Logic(_, v) | Set(_, v) => &mut v[i],
            Truth(_) | SetConst(_) | Pred(..) => unreachable!("leaf has no children"),
            Member(_, a) | Quant(_, _, a) => a,
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
            // a bound letter isn't free: [F]_0^3, lim(x->2), [f]_(x=1)
            Bounds(_, w, a, b) if w == v => a.has_var(v) || b.has_var(v),
            Limit(_, w, p) | At(_, w, p) if w == v => p.has_var(v),
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
    /// A named operation still to be worked out (gcd, mean, ...).
    pub fn has_call(&self) -> bool {
        self.walk().iter().any(|(_, e)| matches!(e, Call(..)))
    }
    pub fn has_deriv(&self) -> bool {
        self.walk().iter().any(|(_, e)| matches!(e, Deriv(..)))
    }
    /// Is calculus notation still waiting to be worked out: a derivative,
    /// an integral, bounds, a limit or a value at a point?
    pub fn is_pending(&self) -> bool {
        self.walk().iter().any(|(_, e)| matches!(e, Deriv(..) | Integral(..) | Bounds(..) | Limit(..) | At(..)))
    }
    pub fn subst(&self, v: &str, val: &Expr) -> Expr {
        match self {
            Var(x) if x == v => val.clone(),
            Deriv(..) | Integral(..) => self.clone(),
            // the bound letter stays; only the bounds or the point change
            Bounds(_, w, ..) | Limit(_, w, _) | At(_, w, _) if w == v => {
                let mut out = self.clone();
                for i in 1..self.children().len() {
                    *out.child_mut(i) = self.children()[i].subst(v, val);
                }
                out
            }
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
                    base.root(e.den() as i64)?.pow(e.num() as i64)
                }
            }
            Func(crate::expr::Func::Sqrt, a) => a.eval_q(env)?.sqrt(),
            Func(crate::expr::Func::Abs, a) => Some(a.eval_q(env)?.abs()),
            Func(..) => None,
            Call(f, args) => calls::eval_q(*f, &args.iter().map(|a| a.eval_q(env)).collect::<Option<Vec<_>>>()?),
            Log(b, a) => exact_log(&b.eval_q(env)?, &a.eval_q(env)?),
            Integral(..) | Limit(..) => None,
            // logic and sets (agent L): truth values, not numbers
            Logic(..) | Truth(_) | Set(..) | SetConst(_) | Member(..) | Quant(..) | Pred(..) => None,
            At(e, v, p) => {
                let at = p.eval_q(env)?;
                e.eval_q(&|n| if n == v { Some(at) } else { env(n) })
            }
            Bounds(f, v, a, b) => {
                if f.walk().iter().any(|(_, n)| matches!(n, Integral(..))) {
                    return None;
                }
                let (lo, hi) = (a.eval_q(env)?, b.eval_q(env)?);
                let top = f.eval_q(&|n| if n == v { Some(hi) } else { env(n) })?;
                top.sub(&f.eval_q(&|n| if n == v { Some(lo) } else { env(n) })?)
            }
        }
    }
    pub fn eval_f(&self, env: &dyn Fn(&str) -> f64) -> f64 {
        match self {
            Num(q) => q.to_f64(),
            Var(v) => env(v),
            Const(Konst::Pi) => std::f64::consts::PI,
            Const(Konst::E) => std::f64::consts::E,
            Const(Konst::Inf) => f64::INFINITY,
            Const(Konst::Deg) => std::f64::consts::PI / 180.0,
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
                    crate::expr::Func::Abs => x.abs(),
                    crate::expr::Func::Sec => 1.0 / x.cos(),
                    crate::expr::Func::Csc => 1.0 / x.sin(),
                    crate::expr::Func::Cot => x.cos() / x.sin(),
                    crate::expr::Func::Asin => x.asin(),
                    crate::expr::Func::Acos => x.acos(),
                    crate::expr::Func::Atan => x.atan(),
                }
            }
            Log(b, a) => {
                let (b, x) = (b.eval_f(env), a.eval_f(env));
                if b <= 0.0 || b == 1.0 {
                    f64::NAN
                } else {
                    x.ln() / b.ln()
                }
            }
            Call(f, args) => calls::eval_f(*f, &args.iter().map(|a| a.eval_f(env)).collect::<Vec<_>>()),
            Deriv(a, v) => {
                // numeric derivative, for checks only
                let h = 1e-5;
                let at = |d: f64| {
                    let env2 = |n: &str| if n == v { env(n) + d } else { env(n) };
                    a.eval_f(&env2)
                };
                (at(h) - at(-h)) / (2.0 * h)
            }
            // an indefinite integral has no single value; a limit is estimated by the checks
            Integral(..) | Limit(..) => f64::NAN,
            // logic and sets (agent L): truth values, not numbers
            Logic(..) | Truth(_) | Set(..) | SetConst(_) | Member(..) | Quant(..) | Pred(..) => f64::NAN,
            At(e, v, p) => {
                let at = p.eval_f(env);
                e.eval_f(&|n| if n == v { at } else { env(n) })
            }
            Bounds(f, v, a, b) => {
                let (lo, hi) = (a.eval_f(env), b.eval_f(env));
                match &**f {
                    // the definite integral itself: Simpson's rule, for checks only
                    Integral(g, w) if w == v => simpson(&|t| g.eval_f(&|n| if n == v { t } else { env(n) }), lo, hi, QUADRATURE_PANELS),
                    // F(b) - F(a), each integral in F measured from a
                    _ => {
                        let anchored = anchor_integrals(f, v, a);
                        let at = |t: f64| anchored.eval_f(&|n| if n == v { t } else { env(n) });
                        at(hi) - at(lo)
                    }
                }
            }
        }
    }
}

/// Panels for the quadrature inside `eval_f` (for checks only, like the
/// step of the numerical derivative above); the definite-integral check
/// takes its own count from rules.toml.
const QUADRATURE_PANELS: usize = 1000;

/// Composite Simpson's rule over [a, b] with `n` panels (rounded up to even).
pub fn simpson(f: &dyn Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
    let n = n.max(2).div_ceil(2) * 2;
    let h = (b - a) / n as f64;
    let mut s = f(a) + f(b);
    for i in 1..n {
        s += f(a + i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * h / 3.0
}

/// Every integral d(v) inside `e` becomes the definite integral from `from`
/// to v: one antiderivative out of all of them, so it has a value.
pub fn anchor_integrals(e: &Expr, v: &str, from: &Expr) -> Expr {
    match e {
        Integral(_, w) if w == v => Bounds(Box::new(e.clone()), v.to_string(), Box::new(from.clone()), Box::new(Var(v.to_string()))),
        _ => {
            let mut out = e.clone();
            for i in 0..e.children().len() {
                *out.child_mut(i) = anchor_integrals(e.children()[i], v, from);
            }
            out
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
        Call(f, args) => Call(f, args.into_iter().map(tidy).collect()),
        Log(b, a) => Log(Box::new(tidy(*b)), Box::new(tidy(*a))),
        Integral(a, v) => Integral(Box::new(tidy(*a)), v),
        Bounds(f, v, a, b) => Bounds(Box::new(tidy(*f)), v, Box::new(tidy(*a)), Box::new(tidy(*b))),
        Limit(a, v, p) => Limit(Box::new(tidy(*a)), v, Box::new(tidy(*p))),
        At(a, v, p) => At(Box::new(tidy(*a)), v, Box::new(tidy(*p))),
        // logic and sets (agent L): and, or, union, intersection are flat
        Logic(c, v) => {
            let v: Vec<Expr> = v.into_iter().map(tidy).collect();
            if matches!(c, Conn::And | Conn::Or) {
                let mut out = Vec::new();
                for x in v {
                    match x {
                        Logic(d, inner) if d == c => out.extend(inner),
                        x => out.push(x),
                    }
                }
                if out.len() == 1 {
                    return out.pop().unwrap();
                }
                Logic(c, out)
            } else {
                Logic(c, v)
            }
        }
        Set(o, v) => {
            let v: Vec<Expr> = v.into_iter().map(tidy).collect();
            if matches!(o, SetOp::Union | SetOp::Inter) {
                let mut out = Vec::new();
                for x in v {
                    match x {
                        Set(d, inner) if d == o => out.extend(inner),
                        x => out.push(x),
                    }
                }
                if out.len() == 1 {
                    return out.pop().unwrap();
                }
                Set(o, out)
            } else {
                Set(o, v)
            }
        }
        Member(x, a) => Member(x, Box::new(tidy(*a))),
        Quant(q, x, a) => Quant(q, x, Box::new(tidy(*a))),
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
    /// An inequality: left, sign, right.
    Ineq(Expr, Rel, Expr),
    /// The answer to an inequality: the letter and a union of intervals, left to right.
    Intervals(String, Vec<Interval>),
    /// Simultaneous equations, all to hold at once.
    System(Vec<(Expr, Expr)>),
    /// The end of a proof: the statement holds.
    Proved,
    // logic and sets (agent L)
    /// A statement to prove true for every choice of its letters (a tautology).
    Taut(Expr),
    /// Two statements to prove logically equivalent.
    Equiv(Expr, Expr),
    /// Assuming the left statement, prove the right one.
    Entails(Expr, Expr),
    /// One set inside another.
    Subset(Expr, Expr),
}

impl Math {
    /// The expressions a rule may rewrite, in a fixed order: the expression;
    /// left then right side; each alternative's left then right side.
    pub fn slots(&self) -> Vec<&Expr> {
        match self {
            Math::Expr(e) => vec![e],
            Math::Eq(l, r) => vec![l, r],
            Math::Or(v) => v.iter().flat_map(|(l, r)| [l, r]).collect(),
            Math::NoSolution | Math::AllReals | Math::Proved => vec![],
            Math::Ineq(l, _, r) => vec![l, r],
            Math::Intervals(_, v) => v.iter().flat_map(|i| i.lo.iter().chain(i.hi.iter()).map(|b| &b.at)).collect(),
            Math::System(v) => v.iter().flat_map(|(l, r)| [l, r]).collect(),
            // logic and sets (agent L)
            Math::Taut(e) => vec![e],
            Math::Equiv(l, r) | Math::Entails(l, r) | Math::Subset(l, r) => vec![l, r],
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
            Math::Ineq(l, rel, r) => {
                if i == 0 {
                    Math::Ineq(e, *rel, r.clone())
                } else {
                    Math::Ineq(l.clone(), *rel, e)
                }
            }
            Math::Intervals(x, v) => {
                let mut v = v.clone();
                let mut k = 0;
                for iv in v.iter_mut() {
                    for b in iv.lo.iter_mut().chain(iv.hi.iter_mut()) {
                        if k == i {
                            b.at = e.clone();
                        }
                        k += 1;
                    }
                }
                Math::Intervals(x.clone(), v)
            }
            Math::System(v) => {
                let mut v = v.clone();
                if i % 2 == 0 {
                    v[i / 2].0 = e;
                } else {
                    v[i / 2].1 = e;
                }
                Math::System(v)
            }
            // logic and sets (agent L)
            Math::Taut(_) => Math::Taut(e),
            Math::Equiv(l, r) => {
                if i == 0 {
                    Math::Equiv(e, r.clone())
                } else {
                    Math::Equiv(l.clone(), e)
                }
            }
            Math::Entails(l, r) => {
                if i == 0 {
                    Math::Entails(e, r.clone())
                } else {
                    Math::Entails(l.clone(), e)
                }
            }
            Math::Subset(l, r) => {
                if i == 0 {
                    Math::Subset(e, r.clone())
                } else {
                    Math::Subset(l.clone(), e)
                }
            }
            m => m.clone(),
        }
    }
    pub fn size(&self) -> usize {
        self.slots().iter().map(|e| e.size()).sum()
    }
}

// ---- algebra (agent A): inequalities, intervals, exact logarithms ----

/// An inequality sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rel {
    Lt,
    Le,
    Gt,
    Ge,
}

impl Rel {
    /// The sign after multiplying both sides by a negative number: < becomes >.
    pub fn flip(self) -> Rel {
        match self {
            Rel::Lt => Rel::Gt,
            Rel::Le => Rel::Ge,
            Rel::Gt => Rel::Lt,
            Rel::Ge => Rel::Le,
        }
    }
    pub fn strict(self) -> bool {
        matches!(self, Rel::Lt | Rel::Gt)
    }
    /// Does `a rel b` hold? Values within `tol` of each other count as equal.
    pub fn holds(self, a: f64, b: f64, tol: f64) -> bool {
        let eq = (a - b).abs() <= tol * 1f64.max(a.abs()).max(b.abs());
        match self {
            Rel::Lt => a < b && !eq,
            Rel::Le => a < b || eq,
            Rel::Gt => a > b && !eq,
            Rel::Ge => a > b || eq,
        }
    }
    pub fn holds_q(self, a: &Q, b: &Q) -> bool {
        match self {
            Rel::Lt => a < b,
            Rel::Le => a <= b,
            Rel::Gt => a > b,
            Rel::Ge => a >= b,
        }
    }
    pub fn words(self) -> &'static str {
        match self {
            Rel::Lt => "less than",
            Rel::Le => "at most",
            Rel::Gt => "greater than",
            Rel::Ge => "at least",
        }
    }
}

/// One end of an interval: the value, and whether it belongs to the interval.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Bound {
    pub at: Expr,
    pub closed: bool,
}

/// An interval of the real line; a missing end is infinite.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Interval {
    pub lo: Option<Bound>,
    pub hi: Option<Bound>,
}

impl Interval {
    /// Where "x rel a" holds: x < 2 is (-inf, 2), x >= 1 is [1, inf).
    pub fn of(rel: Rel, a: &Expr) -> Interval {
        let b = Bound { at: a.clone(), closed: !rel.strict() };
        match rel {
            Rel::Lt | Rel::Le => Interval { lo: None, hi: Some(b) },
            Rel::Gt | Rel::Ge => Interval { lo: Some(b), hi: None },
        }
    }
    pub fn contains(&self, x: f64, tol: f64) -> bool {
        let near = |a: f64| (x - a).abs() <= tol * 1f64.max(a.abs());
        let above = self.lo.as_ref().is_none_or(|b| {
            let a = b.at.eval_f(&|_| f64::NAN);
            if near(a) {
                b.closed
            } else {
                x > a
            }
        });
        let below = self.hi.as_ref().is_none_or(|b| {
            let a = b.at.eval_f(&|_| f64::NAN);
            if near(a) {
                b.closed
            } else {
                x < a
            }
        });
        above && below
    }
}

/// log_b(a) when it is rational: log_2(8) = 3, log_4(2) = 1/2, log_10(1/100) = -2.
pub fn exact_log(b: &Q, a: &Q) -> Option<Q> {
    if b.is_neg() || b.is_zero() || b.is_one() || a.is_neg() || a.is_zero() {
        return None;
    }
    if a.is_one() {
        return Some(Q::ZERO);
    }
    // a^q = b^p for small q
    for q in 1..=6i64 {
        let Some(t) = a.pow(q) else { continue };
        let (mut up, mut down) = (Q::ONE, Q::ONE);
        for p in 1..=64i128 {
            let (Some(u), Some(d)) = (up.mul(b), down.div(b)) else { break };
            (up, down) = (u, d);
            if up == t {
                return Q::new(p, q as i128);
            }
            if down == t {
                return Q::new(-p, q as i128);
            }
        }
    }
    None
}

// ---- logic and sets (agent L) ----

/// A connective of propositional logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Conn {
    Not,
    And,
    Or,
    Implies,
    Iff,
}

/// An operation on sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SetOp {
    Complement,
    Union,
    Inter,
    Diff,
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
