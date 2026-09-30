//! `rules.toml`: every number the engine uses. Rule code only names keys.
//! Validated at load: a typo in a rule or variant name fails immediately.

use crate::model::{Modifier, Task};
use crate::rules;
use anyhow::{bail, Context as _, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The built-in configuration, so the binary works without a rules file.
pub const DEFAULT_RULES: &str = include_str!("../rules.toml");

/// A point in tradeoff space: what a step offers, or what the sentence asks
/// for (the profile).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Axes {
    #[serde(default)]
    pub brevity: f64,
    #[serde(default)]
    pub clarity: f64,
    #[serde(default)]
    pub elegance: f64,
}

impl Axes {
    pub fn dot(&self, o: &Axes) -> f64 {
        self.brevity * o.brevity + self.clarity * o.clarity + self.elegance * o.elegance
    }
    pub fn add(&self, o: &Axes) -> Axes {
        Axes { brevity: self.brevity + o.brevity, clarity: self.clarity + o.clarity, elegance: self.elegance + o.elegance }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchConfig {
    pub beam: usize,
    pub fallback: usize,
    pub max_steps: usize,
    /// Weight of the distance-to-done estimate when ranking unfinished paths.
    pub progress: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    /// Every step costs this much...
    pub cost: f64,
    /// ...plus this much per unit of brevity in the profile.
    pub brevity_cost: f64,
    /// A minor step (bookkeeping) costs this fraction of a full step.
    pub minor_share: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCfg {
    pub weight: f64,
    /// Bookkeeping: shown merged into the step before it when the profile is brief.
    #[serde(default)]
    pub minor: bool,
    pub variants: BTreeMap<String, Axes>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JudgeCfg {
    pub weight: f64,
    #[serde(default)]
    pub axes: Axes,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckCfg {
    /// Values the letters take when two expressions are compared.
    pub samples: Vec<f64>,
    /// Relative tolerance for floating-point comparisons.
    pub tolerance: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Display {
    /// Decimal places for "approximately" with no number given.
    pub decimals: u32,
    /// An exact answer longer than this many characters leads with its decimal.
    pub answer_digits: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Numbers {
    pub write_out_factorial: u64,
}

/// How proofs are checked.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProofCfg {
    /// The largest exact grid used to confirm a polynomial identity.
    pub max_grid: usize,
    /// Points tried when the sides aren't polynomials (evidence, not proof).
    pub samples: usize,
    /// "Check every remainder mod d" is offered up to this d.
    pub max_residues: u64,
    /// An induction proof is also confirmed exactly for n up to this.
    pub induction_checks: i64,
}

/// An open (or famously hard) problem, described so a refusal can say why.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenProblem {
    pub name: String,
    pub statement: String,
    pub status: String,
    #[serde(default)]
    pub known: Vec<String>,
    /// The problem as a Nuome question: the engine really runs on it.
    #[serde(default)]
    pub formal: Option<String>,
    /// For a problem that can be computed on: check every case up to this.
    #[serde(default)]
    pub check_up_to: Option<u64>,
    /// Only when asked for a guess: the expected answer, not a result.
    #[serde(default)]
    pub guess: Option<Guess>,
    /// "check up to 10^10" in the sentence may ask for at most this.
    #[serde(default)]
    pub max_check: Option<u64>,
    /// How much Nuome doubts the heuristic model itself (not measurable by
    /// computing; stated here so the confidence score shows where it comes from).
    #[serde(default)]
    pub model_doubt: Option<f64>,
    /// "find a formula": fit on records with n from `formula_min_n` up to
    /// `formula_split`, test on the records above it.
    #[serde(default)]
    pub formula_min_n: Option<u64>,
    #[serde(default)]
    pub formula_split: Option<u64>,
    /// Added to a family's training error per parameter: simpler formulas win ties.
    #[serde(default)]
    pub formula_price: Option<f64>,
    /// Evolving formulas: population, generations, seed, and the fitness
    /// price per node of a formula.
    #[serde(default)]
    pub evolve_population: Option<usize>,
    #[serde(default)]
    pub evolve_generations: Option<usize>,
    #[serde(default)]
    pub evolve_seed: Option<u64>,
    #[serde(default)]
    pub evolve_price: Option<f64>,
    /// Fitness by forward prediction (true) or by fit (false).
    #[serde(default)]
    pub evolve_forward: Option<bool>,
    /// Improving steps per child in a fitness search over graphs.
    #[serde(default)]
    pub evolve_climb: Option<usize>,
    /// A folder (from the crate root) for pictures of what was built.
    #[serde(default)]
    pub pictures: Option<String>,
    /// A graph to start the repair search from ("a-b" lines, from the crate root).
    #[serde(default)]
    pub seed_graph: Option<String>,
    /// Lattice simulations: side of the lattice, couplings, sweeps.
    #[serde(default)]
    pub lattice: Option<usize>,
    #[serde(default)]
    pub betas: Vec<f64>,
    #[serde(default)]
    pub thermalise: Option<usize>,
    #[serde(default)]
    pub measure: Option<usize>,
    /// Flow simulations: grid side, viscosity, end time, time step.
    #[serde(default)]
    pub grid: Option<usize>,
    #[serde(default)]
    pub viscosity: Option<f64>,
    #[serde(default)]
    pub t_end: Option<f64>,
    #[serde(default)]
    pub dt: Option<f64>,
    /// Experiments over problem sizes: the sizes, instances per size, a ratio.
    #[serde(default)]
    pub sizes: Vec<usize>,
    #[serde(default)]
    pub instances: Option<usize>,
    #[serde(default)]
    pub ratio: Option<f64>,
    /// Published tables to compare with (zeta zeros, elliptic curves; paths
    /// from the crate root); the first one that exists is used.
    #[serde(default)]
    pub tables: Vec<String>,
}

/// A best guess at an open problem: what is expected, why, and how firmly.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guess {
    pub answer: String,
    pub confidence: String,
    pub basis: Vec<String>,
}

/// Calendar conventions for growth over time ("a month is 30 days").
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finance {
    pub days_per_week: u32,
    pub days_per_month: u32,
    pub days_per_year: u32,
    pub months_per_year: u32,
}

/// Calculus and trigonometry (agent B): the numbers the rules and checks
/// for integrals, limits, higher derivatives and trig equations use.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalcCfg {
    /// Simpson panels for checking a definite integral against quadrature.
    pub panels: usize,
    /// Relative tolerance for a definite integral against quadrature.
    pub integral_tolerance: f64,
    /// Step for the numerical n-th derivative that checks a higher derivative.
    pub higher_step: f64,
    /// Relative tolerance for higher derivatives and nested d/dx lines.
    pub higher_tolerance: f64,
    /// Distances from the point at which a limit is checked, far to near.
    pub limit_steps: Vec<f64>,
    /// Where "x -> infinity" is checked, near to far.
    pub limit_far: Vec<f64>,
    /// Relative tolerance for a limit at the nearest (or farthest) step.
    pub limit_tolerance: f64,
    /// |f| beyond this at the nearest step counts as growing without bound.
    pub unbounded: f64,
    /// A number smaller than this counts as 0 when a rule reads a value
    /// (0/0 for L'Hopital, a trig value from the table).
    pub zero: f64,
    /// Grid points per unit of length when counting a trig equation's solutions.
    pub trig_scan: usize,
    /// General solutions are checked for k = -k_range ..= k_range.
    pub k_range: i64,
    /// When too few sample points lie in an expression's domain (arcsin x),
    /// the samples are scaled by this and tried again.
    pub narrow: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub search: SearchConfig,
    pub profile: Axes,
    pub modifiers: BTreeMap<String, Axes>,
    pub steps: Steps,
    pub tasks: BTreeMap<String, Vec<String>>,
    pub methods: BTreeMap<String, Vec<String>>,
    /// Methods that finish with another method of the same group:
    /// completing the square ends by taking square roots.
    pub method_follow: BTreeMap<String, Vec<String>>,
    pub rules: BTreeMap<String, RuleCfg>,
    pub judges: BTreeMap<String, JudgeCfg>,
    pub check: CheckCfg,
    pub display: Display,
    pub finance: Finance,
    /// Famous problems Nuome recognises: what they say and where they stand.
    pub open: BTreeMap<String, OpenProblem>,
    pub proof: ProofCfg,
    pub numbers: Numbers,
    pub algebra: AlgebraCfg,
    pub calculus: CalcCfg,
    // logic and sets (agent L)
    pub logic: LogicCfg,
    /// abstract algebra (agent G): group and ring proofs.
    pub abstract_algebra: crate::abstract_algebra::Cfg,
}

/// Logic and sets (agent L): how big a truth table gets.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LogicCfg {
    /// The most letters a truth table written out as a proof may have.
    pub table_letters: usize,
    /// The most letters a statement may have: every row of its truth table
    /// is checked.
    pub check_letters: usize,
    /// A step may make the statement at most this many times as big as it
    /// was asked (plus 2): spreading and over or can grow without end.
    pub growth: f64,
    /// The most one-place predicates a statement with quantifiers may have:
    /// it is checked in every kind of domain, 2^(2^k) - 1 of them.
    pub predicates: usize,
}

/// Algebra checks (inequalities).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlgebraCfg {
    /// Inequality checks test this far (relative) either side of each boundary.
    pub boundary_offset: f64,
    /// Two sides this close count as equal when an inequality is tested.
    pub sign_tolerance: f64,
}

pub const JUDGES: &[&str] = &["fractions", "negative_lead", "method_switch", "growth", "branch_order", "brackets_first", "arithmetic_first"];

impl Config {
    pub fn builtin() -> Config {
        Config::parse(DEFAULT_RULES).expect("built-in rules.toml is valid")
    }
    pub fn load(p: &Path) -> Result<Config> {
        let text = std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
        Config::parse(&text).with_context(|| format!("in {}", p.display()))
    }
    pub fn parse(text: &str) -> Result<Config> {
        let cfg: Config = toml::from_str(text)?;
        cfg.validate()?;
        Ok(cfg)
    }
    fn validate(&self) -> Result<()> {
        for k in crate::lexicon::OPEN_KEYS {
            if !self.open.contains_key(*k) {
                bail!("[open.{k}] is missing");
            }
        }
        for m in Modifier::ALL {
            if !self.modifiers.contains_key(m.key()) {
                bail!("[modifiers.{}] is missing", m.key());
            }
        }
        for k in self.modifiers.keys() {
            if !Modifier::ALL.iter().any(|m| m.key() == k) {
                bail!("[modifiers.{k}]: no such modifier");
            }
        }
        for t in Task::ALL {
            if !self.tasks.contains_key(t.key()) {
                bail!("[tasks] has no entry for {}", t.key());
            }
        }
        for (t, names) in self.tasks.iter().chain(self.methods.iter()).chain(self.method_follow.iter()) {
            for n in names {
                if rules::by_name(n).is_none() {
                    bail!("\"{n}\" (in {t}): no such rule");
                }
            }
        }
        for r in rules::all() {
            let Some(rc) = self.rules.get(r.name()) else { bail!("[rules.{}] is missing", r.name()) };
            for v in r.variants() {
                if !rc.variants.contains_key(*v) {
                    bail!("[rules.{}.variants] has no entry for \"{v}\"", r.name());
                }
            }
            for v in rc.variants.keys() {
                if !r.variants().contains(&v.as_str()) {
                    bail!("[rules.{}.variants.{v}]: the rule has no such variant", r.name());
                }
            }
        }
        for k in self.rules.keys() {
            if rules::by_name(k).is_none() {
                bail!("[rules.{k}]: no such rule");
            }
        }
        for j in JUDGES {
            if !self.judges.contains_key(*j) {
                bail!("[judges.{j}] is missing");
            }
        }
        for k in self.judges.keys() {
            if !JUDGES.contains(&k.as_str()) {
                bail!("[judges.{k}]: no such judge");
            }
        }
        if self.search.beam == 0 || self.search.fallback < self.search.beam {
            bail!("[search]: beam must be at least 1 and fallback at least beam");
        }
        if self.check.samples.len() < 3 {
            bail!("[check] needs at least 3 samples");
        }
        if !(self.algebra.boundary_offset > 0.0 && self.algebra.boundary_offset < 0.1) {
            bail!("[algebra] boundary_offset must be between 0 and 0.1");
        }
        if !(self.algebra.sign_tolerance > 0.0 && self.algebra.sign_tolerance < self.algebra.boundary_offset * self.algebra.boundary_offset) {
            bail!("[algebra] sign_tolerance must be positive and below boundary_offset squared (or a double root looks like a sign change)");
        }
        // calculus and trig (agent B)
        let c = &self.calculus;
        if c.panels < 2 || c.trig_scan < 10 || c.k_range < 1 {
            bail!("[calculus]: panels must be at least 2, trig_scan at least 10, k_range at least 1");
        }
        let positive = [c.integral_tolerance, c.higher_step, c.higher_tolerance, c.limit_tolerance, c.unbounded, c.zero, c.narrow];
        if positive.iter().any(|x| !(x.is_finite() && *x > 0.0)) {
            bail!("[calculus]: tolerances, steps, unbounded and zero must be positive numbers");
        }
        if c.limit_steps.len() < 2 || !c.limit_steps.windows(2).all(|w| w[0] > w[1] && w[1] > 0.0) {
            bail!("[calculus] limit_steps: at least 2 positive steps, far to near (decreasing)");
        }
        if c.limit_far.len() < 2 || !c.limit_far.windows(2).all(|w| w[1] > w[0] && w[0] > 0.0) {
            bail!("[calculus] limit_far: at least 2 positive points, near to far (increasing)");
        }
        // logic and sets (agent L)
        let l = &self.logic;
        if l.table_letters == 0 || l.table_letters > l.check_letters || l.check_letters > 20 {
            bail!("[logic]: need 1 <= table_letters <= check_letters <= 20");
        }
        if !(l.growth.is_finite() && l.growth >= 1.0) {
            bail!("[logic]: growth must be at least 1");
        }
        if l.predicates == 0 || l.predicates > 4 {
            bail!("[logic]: predicates must be 1 to 4");
        }
        // abstract algebra (agent G)
        crate::abstract_algebra::validate(self)?;
        Ok(())
    }
    /// The rules a task may use, in order.
    pub fn task_rules(&self, t: Task) -> Vec<&'static dyn rules::Rule> {
        self.tasks[t.key()].iter().filter_map(|n| rules::by_name(n)).collect()
    }
    /// May `rule` follow `method` (a different rule of its group)?
    pub fn follows(&self, method: &str, rule: &str) -> bool {
        self.method_follow.get(method).is_some_and(|v| v.iter().any(|r| r == rule))
    }
    /// The method group a rule belongs to, if any.
    pub fn group_of(&self, rule: &str) -> Option<&str> {
        self.methods.iter().find(|(_, v)| v.iter().any(|r| r == rule)).map(|(k, _)| k.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_rules_load_and_typos_fail() {
        let cfg = Config::builtin();
        assert!(cfg.search.beam > 0);
        let broken = DEFAULT_RULES.replacen("[rules.fold.variants.one]", "[rules.fold.variants.onee]", 1);
        assert!(Config::parse(&broken).is_err());
    }

    #[test]
    fn calculus_section_is_validated() {
        let cfg = Config::builtin();
        assert!(cfg.calculus.panels >= 2);
        let upward = DEFAULT_RULES.replacen("limit_steps = [1e-2, 1e-3, 1e-4, 1e-5]", "limit_steps = [1e-5, 1e-2]", 1);
        assert!(Config::parse(&upward).is_err());
        let no_task = DEFAULT_RULES.replacen("\nlimit = [", "\nlimits = [", 1);
        assert!(Config::parse(&no_task).is_err());
    }
}
