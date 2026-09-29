//! The closed vocabulary: words, phrases and symbols -> tokens. A word that
//! isn't listed is an error with the nearest known word, never a guess.

use crate::expr::{Func, Konst};
use crate::model::{Modifier, Task};
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
    /// "approaches", "tends to", "->": the point a limit is taken at follows.
    Approaches,
    /// "from", "between": bounds follow ("from 0 to 3", "between 0 and pi").
    From,
    /// "second derivative", "twice": differentiate this many times.
    Order(u32),
    /// "degrees", "°": the number before it is an angle in degrees.
    Degrees,
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
    ("from", From),
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
    ("abs", Func(Func::Abs)),
    ("absolute value of", Func(Func::Abs)),
    ("the absolute value of", Func(Func::Abs)),
    // recognisably math, not in v0
    ("matrix", Unsupported("matrix algebra")),
    ("matrices", Unsupported("matrix algebra")),
    ("determinant", Unsupported("matrix algebra")),
    ("system", Unsupported("solving systems of equations")),
    ("simultaneous", Unsupported("solving systems of equations")),
    ("simultaneously", Unsupported("solving systems of equations")),
    ("log", Unsupported("a logarithm other than ln")),
    ("logarithm", Unsupported("a logarithm other than ln")),
    ("inequality", Unsupported("solving inequalities")),
    ("graph", Unsupported("plotting")),
    ("plot", Unsupported("plotting")),
    ("prove", Unsupported("writing proofs")),
    ("probability", Unsupported("probability")),
    ("mean", Unsupported("statistics")),
    ("median", Unsupported("statistics")),
    ("average", Unsupported("statistics")),
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
        } else if c == '∞' {
            out.push(Raw::Word("infinity".into()));
            i += 1;
        } else {
            let sym = match c {
                '+' | '-' | '*' | '/' | '^' | '(' | ')' | '=' | '%' | ',' | ';' | ':' | '?' | '!' | '.' => c,
                '[' | '{' => '(',
                ']' | '}' => ')',
                '×' | '·' | '⋅' => '*',
                '÷' => '/',
                '−' | '–' => '-',
                '²' | '³' | '√' | 'π' | '°' => c,
                '<' | '>' | '≤' | '≥' => '<',
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
    let raw = scan(s)?;
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
                    '°' => Degrees,
                    '<' => Unsupported("solving inequalities"),
                    ',' | ';' | ':' | '?' | '!' | '.' => Sep,
                    c => Op(*c),
                };
                out.push(Token { tok, words: c.to_string() });
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

/// Every word and phrase, for --vocabulary.
pub fn vocabulary() -> Vec<String> {
    let mut v: Vec<String> = PHRASES.iter().map(|(p, _)| p.to_string()).collect();
    v.extend(["any single letter (a variable)", "e", "d/dx", "+ - * / ^ ( ) = % ² ³ √ π × ÷"].map(String::from));
    v.extend(["dx (after an integrand)", "-> → ° ∞"].map(String::from));
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
}
