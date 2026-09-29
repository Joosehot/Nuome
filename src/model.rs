//! The structured request a sentence becomes. Every value keeps the words
//! that produced it, so every step can say who asked for it.

use crate::expr::{Expr, Math};

#[derive(Clone, Debug, PartialEq)]
pub struct Said<T> {
    pub value: T,
    pub words: String,
}

impl<T> Said<T> {
    pub fn new(value: T, words: impl Into<String>) -> Said<T> {
        Said { value, words: words.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Task {
    Evaluate,
    Simplify,
    Expand,
    Factor,
    Solve,
    Differentiate,
    /// Polynomial division: quotient and remainder.
    Divide,
    Integrate,
    Limit,
    Tangent,
    /// Prove a statement: an identity, an inequality, a law of logic...
    Prove,
}

impl Task {
    pub const ALL: [Task; 11] = [Task::Evaluate, Task::Simplify, Task::Expand, Task::Factor, Task::Solve, Task::Differentiate, Task::Divide, Task::Integrate, Task::Limit, Task::Tangent, Task::Prove];
    pub fn key(self) -> &'static str {
        match self {
            Task::Evaluate => "evaluate",
            Task::Simplify => "simplify",
            Task::Expand => "expand",
            Task::Factor => "factor",
            Task::Solve => "solve",
            Task::Differentiate => "differentiate",
            Task::Divide => "divide",
            Task::Integrate => "integrate",
            Task::Limit => "limit",
            Task::Tangent => "tangent",
            Task::Prove => "prove",
        }
    }
    /// The task as a verb in a sentence: "none of my rules can find the limit of ...".
    pub fn verb(self) -> &'static str {
        match self {
            Task::Limit => "find the limit of",
            Task::Tangent => "find the tangent to",
            t => t.key(),
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Task::Evaluate => "Evaluate",
            Task::Simplify => "Simplify",
            Task::Expand => "Expand",
            Task::Factor => "Factor",
            Task::Solve => "Solve",
            Task::Differentiate => "Differentiate",
            Task::Divide => "Divide",
            Task::Integrate => "Integrate",
            Task::Limit => "Find the limit of",
            Task::Tangent => "Find the tangent to",
            Task::Prove => "Prove",
        }
    }
}

/// Words that shift the profile: which tradeoff the explanation spends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Modifier {
    Brief,
    Detailed,
    Beginner,
    Elegant,
}

impl Modifier {
    pub const ALL: [Modifier; 4] = [Modifier::Brief, Modifier::Detailed, Modifier::Beginner, Modifier::Elegant];
    pub fn key(self) -> &'static str {
        match self {
            Modifier::Brief => "brief",
            Modifier::Detailed => "detailed",
            Modifier::Beginner => "beginner",
            Modifier::Elegant => "elegant",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub sentence: String,
    pub task: Said<Task>,
    /// The problem as written: an expression or an equation.
    pub problem: Said<Math>,
    /// The letter to solve for or differentiate by.
    pub var: Said<String>,
    /// "when x = 3": values to substitute.
    pub given: Vec<Said<(String, Expr)>>,
    /// A method the sentence insists on ("using the quadratic formula"): a rule name.
    pub method: Option<Said<String>>,
    pub modifiers: Vec<Said<Modifier>>,
    /// "to 3 decimal places", "as a decimal": also give a decimal answer.
    pub decimals: Option<Said<u32>>,
    /// Conventions the answer relies on ("a month is taken as 30 days").
    pub notes: Vec<Said<String>>,
    /// Calculus and trig details: derivative order, bounds, the point.
    pub calc: Calc,
}

/// What a calculus or trig question adds to a request (agent B).
#[derive(Clone, Debug, Default)]
pub struct Calc {
    /// "second derivative": how many times to differentiate (none = once).
    pub order: Option<Said<u32>>,
    /// "from 0 to 3", "between 0 and 2pi": a definite integral's bounds, or
    /// the interval a trig equation is solved in.
    pub bounds: Option<Said<(Expr, Expr)>>,
    /// "as x approaches 2", "at x = 1": where a limit is taken or a tangent touches.
    pub point: Option<Said<Expr>>,
    /// "degrees": angles are in degrees (the words that said so).
    pub degrees: Option<String>,
}

impl Calc {
    pub fn order(&self) -> u32 {
        self.order.as_ref().map_or(1, |o| o.value)
    }
}

impl Request {
    /// A tangent's curve: "y = x^2" or just "x^2" -> ("y", x^2).
    pub fn curve(&self) -> (String, Expr) {
        match &self.problem.value {
            Math::Eq(Expr::Var(y), f) => (y.clone(), f.clone()),
            Math::Expr(f) => ("y".to_string(), f.clone()),
            Math::Eq(_, f) => ("y".to_string(), f.clone()),
            _ => ("y".to_string(), Expr::Num(crate::q::Q::ZERO)),
        }
    }
    /// The state the search starts from.
    pub fn start(&self) -> Math {
        match (&self.task.value, &self.problem.value) {
            (Task::Differentiate, Math::Expr(e)) => Math::Expr((0..self.calc.order()).fold(e.clone(), |d, _| Expr::Deriv(Box::new(d), self.var.value.clone()))),
            (Task::Integrate, Math::Expr(e)) => {
                let v = &self.var.value;
                let int = Expr::Integral(Box::new(e.clone()), v.clone());
                Math::Expr(match &self.calc.bounds {
                    Some(b) => Expr::Bounds(Box::new(int), v.clone(), Box::new(b.value.0.clone()), Box::new(b.value.1.clone())),
                    None => int,
                })
            }
            (Task::Limit, Math::Expr(e)) => {
                let p = self.calc.point.as_ref().map_or(Expr::Const(crate::expr::Konst::Inf), |p| p.value.clone());
                Math::Expr(Expr::Limit(Box::new(e.clone()), self.var.value.clone(), Box::new(p)))
            }
            (Task::Tangent, _) => {
                // y = f(a) + f'(a)(x - a)
                let (y, f) = self.curve();
                let v = &self.var.value;
                let a = self.calc.point.as_ref().map_or(Expr::Num(crate::q::Q::ZERO), |p| p.value.clone());
                let at = |e: Expr| Expr::At(Box::new(e), v.clone(), Box::new(a.clone()));
                let slope = at(Expr::Deriv(Box::new(f.clone()), v.clone()));
                let run = if a.is_num(0) { Expr::Var(v.clone()) } else { crate::expr::add(vec![Expr::Var(v.clone()), crate::expr::neg(a.clone())]) };
                Math::Eq(Expr::Var(y), Expr::Add(vec![at(f), Expr::Mul(vec![slope, run])]))
            }
            (_, m) => m.clone(),
        }
    }
}
