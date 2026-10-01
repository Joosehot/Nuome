//! The closed vocabulary: words, phrases and symbols -> tokens. A word that
//! isn't listed is an error with the nearest known word, never a guess.

use crate::expr::{Func, Konst, Rel};
use crate::abstract_algebra::Kind;
use crate::model::{Modifier, Task};
use crate::calls::Named;
use crate::q::Q;

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(Q),
    Var(String),
    Const(Konst),
    Func(Func),
    /// + - * / ^ ( ) =
    Op(char),
    Squared,
    Cubed,
    Percent,
    /// "of": multiplication after a percentage, otherwise filler.
    Of,
    Task(Task),
    /// "for", "with respect to": a letter follows.
    For,
    /// "when", "at", "if": a condition follows ("when x = 3").
    When,
    /// A method the sentence insists on: the rule's name.
    Method(&'static str),
    Mod(Modifier),
    /// "as a decimal", "approximately".
    Decimal,
    /// "decimal places", "dp": after a number.
    Places,
    To,
    Exact,
    /// "is": an equals sign between a letter and a value, otherwise filler.
    Is,
    /// Ends a math span: "and", "then", commas.
    Sep,
    Filler,
    /// Something recognisably mathematical that v0 doesn't do.
    Unsupported(&'static str),
    Unknown,
    // numbers, sequences, statistics, finance (main)
    /// "gcd of", "mean of": a list of numbers follows.
    List(Named),
    /// "mod", "choose": between two numbers.
    Infix(Named),
    /// "!": factorial.
    Bang,
    /// "20th", "3rd".
    Ordinal(i128),
    /// "sum of".
    SumOf,
    /// "the first" (terms).
    First,
    /// "terms of".
    TermsOf,
    /// "term of".
    TermOf,
    /// "sum to infinity of".
    InfSum,
    From,
    /// "combinations of", "permutations of": k from n.
    Pick(Named),
    /// "remainder when".
    Remainder,
    /// "compound", "invest", "grows": a growth problem.
    Grow,
    Per,
    /// A length of time: days, weeks, months, years.
    Period(Period),
    /// "daily", "monthly": compounding frequency.
    Every(Period),
    /// "increase" (+1), "decrease" (-1).
    Change(i8),
    By,
    /// "what percent of".
    WhatPct,
    /// "is what percent of".
    IsWhatPct,
    /// "percentage change from".
    PctChange,
    /// Built by the word grammar (words.rs): a finished expression.
    Built(crate::expr::Expr),
    /// A famous problem Nuome can name but not solve (details in rules.toml [open]).
    Open(&'static str),
    /// A word from an area of mathematics Nuome has no rules for.
    Topic(&'static str),
    // proofs (main)
    /// "divides": d divides e.
    DividesW,
    /// "is divisible by", "is a multiple of": e ... d.
    DivisibleBy,
    /// "is even": 2 divides it.
    Even,
    /// < <= > >=, "less than", "at least".
    Rel(Rel),
    /// "log": base 10 unless "_b" follows.
    Log,
    /// "log base": the base comes next.
    LogBase,
    /// "approaches", "tends to", "->": the point a limit is taken at follows.
    Approaches,
    /// "second derivative", "twice": differentiate this many times.
    Order(u32),
    /// "degrees", "°": the number before it is an angle in degrees.
    Degrees,
    // logic and sets (agent L)
    /// A word of logic or of sets ("implies", "union", "is a tautology"): it
    /// only means something inside a statement to prove (logic.rs).
    Logic(&'static str),
    // abstract algebra (agent G)
    /// "in a group", "any ring": what the letters are elements of.
    Structure(crate::abstract_algebra::Kind),
    /// A statement said in words ("the group is abelian"): its math.
    Statement(crate::abstract_algebra::Kind, &'static str),
    /// "the inverse of": the group inverse of what follows.
    InverseOf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    Day,
    Week,
    Month,
    Year,
}

impl Period {
    pub fn key(self) -> &'static str {
        match self {
            Period::Day => "day",
            Period::Week => "week",
            Period::Month => "month",
            Period::Year => "year",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub words: String,
}

use Tok::*;

const PHRASES: &[(&str, Tok)] = &[
    // tasks
    ("solve", Task(Task::Solve)),
    ("evaluate", Task(Task::Evaluate)),
    ("calculate", Task(Task::Evaluate)),
    ("compute", Task(Task::Evaluate)),
    ("work out", Task(Task::Evaluate)),
    ("what is", Task(Task::Evaluate)),
    ("what's", Task(Task::Evaluate)),
    ("whats", Task(Task::Evaluate)),
    ("how much is", Task(Task::Evaluate)),
    ("simplify", Task(Task::Simplify)),
    ("expand", Task(Task::Expand)),
    ("multiply out", Task(Task::Expand)),
    ("expand and simplify", Task(Task::Expand)),
    ("factor", Task(Task::Factor)),
    ("factorise", Task(Task::Factor)),
    ("factorize", Task(Task::Factor)),
    ("factor completely", Task(Task::Factor)),
    ("factorise fully", Task(Task::Factor)),
    ("factorize fully", Task(Task::Factor)),
    ("fully factorise", Task(Task::Factor)),
    ("fully factorize", Task(Task::Factor)),
    ("differentiate", Task(Task::Differentiate)),
    ("derivative of", Task(Task::Differentiate)),
    ("the derivative of", Task(Task::Differentiate)),
    ("derivative", Task(Task::Differentiate)),
    ("for", For),
    ("with respect to", For),
    ("wrt", For),
    ("when", When),
    ("at", When),
    ("if", When),
    ("where", When),
    ("given", When),
    ("given that", When),
    ("after", For),
    // methods
    ("by factoring", Method("factor_solve")),
    ("by factorising", Method("factor_solve")),
    ("by factorizing", Method("factor_solve")),
    ("by factorisation", Method("factor_solve")),
    ("by factorization", Method("factor_solve")),
    ("using factoring", Method("factor_solve")),
    ("using the quadratic formula", Method("quadratic_formula")),
    ("with the quadratic formula", Method("quadratic_formula")),
    ("by the quadratic formula", Method("quadratic_formula")),
    ("using the formula", Method("quadratic_formula")),
    ("with the formula", Method("quadratic_formula")),
    ("by formula", Method("quadratic_formula")),
    ("by square roots", Method("square_root")),
    ("by taking square roots", Method("square_root")),
    ("by taking the square root", Method("square_root")),
    ("using square roots", Method("square_root")),
    ("by completing the square", Method("complete_square")),
    ("completing the square", Method("complete_square")),
    // profile modifiers
    ("quickly", Mod(Modifier::Brief)),
    ("briefly", Mod(Modifier::Brief)),
    ("brief", Mod(Modifier::Brief)),
    ("fast", Mod(Modifier::Brief)),
    ("short", Mod(Modifier::Brief)),
    ("in short", Mod(Modifier::Brief)),
    ("concisely", Mod(Modifier::Brief)),
    ("just the answer", Mod(Modifier::Brief)),
    ("only the answer", Mod(Modifier::Brief)),
    ("step by step", Mod(Modifier::Detailed)),
    ("step-by-step", Mod(Modifier::Detailed)),
    ("in detail", Mod(Modifier::Detailed)),
    ("detailed", Mod(Modifier::Detailed)),
    ("carefully", Mod(Modifier::Detailed)),
    ("slowly", Mod(Modifier::Detailed)),
    ("show your work", Mod(Modifier::Detailed)),
    ("show your working", Mod(Modifier::Detailed)),
    ("show working", Mod(Modifier::Detailed)),
    ("show all steps", Mod(Modifier::Detailed)),
    ("show the steps", Mod(Modifier::Detailed)),
    ("with steps", Mod(Modifier::Detailed)),
    ("with working", Mod(Modifier::Detailed)),
    ("explain", Mod(Modifier::Detailed)),
    ("for a beginner", Mod(Modifier::Beginner)),
    ("for beginners", Mod(Modifier::Beginner)),
    ("for a kid", Mod(Modifier::Beginner)),
    ("for a child", Mod(Modifier::Beginner)),
    ("like i'm five", Mod(Modifier::Beginner)),
    ("like im five", Mod(Modifier::Beginner)),
    ("eli5", Mod(Modifier::Beginner)),
    ("simply", Mod(Modifier::Beginner)),
    ("gently", Mod(Modifier::Beginner)),
    ("elegantly", Mod(Modifier::Elegant)),
    ("elegant", Mod(Modifier::Elegant)),
    ("cleverly", Mod(Modifier::Elegant)),
    ("neatly", Mod(Modifier::Elegant)),
    ("nicely", Mod(Modifier::Elegant)),
    // output form
    ("as a decimal", Decimal),
    ("in decimals", Decimal),
    ("as decimals", Decimal),
    ("approximately", Decimal),
    ("approx", Decimal),
    ("roughly", Decimal),
    ("numerically", Decimal),
    ("decimal places", Places),
    ("decimal place", Places),
    ("dp", Places),
    ("places", Places),
    ("decimals", Places),
    ("to", To),
    ("exactly", Exact),
    ("exact", Exact),
    ("as a fraction", Exact),
    ("in exact form", Exact),
    ("exact form", Exact),
    // spoken math
    ("plus", Op('+')),
    ("minus", Op('-')),
    ("take away", Op('-')),
    ("times", Op('*')),
    ("multiplied by", Op('*')),
    ("divided by", Op('/')),
    ("over", Op('/')),
    ("to the power of", Op('^')),
    ("to the power", Op('^')),
    ("raised to", Op('^')),
    ("raised to the power of", Op('^')),
    ("squared", Squared),
    ("cubed", Cubed),
    ("square root of", Func(Func::Sqrt)),
    ("the square root of", Func(Func::Sqrt)),
    ("root of", Func(Func::Sqrt)),
    ("sqrt", Func(Func::Sqrt)),
    ("sin", Func(Func::Sin)),
    ("sine", Func(Func::Sin)),
    ("sine of", Func(Func::Sin)),
    ("cos", Func(Func::Cos)),
    ("cosine", Func(Func::Cos)),
    ("cosine of", Func(Func::Cos)),
    ("tan", Func(Func::Tan)),
    ("tangent", Func(Func::Tan)),
    ("tangent of", Func(Func::Tan)),
    ("ln", Func(Func::Ln)),
    ("natural log of", Func(Func::Ln)),
    ("natural log", Func(Func::Ln)),
    ("exp", Func(Func::Exp)),
    ("pi", Const(Konst::Pi)),
    ("equals", Op('=')),
    ("is equal to", Op('=')),
    ("equal to", Op('=')),
    ("is", Is),
    ("percent", Percent),
    ("per cent", Percent),
    ("of", Of),
    ("zero", Num(Q::ZERO)),
    ("one", Num(Q::ONE)),
    ("two", Num(Q::int(2))),
    ("three", Num(Q::int(3))),
    ("four", Num(Q::int(4))),
    ("five", Num(Q::int(5))),
    ("six", Num(Q::int(6))),
    ("seven", Num(Q::int(7))),
    ("eight", Num(Q::int(8))),
    ("nine", Num(Q::int(9))),
    ("ten", Num(Q::int(10))),
    // joints
    ("and", Sep),
    ("then", Sep),
    ("also", Sep),
    // filler
    ("the", Filler),
    ("an", Filler),
    ("please", Filler),
    ("can", Filler),
    ("could", Filler),
    ("would", Filler),
    ("you", Filler),
    ("me", Filler),
    ("for me", Filler),
    ("find", Filler),
    ("value", Filler),
    ("what", Filler),
    ("how", Filler),
    ("do", Filler),
    ("i", Filler),
    ("want", Filler),
    ("need", Filler),
    ("help", Filler),
    ("this", Filler),
    ("that", Filler),
    ("it", Filler),
    ("equation", Filler),
    ("expression", Filler),
    ("answer", Filler),
    ("give", Filler),
    ("show", Filler),
    ("tell", Filler),
    ("hey", Filler),
    ("thanks", Filler),
    ("thank you", Filler),
    ("in", Filler),
    ("using", Filler),
    ("result", Filler),
    ("get", Filler),
    // numbers, sequences, statistics, finance (main)
    ("gcd of", List(Named::Gcd)),
    ("gcd", List(Named::Gcd)),
    ("greatest common divisor of", List(Named::Gcd)),
    ("greatest common factor of", List(Named::Gcd)),
    ("highest common factor of", List(Named::Gcd)),
    ("hcf of", List(Named::Gcd)),
    ("lcm of", List(Named::Lcm)),
    ("lcm", List(Named::Lcm)),
    ("least common multiple of", List(Named::Lcm)),
    ("lowest common multiple of", List(Named::Lcm)),
    ("mean of", List(Named::Mean)),
    ("average of", List(Named::Mean)),
    ("median of", List(Named::Median)),
    ("mode of", List(Named::Mode)),
    ("range of", List(Named::Range)),
    ("variance of", List(Named::Variance)),
    ("population variance of", List(Named::Variance)),
    ("sample variance of", List(Named::SampleVariance)),
    ("standard deviation of", List(Named::StdDev)),
    ("population standard deviation of", List(Named::StdDev)),
    ("sample standard deviation of", List(Named::SampleStdDev)),
    ("factorial of", List(Named::Factorial)),
    ("prime factors of", Task(Task::Factor)),
    ("prime factorization of", Task(Task::Factor)),
    ("prime factorisation of", Task(Task::Factor)),
    ("mod", Infix(Named::Mod)),
    ("modulo", Infix(Named::Mod)),
    ("choose", Infix(Named::Choose)),
    ("remainder when", Remainder),
    ("remainder of", Remainder),
    ("combinations of", Pick(Named::Choose)),
    ("permutations of", Pick(Named::Perm)),
    ("arrangements of", Pick(Named::Perm)),
    ("sum of", SumOf),
    ("add up", SumOf),
    ("first", First),
    ("terms of", TermsOf),
    ("terms in", TermsOf),
    ("term of", TermOf),
    ("term in", TermOf),
    ("sum to infinity of", InfSum),
    ("infinite sum of", InfSum),
    ("from", From),
    ("out of", From),
    ("numbers", Filler),
    ("integers", Filler),
    ("all", Filler),
    ("whole", Filler),
    ("full", Filler),
    ("compound", Grow),
    ("compounding", Grow),
    ("compound interest", Grow),
    ("compound growth", Grow),
    ("interest", Grow),
    ("invest", Grow),
    ("invested", Grow),
    ("grows", Grow),
    ("growth", Grow),
    ("compounded", Grow),
    ("per", Per),
    ("each", Per),
    ("every", Per),
    ("day", Period(Period::Day)),
    ("days", Period(Period::Day)),
    ("week", Period(Period::Week)),
    ("weeks", Period(Period::Week)),
    ("month", Period(Period::Month)),
    ("months", Period(Period::Month)),
    ("year", Period(Period::Year)),
    ("years", Period(Period::Year)),
    ("daily", Every(Period::Day)),
    ("weekly", Every(Period::Week)),
    ("monthly", Every(Period::Month)),
    ("yearly", Every(Period::Year)),
    ("annually", Every(Period::Year)),
    ("increase", Change(1)),
    ("raise", Change(1)),
    ("decrease", Change(-1)),
    ("reduce", Change(-1)),
    ("by", By),
    ("what percent of", WhatPct),
    ("what percentage of", WhatPct),
    ("is what percent of", IsWhatPct),
    ("is what percentage of", IsWhatPct),
    ("as a percentage of", IsWhatPct),
    ("as a percent of", IsWhatPct),
    ("percent change from", PctChange),
    ("percentage change from", PctChange),
    ("percent increase from", PctChange),
    ("percentage increase from", PctChange),
    ("change from", PctChange),
    // calculus and trig (agent B)
    ("integrate", Task(Task::Integrate)),
    ("integral", Task(Task::Integrate)),
    ("integral of", Task(Task::Integrate)),
    ("the integral of", Task(Task::Integrate)),
    ("definite integral of", Task(Task::Integrate)),
    ("the definite integral of", Task(Task::Integrate)),
    ("indefinite integral of", Task(Task::Integrate)),
    ("the indefinite integral of", Task(Task::Integrate)),
    ("antiderivative", Task(Task::Integrate)),
    ("antiderivative of", Task(Task::Integrate)),
    ("an antiderivative of", Task(Task::Integrate)),
    ("the antiderivative of", Task(Task::Integrate)),
    ("limit", Task(Task::Limit)),
    ("limit of", Task(Task::Limit)),
    ("the limit of", Task(Task::Limit)),
    ("lim", Task(Task::Limit)),
    ("tangent to", Task(Task::Tangent)),
    ("tangent line to", Task(Task::Tangent)),
    ("the tangent to", Task(Task::Tangent)),
    ("the tangent line to", Task(Task::Tangent)),
    ("equation of the tangent to", Task(Task::Tangent)),
    ("equation of the tangent line to", Task(Task::Tangent)),
    ("second derivative", Order(2)),
    ("second derivative of", Order(2)),
    ("the second derivative of", Order(2)),
    ("third derivative", Order(3)),
    ("third derivative of", Order(3)),
    ("the third derivative of", Order(3)),
    ("twice", Order(2)),
    ("three times", Order(3)),
    ("approaches", Approaches),
    ("approach", Approaches),
    ("tends to", Approaches),
    ("goes to", Approaches),
    ("between", From),
    ("from the left", Unsupported("a one-sided limit")),
    ("from the right", Unsupported("a one-sided limit")),
    ("from below", Unsupported("a one-sided limit")),
    ("from above", Unsupported("a one-sided limit")),
    ("infinity", Const(Konst::Inf)),
    ("infty", Const(Konst::Inf)),
    ("degrees", Degrees),
    ("degree", Degrees),
    ("deg", Degrees),
    ("radians", Filler),
    ("in radians", Filler),
    ("as", Filler),
    ("general solution", Filler),
    ("the general solution of", Filler),
    ("all solutions of", Filler),
    ("sec", Func(Func::Sec)),
    ("secant", Func(Func::Sec)),
    ("csc", Func(Func::Csc)),
    ("cosec", Func(Func::Csc)),
    ("cosecant", Func(Func::Csc)),
    ("cot", Func(Func::Cot)),
    ("cotangent", Func(Func::Cot)),
    ("arcsin", Func(Func::Asin)),
    ("asin", Func(Func::Asin)),
    ("arccos", Func(Func::Acos)),
    ("acos", Func(Func::Acos)),
    ("arctan", Func(Func::Atan)),
    ("atan", Func(Func::Atan)),
    // proofs (main)
    ("divides", DividesW),
    ("is divisible by", DivisibleBy),
    ("divisible by", DivisibleBy),
    ("is a multiple of", DivisibleBy),
    ("is always divisible by", DivisibleBy),
    ("is even", Even),
    ("is always even", Even),
    ("for every integer", Filler),
    ("for all integers", Filler),
    ("for every positive integer", Filler),
    ("for all positive integers", Filler),
    ("for every natural number", Filler),
    ("for all natural numbers", Filler),
    ("by induction", Filler),
    ("using induction", Filler),
    // open problems and areas without rules: known words, so the refusal can say why
    ("hodge conjecture", Open("hodge")),
    ("the hodge conjecture", Open("hodge")),
    ("riemann hypothesis", Open("riemann")),
    ("the riemann hypothesis", Open("riemann")),
    ("p vs np", Open("p_vs_np")),
    ("p versus np", Open("p_vs_np")),
    ("the p vs np problem", Open("p_vs_np")),
    ("p vs np problem", Open("p_vs_np")),
    ("navier-stokes", Open("navier_stokes")),
    ("navier-stokes problem", Open("navier_stokes")),
    ("navier-stokes equations", Open("navier_stokes")),
    ("the navier-stokes problem", Open("navier_stokes")),
    ("yang-mills", Open("yang_mills")),
    ("yang-mills mass gap", Open("yang_mills")),
    ("the yang-mills mass gap", Open("yang_mills")),
    ("mass gap", Open("yang_mills")),
    ("birch and swinnerton-dyer conjecture", Open("bsd")),
    ("the birch and swinnerton-dyer conjecture", Open("bsd")),
    ("bsd conjecture", Open("bsd")),
    ("poincare conjecture", Open("poincare")),
    ("the poincare conjecture", Open("poincare")),
    ("goldbach conjecture", Open("goldbach")),
    ("goldbach's conjecture", Open("goldbach")),
    ("the goldbach conjecture", Open("goldbach")),
    ("twin prime conjecture", Open("twin_primes")),
    ("the twin prime conjecture", Open("twin_primes")),
    ("collatz conjecture", Open("collatz")),
    ("the collatz conjecture", Open("collatz")),
    // "99-graph" is spelled out before scanning (see `lex`)
    ("conway's ninety-nine-graph problem", Open("conway99")),
    ("the ninety-nine-graph problem", Open("conway99")),
    ("ninety-nine-graph problem", Open("conway99")),
    ("the ninety-nine-graph", Open("conway99")),
    ("ninety-nine-graph", Open("conway99")),
    ("the beal conjecture", Open("beal")),
    ("beal conjecture", Open("beal")),
    ("beal's conjecture", Open("beal")),
    ("the beal prize", Open("beal")),
    ("beal prize", Open("beal")),
    ("the eff prize", Open("eff_prime")),
    ("eff prize", Open("eff_prime")),
    ("the hundred-million-digit prime", Open("eff_prime")),
    ("hundred-million-digit prime", Open("eff_prime")),
    ("a hundred-million-digit prime", Open("eff_prime")),
    ("3n + 1 problem", Open("collatz")),
    ("variety", Topic("algebraic geometry")),
    ("varieties", Topic("algebraic geometry")),
    ("projective variety", Topic("algebraic geometry")),
    ("projective", Topic("algebraic geometry")),
    ("non-singular", Topic("algebraic geometry")),
    ("nonsingular", Topic("algebraic geometry")),
    ("cohomology", Topic("algebraic geometry")),
    ("cohomology class", Topic("algebraic geometry")),
    ("hodge", Topic("algebraic geometry")),
    ("hodge class", Topic("algebraic geometry")),
    ("hodge classes", Topic("algebraic geometry")),
    ("algebraic cycle", Topic("algebraic geometry")),
    ("algebraic cycles", Topic("algebraic geometry")),
    ("cycle", Topic("algebraic geometry")),
    ("cycles", Topic("algebraic geometry")),
    ("class", Topic("algebraic geometry")),
    ("classes", Topic("algebraic geometry")),
    ("rational linear combination", Topic("algebraic geometry")),
    ("linear combination", Topic("algebraic geometry")),
    ("combination", Topic("algebraic geometry")),
    ("intersection", Topic("algebraic geometry")),
    ("intersected with", Topic("algebraic geometry")),
    ("subset", Topic("algebraic geometry")),
    ("rationals", Topic("algebraic geometry")),
    ("complex numbers", Topic("algebraic geometry")),
    ("manifold", Topic("topology")),
    ("manifolds", Topic("topology")),
    ("3-manifold", Topic("topology")),
    ("homeomorphic", Topic("topology")),
    ("simply connected", Topic("topology")),
    ("3-sphere", Topic("topology")),
    ("kahler manifold", Topic("algebraic geometry")),
    ("span", Topic("algebraic geometry")),
    ("codimension", Topic("algebraic geometry")),
    ("subvariety", Topic("algebraic geometry")),
    ("subvarieties", Topic("algebraic geometry")),
    ("zeta function", Topic("number theory beyond arithmetic")),
    ("zeros", Topic("number theory beyond arithmetic")),
    ("nontrivial zeros", Topic("number theory beyond arithmetic")),
    ("critical line", Topic("number theory beyond arithmetic")),
    ("elliptic curve", Topic("number theory beyond arithmetic")),
    ("elliptic curves", Topic("number theory beyond arithmetic")),
    ("primes", Topic("number theory beyond arithmetic")),
    ("twin primes", Topic("number theory beyond arithmetic")),
    ("polynomial time", Topic("complexity theory")),
    ("np-complete", Topic("complexity theory")),
    ("let", Filler),
    ("be", Filler),
    ("prove that", Task(Task::Prove)),
    ("show that", Task(Task::Prove)),
    ("verify that", Task(Task::Prove)),
    ("that", Filler),
    ("is a", Filler),
    ("type", Filler),
    // recognisably math, not in v0
    ("matrix", Unsupported("matrix algebra")),
    ("matrices", Unsupported("matrix algebra")),
    ("determinant", Unsupported("matrix algebra")),

    ("graph", Unsupported("plotting")),
    ("plot", Unsupported("plotting")),
    ("prove", Task(Task::Prove)),
    ("probability", Unsupported("probability")),
    // algebra (agent A)
    ("divide", Task(Task::Divide)),
    ("by long division", Filler),
    ("using long division", Filler),
    ("long division", Filler),
    ("rationalise", Task(Task::Simplify)),
    ("rationalize", Task(Task::Simplify)),
    ("rationalise the denominator of", Task(Task::Simplify)),
    ("rationalize the denominator of", Task(Task::Simplify)),
    ("combine", Task(Task::Simplify)),
    ("as a single fraction", Filler),
    ("as one fraction", Filler),
    ("log", Log),
    ("log of", Log),
    ("logarithm", Log),
    ("logarithm of", Log),
    ("log base", LogBase),
    ("log to base", LogBase),
    ("log to the base", LogBase),
    ("logarithm base", LogBase),
    ("logarithm to base", LogBase),
    ("logarithm to the base", LogBase),
    ("abs", Func(Func::Abs)),
    ("absolute value of", Func(Func::Abs)),
    ("the absolute value of", Func(Func::Abs)),
    ("modulus of", Func(Func::Abs)),
    ("less than", Rel(Rel::Lt)),
    ("is less than", Rel(Rel::Lt)),
    ("smaller than", Rel(Rel::Lt)),
    ("is smaller than", Rel(Rel::Lt)),
    ("greater than", Rel(Rel::Gt)),
    ("is greater than", Rel(Rel::Gt)),
    ("more than", Rel(Rel::Gt)),
    ("is more than", Rel(Rel::Gt)),
    ("bigger than", Rel(Rel::Gt)),
    ("less than or equal to", Rel(Rel::Le)),
    ("is less than or equal to", Rel(Rel::Le)),
    ("greater than or equal to", Rel(Rel::Ge)),
    ("is greater than or equal to", Rel(Rel::Ge)),
    ("at most", Rel(Rel::Le)),
    ("is at most", Rel(Rel::Le)),
    ("at least", Rel(Rel::Ge)),
    ("is at least", Rel(Rel::Ge)),
    ("no more than", Rel(Rel::Le)),
    ("no less than", Rel(Rel::Ge)),
    ("inequality", Filler),
    ("interval notation", Filler),
    ("system", Filler),
    ("system of equations", Filler),
    ("simultaneous", Filler),
    ("simultaneous equations", Filler),
    ("simultaneously", Filler),
    ("equations", Filler),
    ("by substitution", Method("substitution")),
    ("using substitution", Method("substitution")),
    ("by the substitution method", Method("substitution")),
    ("by elimination", Method("eliminate")),
    ("using elimination", Method("eliminate")),
    ("by the elimination method", Method("eliminate")),
    // logic and sets (agent L): read by logic.rs inside a statement to prove
    ("not", Logic("not")),
    ("or", Logic("or")),
    ("implies", Logic("implies")),
    ("iff", Logic("iff")),
    ("if and only if", Logic("iff")),
    ("is equivalent to", Logic("equiv")),
    ("equivalent to", Logic("equiv")),
    ("is logically equivalent to", Logic("equiv")),
    ("logically equivalent to", Logic("equiv")),
    ("is a tautology", Logic("tautology")),
    ("a tautology", Logic("tautology")),
    ("tautology", Logic("tautology")),
    ("is always true", Logic("tautology")),
    ("true", Logic("true")),
    ("false", Logic("false")),
    ("union", Logic("union")),
    ("union with", Logic("union")),
    ("intersect", Logic("inter")),
    ("complement of", Logic("complement")),
    ("the complement of", Logic("complement")),
    ("is a subset of", Logic("subset")),
    ("subset of", Logic("subset")),
    ("is contained in", Logic("subset")),
    ("empty set", Logic("empty")),
    ("the empty set", Logic("empty")),
    ("by truth table", Method("truth_table")),
    ("by a truth table", Method("truth_table")),
    ("by truth tables", Method("truth_table")),
    ("using a truth table", Method("truth_table")),
    ("using the truth table", Method("truth_table")),
    ("using truth tables", Method("truth_table")),
    ("with a truth table", Method("truth_table")),
    ("by a membership table", Method("truth_table")),
    ("using a membership table", Method("truth_table")),
    ("with a membership table", Method("truth_table")),
    ("by element chasing", Method("element_chase")),
    ("using element chasing", Method("element_chase")),
    ("by chasing elements", Method("element_chase")),
    // abstract algebra (agent G)
    ("group", Structure(Kind::Group)),
    ("a group", Structure(Kind::Group)),
    ("any group", Structure(Kind::Group)),
    ("every group", Structure(Kind::Group)),
    ("the group", Structure(Kind::Group)),
    ("in a group", Structure(Kind::Group)),
    ("in any group", Structure(Kind::Group)),
    ("in every group", Structure(Kind::Group)),
    ("in the group", Structure(Kind::Group)),
    ("abelian group", Structure(Kind::Abelian)),
    ("an abelian group", Structure(Kind::Abelian)),
    ("any abelian group", Structure(Kind::Abelian)),
    ("every abelian group", Structure(Kind::Abelian)),
    ("in an abelian group", Structure(Kind::Abelian)),
    ("in any abelian group", Structure(Kind::Abelian)),
    ("in every abelian group", Structure(Kind::Abelian)),
    ("commutative group", Structure(Kind::Abelian)),
    ("in a commutative group", Structure(Kind::Abelian)),
    ("ring", Structure(Kind::Ring)),
    ("a ring", Structure(Kind::Ring)),
    ("any ring", Structure(Kind::Ring)),
    ("every ring", Structure(Kind::Ring)),
    ("the ring", Structure(Kind::Ring)),
    ("in a ring", Structure(Kind::Ring)),
    ("in any ring", Structure(Kind::Ring)),
    ("in every ring", Structure(Kind::Ring)),
    ("is abelian", Statement(Kind::Group, "ab = ba")),
    ("group is abelian", Statement(Kind::Group, "ab = ba")),
    ("every group is abelian", Statement(Kind::Group, "ab = ba")),
    ("every element is its own inverse", Statement(Kind::Group, "a^-1 = a for all a")),
    ("each element is its own inverse", Statement(Kind::Group, "a^-1 = a for all a")),
    // at most seven words, and none starting "the": "the" is read on its own first
    ("inverse of the inverse is the element", Statement(Kind::Group, "(a^-1)^-1 = a")),
    ("inverse of an inverse is the element", Statement(Kind::Group, "(a^-1)^-1 = a")),
    ("inverse of the inverse is itself", Statement(Kind::Group, "(a^-1)^-1 = a")),
    ("inverse of the identity is the identity", Statement(Kind::Group, "e^-1 = e")),
    ("the identity is its own inverse", Statement(Kind::Group, "e^-1 = e")),
    ("inverses are unique", Statement(Kind::Group, "if ab = e then b = a^-1")),
    ("inverse is unique", Statement(Kind::Group, "if ab = e then b = a^-1")),
    ("inverses in a group are unique", Statement(Kind::Group, "if ab = e then b = a^-1")),
    ("identity is unique", Statement(Kind::Group, "if fa = a for all a then f = e")),
    ("identity element is unique", Statement(Kind::Group, "if fa = a for all a then f = e")),
    ("cancellation law holds", Statement(Kind::Group, "if ab = ac then b = c")),
    ("cancellation holds", Statement(Kind::Group, "if ab = ac then b = c")),
    ("minus times minus is plus", Statement(Kind::Ring, "(-a)(-b) = ab")),
    ("additive inverse is unique", Statement(Kind::Ring, "if a + b = 0 then b = -a")),
    ("negatives are unique", Statement(Kind::Ring, "if a + b = 0 then b = -a")),
    ("inverse of", InverseOf),
];

enum Raw {
    Word(String),
    Number(String),
    Sym(char),
}

fn scan(s: &str) -> Result<Vec<Raw>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || (chars[i] == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()))) {
                i += 1;
            }
            out.push(Raw::Number(chars[st..i].iter().collect()));
            // "20th", "1st", "2nd", "3rd": the suffix goes with the number
            let rest: String = chars[i..].iter().take(3).collect::<String>().to_lowercase();
            for suf in ["th", "st", "nd", "rd"] {
                if rest.starts_with(suf) && !rest.chars().nth(2).is_some_and(|c| c.is_alphabetic()) {
                    out.push(Raw::Word(format!("#{suf}")));
                    i += 2;
                    break;
                }
            }
        } else if c.is_alphabetic() {
            // "d/dx" is one symbol
            if c == 'd' && chars.get(i + 1) == Some(&'/') && chars.get(i + 2) == Some(&'d') && chars.get(i + 3).is_some_and(|x| x.is_ascii_alphabetic()) && !chars.get(i + 4).is_some_and(|x| x.is_alphabetic()) {
                out.push(Raw::Word("derivative".into()));
                out.push(Raw::Word("wrt".into()));
                out.push(Raw::Word(chars[i + 3].to_string()));
                i += 4;
                continue;
            }
            let st = i;
            while i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '\'' || (chars[i] == '-' && i > st + 1 && chars.get(i + 1).is_some_and(|x| x.is_alphabetic()) && chars.get(i + 2).is_some_and(|x| x.is_alphabetic()))) {
                i += 1;
            }
            let w = chars[st..i].iter().collect::<String>().to_lowercase();
            // calculus and trig (agent B): "dx" after an integrand names the letter
            if w.len() == 2 && w.starts_with('d') && "xyztuvwsr".contains(&w[1..]) {
                out.push(Raw::Word("wrt".into()));
                out.push(Raw::Word(w[1..].to_string()));
                continue;
            }
            out.push(Raw::Word(w));
        } else if (c == '-' && chars.get(i + 1) == Some(&'>')) || c == '→' {
            // calculus and trig (agent B): x -> 2
            out.push(Raw::Word("approaches".into()));
            i += if c == '→' { 1 } else { 2 };
        } else if c == '…' {
            for _ in 0..3 {
                out.push(Raw::Sym('.'));
            }
            i += 1;
        } else if let Some(w) = match c {
            // set and number-system symbols, read as the words they stand for
            '∩' => Some("intersection"),
            '⊂' | '⊆' => Some("subset"),
            'ℚ' => Some("rationals"),
            'ℂ' => Some("complex numbers"),
            _ => None,
        } {
            out.push(Raw::Word(w.into()));
            i += 1;
        } else if c == '∞' {
            out.push(Raw::Word("infinity".into()));
            i += 1;
        } else {
            let sym = match c {
                '+' | '-' | '*' | '/' | '^' | '(' | ')' | '=' | '%' | ',' | ';' | ':' | '?' | '!' | '.' | '$' | '€' | '£' => c,
                '[' | '{' => '(',
                ']' | '}' => ')',
                '×' | '·' | '⋅' => '*',
                '÷' => '/',
                '−' | '–' => '-',
                '²' | '³' | '√' | 'π' | '°' => c,
                '<' | '>' | '≤' | '≥' | '|' | '_' => c,
                _ => return Err(format!("unknown symbol \"{c}\"")),
            };
            out.push(Raw::Sym(sym));
            i += 1;
        }
    }
    Ok(out)
}

/// Turn a sentence into tokens. Unknown words come back as `Unknown` tokens;
/// the parser decides whether that's an error (it is, unless --lenient).
pub fn lex(s: &str) -> Result<Vec<Token>, String> {
    // a name with a number in it: Conway's 99-graph problem
    let s = s.replace("99-graph", "ninety-nine-graph").replace("99-Graph", "ninety-nine-graph").replace("99 graph", "ninety-nine-graph");
    // and the EFF prize's 100-million-digit prime
    let s = s.replace("100-million-digit", "hundred-million-digit").replace("100 million digit", "hundred-million-digit");
    let raw = scan(&s)?;
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        match &raw[i] {
            Raw::Number(n) => {
                out.push(Token { tok: Num(Q::parse(n).ok_or_else(|| format!("number \"{n}\" is too large"))?), words: n.clone() });
                i += 1;
            }
            Raw::Sym(c) => {
                let tok = match c {
                    '%' => Percent,
                    '²' => Squared,
                    '³' => Cubed,
                    '√' => Func(Func::Sqrt),
                    'π' => Const(Konst::Pi),
                    // algebra (agent A): inequality signs; "<=" is two symbols
                    '<' | '>' if matches!(raw.get(i + 1), Some(Raw::Sym('='))) => {
                        let tok = Rel(if *c == '<' { Rel::Le } else { Rel::Ge });
                        out.push(Token { tok, words: format!("{c}=") });
                        i += 2;
                        continue;
                    }
                    '<' => Rel(Rel::Lt),
                    '>' => Rel(Rel::Gt),
                    '≤' => Rel(Rel::Le),
                    '≥' => Rel(Rel::Ge),
                    '!' => Bang,
                    '$' | '€' | '£' => Filler,
                    ',' | ';' | ':' | '?' | '.' => Sep,
                    '°' => Degrees,
                    c => Op(*c),
                };
                out.push(Token { tok, words: c.to_string() });
                i += 1;
            }
            Raw::Word(w) if w.starts_with('#') => {
                // the suffix of "20th": the number before it becomes an ordinal
                if let Some(Token { tok: Num(q), words }) = out.last().cloned() {
                    if q.is_int() {
                        out.pop();
                        out.push(Token { tok: Ordinal(q.num()), words: format!("{words}{}", &w[1..]) });
                    }
                }
                i += 1;
            }
            Raw::Word(_) => {
                // longest phrase over the following words
                let mut best: Option<(usize, &Tok)> = None;
                let mut phrase = String::new();
                for j in i..raw.len() {
                    let Raw::Word(w) = &raw[j] else { break };
                    if j > i {
                        phrase.push(' ');
                    }
                    phrase.push_str(w);
                    if let Some((_, t)) = PHRASES.iter().find(|(p, _)| *p == phrase) {
                        best = Some((j + 1 - i, t));
                    }
                    if j - i > 5 {
                        break;
                    }
                }
                let words: Vec<String> = raw[i..].iter().take(best.map_or(1, |b| b.0)).map(|r| if let Raw::Word(w) = r { w.clone() } else { String::new() }).collect();
                let text = words.join(" ");
                match best {
                    Some((n, t)) => {
                        out.push(Token { tok: t.clone(), words: text });
                        i += n;
                    }
                    None => {
                        let w = &words[0];
                        let tok = if w.chars().count() == 1 {
                            match w.as_str() {
                                "e" => Const(Konst::E),
                                "a" => {
                                    // an article unless it sits in math: "a + 2"
                                    let math_next = matches!(raw.get(i + 1), Some(Raw::Sym(c)) if "+-*/^=)".contains(*c));
                                    let math_prev = matches!(i.checked_sub(1).and_then(|p| raw.get(p)), Some(Raw::Sym(c)) if "+-*/^=(".contains(*c)) || matches!(i.checked_sub(1).and_then(|p| raw.get(p)), Some(Raw::Number(_)));
                                    if math_next || math_prev {
                                        Var("a".into())
                                    } else {
                                        Filler
                                    }
                                }
                                v => Var(v.into()),
                            }
                        } else {
                            // "2ab", "ab + c": letters written together inside math are a product
                            let in_math = matches!(i.checked_sub(1).and_then(|p| raw.get(p)), Some(Raw::Number(_)) | Some(Raw::Sym('+' | '-' | '*' | '/' | '^' | '=' | '(')))
                                || matches!(raw.get(i + 1), Some(Raw::Sym('+' | '-' | '*' | '/' | '^' | '=' | ')')));
                            if in_math && (2..=3).contains(&w.len()) && w.chars().all(|c| c.is_ascii_lowercase()) {
                                for c in w.chars() {
                                    out.push(Token { tok: Var(c.to_string()), words: c.to_string() });
                                }
                                i += 1;
                                continue;
                            }
                            Unknown
                        };
                        out.push(Token { tok, words: text });
                        i += 1;
                    }
                }
            }
        }
    }
    // "is" means "=" between a letter and a value: "when x is 3"
    for k in 0..out.len() {
        if out[k].tok == Is {
            let prev_var = k > 0 && matches!(out[k - 1].tok, Var(_));
            let next_val = matches!(out.get(k + 1).map(|t| &t.tok), Some(Num(_) | Var(_) | Op('-') | Op('(') | Const(_)));
            out[k].tok = if prev_var && next_val { Op('=') } else { Filler };
        }
    }
    Ok(out)
}

/// A hint for an unknown word: the nearest known one, or a note.
pub fn suggest(word: &str) -> Option<String> {
    if word.len() <= 4 && word.chars().all(|c| c.is_ascii_lowercase()) && !PHRASES.iter().any(|(p, _)| p.starts_with(word)) {
        let letters: Vec<String> = word.chars().map(String::from).collect();
        if word.len() <= 3 {
            return Some(format!("for a product of letters write {}", letters.join("*")));
        }
    }
    let mut best: Option<(usize, &str)> = None;
    for (p, _) in PHRASES {
        let d = distance(word, p);
        if best.map_or(true, |(b, _)| d < b) {
            best = Some((d, p));
        }
    }
    match best {
        Some((d, p)) if d <= (word.len() / 3).max(1) => Some(format!("did you mean \"{p}\"?")),
        _ => None,
    }
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + usize::from(a[i - 1] != b[j - 1]));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// A phrase of the vocabulary, if it is one (logic and sets, agent L: the
/// statement grammar in logic.rs reads the same vocabulary).
pub fn phrase(p: &str) -> Option<&'static Tok> {
    PHRASES.iter().find(|(w, _)| *w == p).map(|(_, t)| t)
}

/// Every word and phrase, for --vocabulary.
/// Every key an `Open` token can carry: rules.toml must describe each.
pub const OPEN_KEYS: &[&str] = &["hodge", "riemann", "p_vs_np", "navier_stokes", "yang_mills", "bsd", "poincare", "goldbach", "twin_primes", "collatz"];

pub fn vocabulary() -> Vec<String> {
    let mut v: Vec<String> = PHRASES.iter().map(|(p, _)| p.to_string()).collect();
    v.extend(["any single letter (a variable)", "e", "d/dx", "+ - * / ^ ( ) = % ² ³ √ π × ÷", "< <= > >= ≤ ≥ | _"].map(String::from));
    v.extend(["dx (after an integrand)", "-> → ° ∞"].map(String::from));
    // logic and sets (agent L)
    v.extend([r"~ ¬ ! & ^ ∧ /\ | ∨ \/ -> => → <-> <=> ↔ ≡ T F (in a statement of logic)", r"∪ ∩ ' ^c \ ∖ ⊆ ⊂ ∅ {} U, capital letters (in a statement about sets)", "forall, for all, for every, exists, there exists, for some, such that, ∀ ∃, P(x) (in a statement with quantifiers)"].map(String::from));
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_and_symbols() {
        let t = lex("Solve 2x + 3 = 7 step by step").unwrap();
        let toks: Vec<Tok> = t.into_iter().map(|t| t.tok).collect();
        assert_eq!(toks[0], Task(Task::Solve));
        assert_eq!(toks[1], Num(Q::int(2)));
        assert_eq!(toks[2], Var("x".into()));
        assert_eq!(toks.last(), Some(&Mod(Modifier::Detailed)));
        let t = lex("d/dx sin x").unwrap();
        assert_eq!(t[0].tok, Task(Task::Differentiate));
        assert_eq!(t[2].tok, Var("x".into()));
        let t = lex("evaluate x^2 when x is 3").unwrap();
        assert!(t.iter().any(|t| t.tok == Op('=')));
        assert_eq!(lex("solfe x = 2").unwrap()[0].tok, Unknown);
        assert_eq!(suggest("solfe").as_deref(), Some("did you mean \"solve\"?"));
    }

    #[test]
    fn calculus_words_and_symbols() {
        let toks = |s: &str| lex(s).unwrap().into_iter().map(|t| t.tok).collect::<Vec<Tok>>();
        assert_eq!(toks("integrate x^2 dx")[4..], [For, Var("x".into())]);
        assert_eq!(toks("lim x->0")[2], Approaches);
        assert_eq!(toks("x → ∞")[1..], [Approaches, Const(Konst::Inf)]);
        assert_eq!(toks("cos 45°")[2], Degrees);
        assert_eq!(toks("tangent to y")[0], Task(Task::Tangent));
        assert_eq!(toks("tangent of x")[0], Func(Func::Tan));
        assert_eq!(toks("do")[0], Filler);
    }
}
