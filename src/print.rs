//! Printing: plain ASCII (the default, safe in any terminal), Unicode
//! (√, ·, ², ±) or LaTeX. Parentheses only where precedence needs them.

use crate::calls::Named;
use crate::expr::{Expr, Func, Konst, Math};
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
        Math::AllReals => if s == Style::Latex { "\\text{every real number}" } else { "every real number" }.into(),
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
        Expr::Var(_) | Expr::Const(_) | Expr::Func(..) => true,
        Expr::Pow(b, _) => matches!(**b, Expr::Var(_) | Expr::Const(_) | Expr::Add(_)),
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
        Expr::Num(q) => q.is_int() && letterish(next),
        Expr::Var(_) | Expr::Const(_) => letterish(next),
        Expr::Pow(b, _) => matches!(**b, Expr::Var(_)) && matches!(next, Expr::Var(_) | Expr::Func(..) | Expr::Add(_)),
        Expr::Add(_) => letterish(next),
        _ => false,
    }
}

/// Juxtaposed factors that read better with a space: x^3 sin(x), x sin(x).
fn spaced(prev: &Expr, next: &Expr) -> bool {
    matches!(next, Expr::Func(..)) && !matches!(prev, Expr::Num(_))
        || matches!(prev, Expr::Pow(..)) && matches!(next, Expr::Var(_) | Expr::Const(_))
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
                if implicit && spaced(prev, f) && s != Style::Latex {
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
            at_least(a, MUL + 1, s, out);
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
        Expr::Func(f, a) => {
            if s == Style::Latex {
                out.push('\\');
            }
            out.push_str(f.name());
            paren(a, s, out);
        }
        Expr::Call(f, args) => call(*f, args, s, out),
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
