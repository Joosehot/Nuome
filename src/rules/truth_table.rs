//! Proof by truth table (logic and sets, agent L): with few letters, write
//! down every row, one for each way of making the letters true or false, and
//! read off that the statement holds in all of them. For sets it is the
//! membership table: one row per region of the Venn diagram, 1 where the
//! region lies inside the set. The rows are all the cases there are, so the
//! table is itself a proof. It is offered only for the statement as asked,
//! and only when it really holds in every row.

use super::{Cx, Line, Move, Rule};
use crate::expr::{Expr, Math};
use crate::logic;

pub struct TruthTable;

impl Rule for TruthTable {
    fn name(&self) -> &'static str {
        "truth_table"
    }
    fn variants(&self) -> &'static [&'static str] {
        &["truth", "membership"]
    }
    fn moves(&self, m: &Math, cx: &Cx) -> Vec<Move> {
        if !logic::applies("truth_table", m, cx) || *m != cx.req.start() {
            return vec![];
        }
        let names = logic::letters(m);
        let n = names.len();
        if n == 0 || n > cx.cfg.logic.table_letters {
            return vec![];
        }
        let sets = matches!(m, Math::Eq(..) | Math::Subset(..));
        // the columns: the statement, or its two sides
        let cols: Vec<(&str, &Expr)> = match m {
            Math::Taut(e) => vec![("S", e)],
            Math::Equiv(l, r) | Math::Eq(l, r) | Math::Subset(l, r) | Math::Entails(l, r) => vec![("L", l), ("R", r)],
            _ => return vec![],
        };
        let mark = |b: bool| match (sets, b) {
            (true, true) => "1",
            (true, false) => "0",
            (false, true) => "T",
            (false, false) => "F",
        };
        let rows = 1usize << n;
        let mut work: Vec<Line> = Vec::new();
        let mut legend = Line::new();
        for (k, (name, e)) in cols.iter().enumerate() {
            legend = legend.t(if k == 0 { format!("{name} = ") } else { format!(",  {name} = ") }).e(e);
        }
        work.push(legend);
        let head: Vec<&str> = names.iter().map(|s| s.as_str()).chain(std::iter::once("|")).chain(cols.iter().map(|c| c.0)).collect();
        work.push(Line::new().t(head.join("  ")));
        for k in 0..rows {
            let vals = logic::row(n, k);
            if logic::at(m, &names, &vals) != Some(true) {
                return vec![];
            }
            let mut cells: Vec<&str> = vals.iter().map(|b| mark(*b)).collect();
            cells.push("|");
            for (_, e) in &cols {
                cells.push(mark(logic::value_at(e, &names, &vals) == Some(true)));
            }
            work.push(Line::new().t(cells.join("  ")));
        }
        let says = match m {
            Math::Taut(_) => format!("Truth table: S is T in all {rows} rows."),
            Math::Equiv(..) => format!("Truth table: L and R agree in all {rows} rows."),
            Math::Eq(..) => format!("Membership table, one row per region of the Venn diagram: L and R contain the same regions, in all {rows} rows."),
            _ if sets => format!("Membership table, one row per region of the Venn diagram: every region in L is in R, in all {rows} rows."),
            _ => format!("Truth table: wherever L is T, R is T too, in all {rows} rows."),
        };
        vec![Move { rule: "truth_table", variant: if sets { "membership" } else { "truth" }, result: Math::Proved, says: Line::new().t(says), work }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::test_moves;

    #[test]
    fn every_row_holds() {
        assert_eq!(test_moves(&TruthTable, "prove not (p and q) is equivalent to not p or not q"), vec!["proved"]);
        assert_eq!(test_moves(&TruthTable, "prove A ∩ B ⊆ A"), vec!["proved"]);
        // four letters: too many rows to write out as the proof
        assert!(test_moves(&TruthTable, "prove (p and q) or (r and s) -> p or r").is_empty());
    }
}
