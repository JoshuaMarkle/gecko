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
    /// Metric id → spread of that metric's cost over the reference layouts (`gecko profile
    /// normalize`). The scorer uses `weight / scale`, so with scales set a weight of 1 means
    /// "one reference spread". Missing = 1 (weight applies to natural units).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scale: BTreeMap<String, f32>,
    /// How `scale` was measured (corpus, geometry, layouts); informational.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scale_basis: String,
    /// Target finger load in % (LP LR LM LI LT RT RI RM RR RP) for M02.
    #[serde(default = "default_finger_target")]
    pub finger_target: [f32; FINGERS],
    /// Keep the most frequent trigrams covering this share of trigram volume (renormalized).
    /// 1.0 = exact. Trades a little fidelity for search speed; see `context/objective.md`.
    #[serde(default = "default_trigram_coverage")]
    pub trigram_coverage: f32,
    /// How same-finger distance metrics (B03, S03) and decayed skipgrams (S02) are costed.
    #[serde(default)]
    pub sfb: SfbParams,
}

/// Distance transform for same-finger travel.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DistanceFn {
    /// `d`
    #[default]
    Linear,
    /// `d²` (Oxeylyzer/genkey "squared" finger speed)
    Squared,
    /// `log2(1 + d)` (Fitts index of difficulty with a 1u target)
    Fitts,
}

/// Parameters for same-finger costs: `cost = g(distance) · finger_factor[finger]`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SfbParams {
    pub distance: DistanceFn,
    /// Multiplier on the horizontal component of the distance (lateral moves are harder).
    pub lateral: f32,
    /// Cost multiplier per finger, pinky ring middle index thumb (higher = worse).
    pub finger_factor: [f32; 5],
    /// Skipgram weight per extra key between: S02 counts gap-2 × decay + gap-3 × decay².
    pub skip_decay: f32,
}

impl Default for SfbParams {
    fn default() -> Self {
        // finger_factor = 1 / old finger strength (0.5, 0.65, 0.9, 1.0), so B03 is unchanged.
        SfbParams { distance: DistanceFn::Linear, lateral: 1.0, finger_factor: [2.0, 1.538, 1.111, 1.0, 1.0], skip_decay: 0.5 }
    }
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
        for (k, v) in &p.scale {
            if crate::metrics::def(k).is_none() || !v.is_finite() || *v <= 0.0 {
                return Err(Error::invalid("profile", format!("scale for '{k}' must be a known metric and > 0")));
            }
        }
        let s = &p.sfb;
        let ok = s.lateral.is_finite() && s.lateral > 0.0 && s.finger_factor.iter().all(|f| f.is_finite() && *f >= 0.0);
        if !ok || !(0.0..=1.0).contains(&s.skip_decay) {
            return Err(Error::invalid("profile", "sfb: lateral must be > 0, finger_factor ≥ 0, skip_decay in [0, 1]"));
        }
        Ok(p)
    }

    pub fn weight(&self, id: &str) -> f32 {
        self.weights.get(id).copied().unwrap_or(0.0)
    }

    /// The weight the scorer applies to the metric's natural-unit cost: `weight / scale`.
    pub fn effective_weight(&self, id: &str) -> f32 {
        self.weight(id) / self.scale.get(id).copied().unwrap_or(1.0)
    }

    /// Hash of the parameters that shape metric tables and raw metric values (everything but the
    /// weights and scales). Raw vectors measured under one table hash can be re-scored exactly
    /// with any profile that has the same table hash.
    pub fn table_hash(&self) -> u64 {
        let v = serde_json::json!({"finger_target": self.finger_target, "trigram_coverage": self.trigram_coverage, "sfb": self.sfb});
        hash::fnv1a(v.to_string().as_bytes())
    }

    pub fn hash(&self) -> u64 {
        hash::fnv1a(serde_json::to_string(self).unwrap_or_default().as_bytes())
    }
}
