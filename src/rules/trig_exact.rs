//! Exact trig values at the special angles: multiples of pi/6 and pi/4 (30
//! and 45 degrees). Beyond the first quadrant: the value at the reference
//! angle, with the sign of the quadrant (sin is positive in I and II, cos
//! in I and IV, tan in I and III).

use super::{local, Cx, Line, Move, Rewrite, Rule};
use crate::expr::{self, Expr, Func, Konst, Math};
use crate::q::Q;

pub struct TrigExact;

/// An angle as a multiple of pi: pi/6 -> 1/6, 45 deg -> 1/4, 0 -> 0.
pub fn turns(e: &Expr) -> Option<Q> {
    match e {
        Expr::Num(q) if q.is_zero() => Some(Q::ZERO),
        Expr::Const(Konst::Pi) => Some(Q::ONE),
        Expr::Const(Konst::Deg) => Q::new(1, 180),
        Expr::Mul(v) => {
            let unit: Vec<&Expr> = v.iter().filter(|f| f.as_num().is_none()).collect();
            if unit.len() != 1 {
                return None;
            }
            let t = turns(unit[0]).filter(|t| !t.is_zero())?;
            v.iter().filter_map(|f| f.as_num()).try_fold(t, |a, n| a.mul(&n))
        }
        Expr::Div(a, b) => turns(a)?.div(&b.as_num()?),
        Expr::Neg(a) => Some(turns(a)?.neg()),
        Expr::Add(v) => v.iter().try_fold(Q::ZERO, |acc, t| acc.add(&turns(t)?)),
        _ => None,
    }
}

/// The value of a whole or half square root over a number: sqrt(3)/2, 2sqrt(3)/3.
fn surd(k: i128, root: i128, den: i128) -> Expr {
    let top = if root == 1 {
        expr::num(k)
    } else if k == 1 {
        expr::sqrt(expr::num(root))
    } else {
        Expr::Mul(vec![expr::num(k), expr::sqrt(expr::num(root))])
    };
    if den == 1 {
        top
    } else if root == 1 {
        Expr::Num(Q::new(k, den).expect("nonzero"))
    } else {
        expr::div(top, expr::num(den))
    }
}

/// f at a first-quadrant special angle (0, 1/6, 1/4, 1/3, 1/2 of pi); None
/// where f is undefined.
fn first_quadrant(f: Func, a: Q) -> Option<Expr> {
    let i = [Q::ZERO, Q::new(1, 6)?, Q::new(1, 4)?, Q::new(1, 3)?, Q::new(1, 2)?].iter().position(|x| *x == a)?;
    // (k, root, den) per angle; None = undefined
    let t: [Option<(i128, i128, i128)>; 5] = match f {
        Func::Sin => [Some((0, 1, 1)), Some((1, 1, 2)), Some((1, 2, 2)), Some((1, 3, 2)), Some((1, 1, 1))],
        Func::Cos => [Some((1, 1, 1)), Some((1, 3, 2)), Some((1, 2, 2)), Some((1, 1, 2)), Some((0, 1, 1))],
        Func::Tan => [Some((0, 1, 1)), Some((1, 3, 3)), Some((1, 1, 1)), Some((1, 3, 1)), None],
        Func::Cot => [None, Some((1, 3, 1)), Some((1, 1, 1)), Some((1, 3, 3)), Some((0, 1, 1))],
        Func::Sec => [Some((1, 1, 1)), Some((2, 3, 3)), Some((1, 2, 1)), Some((2, 1, 1)), None],
        Func::Csc => [None, Some((2, 1, 1)), Some((1, 2, 1)), Some((2, 3, 3)), Some((1, 1, 1))],
        _ => return None,
    };
    let (k, root, den) = t[i]?;
    Some(surd(k, root, den))
}

/// Is f positive in quadrant q (1..=4)?
fn positive(f: Func, q: u8) -> bool {
    match f {
        Func::Sin | Func::Csc => q <= 2,
        Func::Cos | Func::Sec => q == 1 || q == 4,
        _ => q == 1 || q == 3,
    }
}

/// f(angle) exactly, with the reference angle and quadrant when outside
/// the first quadrant: (value, Some((reference, quadrant))).
pub fn exact(f: Func, t: Q) -> Option<(Expr, Option<(Q, u8)>)> {
    let two = Q::int(2);
    // t mod 2
    let k = t.div(&two)?;
    let fl = Q::int(k.num().div_euclid(k.den()));
    let r = t.sub(&fl.mul(&two)?)?;
    let half = Q::new(1, 2)?;
    if r <= half {
        return Some((first_quadrant(f, r)?, None));
    }
    // quadrant and reference angle; the axes (pi, 3pi/2) fall in with q2, q3
    let (q, reference) = if r <= Q::ONE {
        (2, Q::ONE.sub(&r)?)
    } else if r <= Q::new(3, 2)? {
        (3, r.sub(&Q::ONE)?)
    } else {
        (4, two.sub(&r)?)
    };
    let v = first_quadrant(f, reference)?;
    let sign = if v.is_num(0) || positive(f, q) { v } else { expr::neg(v) };
    Some((sign, Some((reference, q))))
}

/// An angle of t turns of pi, written in radians (5pi/6) or degrees (150 deg).
pub fn angle(t: Q, degrees: bool) -> Expr {
    if degrees {
        return expr::mul(vec![Expr::Num(t.mul(&Q::int(180)).unwrap_or(t)), Expr::Const(Konst::Deg)]);
    }
    if t.is_zero() {
        return expr::num(0);
    }
    let top = expr::with_coeff(Q::int(t.num()), Expr::Const(Konst::Pi));
    if t.den() == 1 {
        top
    } else {
        expr::div(top, expr::num(t.den()))
    }
}

impl Rule for TrigExact {
    fn name(&self) -> &'static str {
        "trig_exact"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["special_angle", "reference_angle"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        let degrees = cx.req.calc.degrees.is_some();
        local("trig_exact", m, |e, _| {
            let Expr::Func(f, a) = e else { return vec![] };
            let Some(t) = turns(a) else { return vec![] };
            let Some((value, reference)) = exact(*f, t) else { return vec![] };
            let says = Line::new().t("Exact value: ").e(e).t(" = ").e(&value).t(".");
            // on an axis (pi, 3pi/2) the value is read straight off the unit circle
            let on_axis = reference.is_some_and(|(r, _)| r.is_zero() || r == Q::new(1, 2).unwrap_or(Q::ONE));
            match reference.filter(|_| !on_axis) {
                None => vec![Rewrite { variant: "special_angle", new: value, says, work: vec![] }],
                Some((r, q)) => {
                    let quadrant = ["first", "second", "third", "fourth"][(q - 1) as usize];
                    let sign = if positive(*f, q) { "positive" } else { "negative" };
                    let work = vec![Line::new().t("reference angle ").e(&angle(r, degrees)).t(format!("; {} is {sign} in the {quadrant} quadrant", f.name()))];
                    vec![Rewrite { variant: "reference_angle", new: value, says, work }]
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_moves;

    #[test]
    fn special_angles() {
        assert_eq!(test_moves(&TrigExact, "what is sin(pi/6)"), vec!["1/2"]);
        assert_eq!(test_moves(&TrigExact, "what is cos(45 degrees)"), vec!["sqrt(2)/2"]);
        assert_eq!(test_moves(&TrigExact, "what is cos(5pi/6)"), vec!["-sqrt(3)/2"]);
        assert_eq!(test_moves(&TrigExact, "what is tan(pi/3)"), vec!["sqrt(3)"]);
        assert!(test_moves(&TrigExact, "what is tan(pi/2)").is_empty());
        assert!(test_moves(&TrigExact, "what is sin(1)").is_empty());
    }
}
