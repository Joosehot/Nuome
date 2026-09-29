//! A product is zero only when a factor is: (x - 2)(x - 3) = 0 ->
//! x - 2 = 0 or x - 3 = 0. Number factors can't be zero and drop out.

use super::{per_eq, Branch, Cx, EqRewrite, Line, Move, Rule};
use crate::expr::{self, Expr, Math};

pub struct ZeroProduct;

impl Rule for ZeroProduct {
    fn name(&self) -> &'static str {
        "zero_product"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["split", "power"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let v = cx.var;
        per_eq("zero_product", m, |l, r| {
            if !r.is_num(0) {
                return vec![];
            }
            let factors: Vec<Expr> = match l {
                Expr::Mul(f) => f.clone(),
                Expr::Neg(a) => match &**a {
                    Expr::Mul(f) => f.clone(),
                    a => vec![a.clone()],
                },
                Expr::Pow(..) => vec![l.clone()],
                _ => return vec![],
            };
            let mut parts: Vec<Expr> = Vec::new();
            let mut had_power = false;
            for f in factors.iter().filter(|f| f.has_var(v)) {
                let f = match f {
                    Expr::Pow(b, n) if n.as_num().is_some_and(|q| q.is_int() && !q.is_neg() && !q.is_zero()) => {
                        had_power = true;
                        (**b).clone()
                    }
                    f => f.clone(),
                };
                if !parts.contains(&f) {
                    parts.push(f);
                }
            }
            if parts.is_empty() || (parts.len() == 1 && !had_power && factors.iter().filter(|f| f.has_var(v)).count() == factors.len()) {
                return vec![];
            }
            let to = if parts.len() == 1 { Branch::One(parts[0].clone(), expr::num(0)) } else { Branch::Many(parts.iter().map(|p| (p.clone(), expr::num(0))).collect()) };
            let (variant, says) = if parts.len() == 1 && had_power {
                ("power", Line::new().t("A power is 0 only when its base is 0."))
            } else {
                ("split", Line::new().t("A product is 0 only when one of its factors is 0."))
            };
            vec![EqRewrite { variant, to, says, work: vec![] }]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn splits_products() {
        assert_eq!(test_moves(&ZeroProduct, "solve (x - 2)(x - 3) = 0"), vec!["x - 2 = 0 or x - 3 = 0"]);
        assert_eq!(test_moves(&ZeroProduct, "solve 3(x - 2) = 0"), vec!["x - 2 = 0"]);
        assert_eq!(test_moves(&ZeroProduct, "solve (x + 1)^2 = 0"), vec!["x + 1 = 0"]);
        assert!(test_moves(&ZeroProduct, "solve (x - 2)(x - 3) = 6").is_empty());
    }
}
