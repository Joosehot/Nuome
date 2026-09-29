//! The letter cancelled out: 1 = 3 has no solution, 3 = 3 holds for every x.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::Math;

pub struct Verdict;

impl Rule for Verdict {
    fn name(&self) -> &'static str {
        "verdict"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["never", "always"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var.to_string();
        per_eq("verdict", m, |l, r| {
            if l.has_var(&v) || r.has_var(&v) {
                return vec![];
            }
            let (Some(a), Some(b)) = (l.as_num(), r.as_num()) else { return vec![] };
            if a == b {
                vec![EqRewrite { variant: "always", to: Branch::Everything, says: Line::new().t(format!("{a} = {b} is always true, whatever {v} is.")), work: vec![] }]
            } else {
                vec![EqRewrite { variant: "never", to: Branch::Nothing, says: Line::new().t(format!("{a} = {b} is never true, so no {v} works.")), work: vec![] }]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{num, Math};
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn decides_letter_free_equations() {
        let req = test_cx_req("solve x = 1");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        assert_eq!(Verdict.moves(&Math::Eq(num(1), num(3)), &cx)[0].result, Math::NoSolution);
        assert_eq!(Verdict.moves(&Math::Eq(num(3), num(3)), &cx)[0].result, Math::AllReals);
        assert!(Verdict.moves(&req.start(), &cx).is_empty());
    }
}
