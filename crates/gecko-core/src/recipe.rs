//! Languages as recipes: a weighted mix of component corpora, optionally with modelled
//! Backspace / arrow keystrokes, stored as `data/corpora/<name>.mix.toml` beside the built
//! `<name>.gkc` (`gecko corpus mix`). See `context/representation.md#corpus`.
//!
//! ```toml
//! description = "General English typing"
//! edits = 0.056          # ⌫ share of all keystrokes added after mixing (optional)
//! [[part]]
//! corpus = "prose-web"
//! weight = 0.5
//! ```

use serde::{Deserialize, Serialize};

use crate::{Corpus, Error, Result};

/// Seed of the Backspace / arrow models in recipe builds (fixed, so builds are reproducible).
pub const SEED: u64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Part {
    pub corpus: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Recipe {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// `⌫` share of all keystrokes (`Corpus::with_edits`), 0 = none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub edits: f64,
    /// Arrow share of all keystrokes (`Corpus::with_nav`), 0 = none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub nav: f64,
    #[serde(rename = "part")]
    pub parts: Vec<Part>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Recipe {
    pub fn from_toml(s: &str) -> Result<Self> {
        let r: Recipe = toml::from_str(s).map_err(|e| Error::parse("recipe", e))?;
        r.check()?;
        Ok(r)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).expect("recipe serializes")
    }

    pub fn check(&self) -> Result<()> {
        if self.parts.is_empty() {
            return Err(Error::invalid("recipe", "no parts"));
        }
        if let Some(p) = self.parts.iter().find(|p| !(p.weight > 0.0 && p.weight.is_finite())) {
            return Err(Error::invalid("recipe", format!("part '{}': weight must be positive", p.corpus)));
        }
        for (what, v) in [("edits", self.edits), ("nav", self.nav)] {
            if !(0.0..0.5).contains(&v) {
                return Err(Error::invalid("recipe", format!("{what} must be a share in [0, 0.5)")));
            }
        }
        Ok(())
    }

    /// Parts as `name:weight` arguments (`gecko corpus mix`).
    pub fn parse_parts(args: &[String]) -> Result<Vec<Part>> {
        args.iter()
            .map(|p| {
                let (name, w) = p.split_once(':').unwrap_or((p, "1"));
                let weight = w.parse::<f64>().map_err(|_| Error::invalid("recipe", format!("bad weight in '{p}'")))?;
                Ok(Part { corpus: name.to_string(), weight })
            })
            .collect()
    }

    /// Weights as shares of the mix (sum 1).
    pub fn shares(&self) -> Vec<f64> {
        let tot: f64 = self.parts.iter().map(|p| p.weight).sum();
        self.parts.iter().map(|p| p.weight / tot).collect()
    }

    /// Mix the loaded parts (in recipe order), then add the modelled keystrokes.
    pub fn build(&self, name: &str, parts: &[Corpus]) -> Result<Corpus> {
        self.check()?;
        let refs: Vec<(&Corpus, f64)> = parts.iter().zip(&self.parts).map(|(c, p)| (c, p.weight)).collect();
        let mut c = Corpus::mix(name, &refs)?;
        if self.edits > 0.0 {
            c = c.with_edits(name, self.edits, SEED)?;
        }
        if self.nav > 0.0 {
            c = c.with_nav(name, self.nav, SEED)?;
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_builds() {
        let r = Recipe::from_toml(
            "description = \"d\"\nedits = 0.05\n[[part]]\ncorpus = \"a\"\nweight = 3\n[[part]]\ncorpus = \"b\"\nweight = 1\n",
        )
        .unwrap();
        assert_eq!(Recipe::from_toml(&r.to_toml()).unwrap(), r);
        assert_eq!(r.shares(), vec![0.75, 0.25]);
        let mut a = Corpus::new("a", "ab\u{2423}");
        a.add_text("ab ba ab abba");
        let mut b = Corpus::new("b", "a(");
        b.add_text("(a(a");
        let m = r.build("m", &[a, b]).unwrap();
        assert_eq!(m.name, "m");
        assert!(m.index_of('(').is_some() && m.index_of(crate::layout::BACKSPACE).is_some());
        assert!(Recipe::from_toml("[[part]]\ncorpus = \"a\"\nweight = 0\n").is_err());
        assert_eq!(Recipe::parse_parts(&["x:2".into(), "y".into()]).unwrap()[1], Part { corpus: "y".into(), weight: 1.0 });
    }
}
