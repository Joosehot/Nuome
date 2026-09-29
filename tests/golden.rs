//! The v0 proof:
//! 1. determinism: every example gives byte-identical output, run after run
//! 2. stability: the output matches the checked-in golden file
//! 3. honesty: every question in an example file is answered and passes
//!    every check, and every question in refusals.txt is refused
//!
//! Each example file holds one question per line; its golden is
//! examples/out/<name>.txt. Regenerate goldens after an intended change
//! with `NUOME_BLESS=1 cargo test`.

use nuome::config::Config;
use nuome::{solve, Options};
use std::fs;
use std::path::{Path, PathBuf};

fn examples() -> Vec<(String, Vec<String>)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut out: Vec<(String, Vec<String>)> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .map(|p| {
            let text = fs::read_to_string(&p).unwrap();
            let lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect();
            (p.file_stem().unwrap().to_string_lossy().into_owned(), lines)
        })
        .collect();
    out.sort();
    assert!(!out.is_empty(), "no examples found");
    out
}

/// Everything a file of questions prints: solutions and refusals alike.
fn transcript(lines: &[String], cfg: &Config) -> String {
    let mut out = String::new();
    for q in lines {
        out.push_str(&format!("### {q}\n\n"));
        match solve(q, cfg, &Options::default()) {
            Ok(s) => out.push_str(&s.text),
            Err(ds) => {
                for d in ds {
                    out.push_str(&format!("{d}\n"));
                }
            }
        }
        out.push('\n');
    }
    out
}

fn golden_path(stem: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/out").join(format!("{stem}.txt"))
}

#[test]
fn examples_are_deterministic_and_match_goldens() {
    let cfg = Config::builtin();
    let bless = std::env::var_os("NUOME_BLESS").is_some();
    let mut failures = Vec::new();
    for (stem, lines) in examples() {
        let first = transcript(&lines, &cfg);
        for _ in 0..2 {
            assert_eq!(first, transcript(&lines, &cfg), "{stem}: output changed between runs");
        }
        let path = golden_path(&stem);
        if bless {
            fs::write(&path, &first).unwrap();
            continue;
        }
        match fs::read_to_string(&path) {
            Ok(golden) if golden.replace("\r\n", "\n") == first => {}
            Ok(_) => failures.push(format!("{stem}: differs from {}", path.display())),
            Err(_) => failures.push(format!("{stem}: missing {}", path.display())),
        }
    }
    assert!(failures.is_empty(), "{}\n(run with NUOME_BLESS=1 to accept)", failures.join("\n"));
}

#[test]
fn examples_are_answered_and_refusals_refused() {
    let cfg = Config::builtin();
    for (stem, lines) in examples() {
        for q in &lines {
            let r = solve(q, &cfg, &Options::default());
            if stem == "refusals" {
                assert!(r.is_err(), "{stem}: \"{q}\" should be refused");
                continue;
            }
            let s = r.unwrap_or_else(|d| panic!("{stem}: \"{q}\": {}", d.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("; ")));
            let checks = s.outcome.checks();
            assert!(checks.len() >= 2, "{stem}: \"{q}\": only {} checks ran", checks.len());
            for c in checks {
                assert!(c.ok, "{stem}: \"{q}\": {} failed: {}", c.name, c.detail);
            }
        }
    }
}
