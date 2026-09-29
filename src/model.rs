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
}

impl Task {
    pub const ALL: [Task; 7] = [Task::Evaluate, Task::Simplify, Task::Expand, Task::Factor, Task::Solve, Task::Differentiate, Task::Divide];
    pub fn key(self) -> &'static str {
        match self {
            Task::Evaluate => "evaluate",
            Task::Simplify => "simplify",
            Task::Expand => "expand",
            Task::Factor => "factor",
            Task::Solve => "solve",
            Task::Differentiate => "differentiate",
            Task::Divide => "divide",
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
}

impl Request {
    /// The state the search starts from.
    pub fn start(&self) -> Math {
        match (&self.task.value, &self.problem.value) {
            (Task::Differentiate, Math::Expr(e)) => Math::Expr(Expr::Deriv(Box::new(e.clone()), self.var.value.clone())),
            (_, m) => m.clone(),
        }
    }
}
