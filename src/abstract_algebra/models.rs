//! Concrete groups and rings, as tables: the model check tries a statement
//! in each of them for every assignment of elements to its letters. This
//! shares nothing with the rules: a statement true in every group holds in
//! each table, and one that fails somewhere comes back with the elements
//! that break it.

use super::parse::{Eqn, Statement};
use super::term::Term;
use super::Kind;

/// A finite structure: a group (mul, e, inv) or a ring (add, zero, neg, mul).
#[derive(Clone, Debug)]
pub struct Model {
    pub name: String,
    /// How to read its elements, said with a counterexample.
    pub gloss: String,
    pub names: Vec<String>,
    pub mul: Vec<Vec<usize>>,
    pub e: usize,
    pub inv: Vec<usize>,
    pub add: Vec<Vec<usize>>,
    pub zero: usize,
    pub neg: Vec<usize>,
}

impl Model {
    pub fn size(&self) -> usize {
        self.names.len()
    }
    fn group(name: &str, names: Vec<String>, mul: Vec<Vec<usize>>) -> Model {
        let n = names.len();
        let e = (0..n).find(|&e| (0..n).all(|x| mul[e][x] == x && mul[x][e] == x)).expect("a group has an identity");
        let inv = (0..n).map(|x| (0..n).find(|&y| mul[x][y] == e).expect("every element has an inverse")).collect();
        Model { name: name.into(), gloss: String::new(), names, mul, e, inv, add: vec![], zero: 0, neg: vec![] }
    }
    fn ring(name: &str, names: Vec<String>, add: Vec<Vec<usize>>, mul: Vec<Vec<usize>>) -> Model {
        let n = names.len();
        let zero = (0..n).find(|&z| (0..n).all(|x| add[z][x] == x)).expect("a ring has a zero");
        let neg = (0..n).map(|x| (0..n).find(|&y| add[x][y] == zero).expect("every element has a negative")).collect();
        Model { name: name.into(), gloss: String::new(), names, mul, e: 0, inv: vec![], add, zero, neg }
    }
    pub fn eval(&self, t: &Term, env: &dyn Fn(&str) -> usize) -> usize {
        match t {
            Term::El(v) => env(v),
            Term::E => self.e,
            Term::Zero => self.zero,
            Term::Mul(a, b) => self.mul[self.eval(a, env)][self.eval(b, env)],
            Term::Add(a, b) => self.add[self.eval(a, env)][self.eval(b, env)],
            Term::Inv(a) => self.inv[self.eval(a, env)],
            Term::Neg(a) => self.neg[self.eval(a, env)],
        }
    }
    pub fn commutative(&self) -> bool {
        let n = self.size();
        (0..n).all(|a| (0..n).all(|b| self.mul[a][b] == self.mul[b][a]))
    }
}

/// Z/n under addition, written multiplicatively.
fn cyclic(n: usize) -> Model {
    let names = (0..n).map(|i| i.to_string()).collect();
    let mul = (0..n).map(|a| (0..n).map(|b| (a + b) % n).collect()).collect();
    let mut m = Model::group(&format!("Z/{n}"), names, mul);
    m.gloss = format!(" (whole numbers mod {n} under +, so e = 0)");
    m
}

/// S3: permutations of 1, 2, 3; ab means b first, then a.
fn s3() -> Model {
    let perms: [[usize; 3]; 6] = [[0, 1, 2], [1, 0, 2], [2, 1, 0], [0, 2, 1], [1, 2, 0], [2, 0, 1]];
    let names = ["e", "(1 2)", "(1 3)", "(2 3)", "(1 2 3)", "(1 3 2)"].map(String::from).to_vec();
    let find = |p: [usize; 3]| perms.iter().position(|q| *q == p).expect("closed");
    let mul = perms.iter().map(|a| perms.iter().map(|b| find([a[b[0]], a[b[1]], a[b[2]]])).collect()).collect();
    let mut m = Model::group("S3", names, mul);
    m.gloss = " (permutations of 1, 2, 3; ab is b, then a)".into();
    m
}

/// The quaternion group: +-1, +-i, +-j, +-k.
fn q8() -> Model {
    // unit index 0..4 = 1, i, j, k; element = 2 * unit + (1 if negative)
    let names = ["1", "-1", "i", "-i", "j", "-j", "k", "-k"].map(String::from).to_vec();
    // unit products: (sign, unit)
    let table: [[(bool, usize); 4]; 4] = [
        [(false, 0), (false, 1), (false, 2), (false, 3)],
        [(false, 1), (true, 0), (false, 3), (true, 2)],
        [(false, 2), (true, 3), (true, 0), (false, 1)],
        [(false, 3), (false, 2), (true, 1), (true, 0)],
    ];
    let mul = (0..8)
        .map(|a| {
            (0..8)
                .map(|b| {
                    let (s, u) = table[a / 2][b / 2];
                    let neg = s ^ (a % 2 == 1) ^ (b % 2 == 1);
                    2 * u + usize::from(neg)
                })
                .collect()
        })
        .collect();
    Model::group("Q8", names, mul)
}

fn product(g: &Model, h: &Model) -> Model {
    let (m, n) = (g.size(), h.size());
    let names = (0..m * n).map(|k| format!("({}, {})", g.names[k / n], h.names[k % n])).collect();
    let mul = (0..m * n).map(|a| (0..m * n).map(|b| g.mul[a / n][b / n] * n + h.mul[a % n][b % n]).collect()).collect();
    let mut m = Model::group(&format!("{} x {}", g.name, h.name), names, mul);
    m.gloss = format!(" (pairs, multiplied in each place; e = ({}, {}))", g.names[g.e], h.names[h.e]);
    m
}

/// Z/n as a ring.
fn ring_mod(n: usize) -> Model {
    let names = (0..n).map(|i| i.to_string()).collect();
    let add = (0..n).map(|a| (0..n).map(|b| (a + b) % n).collect()).collect();
    let mul = (0..n).map(|a| (0..n).map(|b| (a * b) % n).collect()).collect();
    Model::ring(&format!("Z/{n}"), names, add, mul)
}

/// 2x2 matrices over Z/2: not commutative.
fn matrices() -> Model {
    // bits: a b / c d  -> index a*8 + b*4 + c*2 + d
    let get = |k: usize| [(k >> 3) & 1, (k >> 2) & 1, (k >> 1) & 1, k & 1];
    let put = |m: [usize; 4]| m[0] * 8 + m[1] * 4 + m[2] * 2 + m[3];
    let names = (0..16).map(|k| {
        let m = get(k);
        format!("[{} {}; {} {}]", m[0], m[1], m[2], m[3])
    });
    let add = (0..16).map(|a| (0..16).map(|b| a ^ b).collect()).collect();
    let mul = (0..16)
        .map(|a| {
            (0..16)
                .map(|b| {
                    let (x, y) = (get(a), get(b));
                    put([(x[0] * y[0] + x[1] * y[2]) % 2, (x[0] * y[1] + x[1] * y[3]) % 2, (x[2] * y[0] + x[3] * y[2]) % 2, (x[2] * y[1] + x[3] * y[3]) % 2])
                })
                .collect()
        })
        .collect();
    Model::ring("2x2 matrices over Z/2", names.collect(), add, mul)
}

/// The test structures for a kind of statement, smallest first.
pub fn models(kind: Kind) -> Vec<Model> {
    match kind {
        Kind::Group => {
            let z2 = cyclic(2);
            vec![cyclic(4), cyclic(5), product(&z2, &z2), s3(), q8(), product(&z2, &s3())]
        }
        Kind::Abelian => models(Kind::Group).into_iter().filter(Model::commutative).collect(),
        Kind::Ring => vec![ring_mod(4), ring_mod(6), matrices()],
    }
}

/// What trying a statement in the models found.
pub struct Verdict {
    /// (model, assignments where the hypotheses hold)
    pub held: Vec<(String, usize)>,
    /// Models where the hypotheses never hold.
    pub vacuous: Vec<String>,
    /// Models with too many assignments to try.
    pub skipped: Vec<String>,
    pub counterexample: Option<String>,
}

fn show(t: &Term) -> String {
    super::term::show(t, crate::print::Style::Ascii)
}

fn free_letters(st: &Statement, goal: &Eqn) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    let mut push = |e: &Eqn| {
        for x in e.l.letters().into_iter().chain(e.r.letters()) {
            if !e.all.contains(&x) && !v.contains(&x) {
                v.push(x);
            }
        }
    };
    for h in &st.hyps {
        push(h);
    }
    push(goal);
    v.sort();
    v
}

/// Does `e` hold under `env`, for every value of its "for all" letters?
fn holds(m: &Model, e: &Eqn, env: &[(String, usize)]) -> bool {
    let n = m.size();
    let mut bound = vec![0usize; e.all.len()];
    loop {
        let look = |v: &str| e.all.iter().position(|a| a == v).map(|k| bound[k]).or_else(|| env.iter().find(|(x, _)| x == v).map(|p| p.1)).unwrap_or(0);
        if m.eval(&e.l, &look) != m.eval(&e.r, &look) {
            return false;
        }
        let mut k = 0;
        while k < bound.len() {
            bound[k] += 1;
            if bound[k] < n {
                break;
            }
            bound[k] = 0;
            k += 1;
        }
        if k == bound.len() {
            return true;
        }
    }
}

fn shown_eqn(e: &Eqn) -> String {
    let all = if e.all.is_empty() { String::new() } else { format!(" for every {}", e.all.join(" and ")) };
    format!("{} = {}{all}", show(&e.l), show(&e.r))
}

/// Try `hyps => goal` in every model of the kind, every assignment.
pub fn check(kind: Kind, st: &Statement, goal: &Eqn, max: usize) -> Verdict {
    let letters = free_letters(st, goal);
    let mut v = Verdict { held: vec![], vacuous: vec![], skipped: vec![], counterexample: None };
    for m in models(kind) {
        let n = m.size();
        if n.checked_pow(letters.len() as u32).is_none_or(|c| c > max) {
            v.skipped.push(m.name.clone());
            continue;
        }
        let mut idx = vec![0usize; letters.len()];
        let mut count = 0;
        loop {
            let env: Vec<(String, usize)> = letters.iter().cloned().zip(idx.iter().copied()).collect();
            if st.hyps.iter().all(|h| holds(&m, h, &env)) {
                count += 1;
                if !holds(&m, goal, &env) {
                    let look = |x: &str| env.iter().find(|(y, _)| y == x).map_or(0, |p| p.1);
                    let at: Vec<String> = env.iter().map(|(x, k)| format!("{x} = {}", m.names[*k])).collect();
                    let hyps: Vec<String> = st.hyps.iter().map(shown_eqn).collect();
                    let given = if hyps.is_empty() { String::new() } else { format!("{} holds, but ", hyps.join(" and ")) };
                    let (l, r) = (m.eval(&goal.l, &look), m.eval(&goal.r, &look));
                    let at = if at.is_empty() { String::new() } else { format!(", {}", at.join(", ")) };
                    // a side with no letters is just named: "a = 2, not 0"
                    let side = |t: &Term, k: usize| if show(t) == m.names[k] { show(t) } else { format!("{} = {}", show(t), m.names[k]) };
                    let differ = match (goal.l.letters().is_empty(), goal.r.letters().is_empty()) {
                        (false, true) => format!("{} = {}, not {}", show(&goal.l), m.names[l], side(&goal.r, r)),
                        (true, false) => format!("{} = {}, not {}", show(&goal.r), m.names[r], side(&goal.l, l)),
                        _ => format!("{} = {} {} {} = {}", show(&goal.l), m.names[l], if hyps.is_empty() { "but" } else { "and" }, show(&goal.r), m.names[r]),
                    };
                    v.counterexample = Some(format!("in {}{}{at}: {given}{differ}", m.name, m.gloss));
                    return v;
                }
            }
            // the last letter moves fastest: a = (1 2), b = (1 3) comes before a = (1 3), b = (1 2)
            let mut k = letters.len();
            while k > 0 {
                idx[k - 1] += 1;
                if idx[k - 1] < n {
                    break;
                }
                idx[k - 1] = 0;
                k -= 1;
            }
            if k == 0 {
                break;
            }
        }
        if count == 0 {
            v.vacuous.push(m.name.clone());
        } else {
            v.held.push((m.name.clone(), count));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tables really are groups and rings: the axioms hold everywhere.
    #[test]
    fn tables_satisfy_the_axioms() {
        for m in models(Kind::Group) {
            let n = m.size();
            for a in 0..n {
                assert_eq!(m.mul[m.e][a], a, "{}", m.name);
                assert_eq!(m.mul[a][m.inv[a]], m.e, "{}", m.name);
                for b in 0..n {
                    for c in 0..n {
                        assert_eq!(m.mul[m.mul[a][b]][c], m.mul[a][m.mul[b][c]], "{}", m.name);
                    }
                }
            }
        }
        for m in models(Kind::Ring) {
            let n = m.size();
            for a in 0..n {
                assert_eq!(m.add[a][m.neg[a]], m.zero);
                for b in 0..n {
                    assert_eq!(m.add[a][b], m.add[b][a]);
                    for c in 0..n {
                        assert_eq!(m.mul[m.mul[a][b]][c], m.mul[a][m.mul[b][c]], "{}", m.name);
                        assert_eq!(m.mul[a][m.add[b][c]], m.add[m.mul[a][b]][m.mul[a][c]], "{}", m.name);
                        assert_eq!(m.mul[m.add[a][b]][c], m.add[m.mul[a][c]][m.mul[b][c]], "{}", m.name);
                    }
                }
            }
        }
        assert!(!models(Kind::Group).iter().all(Model::commutative));
        assert!(!models(Kind::Ring).iter().all(Model::commutative));
    }
}
