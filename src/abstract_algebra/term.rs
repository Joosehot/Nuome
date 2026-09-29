//! Terms of a group or a ring. Products are binary and never reordered:
//! (ab)c and a(bc) are different terms, and only the associativity axiom
//! turns one into the other. Nothing here knows any law; laws are
//! equations between terms whose letters are pattern variables.

use crate::print::Style;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Term {
    /// A letter: an element, or in a law a pattern variable.
    El(String),
    /// The identity e of a group.
    E,
    /// A product, not commutative: Mul(a, b) is ab.
    Mul(Box<Term>, Box<Term>),
    /// A group inverse a^-1.
    Inv(Box<Term>),
    /// The zero of a ring.
    Zero,
    /// A ring sum a + b.
    Add(Box<Term>, Box<Term>),
    /// A ring's additive inverse -a.
    Neg(Box<Term>),
}

use Term::*;

pub fn el(v: &str) -> Term {
    El(v.to_string())
}
pub fn mul(a: Term, b: Term) -> Term {
    Mul(Box::new(a), Box::new(b))
}
pub fn inv(a: Term) -> Term {
    Inv(Box::new(a))
}
pub fn add(a: Term, b: Term) -> Term {
    Add(Box::new(a), Box::new(b))
}
pub fn neg(a: Term) -> Term {
    Neg(Box::new(a))
}

/// A substitution: pattern variable -> term.
pub type Subst = BTreeMap<String, Term>;

impl Term {
    pub fn children(&self) -> Vec<&Term> {
        match self {
            Mul(a, b) | Add(a, b) => vec![a, b],
            Inv(a) | Neg(a) => vec![a],
            El(_) | E | Zero => vec![],
        }
    }
    fn child_mut(&mut self, i: usize) -> &mut Term {
        match self {
            Mul(a, b) | Add(a, b) => {
                if i == 0 {
                    a
                } else {
                    b
                }
            }
            Inv(a) | Neg(a) => a,
            El(_) | E | Zero => unreachable!("a leaf has no children"),
        }
    }
    pub fn get(&self, path: &[usize]) -> &Term {
        match path.split_first() {
            None => self,
            Some((i, rest)) => self.children()[*i].get(rest),
        }
    }
    pub fn replace(&self, path: &[usize], new: Term) -> Term {
        let mut out = self.clone();
        let mut cur = &mut out;
        for &i in path {
            cur = cur.child_mut(i);
        }
        *cur = new;
        out
    }
    /// Every subterm with its position, parents first, left to right.
    pub fn walk(&self) -> Vec<(Vec<usize>, &Term)> {
        fn go<'a>(t: &'a Term, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, &'a Term)>) {
            out.push((path.clone(), t));
            for (i, c) in t.children().into_iter().enumerate() {
                path.push(i);
                go(c, path, out);
                path.pop();
            }
        }
        let mut out = Vec::new();
        go(self, &mut Vec::new(), &mut out);
        out
    }
    pub fn size(&self) -> usize {
        1 + self.children().iter().map(|c| c.size()).sum::<usize>()
    }
    pub fn letters(&self) -> BTreeSet<String> {
        self.walk().into_iter().filter_map(|(_, t)| if let El(v) = t { Some(v.clone()) } else { None }).collect()
    }
    pub fn has_letter_in(&self, vars: &BTreeSet<String>) -> bool {
        self.walk().iter().any(|(_, t)| matches!(t, El(v) if vars.contains(v)))
    }
    /// Replace pattern variables by their values (repeatedly, for unifiers).
    pub fn subst(&self, s: &Subst) -> Term {
        match self {
            El(v) => match s.get(v) {
                Some(t) if t != self => t.subst(s),
                _ => self.clone(),
            },
            Mul(a, b) => mul(a.subst(s), b.subst(s)),
            Add(a, b) => add(a.subst(s), b.subst(s)),
            Inv(a) => inv(a.subst(s)),
            Neg(a) => neg(a.subst(s)),
            E | Zero => self.clone(),
        }
    }
    /// Rename every letter.
    pub fn map_letters(&self, f: &dyn Fn(&str) -> String) -> Term {
        match self {
            El(v) => El(f(v)),
            Mul(a, b) => mul(a.map_letters(f), b.map_letters(f)),
            Add(a, b) => add(a.map_letters(f), b.map_letters(f)),
            Inv(a) => inv(a.map_letters(f)),
            Neg(a) => neg(a.map_letters(f)),
            t => t.clone(),
        }
    }
    /// Rename letters: x -> prefix + x for the letters in `vars`.
    pub fn rename(&self, vars: &BTreeSet<String>, prefix: &str) -> Term {
        self.map_letters(&|v| if vars.contains(v) { format!("{prefix}{v}") } else { v.to_string() })
    }
}

/// One-way matching: does `pat` (letters in `vars` are variables) match
/// `t`, consistently with `s`? Letters of `t` are always constants.
pub fn matches(pat: &Term, t: &Term, vars: &BTreeSet<String>, s: &mut Subst) -> bool {
    match (pat, t) {
        (El(v), _) if vars.contains(v) => match s.get(v) {
            Some(bound) => bound == t,
            None => {
                s.insert(v.clone(), t.clone());
                true
            }
        },
        (El(a), El(b)) => a == b,
        (E, E) | (Zero, Zero) => true,
        (Mul(a, b), Mul(c, d)) | (Add(a, b), Add(c, d)) => matches(a, c, vars, s) && matches(b, d, vars, s),
        (Inv(a), Inv(b)) | (Neg(a), Neg(b)) => matches(a, b, vars, s),
        _ => false,
    }
}

/// Follow a variable's binding to what it stands for.
fn resolve<'a>(t: &'a Term, vars: &BTreeSet<String>, s: &'a Subst) -> &'a Term {
    match t {
        El(v) if vars.contains(v) => match s.get(v) {
            Some(b) => resolve(b, vars, s),
            None => t,
        },
        _ => t,
    }
}

fn occurs(v: &str, t: &Term, vars: &BTreeSet<String>, s: &Subst) -> bool {
    match resolve(t, vars, s) {
        El(w) => w == v,
        t => t.children().iter().any(|c| occurs(v, c, vars, s)),
    }
}

/// Syntactic unification: letters in `vars` on either side are variables.
/// Bindings may refer to each other; `Term::subst` reads them out.
pub fn unify(a: &Term, b: &Term, vars: &BTreeSet<String>, s: &mut Subst) -> bool {
    let (a, b) = (resolve(a, vars, s).clone(), resolve(b, vars, s).clone());
    let var = |t: &Term| match t {
        El(v) if vars.contains(v) => Some(v.clone()),
        _ => None,
    };
    match (var(&a), var(&b)) {
        (Some(x), Some(y)) if x == y => return true,
        (Some(x), _) => {
            if occurs(&x, &b, vars, s) {
                return false;
            }
            s.insert(x, b);
            return true;
        }
        (_, Some(y)) => {
            if occurs(&y, &a, vars, s) {
                return false;
            }
            s.insert(y, a);
            return true;
        }
        _ => {}
    }
    match (&a, &b) {
        (El(x), El(y)) => x == y,
        (E, E) | (Zero, Zero) => true,
        (Mul(p, q), Mul(r, t)) | (Add(p, q), Add(r, t)) => unify(p, r, vars, s) && unify(q, t, vars, s),
        (Inv(p), Inv(q)) | (Neg(p), Neg(q)) => unify(p, q, vars, s),
        _ => false,
    }
}

/// The deepest position where two terms differ (None if they are equal).
pub fn diff(a: &Term, b: &Term) -> Option<Vec<usize>> {
    if a == b {
        return None;
    }
    let same_shape = std::mem::discriminant(a) == std::mem::discriminant(b) && a.children().len() == b.children().len() && !a.children().is_empty();
    if same_shape {
        let differing: Vec<usize> = (0..a.children().len()).filter(|&i| a.children()[i] != b.children()[i]).collect();
        if let [i] = differing[..] {
            let mut p = vec![i];
            p.extend(diff(a.children()[i], b.children()[i]).unwrap_or_default());
            return Some(p);
        }
    }
    Some(vec![])
}

// ---- normal forms (for the search's sense of distance only) ----

/// A group term's letters in order, with brackets and e dropped but
/// nothing cancelled: (ba)a^-1 and b(aa^-1) are both b a a^-1.
pub fn flat(t: &Term) -> Vec<(String, bool)> {
    fn go(t: &Term, inverse: bool, out: &mut Vec<(String, bool)>) {
        match t {
            El(v) => out.push((v.clone(), inverse)),
            Mul(a, b) if inverse => {
                go(b, true, out);
                go(a, true, out);
            }
            Mul(a, b) | Add(a, b) => {
                go(a, inverse, out);
                go(b, inverse, out);
            }
            Inv(a) | Neg(a) => go(a, !inverse, out),
            E | Zero => {}
        }
    }
    let mut out = Vec::new();
    go(t, false, &mut out);
    out
}

/// A group term as a freely reduced word: brackets and e dropped, x x^-1
/// cancelled. Two sides with the same word differ only by axiom steps.
pub fn word(t: &Term) -> Vec<(String, bool)> {
    fn go(t: &Term, inverse: bool, out: &mut Vec<(String, bool)>) {
        match t {
            El(v) => {
                if out.last().is_some_and(|(w, i)| w == v && *i != inverse) {
                    out.pop();
                } else {
                    out.push((v.clone(), inverse));
                }
            }
            Mul(a, b) if inverse => {
                go(b, true, out);
                go(a, true, out);
            }
            Mul(a, b) | Add(a, b) => {
                go(a, inverse, out);
                go(b, inverse, out);
            }
            Inv(a) | Neg(a) => go(a, !inverse, out),
            E | Zero => {}
        }
    }
    let mut out = Vec::new();
    go(t, false, &mut out);
    out
}

/// A ring term multiplied out: each word of letters with its whole-number
/// coefficient. Two sides with the same sum differ only by axiom steps.
pub fn monomials(t: &Term) -> BTreeMap<Vec<String>, i64> {
    let mut out: BTreeMap<Vec<String>, i64> = BTreeMap::new();
    match t {
        El(v) => {
            out.insert(vec![v.clone()], 1);
        }
        E => {
            out.insert(vec![], 1);
        }
        Zero => {}
        Add(a, b) => {
            out = monomials(a);
            for (w, c) in monomials(b) {
                *out.entry(w).or_insert(0) += c;
            }
        }
        Neg(a) | Inv(a) => {
            out = monomials(a).into_iter().map(|(w, c)| (w, -c)).collect();
        }
        Mul(a, b) => {
            let (x, y) = (monomials(a), monomials(b));
            for (wa, ca) in &x {
                for (wb, cb) in &y {
                    let mut w = wa.clone();
                    w.extend(wb.iter().cloned());
                    *out.entry(w).or_insert(0) += ca * cb;
                }
            }
        }
    }
    out.retain(|_, c| *c != 0);
    out
}

// ---- printing ----

fn product_part(t: &Term, s: Style, out: &mut String) {
    match t {
        Mul(..) | Add(..) | Neg(_) => paren(t, s, out),
        _ => write(t, s, out),
    }
}

fn paren(t: &Term, s: Style, out: &mut String) {
    let (l, r) = if s == Style::Latex { ("\\left(", "\\right)") } else { ("(", ")") };
    out.push_str(l);
    write(t, s, out);
    out.push_str(r);
}

fn ends_with_inverse(t: &Term) -> bool {
    match t {
        Inv(_) => true,
        _ => false,
    }
}

pub fn write(t: &Term, s: Style, out: &mut String) {
    match t {
        El(v) => out.push_str(v),
        E => out.push('e'),
        Zero => out.push('0'),
        Mul(a, b) => {
            product_part(a, s, out);
            // a^-1 b: a space keeps the exponent apart from the next factor
            if s == Style::Ascii && ends_with_inverse(a) {
                out.push(' ');
            }
            // a0, 0a: a zero next to a letter reads as a product
            product_part(b, s, out);
        }
        Inv(a) => {
            match &**a {
                El(_) | E => write(a, s, out),
                _ => paren(a, s, out),
            }
            out.push_str(match s {
                Style::Ascii => "^-1",
                Style::Unicode => "⁻¹",
                Style::Latex => "^{-1}",
            });
        }
        Add(a, b) => {
            match &**a {
                Add(..) => paren(a, s, out),
                _ => write(a, s, out),
            }
            out.push_str(" + ");
            match &**b {
                Add(..) | Neg(_) => paren(b, s, out),
                _ => write(b, s, out),
            }
        }
        Neg(a) => {
            out.push('-');
            match &**a {
                El(_) | Zero => write(a, s, out),
                _ => paren(a, s, out),
            }
        }
    }
}

pub fn show(t: &Term, s: Style) -> String {
    let mut out = String::new();
    write(t, s, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_match_unify() {
        let ab_inv = inv(mul(el("a"), el("b")));
        assert_eq!(show(&ab_inv, Style::Ascii), "(ab)^-1");
        let t = mul(inv(el("b")), inv(el("a")));
        assert_eq!(show(&t, Style::Ascii), "b^-1 a^-1");
        assert_eq!(show(&t, Style::Unicode), "b⁻¹a⁻¹");
        assert_eq!(show(&t, Style::Latex), "b^{-1}a^{-1}");
        assert_eq!(show(&mul(neg(el("a")), neg(el("b"))), Style::Ascii), "(-a)(-b)");
        assert_eq!(show(&add(el("a"), neg(el("a"))), Style::Ascii), "a + (-a)");
        let vars: BTreeSet<String> = ["x".to_string()].into();
        let mut s = Subst::new();
        assert!(matches(&inv(el("x")), &ab_inv, &vars, &mut s));
        assert_eq!(s["x"], mul(el("a"), el("b")));
        let mut s = Subst::new();
        assert!(!matches(&mul(el("x"), el("x")), &mul(el("a"), el("b")), &vars, &mut s));
        let both: BTreeSet<String> = ["x".to_string(), "y".to_string()].into();
        let mut s = Subst::new();
        assert!(unify(&mul(el("x"), el("a")), &mul(el("b"), el("y")), &both, &mut s));
        assert_eq!(mul(el("x"), el("y")).subst(&s), mul(el("b"), el("a")));
        assert_eq!(diff(&mul(el("a"), el("b")), &mul(el("a"), el("c"))), Some(vec![1]));
    }
}
