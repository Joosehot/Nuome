//! The letter cancelled out of an inequality: 0 < 5 holds for every x,
//! 3 < 1 for none.

use super::{Cx, Line, Move, Rule};
use crate::expr::Math;

pub struct IneqVerdict;

impl Rule for IneqVerdict {
    fn name(&self) -> &'static str {
        "ineq_verdict"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["never", "always"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        let Math::Ineq(l, rel, r) = m else { return vec![] };
        let (Some(a), Some(b)) = (l.as_num(), r.as_num()) else { return vec![] };
        let (variant, result, how) = if rel.holds_q(&a, &b) { ("always", Math::AllReals, format!("is always true, whatever {v} is.")) } else { ("never", Math::NoSolution, format!("is never true, so no {v} works.")) };
        let says = Line::new().m(m).t(" ").t(how);
        vec![Move { rule: "ineq_verdict", variant, result, says, work: vec![] }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{num, Rel};
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn decides_letter_free_inequalities() {
        let req = test_cx_req("solve x < 1");
        let cfg = crate::config::Config::builtin();
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        assert_eq!(IneqVerdict.moves(&Math::Ineq(num(0), Rel::Lt, num(5)), &cx)[0].result, Math::AllReals);
        assert_eq!(IneqVerdict.moves(&Math::Ineq(num(3), Rel::Lt, num(1)), &cx)[0].result, Math::NoSolution);
        assert!(IneqVerdict.moves(&req.start(), &cx).is_empty());
    }
}
