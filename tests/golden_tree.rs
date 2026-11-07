//! Golden Tree's promises (src/golden_tree.rs):
//! 1. the same seed grows the same tree;
//! 2. no branch is ever made deeper than the render distance beyond where Golden stands;
//! 3. a Collatz-like rule (built here, in the test; the engine knows no rule) gets a long quiet run and a high score;
//! 4. trivial rules are rejected: constants, plain loops, plain explosion;
//! 5. the GPU's iteration is the CPU's, number for number (when there is a GPU).

use nuome::golden_tree::{self as gt, Rule, Step, World};

fn rule(steps: &[Step]) -> Rule {
    Rule { steps: steps.to_vec() }
}

#[test]
fn the_same_seed_grows_the_same_tree() {
    let a = gt::grow(6, 3, 42, None);
    let b = gt::grow(6, 3, 42, None);
    let key = |t: &gt::Tree| -> Vec<(usize, Option<Step>, usize, String)> {
        t.nodes
            .iter()
            .map(|n| (n.id, n.step, n.depth, n.weighed.as_ref().map(|w| format!("{:.12}", w.score)).unwrap_or_default()))
            .collect()
    };
    assert_eq!(key(&a), key(&b));
    assert_eq!(a.trunk, b.trunk);
    assert_eq!(gt::best(&a, 5), gt::best(&b, 5));
    let c = gt::grow(6, 3, 43, None);
    assert_ne!(key(&a), key(&c), "another seed grows another tree");
}

#[test]
fn render_distance_is_respected() {
    for d in [1, 2, 4] {
        let t = gt::grow(5, d, 7, None);
        assert!(t.deepest_ahead <= d, "made {} beyond with render distance {d}", t.deepest_ahead);
        // and no node anywhere is deeper than the trunk tip it grew from plus d
        let tip = t.nodes[*t.trunk.last().unwrap()].depth;
        assert!(t.nodes.iter().filter(|n| n.alive).all(|n| n.depth <= tip + d));
    }
}

#[test]
fn a_collatz_like_rule_is_quiet_long_then_complex() {
    // x odd: 3x + 1, then halve; x even: halve
    let collatz = rule(&[
        Step::If { m: 2, r: 1, op: 2, arg: 3 },
        Step::If { m: 2, r: 1, op: 1, arg: 1 },
        Step::Shr(1),
    ]);
    assert_eq!(collatz.world(), World::Number);
    let w = gt::weigh(&collatz);
    assert!(w.reading.rejected.is_none(), "{:?}", w.reading);
    assert!(w.reading.quiet >= 39, "quiet {}", w.reading.quiet);
    assert!(w.reading.after > 0.6, "after {}", w.reading.after);
    assert!(w.score > 2.0, "score {}", w.score);
}

#[test]
fn trivial_rules_are_rejected() {
    let cases: Vec<(Rule, &str)> = vec![
        (rule(&[Step::Add(1)]), "counting up: a plain loop of symbols"),
        (rule(&[Step::Mul(3)]), "plain explosion"),
        (rule(&[Step::Shr(1)]), "falls to a constant"),
        (rule(&[Step::Mod(5)]), "a constant at once"),
        (rule(&[Step::Rot(1)]), "a row turning: never changes"),
        (rule(&[Step::AndC(1)]), "a row that dies"),
    ];
    for (r, what) in cases {
        let w = gt::weigh(&r);
        assert_eq!(w.score, 0.0, "{what}: {} scored {:?}", r.text(), w);
        assert!(w.reading.rejected.is_some(), "{what}: {} not rejected", r.text());
    }
}

#[test]
fn the_gpu_iterates_as_the_cpu() {
    let Ok(gpu) = gt::Gpu::new() else {
        eprintln!("no GPU here; the CPU is the reference and the other tests ran on it");
        return;
    };
    let mut rng = gt::Rng::new(5);
    for world in [World::Number, World::Row] {
        let rules: Vec<Rule> = (0..gt::GPU_MIN + 40).map(|i| rule(&(0..1 + i % 5).map(|_| gt::draw_step(&mut rng, world)).collect::<Vec<_>>())).collect();
        let on_gpu = gt::weigh_all(&rules, Some(&gpu));
        for (r, g) in rules.iter().zip(on_gpu) {
            let c = gt::weigh(r);
            assert_eq!((c.reading.quiet, c.start), (g.reading.quiet, g.start), "{}", r.text());
            // the entropies are read with the GPU's log2: equal to the last few bits
            assert!((c.score - g.score).abs() < 1e-9, "{}: cpu {} gpu {}", r.text(), c.score, g.score);
        }
    }
}
