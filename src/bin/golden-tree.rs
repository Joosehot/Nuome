//! Golden Tree, the path engine (src/golden_tree.rs).
//!
//!   golden-tree grow --steps N --render-distance D --seed S   grow, keep the best paths, draw them and the tree
//!   golden-tree vault list                                     the kept paths, best first
//!   golden-tree vault show <id>                                one path, checked again from scratch
//!   golden-tree render <id>                                    draw a kept path
//!
//! The same through Golden Answer: nuome --answer logical "Grow golden tree paths."

use clap::{Parser, Subcommand};
use nuome::golden_tree as gt;

#[derive(Parser)]
#[command(name = "golden-tree", about = "Golden grows paths out of randomness and keeps the long quiet ones that turn complex")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Grow a tree and keep its best paths in the vault (out/golden_tree/vault).
    Grow {
        #[arg(long, default_value_t = 40)]
        steps: usize,
        #[arg(long, default_value_t = gt::RENDER_DISTANCE)]
        render_distance: usize,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        /// how many of the best paths to keep
        #[arg(long, default_value_t = 12)]
        keep: usize,
        /// iterate on the CPU even if there is a GPU
        #[arg(long)]
        cpu: bool,
    },
    /// The vault: list, or show one path.
    Vault {
        #[command(subcommand)]
        what: VaultCmd,
    },
    /// Draw a kept path, golden on black.
    Render { id: String },
}

#[derive(Subcommand)]
enum VaultCmd {
    List,
    Show { id: String },
}

fn main() -> Result<(), String> {
    match Cli::parse().cmd {
        Cmd::Grow { steps, render_distance, seed, keep, cpu } => {
            let gpu = if cpu {
                None
            } else {
                match gt::Gpu::new() {
                    Ok(g) => Some(g),
                    Err(e) => {
                        eprintln!("no GPU, iterating on the CPU: {e}");
                        None
                    }
                }
            };
            let t0 = std::time::Instant::now();
            let tree = gt::grow(steps, render_distance, seed, gpu.as_ref());
            let found = gt::best(&tree, keep);
            gt::save(&found)?;
            println!(
                "seed {seed}: {} moves, render distance {}, {} paths weighed on the {} in {:.1} s; none made deeper than {} beyond where Golden stood",
                tree.trunk.len() - 1,
                tree.render_distance,
                tree.weighed,
                if gpu.is_some() { "GPU" } else { "CPU" },
                t0.elapsed().as_secs_f64(),
                tree.deepest_ahead
            );
            for e in &found {
                let pic = gt::render(e)?;
                println!("  {}  score {:.3}  quiet {:>4}  after {:.2}  {}   {}", e.id, e.score, e.quiet, e.after, e.rule, pic.display());
            }
            println!("tree: {}", gt::render_tree(&tree)?.display());
        }
        Cmd::Vault { what: VaultCmd::List } => {
            let v = gt::vault();
            if v.is_empty() {
                println!("the vault is empty; grow first");
            }
            for e in v {
                println!("{}  score {:.3}  quiet {:>4}  after {:.2}  seed {}  {}", e.id, e.score, e.quiet, e.after, e.seed, e.rule);
            }
        }
        Cmd::Vault { what: VaultCmd::Show { id } } => {
            let e = gt::find(&id).ok_or(format!("no path {id} in the vault"))?;
            println!("{}\n  rule:   {}\n  world:  {:?}\n  seed:   {}\n  start:  {}\n  quiet:  {} iterations\n  after:  {:.3}\n  score:  {:.4}", e.id, e.rule, e.world, e.seed, e.start, e.quiet, e.after, e.score);
            match gt::check(&e) {
                Ok(_) => println!("  checked: iterated again on the CPU from scratch, the same quiet run and score"),
                Err(why) => println!("  DOES NOT CHECK: {why}"),
            }
        }
        Cmd::Render { id } => {
            let e = gt::find(&id).ok_or(format!("no path {id} in the vault"))?;
            println!("{}", gt::render(&e)?.display());
        }
    }
    Ok(())
}
