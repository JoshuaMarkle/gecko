//! Objective profiles: metric weights + targets (TOML in `data/profiles/`). See `context/objective.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{geometry::FINGERS, hash, Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Metric id → weight (≥ 0). Direction comes from the metric definition, so a larger
    /// weight always means "care more". Missing metrics have weight 0.
    #[serde(default)]
    pub weights: BTreeMap<String, f32>,
    /// Target finger load in % (LP LR LM LI LT RT RI RM RR RP) for M02.
    #[serde(default = "default_finger_target")]
    pub finger_target: [f32; FINGERS],
    /// Keep the most frequent trigrams covering this share of trigram volume (renormalized).
    /// 1.0 = exact. Trades a little fidelity for search speed; see `context/objective.md`.
    #[serde(default = "default_trigram_coverage")]
    pub trigram_coverage: f32,
}

pub fn default_trigram_coverage() -> f32 {
    0.99
}

pub fn default_finger_target() -> [f32; FINGERS] {
    [8.0, 11.0, 15.0, 16.0, 0.0, 0.0, 16.0, 15.0, 11.0, 8.0]
}

impl Profile {
    pub fn from_toml(s: &str) -> Result<Self> {
        let p: Profile = toml::from_str(s).map_err(|e| Error::parse("profile", e))?;
        for (k, w) in &p.weights {
            if crate::metrics::def(k).is_none() {
                return Err(Error::invalid("profile", format!("unknown metric id '{k}'")));
            }
            if !w.is_finite() || *w < 0.0 {
                return Err(Error::invalid("profile", format!("weight for {k} must be ≥ 0")));
            }
        }
        Ok(p)
    }

    pub fn weight(&self, id: &str) -> f32 {
        self.weights.get(id).copied().unwrap_or(0.0)
    }

    pub fn hash(&self) -> u64 {
        hash::fnv1a(serde_json::to_string(self).unwrap_or_default().as_bytes())
    }
}
