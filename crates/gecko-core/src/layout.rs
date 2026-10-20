//! Layouts: one symbol per geometry slot. Text form is rows of symbols matching the geometry rows.

use serde::{Deserialize, Serialize};

use crate::{hash, Error, Geometry, Result};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayoutFile {
    pub name: String,
    #[serde(default)]
    pub geometry: String,
    #[serde(default)]
    pub source: String,
    pub rows: Vec<String>,
}

/// A layout: `chars[slot]` is the symbol on geometry key `slot`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layout {
    pub name: String,
    pub geometry: String,
    #[serde(default)]
    pub source: String,
    pub chars: Vec<char>,
}

impl Layout {
    pub fn from_toml(s: &str) -> Result<Self> {
        let f: LayoutFile = toml::from_str(s).map_err(|e| Error::parse("layout", e))?;
        Self::from_rows(&f.name, &f.geometry, &f.source, &f.rows)
    }

    pub fn from_rows(name: &str, geometry: &str, source: &str, rows: &[String]) -> Result<Self> {
        let chars: Vec<char> = rows.iter().flat_map(|r| r.chars().filter(|c| !c.is_whitespace())).collect();
        Self::from_chars(name, geometry, source, chars)
    }

    /// Parse a flat string of slot-ordered symbols; whitespace is ignored.
    pub fn from_str_flat(name: &str, geometry: &str, s: &str) -> Result<Self> {
        let chars: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
        Self::from_chars(name, geometry, "", chars)
    }

    pub fn from_chars(name: &str, geometry: &str, source: &str, chars: Vec<char>) -> Result<Self> {
        let l = Layout { name: name.to_string(), geometry: geometry.to_string(), source: source.to_string(), chars };
        l.check_unique()?;
        Ok(l)
    }

    fn check_unique(&self) -> Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        for &c in &self.chars {
            if !seen.insert(c) {
                return Err(Error::invalid("layout", format!("'{}': symbol '{c}' appears twice", self.name)));
            }
        }
        Ok(())
    }

    pub fn check_geometry(&self, g: &Geometry) -> Result<()> {
        if self.chars.len() != g.len() {
            return Err(Error::invalid(
                "layout",
                format!("'{}' has {} symbols but geometry '{}' has {} keys", self.name, self.chars.len(), g.name, g.len()),
            ));
        }
        Ok(())
    }

    /// Rows split by the geometry's row lengths.
    pub fn rows(&self, g: &Geometry) -> Vec<String> {
        let mut out = Vec::new();
        let mut i = 0;
        for n in g.row_lengths() {
            out.push(self.chars[i..(i + n).min(self.chars.len())].iter().collect());
            i += n;
        }
        out
    }

    /// Single-line form, rows joined by spaces: `qwertyuiop asdfghjkl; zxcvbnm,./`.
    pub fn compact(&self, g: &Geometry) -> String {
        self.rows(g).join(" ")
    }

    pub fn to_toml(&self, g: &Geometry) -> String {
        let f = LayoutFile { name: self.name.clone(), geometry: self.geometry.clone(), source: self.source.clone(), rows: self.rows(g) };
        toml::to_string(&f).unwrap_or_default()
    }

    /// Hash of (geometry, symbols). Identifies a layout independent of its name.
    pub fn hash(&self) -> u64 {
        let s: String = std::iter::once(self.geometry.as_str()).chain(std::iter::once("|")).collect::<String>()
            + &self.chars.iter().collect::<String>();
        hash::fnv1a(s.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let g = Geometry::from_toml(include_str!("../../../data/geometries/ansi-30.toml")).unwrap();
        let l = Layout::from_toml(include_str!("../../../data/layouts/graphite.toml")).unwrap();
        l.check_geometry(&g).unwrap();
        let back = Layout::from_toml(&l.to_toml(&g)).unwrap();
        assert_eq!(l, back);
        assert_eq!(l.rows(&g).len(), 3);
    }

    #[test]
    fn duplicate_rejected() {
        assert!(Layout::from_str_flat("x", "g", "aab").is_err());
    }
}
