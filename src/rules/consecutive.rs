//! A product of k consecutive integers is divisible by k!: it is k! times a
//! binomial coefficient. So (n - 1)n(n + 1) is always a multiple of 6.

use super::{Cx, Line, Move, Rule};
use crate::calls::Named;
use crate::expr::{Expr, Math};

pub struct Consecutive;

impl Rule for Consecutive {
    fn name(&self) -> &'static str {
        "consecutive"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["factorial_divides"]
    }
    fn moves(&self, m: &Math, _cx: &Cx) -> Vec<Move> {
        let Math::Expr(Expr::Call(Named::Divides, a)) = m else { return vec![] };
        let [d, Expr::Mul(fs)] = &a[..] else { return vec![] };
        let Some(d) = d.as_num().filter(|q| q.is_int()).map(|q| q.num()) else { return vec![] };
        let mut c = 1i128;
        let mut shifts = Vec::new();
        let mut letter = None;
        for f in fs {
            if let Some(q) = f.as_num() {
                if !q.is_int() {
                    return vec![];
                }
                c *= q.num();
                continue;
            }
            // n + a with coefficient 1 on n
            let ts = crate::expr::terms(f);
            let (mut shift, mut seen) = (0i128, None);
            for t in &ts {
                match t {
                    Expr::Var(v) => seen = Some(v.clone()),
                    t => match t.as_num().filter(|q| q.is_int()) {
                        Some(q) => shift += q.num(),
                        None => return vec![],
                    },
                }
            }
            let Some(v) = seen else { return vec![] };
            if letter.get_or_insert(v.clone()) != &v {
                return vec![];
            }
            shifts.push(shift);
        }
        shifts.sort();
        let k = shifts.len() as i128;
        if k < 2 || shifts.windows(2).any(|w| w[1] != w[0] + 1) {
            return vec![];
        }
        let kf: i128 = (1..=k).product();
        if (c * kf) % d != 0 {
            return vec![];
        }
        let says = Line::new().t(format!("{k} consecutive integers: their product is divisible by {k}! = {kf}, and {d} divides {}.", c * kf));
        let work = vec![Line::new().t(format!("Among any {k} consecutive integers there is a multiple of each of 2, ..., {k}; exactly, their product is {k}! times a binomial coefficient."))];
        vec![Move { rule: "consecutive", variant: "factorial_divides", result: Math::Proved, says, work }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{add, num, var};
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn consecutive_products() {
        let req = test_cx_req("prove 6 divides n^3 - n");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "n" };
        let prod = Expr::Mul(vec![add(vec![var("n"), num(-1)]), var("n"), add(vec![var("n"), num(1)])]);
        let m = Math::Expr(Expr::Call(Named::Divides, vec![num(6), prod.clone()]));
        assert_eq!(Consecutive.moves(&m, &cx)[0].result, Math::Proved);
        let m = Math::Expr(Expr::Call(Named::Divides, vec![num(12), prod]));
        assert!(Consecutive.moves(&m, &cx).is_empty());
    }
}
