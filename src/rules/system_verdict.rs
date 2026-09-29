//! Every letter cancelled out of an equation of a system: 0 = 4 means the
//! equations contradict each other (no solution); 0 = 0 means that
//! equation followed from the others and says nothing new (it is dropped).

use super::eliminate::{form, letters, used};
use super::{Cx, Line, Move, Rule};
use crate::expr::Math;

pub struct SystemVerdict;

impl Rule for SystemVerdict {
    fn name(&self) -> &'static str {
        "system_verdict"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["never", "dependent"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let Math::System(eqs) = m else { return vec![] };
        let vars = letters(cx);
        for (i, e) in eqs.iter().enumerate() {
            let Some(f) = form(e, &vars) else { continue };
            if used(&f) != 0 {
                continue;
            }
            if !f.1.is_zero() {
                let says = Line::new().m(&Math::Eq(e.0.clone(), e.1.clone())).t(" is never true: the equations contradict each other.");
                return vec![Move { rule: "system_verdict", variant: "never", result: Math::NoSolution, says, work: vec![] }];
            }
            let mut next = eqs.clone();
            next.remove(i);
            let says = Line::new().m(&Math::Eq(e.0.clone(), e.1.clone())).t(" always holds: that equation follows from the others, so drop it.");
            return vec![Move { rule: "system_verdict", variant: "dependent", result: Math::System(next), says, work: vec![] }];
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{add, num, var};
    use crate::rules::{test_cx_req, Cx};

    #[test]
    fn spots_contradictions_and_repeats() {
        let cfg = crate::config::Config::builtin();
        let req = test_cx_req("solve x + y = 3 and 2x + 2y = 7");
        let cx = Cx { req: &req, cfg: &cfg, var: "x" };
        let first = (add(vec![var("x"), var("y")]), num(3));
        assert_eq!(SystemVerdict.moves(&Math::System(vec![first.clone(), (num(0), num(1))]), &cx)[0].result, Math::NoSolution);
        assert_eq!(SystemVerdict.moves(&Math::System(vec![first.clone(), (num(0), num(0))]), &cx)[0].result, Math::System(vec![first]));
        assert!(SystemVerdict.moves(&req.start(), &cx).is_empty());
    }
}
