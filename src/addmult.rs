//! Additive-multiplicative magic squares: every row, column and both main
//! diagonals have the same sum S and the same product P, all entries
//! distinct positive integers. Boyer's enigma #6a asks for a 6 x 6 one
//! (500 euros and a bottle of champagne); 7 x 7 is the smallest known
//! (Miquel 2016), 3 x 3 and 4 x 4 are impossible, 5 x 5 and 6 x 6 are open.
//! Morgenstern (2007) showed there is no 6 x 6 one with largest entry < 136.
//!
//! The search is Miquel's: choose a magic product P, take its divisors up
//! to a bound M, list every 6-set of them with product P (a "line"), group
//! the lines by their sum, and for each sum S build the square from lines:
//! the smallest entry m sits where row 0 meets column 0; the other five
//! entries of column 0 each start a row; the rows fix the 36 entries; the
//! other five entries of row 0 each start a column, which must meet every
//! row exactly once. That is a semi-magic square. Then two disjoint lines
//! that meet every row and column once, arranged as the two diagonals.
//! Every square found is checked from scratch before it is reported.

use std::collections::HashMap;

const W: usize = 16; // bitset words: up to 1024 divisors
type Mask = [u64; W];

fn and(a: &Mask, b: &Mask) -> Mask {
    std::array::from_fn(|i| a[i] & b[i])
}
fn or(a: &Mask, b: &Mask) -> Mask {
    std::array::from_fn(|i| a[i] | b[i])
}
fn count(a: &Mask) -> u32 {
    a.iter().map(|w| w.count_ones()).sum()
}
fn disjoint(a: &Mask, b: &Mask) -> bool {
    a.iter().zip(b).all(|(x, y)| x & y == 0)
}
fn subset(a: &Mask, b: &Mask) -> bool {
    a.iter().zip(b).all(|(x, y)| x & !y == 0)
}

/// Is `sq` (n x n, row by row) additive-multiplicative magic with distinct
/// positive entries? Exact (products in u128, then checked against each other).
pub fn is_addmult(sq: &[u64], n: usize) -> bool {
    if sq.len() != n * n || sq.iter().any(|&v| v == 0) {
        return false;
    }
    let mut seen = sq.to_vec();
    seen.sort();
    seen.dedup();
    if seen.len() != n * n {
        return false;
    }
    let mut lines: Vec<Vec<u64>> = Vec::new();
    for i in 0..n {
        lines.push((0..n).map(|j| sq[i * n + j]).collect());
        lines.push((0..n).map(|j| sq[j * n + i]).collect());
    }
    lines.push((0..n).map(|i| sq[i * n + i]).collect());
    lines.push((0..n).map(|i| sq[i * n + n - 1 - i]).collect());
    let key = |l: &Vec<u64>| -> Option<(u64, u128)> {
        let mut p: u128 = 1;
        for &v in l {
            p = p.checked_mul(v as u128)?;
        }
        Some((l.iter().sum(), p))
    };
    let Some(first) = key(&lines[0]) else { return false };
    lines.iter().all(|l| key(l) == Some(first))
}

/// How many of the 2n + 2 lines have the sum S and the product P of the
/// first row (for near misses).
pub fn score(sq: &[u64], n: usize) -> (usize, usize) {
    let mut lines: Vec<Vec<u64>> = Vec::new();
    for i in 0..n {
        lines.push((0..n).map(|j| sq[i * n + j]).collect());
        lines.push((0..n).map(|j| sq[j * n + i]).collect());
    }
    lines.push((0..n).map(|i| sq[i * n + i]).collect());
    lines.push((0..n).map(|i| sq[i * n + n - 1 - i]).collect());
    let s: u64 = sq[..n].iter().sum();
    let p: u128 = sq[..n].iter().map(|&v| v as u128).product();
    (
        lines.iter().filter(|l| l.iter().sum::<u64>() == s).count(),
        lines.iter().filter(|l| l.iter().map(|&v| v as u128).product::<u128>() == p).count(),
    )
}

/// The divisors of P (given as prime powers) up to `max`.
pub fn divisors(factors: &[(u64, u32)], max: u64) -> Vec<u64> {
    let mut ds = vec![1u64];
    for &(p, e) in factors {
        let mut next = Vec::new();
        for &d in &ds {
            let mut v = d;
            for _ in 0..=e {
                if v > max {
                    break;
                }
                next.push(v);
                v = match v.checked_mul(p) {
                    Some(x) => x,
                    None => break,
                };
            }
        }
        ds = next;
    }
    ds.sort();
    ds
}

/// Every 6-set of distinct divisors (indices into `ds`, increasing) whose product is `p`.
pub fn lines(ds: &[u64], p: u128, n: usize) -> Vec<Vec<u16>> {
    fn go(ds: &[u64], from: usize, left: u128, k: usize, cur: &mut Vec<u16>, out: &mut Vec<Vec<u16>>) {
        if k == 0 {
            if left == 1 {
                out.push(cur.clone());
            }
            return;
        }
        let max = *ds.last().unwrap() as u128;
        for i in from..ds.len() {
            let d = ds[i] as u128;
            // the k entries are at least d each, increasing: d^k <= left
            let mut low = 1u128;
            for _ in 0..k {
                low = low.saturating_mul(d);
            }
            if low > left {
                break;
            }
            if left % d != 0 {
                continue;
            }
            let rest = left / d;
            // the other k - 1 entries are at most max each
            let mut high = 1u128;
            for _ in 0..k - 1 {
                high = high.saturating_mul(max);
            }
            if rest > high {
                continue;
            }
            cur.push(i as u16);
            go(ds, i + 1, rest, k - 1, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    go(ds, 0, p, n, &mut Vec::new(), &mut out);
    out
}

pub struct Found {
    pub semi: u64,
    /// squares whose 14 lines are all lines of the list (before the sum and product check)
    pub arranged: Vec<Vec<u64>>,
    pub magic: Vec<Vec<u64>>,
    pub best_semi: Option<Vec<u64>>,
}

/// Build 6 x 6 squares (n = 6) with sum `s` from the given lines.
pub fn solve(ds: &[u64], lines: &[Vec<u16>], _n: usize, max_magic: usize, deadline: std::time::Instant) -> Found {
    let masks: Vec<Mask> = lines
        .iter()
        .map(|l| {
            let mut m = [0u64; W];
            for &i in l {
                m[i as usize >> 6] |= 1 << (i & 63);
            }
            m
        })
        .collect();
    let mins: Vec<u16> = lines.iter().map(|l| l[0]).collect();
    let mut by_elem: Vec<Vec<u32>> = vec![Vec::new(); ds.len()];
    for (id, l) in lines.iter().enumerate() {
        for &i in l {
            by_elem[i as usize].push(id as u32);
        }
    }
    let mut found = Found { semi: 0, arranged: Vec::new(), magic: Vec::new(), best_semi: None };
    let elems_of = |m: &Mask| -> Vec<usize> {
        let mut v = Vec::new();
        for w in 0..W {
            let mut x = m[w];
            while x != 0 {
                v.push(w * 64 + x.trailing_zeros() as usize);
                x &= x - 1;
            }
        }
        v
    };
    let mut steps = 0u64;
    for m in 0..ds.len() {
        let starts: Vec<u32> = by_elem[m].iter().copied().filter(|&id| mins[id as usize] as usize == m).collect();
        for (a, &r0) in starts.iter().enumerate() {
            for &c0 in &starts[a + 1..] {
                let (mr0, mc0) = (masks[r0 as usize], masks[c0 as usize]);
                if count(&and(&mr0, &mc0)) != 1 {
                    continue;
                }
                let col0: Vec<usize> = elems_of(&mc0).into_iter().filter(|&e| e != m).collect();
                let row0: Vec<usize> = elems_of(&mr0).into_iter().filter(|&e| e != m).collect();
                // rows through the other entries of column 0
                let mut rows: Vec<u32> = vec![r0];
                let mut used = or(&mr0, &mc0);
                // depth-first over the five rows
                fn rows_dfs(
                    k: usize,
                    col0: &[usize],
                    mc0: &Mask,
                    m: usize,
                    masks: &[Mask],
                    mins: &[u16],
                    by_elem: &[Vec<u32>],
                    rows: &mut Vec<u32>,
                    used: &mut Mask,
                    cb: &mut dyn FnMut(&[u32]) -> bool,
                ) -> bool {
                    if k == col0.len() {
                        return cb(rows);
                    }
                    let c = col0[k];
                    for &id in &by_elem[c] {
                        let mk = &masks[id as usize];
                        if (mins[id as usize] as usize) < m {
                            continue;
                        }
                        // meets column 0 only at c, and no earlier row
                        let rest = {
                            let mut x = *mk;
                            x[c >> 6] &= !(1 << (c & 63));
                            x
                        };
                        if !disjoint(&rest, used) {
                            continue;
                        }
                        if count(&and(mk, mc0)) != 1 {
                            continue;
                        }
                        let saved = *used;
                        *used = or(used, mk);
                        rows.push(id);
                        if rows_dfs(k + 1, col0, mc0, m, masks, mins, by_elem, rows, used, cb) {
                            return true;
                        }
                        rows.pop();
                        *used = saved;
                    }
                    false
                }
                let mut on_rows = |rows: &[u32]| -> bool {
                    steps += 1;
                    let row_masks: Vec<Mask> = rows.iter().map(|&id| masks[id as usize]).collect();
                    let all = row_masks.iter().fold([0u64; W], |a, b| or(&a, b));
                    let meets_rows = |mk: &Mask| row_masks.iter().all(|r| count(&and(mk, r)) == 1);
                    // candidate columns through each other entry of row 0
                    let mut cand: Vec<Vec<u32>> = Vec::new();
                    for &e in &row0 {
                        let c: Vec<u32> = by_elem[e]
                            .iter()
                            .copied()
                            .filter(|&id| {
                                let mk = &masks[id as usize];
                                subset(mk, &all) && disjoint(mk, &mc0) && meets_rows(mk)
                            })
                            .collect();
                        if c.is_empty() {
                            return false;
                        }
                        cand.push(c);
                    }
                    // choose disjoint columns
                    let mut cols: Vec<u32> = vec![c0];
                    fn cols_dfs(k: usize, cand: &[Vec<u32>], masks: &[Mask], cols: &mut Vec<u32>, used: Mask, cb: &mut dyn FnMut(&[u32]) -> bool) -> bool {
                        if k == cand.len() {
                            return cb(cols);
                        }
                        for &id in &cand[k] {
                            let mk = &masks[id as usize];
                            if !disjoint(mk, &used) {
                                continue;
                            }
                            cols.push(id);
                            if cols_dfs(k + 1, cand, masks, cols, or(&used, mk), cb) {
                                return true;
                            }
                            cols.pop();
                        }
                        false
                    }
                    let mut on_cols = |cols: &[u32]| -> bool {
                        found.semi += 1;
                        let col_masks: Vec<Mask> = cols.iter().map(|&id| masks[id as usize]).collect();
                        // the grid: entry at (row i, column j)
                        let cell = |i: usize, j: usize| -> usize { elems_of(&and(&row_masks[i], &col_masks[j]))[0] };
                        let grid: Vec<Vec<usize>> = (0..row_masks.len()).map(|i| (0..col_masks.len()).map(|j| cell(i, j)).collect()).collect();
                        if found.best_semi.is_none() {
                            found.best_semi = Some(grid.iter().flatten().map(|&e| ds[e]).collect());
                        }
                        // transversal lines: meet every row and every column once
                        let trans: Vec<u32> = (0..masks.len() as u32)
                            .filter(|&id| {
                                let mk = &masks[id as usize];
                                subset(mk, &all) && meets_rows(mk) && col_masks.iter().all(|c| count(&and(mk, c)) == 1)
                            })
                            .collect();
                        let n = row_masks.len();
                        let col_of = |mk: &Mask, i: usize| -> usize { (0..n).find(|&j| !disjoint(mk, &and(&row_masks[i], &col_masks[j]))).unwrap() };
                        for (x, &d1) in trans.iter().enumerate() {
                            let m1 = masks[d1 as usize];
                            let c1: Vec<usize> = (0..n).map(|i| col_of(&m1, i)).collect();
                            for &d2 in &trans[x + 1..] {
                                let m2 = masks[d2 as usize];
                                if !disjoint(&m1, &m2) {
                                    continue;
                                }
                                let c2: Vec<usize> = (0..n).map(|i| col_of(&m2, i)).collect();
                                // pi(i) = the row whose d1-column is d2's column in row i; must be an involution
                                let pi: Vec<usize> = (0..n).map(|i| c1.iter().position(|&c| c == c2[i]).unwrap()).collect();
                                if (0..n).all(|i| pi[pi[i]] == i) {
                                    // order rows: pairs (i, pi(i)) go to positions (t, n-1-t)
                                    let mut order = vec![usize::MAX; n];
                                    let mut t = 0;
                                    for i in 0..n {
                                        if order.contains(&i) {
                                            continue;
                                        }
                                        order[t] = i;
                                        order[n - 1 - t] = pi[i];
                                        t += 1;
                                    }
                                    let sq: Vec<u64> = (0..n).flat_map(|r| (0..n).map(|cc| ds[grid[order[r]][c1[order[cc]]]]).collect::<Vec<_>>()).collect();
                                    if found.arranged.len() < 4 {
                                        found.arranged.push(sq.clone());
                                    }
                                    if is_addmult(&sq, n) {
                                        found.magic.push(sq);
                                        if found.magic.len() >= max_magic {
                                            return true;
                                        }
                                    }
                                }
                            }
                        }
                        false
                    };
                    cols_dfs(0, &cand, &masks, &mut cols, mc0, &mut on_cols)
                };
                if rows_dfs(0, &col0, &mc0, m, &masks, &mins, &by_elem, &mut rows, &mut used, &mut on_rows) {
                    return found;
                }
                if std::time::Instant::now() > deadline {
                    return found;
                }
            }
        }
    }
    let _ = steps;
    found
}

/// The lines of a product grouped by sum.
pub fn by_sum(ds: &[u64], lines: Vec<Vec<u16>>) -> HashMap<u64, Vec<Vec<u16>>> {
    let mut g: HashMap<u64, Vec<Vec<u16>>> = HashMap::new();
    for l in lines {
        let s = l.iter().map(|&i| ds[i as usize]).sum();
        g.entry(s).or_default().push(l);
    }
    g
}

pub fn factor(mut n: u128) -> Vec<(u64, u32)> {
    let mut out = Vec::new();
    let mut p = 2u128;
    while p * p <= n {
        let mut e = 0;
        while n % p == 0 {
            n /= p;
            e += 1;
        }
        if e > 0 {
            out.push((p as u64, e));
        }
        p += 1;
    }
    if n > 1 {
        out.push((n as u64, 1));
    }
    out
}

pub fn show(sq: &[u64], n: usize) -> String {
    let w = sq.iter().map(|v| v.to_string().len()).max().unwrap_or(1);
    (0..n).map(|i| format!("      {}\n", (0..n).map(|j| format!("{:>w$}", sq[i * n + j])).collect::<Vec<_>>().join(" "))).collect()
}

/// Search one product P with entries up to `max`, every sum, for at most `seconds`.
pub fn search_product(p: u128, max: u64, seconds: f64, threads: usize) -> (usize, usize, u64, Vec<Vec<u64>>, Option<Vec<u64>>, bool) {
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Mutex;
    let ds = divisors(&factor(p), max);
    if ds.len() > W * 64 {
        return (ds.len(), 0, 0, vec![], None, false);
    }
    let all = lines(&ds, p, 6);
    let n_lines = all.len();
    let mut groups: Vec<(u64, Vec<Vec<u16>>)> = by_sum(&ds, all).into_iter().filter(|(_, v)| v.len() >= 14).collect();
    // most lines first: the best chance
    groups.sort_by_key(|(s, v)| (std::cmp::Reverse(v.len()), *s));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs_f64(seconds);
    let next = AtomicUsize::new(0);
    let semi = AtomicU64::new(0);
    let magic = Mutex::new(Vec::new());
    let best = Mutex::new(None);
    let complete = AtomicUsize::new(0);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= groups.len() || std::time::Instant::now() > deadline {
                    break;
                }
                let f = solve(&ds, &groups[i].1, 6, 4, deadline);
                semi.fetch_add(f.semi, Ordering::Relaxed);
                if std::time::Instant::now() <= deadline {
                    complete.fetch_add(1, Ordering::Relaxed);
                }
                if !f.magic.is_empty() {
                    magic.lock().unwrap().extend(f.magic);
                }
                if let Some(b) = f.best_semi {
                    best.lock().unwrap().get_or_insert(b);
                }
            });
        }
    });
    let all_done = complete.load(Ordering::Relaxed) == groups.len();
    (ds.len(), n_lines, semi.load(Ordering::Relaxed), magic.into_inner().unwrap(), best.into_inner().unwrap(), all_done)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scrambled_grid_is_put_back_with_its_diagonals() {
        // a 6 x 6 grid of 0..36 whose 14 lines are given as sets, rows and columns
        // shuffled: the solver must arrange a square whose rows, columns and
        // both diagonals are all among those 14 lines
        let rp = [3usize, 0, 5, 1, 4, 2];
        let cp = [2usize, 4, 0, 5, 1, 3];
        let g = |i: usize, j: usize| (rp[i] * 6 + cp[j]) as u16;
        let mut ls: Vec<Vec<u16>> = Vec::new();
        for i in 0..6 {
            ls.push((0..6).map(|j| g(i, j)).collect());
            ls.push((0..6).map(|j| g(j, i)).collect());
        }
        ls.push((0..6).map(|i| g(i, i)).collect());
        ls.push((0..6).map(|i| g(i, 5 - i)).collect());
        for l in &mut ls {
            l.sort();
        }
        // values: 1..36 (index + 1), so value - 1 is the index
        let ds: Vec<u64> = (1..=36).collect();
        let f = solve(&ds, &ls, 6, 1, std::time::Instant::now() + std::time::Duration::from_secs(10));
        assert!(f.semi > 0);
        let sq = f.arranged.first().expect("an arrangement");
        let set = |v: Vec<u64>| -> Vec<u16> {
            let mut x: Vec<u16> = v.iter().map(|&a| (a - 1) as u16).collect();
            x.sort();
            x
        };
        for i in 0..6 {
            assert!(ls.contains(&set((0..6).map(|j| sq[i * 6 + j]).collect())));
            assert!(ls.contains(&set((0..6).map(|j| sq[j * 6 + i]).collect())));
        }
        assert!(ls.contains(&set((0..6).map(|i| sq[i * 6 + i]).collect())));
        assert!(ls.contains(&set((0..6).map(|i| sq[i * 6 + 5 - i]).collect())));
    }

    #[test]
    fn miquels_7x7_is_addmult() {
        let sq = [
            126, 66, 50, 90, 48, 1, 84, 20, 70, 16, 54, 189, 110, 6, 100, 2, 22, 98, 36, 72, 135, 96, 60, 81, 4, 10, 49, 165, 3, 63, 30, 176, 120, 45, 28, 99, 180, 14, 25, 7, 108, 32, 21, 24, 252, 18, 55, 80, 15,
        ];
        assert!(is_addmult(&sq, 7));
        let mut bad = sq;
        bad.swap(0, 1);
        assert!(!is_addmult(&bad, 7));
    }

    #[test]
    fn morgensterns_6x6_semimagic_is_found_again() {
        // Morgenstern 2007: semi-magic, S = 289, P = 1,596,672,000, largest entry 135
        let given = [24u64, 88, 18, 120, 25, 14, 135, 28, 64, 22, 30, 10, 8, 75, 45, 16, 112, 33, 35, 4, 80, 90, 44, 36, 32, 54, 77, 20, 6, 100, 55, 40, 5, 21, 72, 96];
        let (sums, prods) = score(&given, 6);
        assert!(sums >= 12 && prods >= 12);
        let p = 1_596_672_000u128;
        let ds = divisors(&factor(p), 135);
        let g = by_sum(&ds, lines(&ds, p, 6));
        let f = solve(&ds, &g[&289], 6, 1, std::time::Instant::now() + std::time::Duration::from_secs(120));
        assert!(f.semi > 0, "no semi-magic square found for S = 289");
    }
}

/// Smooth products P (primes up to 13) between `lo` and `hi`, ranked by
/// how many divisors they have up to `max` (more divisors, more lines).
pub fn candidates(lo: u128, hi: u128, max: u64) -> Vec<(u128, usize)> {
    let primes = [2u128, 3, 5, 7, 11, 13];
    let caps = [14u32, 9, 6, 4, 3, 3];
    let mut out = Vec::new();
    fn go(i: usize, p: u128, primes: &[u128], caps: &[u32], hi: u128, f: &mut Vec<(u64, u32)>, out: &mut Vec<(u128, Vec<(u64, u32)>)>) {
        if i == primes.len() {
            out.push((p, f.clone()));
            return;
        }
        let mut v = p;
        for e in 0..=caps[i] {
            if v > hi {
                break;
            }
            if e > 0 {
                f.push((primes[i] as u64, e));
            }
            go(i + 1, v, primes, caps, hi, f, out);
            if e > 0 {
                f.pop();
            }
            v *= primes[i];
        }
    }
    let mut all = Vec::new();
    go(0, 1, &primes, &caps, hi, &mut Vec::new(), &mut all);
    for (p, f) in all {
        if p < lo {
            continue;
        }
        // every prime must appear at least 4 times in P (Weisenberg's Lemma 3.6 needs room) - keep e >= 1 only
        let n = divisors(&f, max).len();
        out.push((p, n));
    }
    out.sort_by_key(|&(p, n)| (std::cmp::Reverse(n), p));
    out
}

/// Search the best `count` products with entries up to `max`, `seconds` each;
/// one line per product goes to `log` as soon as it is done.
pub fn campaign(lo: u128, hi: u128, max: u64, count: usize, seconds: f64, log: &std::path::Path) -> String {
    use std::io::Write;
    let threads = std::thread::available_parallelism().map_or(4, |t| t.get());
    let t0 = std::time::Instant::now();
    let mut out = format!(
        "6 x 6 additive-multiplicative magic square (Boyer's enigma #6a): Miquel's method\n\
         products P from {lo} to {hi} (primes up to 13), entries up to {max}, the {count} products with most divisors, {seconds:.0} s each, {threads} threads\n\
         (Morgenstern 2007: none with largest entry < 136)\n\n"
    );
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log).ok();
    if let Some(f) = file.as_mut() {
        let _ = f.write_all(out.as_bytes());
    }
    let mut total_semi = 0;
    let mut magic_all = Vec::new();
    let mut best: Option<Vec<u64>> = None;
    for (p, nd) in candidates(lo, hi, max).into_iter().take(count) {
        let ts = std::time::Instant::now();
        let (_, n_lines, semi, magic, b, done) = search_product(p, max, seconds, threads);
        total_semi += semi;
        let fac: Vec<String> = factor(p).iter().map(|&(q, e)| if e > 1 { format!("{q}^{e}") } else { q.to_string() }).collect();
        let line = format!(
            "  P = {p} = {}: {nd} divisors <= {max}, {n_lines} lines, {semi} semi-magic squares, {} MAGIC, {} ({:.0} s)\n",
            fac.join(" "),
            magic.len(),
            if done { "every sum searched: none with this P" } else { "time ran out" },
            ts.elapsed().as_secs_f64()
        );
        out.push_str(&line);
        if let Some(f) = file.as_mut() {
            let _ = f.write_all(line.as_bytes());
            for m in &magic {
                let _ = f.write_all(format!("FOUND (checked):\n{}", show(m, 6)).as_bytes());
            }
            let _ = f.flush();
        }
        if best.is_none() {
            best = b;
        }
        magic_all.extend(magic);
    }
    out.push_str(&format!("\n{} semi-magic squares in all ({:.0} s)\n", total_semi, t0.elapsed().as_secs_f64()));
    match magic_all.first() {
        Some(m) => out.push_str(&format!("FOUND a 6 x 6 additive-multiplicative magic square, checked:\n{}", show(m, 6))),
        None => {
            out.push_str("no 6 x 6 additive-multiplicative magic square with these products\n");
            if let Some(b) = best {
                let (s, p) = score(&b, 6);
                out.push_str(&format!("  a semi-magic one found on the way ({s} of 14 sums, {p} of 14 products):\n{}", show(&b, 6)));
            }
        }
    }
    out
}
