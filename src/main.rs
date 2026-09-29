use anyhow::{bail, Context, Result};
use clap::Parser;
use nuome::config::Config;
use nuome::print::Style;
use nuome::{explain, lexicon, solve, Options};
use std::path::PathBuf;

/// Nuome: ask a math question in plain words, get a worked solution where
/// every step names the rule that took it and every answer is checked.
///
///   nuome "solve 2x + 3 = 7"
///   nuome "factor x^2 - 5x + 6"
///   nuome "differentiate x^3 sin x step by step"
///   nuome "solve x^2 - 4x + 1 = 0 using the quadratic formula"
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The question. Several words are joined; omit to read --file.
    words: Vec<String>,
    /// Solve every line of a file (# starts a comment).
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Print math with √, ·, ² and ± instead of plain ASCII.
    #[arg(long, conflicts_with = "latex")]
    unicode: bool,
    /// Print math as LaTeX.
    #[arg(long)]
    latex: bool,
    /// Show the tokens, the request, the profile, every finalist and every check (stderr).
    #[arg(long)]
    explain: bool,
    /// Skip unknown words instead of failing.
    #[arg(long)]
    lenient: bool,
    /// Use this rules.toml instead of the built-in one.
    #[arg(long)]
    rules: Option<PathBuf>,
    /// List every word and phrase Nuome knows.
    #[arg(long)]
    vocabulary: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.vocabulary {
        for w in lexicon::vocabulary() {
            println!("{w}");
        }
        return Ok(());
    }
    let cfg = match &cli.rules {
        Some(p) => Config::load(p)?,
        None => Config::builtin(),
    };
    let style = if cli.latex {
        Style::Latex
    } else if cli.unicode {
        Style::Unicode
    } else {
        Style::Ascii
    };
    let opts = Options { lenient: cli.lenient, style };
    let mut questions = Vec::new();
    if let Some(f) = &cli.file {
        let text = std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?;
        questions.extend(text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from));
    }
    if !cli.words.is_empty() {
        questions.push(cli.words.join(" "));
    }
    if questions.is_empty() {
        bail!("ask something, e.g. nuome \"solve 2x + 3 = 7\"");
    }
    let mut failed = 0;
    for (i, q) in questions.iter().enumerate() {
        if i > 0 {
            println!("\n{}\n", "-".repeat(60));
        }
        match solve(q, &cfg, &opts) {
            Ok(s) => {
                if cli.explain {
                    eprint!("{}", explain::explain(&s.request, &s.outcome, &cfg));
                }
                print!("{}", s.text);
            }
            Err(diags) => {
                failed += 1;
                if questions.len() > 1 {
                    eprintln!("{q}");
                }
                for d in &diags {
                    eprintln!("{d}");
                }
            }
        }
    }
    if failed > 0 {
        bail!("{failed} question{} not answered", if failed == 1 { "" } else { "s" });
    }
    Ok(())
}
