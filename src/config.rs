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

/// Calendar conventions for growth over time ("a month is 30 days").
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finance {
    pub days_per_week: u32,
    pub days_per_month: u32,
    pub days_per_year: u32,
    pub months_per_year: u32,
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
    pub numbers: Numbers,
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
}
