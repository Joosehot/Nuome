//! Printing: plain ASCII (the default, safe in any terminal), Unicode
//! (√, ·, ², ±) or LaTeX. Parentheses only where precedence needs them.

use crate::calls::Named;
use crate::expr::{Bound, Expr, Func, Interval, Konst, Math, Rel};
use crate::q::Q;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Style {
    #[default]
    Ascii,
    Unicode,
    Latex,
}

const ADD: u8 = 1;
const MUL: u8 = 2;
const POW: u8 = 4;
const ATOM: u8 = 5;

fn prec(e: &Expr) -> u8 {
    match e {
        Expr::Add(_) => ADD,
        Expr::Mul(_) | Expr::Neg(_) | Expr::Div(..) => MUL,
        Expr::Num(q) if q.is_neg() || !q.is_int() => MUL,
        Expr::Pow(..) => POW,
        Expr::Func(Func::Exp, _) => POW,
        Expr::Limit(..) => MUL,
        _ => ATOM,
    }
}

pub fn expr(e: &Expr, s: Style) -> String {
    let mut out = String::new();
    write(e, s, &mut out);
    out
}

pub fn math(m: &Math, s: Style) -> String {
    match m {
        Math::Expr(e) => expr(e, s),
        Math::Eq(l, r) => format!("{} = {}", expr(l, s), expr(r, s)),
        Math::Or(v) => v.iter().map(|(l, r)| format!("{} = {}", expr(l, s), expr(r, s))).collect::<Vec<_>>().join(if s == Style::Latex { " \\text{ or } " } else { " or " }),
        Math::NoSolution => if s == Style::Latex { "\\text{no real solution}" } else { "no real solution" }.into(),
        Math::Proved => if s == Style::Latex { "\\blacksquare" } else { "proved" }.into(),
        Math::AllReals => if s == Style::Latex { "\\text{every real number}" } else { "every real number" }.into(),
        Math::Ineq(l, r, rr) => format!("{} {} {}", expr(l, s), rel(*r, s), expr(rr, s)),
        Math::Intervals(v, ivs) => inequalities(v, ivs, s),
        Math::System(eqs) => eqs.iter().map(|(l, r)| format!("{} = {}", expr(l, s), expr(r, s))).collect::<Vec<_>>().join(", "),
        // logic and sets (agent L)
        Math::Taut(e) => expr(e, s),
        Math::Equiv(l, r) | Math::Entails(l, r) => format!("{} {} {}", statement_side(l, s), statement_sign(m, s), statement_side(r, s)),
        Math::Subset(l, r) => format!("{} {} {}", expr(l, s), statement_sign(m, s), expr(r, s)),
    }
}

pub fn number(q: &Q, s: Style) -> String {
    if q.is_int() {
        return q.to_string();
    }
    if s == Style::Latex {
        let sign = if q.is_neg() { "-" } else { "" };
        return format!("{sign}\\frac{{{}}}{{{}}}", q.num().abs(), q.den());
    }
    q.to_string()
}

fn paren(e: &Expr, s: Style, out: &mut String) {
    if s == Style::Latex {
        out.push_str("\\left(");
        write(e, s, out);
        out.push_str("\\right)");
    } else {
        out.push('(');
        write(e, s, out);
        out.push(')');
    }
}

fn at_least(e: &Expr, p: u8, s: Style, out: &mut String) {
    if prec(e) < p {
        paren(e, s, out);
    } else {
        write(e, s, out);
    }
}

fn superscript(n: &str) -> Option<String> {
    n.chars()
        .map(|c| match c {
            '0' => Some('⁰'),
            '1' => Some('¹'),
            '2' => Some('²'),
            '3' => Some('³'),
            '4' => Some('⁴'),
            '5' => Some('⁵'),
            '6' => Some('⁶'),
            '7' => Some('⁷'),
            '8' => Some('⁸'),
            '9' => Some('⁹'),
            '-' => Some('⁻'),
            _ => None,
        })
        .collect()
}

/// Does a factor print as something a number can sit right next to ("2x", "3(x + 1)")?
fn joins(prev: &Expr, next: &Expr) -> bool {
    let letterish = |e: &Expr| match e {
        Expr::Var(_) | Expr::Const(_) | Expr::Func(..) | Expr::Log(..) => true,
        Expr::Pow(b, _) => matches!(**b, Expr::Var(_) | Expr::Const(_) | Expr::Add(_) | Expr::Func(..)),
        Expr::Add(_) => true,
        _ => false,
    };
    // x * x^2 must not read "xx^2"
    let letter = |e: &Expr| match e {
        Expr::Var(v) => Some(v.clone()),
        Expr::Pow(b, _) => match &**b {
            Expr::Var(v) => Some(v.clone()),
            _ => None,
        },
        _ => None,
    };
    if letter(prev).is_some() && letter(prev) == letter(next) {
        return false;
    }
    match prev {
        // 1 * e^x must not read "1e^x"
        Expr::Num(q) => q.is_int() && letterish(next) && !(q.is_one() && matches!(next, Expr::Func(Func::Exp, _))),
        Expr::Var(_) | Expr::Const(_) => letterish(next),
        Expr::Pow(b, _) => matches!(**b, Expr::Var(_)) && matches!(next, Expr::Var(_) | Expr::Func(..) | Expr::Add(_)),
        Expr::Add(_) => letterish(next),
        // sin(x) cos(x)
        Expr::Func(..) => matches!(next, Expr::Func(..)),
        _ => false,
    }
}

/// Juxtaposed factors that read better with a space: x^3 sin(x), x sin(x).
fn spaced(prev: &Expr, next: &Expr) -> bool {
    matches!(next, Expr::Func(..) | Expr::Log(..)) && !matches!(prev, Expr::Num(_))
        || matches!(prev, Expr::Pow(..)) && matches!(next, Expr::Var(_) | Expr::Const(_))
        // 2k pi, 45 deg
        || matches!(prev, Expr::Var(_)) && matches!(next, Expr::Const(Konst::Pi))
        || matches!(next, Expr::Const(Konst::Deg))
}

fn write(e: &Expr, s: Style, out: &mut String) {
    match e {
        Expr::Num(q) => out.push_str(&number(q, s)),
        Expr::Var(v) => out.push_str(v),
        Expr::Const(Konst::Pi) => out.push_str(match s {
            Style::Ascii => "pi",
            Style::Unicode => "π",
            Style::Latex => "\\pi",
        }),
        Expr::Const(Konst::E) => out.push('e'),
        Expr::Const(Konst::Inf) => out.push_str(match s {
            Style::Ascii => "infinity",
            Style::Unicode => "∞",
            Style::Latex => "\\infty",
        }),
        Expr::Const(Konst::Deg) => out.push_str(match s {
            Style::Ascii => "deg",
            Style::Unicode => "°",
            Style::Latex => "^\\circ",
        }),
        Expr::Add(v) => {
            for (i, t) in v.iter().enumerate() {
                let (minus, body) = split_sign(t);
                if i == 0 {
                    if minus {
                        out.push('-');
                        at_least(&body, MUL, s, out);
                    } else {
                        write(t, s, out);
                    }
                    continue;
                }
                out.push_str(if minus { " - " } else { " + " });
                if minus {
                    at_least(&body, MUL, s, out);
                } else {
                    at_least(t, ADD + 1, s, out);
                }
            }
        }
        Expr::Mul(v) => {
            for (i, f) in v.iter().enumerate() {
                if i == 0 {
                    // a leading negative coefficient reads "-3x"
                    match f {
                        Expr::Num(q) if q.is_neg() && q.is_int() => {
                            if *q == Q::int(-1) && v.len() > 1 {
                                out.push('-');
                            } else {
                                out.push_str(&q.to_string());
                            }
                        }
                        _ => at_least(f, MUL + 1, s, out),
                    }
                    continue;
                }
                let prev = &v[i - 1];
                let implicit = joins(prev, f) || (matches!(prev, Expr::Num(q) if *q == Q::int(-1)) && i == 1);
                if implicit && matches!(f, Expr::Const(Konst::Deg)) && s != Style::Ascii {
                    // 45°, 45^\circ: the degree sign sits on the number
                } else if implicit && spaced(prev, f) && s != Style::Latex {
                    out.push(' ');
                } else if implicit && spaced(prev, f) {
                    out.push_str("\\,");
                }
                if !implicit {
                    out.push_str(match s {
                        Style::Ascii => " * ",
                        Style::Unicode => "·",
                        Style::Latex => " \\cdot ",
                    });
                }
                match f {
                    Expr::Add(_) => paren(f, s, out),
                    _ => at_least(f, MUL + 1, s, out),
                }
            }
        }
        Expr::Neg(a) => {
            out.push('-');
            // -1/x, not -(1/x): the two readings are equal
            at_least(a, if matches!(**a, Expr::Div(..)) { MUL } else { MUL + 1 }, s, out);
        }
        Expr::Div(a, b) => {
            if s == Style::Latex {
                out.push_str("\\frac{");
                write(a, s, out);
                out.push_str("}{");
                write(b, s, out);
                out.push('}');
            } else {
                // (10 * 9 * 8)/6, not 10 * 9 * 8/6
                let starred = matches!(&**a, Expr::Mul(v) if v.windows(2).any(|w| !joins(&w[0], &w[1])));
                if starred {
                    paren(a, s, out);
                } else {
                    at_least(a, MUL, s, out);
                }
                out.push('/');
                at_least(b, MUL + 1, s, out);
            }
        }
        Expr::Pow(a, b) => {
            // x^(1/2) reads as a square root
            if matches!(**b, Expr::Num(q) if q == Q::new(1, 2).unwrap()) {
                return write(&Expr::Func(Func::Sqrt, a.clone()), s, out);
            }
            at_least(a, ATOM, s, out);
            match s {
                Style::Latex => {
                    out.push_str("^{");
                    write(b, s, out);
                    out.push('}');
                }
                Style::Unicode => {
                    let plain = expr(b, Style::Ascii);
                    match (b.as_num().is_some_and(|q| q.is_int()), superscript(&plain)) {
                        (true, Some(sup)) => out.push_str(&sup),
                        _ => {
                            out.push('^');
                            at_least(b, ATOM, s, out);
                        }
                    }
                }
                Style::Ascii => {
                    out.push('^');
                    at_least(b, ATOM, s, out);
                }
            }
        }
        Expr::Func(Func::Exp, a) => {
            let p = Expr::Pow(Box::new(Expr::Const(Konst::E)), a.clone());
            write(&p, s, out);
        }
        Expr::Func(Func::Sqrt, a) => match s {
            Style::Latex => {
                out.push_str("\\sqrt{");
                write(a, s, out);
                out.push('}');
            }
            Style::Unicode => {
                out.push('√');
                at_least(a, ATOM, s, out);
            }
            Style::Ascii => {
                out.push_str("sqrt(");
                write(a, s, out);
                out.push(')');
            }
        },
        // ln|x|: the bars are the brackets
        Expr::Func(Func::Ln, a) if matches!(**a, Expr::Func(Func::Abs, _)) => {
            out.push_str(if s == Style::Latex { "\\ln" } else { "ln" });
            write(a, s, out);
        }
        Expr::Func(Func::Abs, a) => {
            out.push_str(if s == Style::Latex { "\\left|" } else { "|" });
            write(a, s, out);
            out.push_str(if s == Style::Latex { "\\right|" } else { "|" });
        }
        Expr::Log(b, a) => {
            out.push_str(if s == Style::Latex { "\\log" } else { "log" });
            // log(x) is base 10
            if !b.is_num(10) {
                match s {
                    Style::Latex => {
                        out.push_str("_{");
                        write(b, s, out);
                        out.push('}');
                    }
                    Style::Unicode if b.as_num().is_some_and(|q| q.is_int() && !q.is_neg()) => {
                        out.extend(expr(b, Style::Ascii).chars().map(|c| c.to_digit(10).and_then(|d| char::from_u32(0x2080 + d)).unwrap_or(c)));
                    }
                    _ => {
                        out.push('_');
                        at_least(b, ATOM, s, out);
                    }
                }
            }
            paren(a, s, out);
        }
        Expr::Func(f, a) => {
            if s == Style::Latex {
                out.push('\\');
            }
            out.push_str(f.name());
            paren(a, s, out);
        }
        Expr::Call(f, args) => call(*f, args, s, out),
        Expr::Integral(a, v) => {
            out.push_str(match s {
                Style::Ascii => "int ",
                Style::Unicode => "∫ ",
                Style::Latex => "\\int ",
            });
            integrand(a, v, s, out);
        }
        Expr::Bounds(f, v, a, b) => {
            let lim = |out: &mut String| {
                if s == Style::Latex {
                    out.push_str("_{");
                    write(a, s, out);
                    out.push_str("}^{");
                    write(b, s, out);
                    out.push('}');
                } else {
                    out.push('_');
                    at_least(a, ATOM, s, out);
                    out.push('^');
                    at_least(b, ATOM, s, out);
                }
            };
            match &**f {
                // the definite integral: int_0^3 x^2 dx
                Expr::Integral(g, w) if w == v => {
                    out.push_str(match s {
                        Style::Ascii => "int",
                        Style::Unicode => "∫",
                        Style::Latex => "\\int",
                    });
                    lim(out);
                    out.push(' ');
                    integrand(g, v, s, out);
                }
                _ => {
                    out.push_str(if s == Style::Latex { "\\left[" } else { "[" });
                    write(f, s, out);
                    out.push_str(if s == Style::Latex { "\\right]" } else { "]" });
                    lim(out);
                }
            }
        }
        Expr::Limit(a, v, p) => {
            match s {
                Style::Latex => {
                    out.push_str(&format!("\\lim_{{{v} \\to "));
                    write(p, s, out);
                    out.push_str("} ");
                }
                _ => {
                    out.push_str(&format!("lim({v}{}", if s == Style::Ascii { "->" } else { "→" }));
                    write(p, s, out);
                    out.push_str(") ");
                }
            }
            at_least(a, MUL, s, out);
        }
        Expr::At(a, v, p) => {
            out.push_str(if s == Style::Latex { "\\left[" } else { "[" });
            write(a, s, out);
            out.push_str(if s == Style::Latex { "\\right]_{" } else { "]_(" });
            out.push_str(&format!("{v}="));
            write(p, s, out);
            out.push(if s == Style::Latex { '}' } else { ')' });
        }
        Expr::Deriv(a, v) => {
            if s == Style::Latex {
                out.push_str(&format!("\\frac{{d}}{{d{v}}}\\left["));
                write(a, s, out);
                out.push_str("\\right]");
            } else {
                out.push_str(&format!("d/d{v}["));
                write(a, s, out);
                out.push(']');
            }
        }
        // logic and sets (agent L)
        Expr::Logic(..) | Expr::Truth(_) | Expr::Set(..) | Expr::SetConst(_) | Expr::Member(..) | Expr::Quant(..) | Expr::Pred(..) => logic(e, s, out),
    }
}

fn call(f: Named, args: &[Expr], s: Style, out: &mut String) {
    let list = |out: &mut String| {
        for (i, a) in args.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write(a, s, out);
        }
    };
    match (f, args) {
        // 1 + 2 + ... + n, written the way it was given
        (Named::Series, [t, lo, Expr::Var(n)]) => {
            let at = |k: i128| {
                let e = t.subst(n, &Expr::Num(crate::q::Q::int(k)));
                // 1^2 stays 1^2 (the pattern shows); 2 * 1 - 1 becomes 1
                if matches!(t, Expr::Pow(..)) {
                    e
                } else {
                    e.eval_q(&|_| None).map_or(e, Expr::Num)
                }
            };
            let Some(lo) = lo.as_num().filter(|q| q.is_int()) else { return write(t, s, out) };
            let dots = match s {
                Style::Ascii => " + ... + ",
                Style::Unicode => " + \u{22ef} + ",
                Style::Latex => " + \\cdots + ",
            };
            at_least(&at(lo.num()), ADD + 1, s, out);
            out.push_str(" + ");
            at_least(&at(lo.num() + 1), ADD + 1, s, out);
            out.push_str(dots);
            at_least(t, ADD + 1, s, out);
        }
        (Named::Divides, [d, e]) => {
            at_least(d, MUL + 1, s, out);
            out.push_str(match s {
                Style::Ascii => " divides ",
                Style::Unicode => " \u{2223} ",
                Style::Latex => " \\mid ",
            });
            write(e, s, out);
        }
        (Named::Mod, [a, b]) => {
            at_least(a, MUL + 1, s, out);
            out.push_str(if s == Style::Latex { " \\bmod " } else { " mod " });
            at_least(b, MUL + 1, s, out);
        }
        (Named::Factorial, [a]) => {
            at_least(a, ATOM, s, out);
            out.push('!');
        }
        (Named::Choose, [n, k]) if s == Style::Latex => {
            out.push_str("\\binom{");
            write(n, s, out);
            out.push_str("}{");
            write(k, s, out);
            out.push('}');
        }
        _ => {
            let name = f.name();
            if s == Style::Latex {
                out.push_str(&format!("\\operatorname{{{name}}}"));
            } else {
                out.push_str(name);
            }
            out.push('(');
            list(out);
            out.push(')');
        }
    }
}

// ---- algebra (agent A): inequality signs, intervals ----

pub fn rel(r: Rel, s: Style) -> &'static str {
    match (r, s) {
        (Rel::Lt, _) => "<",
        (Rel::Gt, _) => ">",
        (Rel::Le, Style::Ascii) => "<=",
        (Rel::Ge, Style::Ascii) => ">=",
        (Rel::Le, Style::Unicode) => "≤",
        (Rel::Ge, Style::Unicode) => "≥",
        (Rel::Le, Style::Latex) => "\\le",
        (Rel::Ge, Style::Latex) => "\\ge",
    }
}

/// The sign between a bound and the letter, read left to right: a < x or a <= x.
fn sign(b: &Bound) -> Rel {
    if b.closed {
        Rel::Le
    } else {
        Rel::Lt
    }
}

/// Intervals as inequalities in the letter: "x < 2 or x >= 3", "-1 <= x < 4".
pub fn inequalities(v: &str, ivs: &[Interval], s: Style) -> String {
    if ivs.is_empty() {
        return math(&Math::NoSolution, s);
    }
    let or = if s == Style::Latex { " \\text{ or } " } else { " or " };
    ivs.iter()
        .map(|iv| match (&iv.lo, &iv.hi) {
            (Some(a), Some(b)) if a.at == b.at => format!("{v} = {}", expr(&a.at, s)),
            (Some(a), Some(b)) => format!("{} {} {v} {} {}", expr(&a.at, s), rel(sign(a), s), rel(sign(b), s), expr(&b.at, s)),
            (Some(a), None) => format!("{v} {} {}", rel(sign(a).flip(), s), expr(&a.at, s)),
            (None, Some(b)) => format!("{v} {} {}", rel(sign(b), s), expr(&b.at, s)),
            (None, None) => math(&Math::AllReals, s),
        })
        .collect::<Vec<_>>()
        .join(or)
}

/// Interval notation: (-inf, 2) U [3, inf).
pub fn interval_notation(ivs: &[Interval], s: Style) -> String {
    let inf = match s {
        Style::Ascii => "inf",
        Style::Unicode => "∞",
        Style::Latex => "\\infty",
    };
    let union = match s {
        Style::Ascii => " U ",
        Style::Unicode => " ∪ ",
        Style::Latex => " \\cup ",
    };
    ivs.iter()
        .map(|iv| {
            if let (Some(a), Some(b)) = (&iv.lo, &iv.hi) {
                if a.at == b.at {
                    return format!("{{{}}}", expr(&a.at, s));
                }
            }
            let lo = iv.lo.as_ref().map_or(format!("(-{inf}"), |b| format!("{}{}", if b.closed { "[" } else { "(" }, expr(&b.at, s)));
            let hi = iv.hi.as_ref().map_or(format!("{inf})"), |b| format!("{}{}", expr(&b.at, s), if b.closed { "]" } else { ")" }));
            format!("{lo}, {hi}")
        })
        .collect::<Vec<_>>()
        .join(union)
}

/// The body of an integral and its "dx": x^2 dx, (x + 1) dx.
fn integrand(a: &Expr, v: &str, s: Style, out: &mut String) {
    at_least(a, MUL, s, out);
    out.push_str(if s == Style::Latex { "\\,d" } else { " d" });
    out.push_str(v);
}

/// A term that prints with a leading minus, and what follows the minus.
pub fn split_sign(t: &Expr) -> (bool, Expr) {
    match t {
        Expr::Neg(a) => (true, (**a).clone()),
        Expr::Num(q) if q.is_neg() => (true, Expr::Num(q.neg())),
        Expr::Mul(v) if matches!(v.first(), Some(Expr::Num(q)) if q.is_neg()) => {
            let mut v = v.clone();
            let c = v[0].as_num().unwrap().neg();
            if c.is_one() {
                v.remove(0);
            } else {
                v[0] = Expr::Num(c);
            }
            (true, crate::expr::mul(v))
        }
        _ => (false, t.clone()),
    }
}

// ---- logic and sets (agent L) ----

/// The sign between the two sides of a statement about statements or sets:
/// <=> (equivalent), => (assuming the left, the right), subset of.
pub fn statement_sign(m: &Math, s: Style) -> &'static str {
    match (m, s) {
        (Math::Equiv(..), Style::Ascii) => "<=>",
        (Math::Equiv(..), Style::Unicode) => "≡",
        (Math::Equiv(..), Style::Latex) => "\\equiv",
        (Math::Entails(..), Style::Ascii) => "=>",
        (Math::Entails(..), Style::Unicode) => "⟹",
        (Math::Entails(..), Style::Latex) => "\\implies",
        (Math::Subset(..), Style::Ascii) => "subset of",
        (Math::Subset(..), Style::Unicode) => "⊆",
        (Math::Subset(..), Style::Latex) => "\\subseteq",
        _ => "=",
    }
}

/// A side of <=> or =>: an implication or biconditional is bracketed, so
/// (p -> q) <=> (~q -> ~p) reads at a glance.
fn statement_side(e: &Expr, s: Style) -> String {
    let mut out = String::new();
    if matches!(e, Expr::Logic(crate::expr::Conn::Implies | crate::expr::Conn::Iff, _)) {
        paren(e, s, &mut out);
    } else {
        write(e, s, &mut out);
    }
    out
}

/// A connective or set operation as it is printed.
pub fn logic_sign(e: &Expr, s: Style) -> &'static str {
    use crate::expr::{Conn, SetOp};
    match (e, s) {
        (Expr::Logic(Conn::Not, _), Style::Ascii) => "~",
        (Expr::Logic(Conn::Not, _), Style::Unicode) => "¬",
        (Expr::Logic(Conn::Not, _), Style::Latex) => "\\neg ",
        (Expr::Logic(Conn::And, _), Style::Ascii) => " and ",
        (Expr::Logic(Conn::And, _), Style::Unicode) => " ∧ ",
        (Expr::Logic(Conn::And, _), Style::Latex) => " \\land ",
        (Expr::Logic(Conn::Or, _), Style::Ascii) => " or ",
        (Expr::Logic(Conn::Or, _), Style::Unicode) => " ∨ ",
        (Expr::Logic(Conn::Or, _), Style::Latex) => " \\lor ",
        (Expr::Logic(Conn::Implies, _), Style::Ascii) => " -> ",
        (Expr::Logic(Conn::Implies, _), Style::Unicode) => " → ",
        (Expr::Logic(Conn::Implies, _), Style::Latex) => " \\to ",
        (Expr::Logic(Conn::Iff, _), Style::Ascii) => " <-> ",
        (Expr::Logic(Conn::Iff, _), Style::Unicode) => " ↔ ",
        (Expr::Logic(Conn::Iff, _), Style::Latex) => " \\leftrightarrow ",
        (Expr::Set(SetOp::Complement, _), Style::Latex) => "^{c}",
        (Expr::Set(SetOp::Complement, _), _) => "'",
        (Expr::Set(SetOp::Union, _), Style::Ascii) => " union ",
        (Expr::Set(SetOp::Union, _), Style::Unicode) => " ∪ ",
        (Expr::Set(SetOp::Union, _), Style::Latex) => " \\cup ",
        (Expr::Set(SetOp::Inter, _), Style::Ascii) => " intersect ",
        (Expr::Set(SetOp::Inter, _), Style::Unicode) => " ∩ ",
        (Expr::Set(SetOp::Inter, _), Style::Latex) => " \\cap ",
        (Expr::Set(SetOp::Diff, _), Style::Ascii) => " \\ ",
        (Expr::Set(SetOp::Diff, _), Style::Unicode) => " ∖ ",
        (Expr::Set(SetOp::Diff, _), Style::Latex) => " \\setminus ",
        (Expr::Truth(true), Style::Latex) => "\\mathrm{T}",
        (Expr::Truth(false), Style::Latex) => "\\mathrm{F}",
        (Expr::Truth(true), _) => "T",
        (Expr::Truth(false), _) => "F",
        (Expr::SetConst(true), _) => "U",
        (Expr::SetConst(false), Style::Ascii) => "{}",
        (Expr::SetConst(false), Style::Unicode) => "∅",
        (Expr::SetConst(false), Style::Latex) => "\\emptyset",
        _ => "",
    }
}

/// Statements and sets. A part that is itself built with a two-sided
/// connective or operation is always bracketed, the way textbooks write
/// (p and q) or r and (A intersect B) union C.
fn logic(e: &Expr, s: Style, out: &mut String) {
    use crate::expr::{Conn, SetOp};
    let binary = |e: &Expr| matches!(e, Expr::Logic(c, _) if *c != Conn::Not) || matches!(e, Expr::Set(o, _) if *o != SetOp::Complement);
    let part = |e: &Expr, out: &mut String| if binary(e) { paren(e, s, out) } else { write(e, s, out) };
    match e {
        Expr::Logic(Conn::Not, v) => match &v[0] {
            // x not in A
            Expr::Member(x, a) => {
                out.push_str(x);
                out.push_str(match s {
                    Style::Ascii => " not in ",
                    Style::Unicode => " ∉ ",
                    Style::Latex => " \\notin ",
                });
                write(a, s, out);
            }
            a => {
                out.push_str(logic_sign(e, s));
                if matches!(a, Expr::Member(..)) {
                    paren(a, s, out);
                } else {
                    part(a, out);
                }
            }
        },
        Expr::Set(SetOp::Complement, v) => {
            let a = &v[0];
            if matches!(a, Expr::Var(_) | Expr::SetConst(_)) {
                write(a, s, out);
            } else {
                paren(a, s, out);
            }
            out.push_str(logic_sign(e, s));
        }
        Expr::Logic(_, v) | Expr::Set(_, v) => {
            for (i, a) in v.iter().enumerate() {
                if i > 0 {
                    out.push_str(logic_sign(e, s));
                }
                part(a, out);
            }
        }
        Expr::Member(x, a) => {
            out.push_str(x);
            out.push_str(match s {
                Style::Ascii => " in ",
                Style::Unicode => " ∈ ",
                Style::Latex => " \\in ",
            });
            write(a, s, out);
        }
        // forall x P(x), exists x (P(x) and Q(x))
        Expr::Quant(all, x, a) => {
            out.push_str(match (all, s) {
                (true, Style::Ascii) => "forall ",
                (false, Style::Ascii) => "exists ",
                (true, Style::Unicode) => "∀",
                (false, Style::Unicode) => "∃",
                (true, Style::Latex) => "\\forall ",
                (false, Style::Latex) => "\\exists ",
            });
            out.push_str(x);
            out.push_str(if s == Style::Latex { "\\, " } else { " " });
            part(a, out);
        }
        Expr::Pred(p, x) => {
            out.push_str(p);
            out.push('(');
            out.push_str(x);
            out.push(')');
        }
        _ => out.push_str(logic_sign(e, s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::*;

    #[test]
    fn prints_like_paper() {
        let e = add(vec![mul(vec![num(2), var("x")]), num(-3)]);
        assert_eq!(expr(&e, Style::Ascii), "2x - 3");
        let e = mul(vec![num(3), add(vec![var("x"), num(1)])]);
        assert_eq!(expr(&e, Style::Ascii), "3(x + 1)");
        let e = pow(var("x"), num(2));
        assert_eq!(expr(&e, Style::Unicode), "x²");
        let e = div(add(vec![num(5), sqrt(num(1))]), num(2));
        assert_eq!(expr(&e, Style::Ascii), "(5 + sqrt(1))/2");
        let e = mul(vec![add(vec![var("x"), num(-2)]), add(vec![var("x"), num(-3)])]);
        assert_eq!(expr(&e, Style::Ascii), "(x - 2)(x - 3)");
        let e = add(vec![num(7), mul(vec![num(-2), var("x")])]);
        assert_eq!(expr(&e, Style::Ascii), "7 - 2x");
        let e = mul(vec![num(3), num(4)]);
        assert_eq!(expr(&e, Style::Ascii), "3 * 4");
    }
}
