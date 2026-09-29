//! Products of brackets: (a + b)(c + d) -> ac + ad + bc + bd. A square of a
//! sum has two variants: the identity (a + b)^2 = a^2 + 2ab + b^2, or write
//! it as a product and multiply out.

use super::distribute::times;
use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, terms, Expr, Math};
use crate::model::Task;

pub struct MultiplyOut;

impl Rule for MultiplyOut {
    fn name(&self) -> &'static str {
        "multiply_out"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["each_by_each", "square_identity", "as_product"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        local("multiply_out", m, |e, at| {
            // in an equation "product = 0" the factored form is what solving wants
            if cx.task() == Task::Solve && at.is_side_against_zero() {
                return vec![];
            }
            // denominators with letters stay factored
            if at.in_letter_denominator() && cx.task() != Task::Expand {
                return vec![];
            }
            match e {
                Expr::Mul(v) => {
                    let sums: Vec<usize> = v.iter().enumerate().filter(|(_, f)| matches!(f, Expr::Add(_))).map(|(i, _)| i).collect();
                    if sums.len() < 2 {
                        return vec![];
                    }
                    let (i, j) = (sums[0], sums[1]);
                    let (a, b) = (terms(&v[i]), terms(&v[j]));
                    let mut prod = Vec::new();
                    for x in &a {
                        for y in &b {
                            prod.push(times(x, y));
                        }
                    }
                    let mut nv = v.clone();
                    nv[i] = expr::add(prod);
                    nv.remove(j);
                    let says = Line::new().t("Multiply each term of ").e(&v[i]).t(" by each term of ").e(&v[j]).t(".");
                    vec![Rewrite { variant: "each_by_each", new: expr::mul(nv), says, work: vec![] }]
                }
                Expr::Pow(base, n) => {
                    let Expr::Add(ts) = &**base else { return vec![] };
                    let Some(n) = n.as_num().filter(|n| n.is_int() && n.num() >= 2 && n.num() <= 4) else { return vec![] };
                    let mut out = Vec::new();
                    // proving an identity with the identity itself would be circular
                    if n.num() == 2 && ts.len() == 2 && cx.task() != Task::Prove {
                        let (a, b) = (&ts[0], &ts[1]);
                        let new = expr::add(vec![times(a, a), times(&expr::num(2), &times(a, b)), times(b, b)]);
                        let says = Line::new().t("Use (a + b)^2 = a^2 + 2ab + b^2 with a = ").e(a).t(", b = ").e(b).t(".");
                        out.push(Rewrite { variant: "square_identity", new, says, work: vec![] });
                    }
                    let copies = vec![(**base).clone(); n.num() as usize];
                    out.push(Rewrite { variant: "as_product", new: Expr::Mul(copies.clone()), says: Line::new().t("Write ").e(e).t(" as ").e(&Expr::Mul(copies)).t("."), work: vec![] });
                    out
                }
                _ => vec![],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn multiplies_brackets() {
        assert_eq!(test_moves(&MultiplyOut, "expand (x + 2)(x - 3)"), vec!["x^2 - 3x + 2x - 6"]);
        assert_eq!(test_moves(&MultiplyOut, "expand (x + 1)^2"), vec!["x^2 + 2x + 1", "(x + 1)(x + 1)"]);
        assert!(test_moves(&MultiplyOut, "solve (x + 2)(x - 3) = 0").is_empty());
        assert!(test_moves(&MultiplyOut, "expand 3(x + 1)").is_empty());
    }
}
