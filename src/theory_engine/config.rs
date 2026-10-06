//! data/chess_rules.toml: every number of the theory engine. Validated at
//! load: a rule, a judge or a modifier that does not exist fails at once.

use serde::Deserialize;
use std::collections::BTreeMap;

pub const FILE: &str = "data/chess_rules.toml";

/// A point in the tradeoff space: what a move offers, or what the request asks for.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Axes {
    #[serde(default)]
    pub initiative: f32,
    #[serde(default)]
    pub safety: f32,
    #[serde(default)]
    pub simplicity: f32,
}

impl Axes {
    pub fn dot(&self, o: &Axes) -> f32 {
        self.initiative * o.initiative + self.safety * o.safety + self.simplicity * o.simplicity
    }
    pub fn add(&self, o: &Axes) -> Axes {
        Axes { initiative: self.initiative + o.initiative, safety: self.safety + o.safety, simplicity: self.simplicity + o.simplicity }
    }
    /// The share of the profile one axis holds (0..1), for the breakable judges.
    pub fn share(&self, axis: &str) -> f32 {
        let total = self.initiative.max(0.0) + self.safety.max(0.0) + self.simplicity.max(0.0);
        let v = match axis { "initiative" => self.initiative, "safety" => self.safety, _ => self.simplicity };
        if total > 0.0 { v.max(0.0) / total } else { 0.0 }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub axis_cp: f32,
    pub follow_weight: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Search {
    pub width: usize,
    pub plan: usize,
    pub variants: usize,
    pub reply_ms: u64,
    pub value_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checks {
    pub finalists: usize,
    pub mate_ms: u64,
    pub material: i32,
    pub reference_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judge {
    pub weight: f32,
    #[serde(default)]
    pub min: i32,
    pub breakable: bool,
    #[serde(default)]
    pub discount: f32,
    #[serde(default)]
    pub axis: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleCfg {
    pub weight: f32,
    #[serde(default)]
    pub initiative: f32,
    #[serde(default)]
    pub safety: f32,
    #[serde(default)]
    pub simplicity: f32,
}

impl RuleCfg {
    pub fn axes(&self) -> Axes {
        Axes { initiative: self.initiative, safety: self.safety, simplicity: self.simplicity }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub profile: Axes,
    pub modifiers: BTreeMap<String, Axes>,
    pub score: Score,
    pub search: Search,
    pub checks: Checks,
    pub judges: BTreeMap<String, Judge>,
    pub rules: BTreeMap<String, RuleCfg>,
}

pub const JUDGES: [&str; 3] = ["threat_first", "material", "king_shield"];

impl Config {
    pub fn load() -> Result<Config, String> {
        let text = std::fs::read_to_string(FILE).map_err(|e| format!("{FILE}: {e}"))?;
        let cfg: Config = toml::from_str(&text).map_err(|e| format!("{FILE}: {e}"))?;
        for id in cfg.rules.keys() {
            if crate::golden_theory::rule(id).is_none() {
                return Err(format!("{FILE}: [rules.{id}] has no rule in golden_theory"));
            }
        }
        for (id, _) in crate::golden_theory::RULES.iter() {
            if !cfg.rules.contains_key(*id) {
                return Err(format!("{FILE}: the rule {id} has no [rules.{id}]"));
            }
        }
        for j in JUDGES {
            if !cfg.judges.contains_key(j) {
                return Err(format!("{FILE}: no [judges.{j}]"));
            }
        }
        for j in cfg.judges.keys() {
            if !JUDGES.contains(&j.as_str()) {
                return Err(format!("{FILE}: [judges.{j}] is no judge"));
            }
        }
        Ok(cfg)
    }
    pub fn judge(&self, name: &str) -> &Judge {
        &self.judges[name]
    }
    pub fn rule(&self, id: &str) -> &RuleCfg {
        &self.rules[id]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_file_loads_and_names_every_rule() {
        let cfg = super::Config::load().unwrap();
        assert_eq!(cfg.rules.len(), crate::golden_theory::RULES.len());
        assert!(cfg.modifiers.contains_key("tal") && cfg.modifiers.contains_key("karpov"));
    }
}
