//! Golden learns from an answer it is given, then uses what it learned.
//!
//! The answer: the Berlekamp-van Lint-Seidel graph srg(243, 22, 1, 2)
//! (Conway's family; built here as the cyclotomic Cayley graph on GF(243)
//! and checked on every vertex). It is written in the same forced frame
//! Golden works in — a vertex x, its 22 neighbours in 11 triangles, the 220
//! far vertices as pairs of them — and Golden invents an expression, in its
//! own symbols and the old ones, that gives exactly that graph's far
//! adjacencies. What it learned (the expression, its symbols, its relation)
//! then starts the search for srg(99, 14, 1, 2).

use crate::golden_invent::{Frame, Naming};
use crate::golden_symbols::{evolve, Lang, Sx};

/// The 243-graph's far adjacencies in the frame: target[f][g] for the frame's far vertices.
pub fn bvls_in_frame() -> Option<(Frame, Vec<Vec<bool>>)> {
    let (found, _) = crate::graph_function::search(243, 22, 1, 2, 20_000_000, 60, 120.0);
    let f = found?;
    let g = &f.group;
    let n = g.order as usize;
    let mut ins = vec![false; n];
    for &x in &f.set {
        ins[x as usize] = true;
    }
    let adj = |a: usize, b: usize| a != b && ins[g.sub(b as u64, a as u64) as usize];
    // x = 0; its neighbours paired by their one common neighbour with x
    let nbrs: Vec<usize> = (0..n).filter(|&y| adj(0, y)).collect();
    let mut tri: Vec<(usize, usize)> = Vec::new();
    let mut used = vec![false; n];
    for &u in &nbrs {
        if used[u] {
            continue;
        }
        let p = *nbrs.iter().find(|&&w| w != u && adj(u, w))?;
        used[u] = true;
        used[p] = true;
        tri.push((u, p));
    }
    let name = |y: usize| -> Option<(u64, u64)> { tri.iter().enumerate().find_map(|(i, &(a, b))| if a == y { Some((i as u64, 0)) } else if b == y { Some((i as u64, 1)) } else { None }) };
    let fr = Frame::new(22);
    let index: std::collections::HashMap<((u64, u64), (u64, u64)), usize> = fr.far.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    // each far vertex: its two neighbours among x's
    let mut far_vertex = vec![0usize; fr.far.len()];
    for w in 0..n {
        if w == 0 || adj(0, w) {
            continue;
        }
        let two: Vec<(u64, u64)> = nbrs.iter().filter(|&&u| adj(w, u)).filter_map(|&u| name(u)).collect();
        if two.len() != 2 {
            return None;
        }
        let (a, b) = if two[0].0 < two[1].0 { (two[0], two[1]) } else { (two[1], two[0]) };
        far_vertex[*index.get(&(a, b))?] = w;
    }
    let target = (0..fr.far.len()).map(|i| (0..fr.far.len()).map(|j| adj(far_vertex[i], far_vertex[j])).collect()).collect();
    Some((fr, target))
}

pub struct Learned {
    pub expr: Sx,
    pub lang: Lang,
    pub naming: Naming,
    pub wrong: u64,
    pub pairs: u64,
    pub seconds: f64,
}

/// Golden invents an expression that gives the 243-graph's far adjacencies.
pub fn learn_243(seconds: f64) -> Option<Learned> {
    let t0 = std::time::Instant::now();
    let (fr, target) = bvls_in_frame()?;
    let n = fr.far.len();
    let pairs = (n * (n - 1) / 2) as u64;
    let fit = |e: &Sx, l: &Lang, nm: &Naming| {
        let a = fr.adjacency_with(nm, |f, g| l.related(e.eval(f, l), e.eval(g, l)));
        let base = 1 + fr.k;
        let mut wrong = 0u64;
        for i in 0..n {
            for j in i + 1..n {
                wrong += (a[base + i][base + j] != target[i][j]) as u64;
            }
        }
        wrong * 64 + e.size() as u64
    };
    let (best, _, _) = evolve(&fr, seconds, 0x1234_5678_9abc_def1, &fit, &[]);
    let (score, expr, lang, naming) = best?;
    Some(Learned { expr, lang, naming, wrong: score / 64, pairs, seconds: t0.elapsed().as_secs_f64() })
}
