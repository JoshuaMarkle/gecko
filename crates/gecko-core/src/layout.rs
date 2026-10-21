//! Layouts: one symbol per geometry slot. Text form is rows of symbols matching the geometry rows.
//!
//! Symbols are chars. Special keys (Space, Enter, …) are tokens with a reserved one-char glyph
//! so every slot stays one char in strings, JSON and the web UI; text files may spell them
//! `{space}`, `{enter}`, … ([`parse_symbols`]). [`EMPTY`] marks a slot with no symbol; it may
//! appear on any number of slots and the optimizer may move symbols onto it.
//!
//! Layers: `chars` is the base layer and `layers[l - 1].chars` layer `l`, each one symbol per
//! physical key (`sym[layer][key]`). Layer `l` is held by the base key with token [`LAYER_KEYS`]
//! `[l - 1]` (`{layer1}` = `①`). For scoring, a layered layout flattens onto the virtual
//! geometry of [`Geometry::layered`]: base keys, then each layer's keys except those on the
//! holding finger ([`Layout::flat`]); the flat char string round-trips via [`Layout::from_flat`].

use serde::{Deserialize, Serialize};

use crate::{hash, Error, Geometry, Result};

pub const SPACE: char = '\u{2423}'; // ␣
pub const ENTER: char = '\u{23CE}'; // ⏎
pub const BACKSPACE: char = '\u{232B}'; // ⌫
pub const TAB: char = '\u{21E5}'; // ⇥
pub const ESC: char = '\u{238B}'; // ⎋
pub const LEFT: char = '\u{2190}'; // ←
pub const UP: char = '\u{2191}'; // ↑
pub const RIGHT: char = '\u{2192}'; // →
pub const DOWN: char = '\u{2193}'; // ↓
/// Arrow keys in Vim order (h j k l).
pub const ARROWS: [char; 4] = [LEFT, DOWN, UP, RIGHT];
/// A slot with no symbol (zero frequency). Not unique: a layout may have several.
pub const EMPTY: char = '\u{00B7}'; // ·
/// Base-layer keys that hold layers 1, 2, 3 (`①②③`). Never moved by the optimizer.
pub const LAYER_KEYS: [char; 3] = ['\u{2460}', '\u{2461}', '\u{2462}'];

/// Token glyphs and their names (first name is canonical).
pub const TOKENS: &[(char, &[&str])] = &[
    (SPACE, &["space", "spc"]),
    (ENTER, &["enter", "ent", "ret"]),
    (BACKSPACE, &["bspc", "backspace"]),
    (TAB, &["tab"]),
    (ESC, &["esc", "escape"]),
    (LEFT, &["left"]),
    (DOWN, &["down"]),
    (UP, &["up"]),
    (RIGHT, &["right"]),
    (EMPTY, &["none", "empty", "_"]),
    (LAYER_KEYS[0], &["layer1", "l1"]),
    (LAYER_KEYS[1], &["layer2", "l2"]),
    (LAYER_KEYS[2], &["layer3", "l3"]),
];

pub fn is_layer_key(c: char) -> bool {
    LAYER_KEYS.contains(&c)
}

/// Glyph for a token name (case-insensitive), e.g. `space` → `␣`.
pub fn token(name: &str) -> Option<char> {
    let name = name.to_ascii_lowercase();
    TOKENS.iter().find(|(_, names)| names.contains(&name.as_str())).map(|&(c, _)| c)
}

/// Canonical token name of a glyph, e.g. `␣` → `space`.
pub fn token_name(c: char) -> Option<&'static str> {
    TOKENS.iter().find(|&&(g, _)| g == c).map(|(_, names)| names[0])
}

/// Slot-ordered symbols from text: one char per symbol, `{name}` for a token, whitespace
/// ignored. A brace not around a name (`{}`, `{(`) is the symbol itself.
pub fn parse_symbols(s: &str) -> Result<Vec<char>> {
    let cs: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '{' {
            let len = cs[i + 1..].iter().take_while(|c| c.is_ascii_alphanumeric() || **c == '_').count();
            if len > 0 && cs.get(i + 1 + len) == Some(&'}') {
                let name: String = cs[i + 1..i + 1 + len].iter().collect();
                out.push(token(&name).ok_or_else(|| Error::invalid("layout", format!("unknown token '{{{name}}}'")))?);
                i += len + 2;
                continue;
            }
        }
        out.push(cs[i]);
        i += 1;
    }
    Ok(out)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayoutFile {
    pub name: String,
    #[serde(default)]
    pub geometry: String,
    #[serde(default)]
    pub source: String,
    pub rows: Vec<String>,
    /// Layers 1, 2, … (`[[layer]]` tables), each with rows like the base.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer: Vec<LayerFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayerFile {
    #[serde(default)]
    pub name: String,
    pub rows: Vec<String>,
}

/// A layer above the base: `chars[key]` per physical key, [`EMPTY`] where unused.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub name: String,
    pub chars: Vec<char>,
}

/// A layout: `chars[slot]` is the symbol on geometry key `slot` (the base layer); `layers`
/// hold the symbols reached through layer keys.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Layout {
    pub name: String,
    pub geometry: String,
    #[serde(default)]
    pub source: String,
    pub chars: Vec<char>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
}

impl Layout {
    pub fn from_toml(s: &str) -> Result<Self> {
        let f: LayoutFile = toml::from_str(s).map_err(|e| Error::parse("layout", e))?;
        let mut l = Self::from_rows(&f.name, &f.geometry, &f.source, &f.rows)?;
        for (i, lf) in f.layer.iter().enumerate() {
            let name = if lf.name.is_empty() { format!("layer{}", i + 1) } else { lf.name.clone() };
            l.layers.push(Layer { name, chars: parse_symbols(&lf.rows.concat())? });
        }
        l.check_unique()?;
        Ok(l)
    }

    pub fn from_rows(name: &str, geometry: &str, source: &str, rows: &[String]) -> Result<Self> {
        Self::from_chars(name, geometry, source, parse_symbols(&rows.concat())?)
    }

    /// Parse a flat string of slot-ordered symbols ([`parse_symbols`]).
    pub fn from_str_flat(name: &str, geometry: &str, s: &str) -> Result<Self> {
        Self::from_chars(name, geometry, "", parse_symbols(s)?)
    }

    pub fn from_chars(name: &str, geometry: &str, source: &str, chars: Vec<char>) -> Result<Self> {
        let l = Layout { name: name.to_string(), geometry: geometry.to_string(), source: source.to_string(), chars, layers: Vec::new() };
        l.check_unique()?;
        Ok(l)
    }

    /// Rebuild a layout from slot-ordered symbols over `g` or, when longer, over the virtual
    /// geometry of a layered layout on `g` (holders found from the layer-key tokens).
    pub fn from_flat(name: &str, g: &Geometry, flat: Vec<char>) -> Result<Self> {
        let n = g.len();
        if flat.len() <= n {
            return Self::from_chars(name, &g.name, "", flat);
        }
        let holders: Vec<usize> = LAYER_KEYS.iter().map_while(|t| flat[..n].iter().position(|c| c == t)).collect();
        let vg = g.layered(&holders)?;
        if vg.len() != flat.len() {
            return Err(Error::invalid(
                "layout",
                format!("'{name}': {} symbols do not fit '{}' with {} layers ({} slots)", flat.len(), g.name, holders.len(), vg.len()),
            ));
        }
        let mut l = Self::from_chars(name, &g.name, "", flat[..n].to_vec())?;
        for li in 1..=holders.len() {
            let mut chars = vec![EMPTY; n];
            for v in (n..vg.len()).filter(|&v| vg.key[v].layer as usize == li) {
                chars[vg.phys(v)] = flat[v];
            }
            l.layers.push(Layer { name: format!("layer{li}"), chars });
        }
        l.check_unique()?;
        Ok(l)
    }

    /// Base keys holding layers 1, 2, … (where the layer-key tokens sit).
    pub fn holders(&self) -> Result<Vec<usize>> {
        (0..self.layers.len())
            .map(|i| {
                self.chars.iter().position(|&c| c == LAYER_KEYS[i]).ok_or_else(|| {
                    Error::invalid("layout", format!("'{}' has layer {} but no {{layer{}}} key on the base", self.name, i + 1, i + 1))
                })
            })
            .collect()
    }

    /// The geometry and slot-ordered symbols the scorer works on: `g` and `chars` without
    /// layers, else the virtual geometry ([`Geometry::layered`]) and the flattened symbols.
    pub fn flat(&self, g: &Geometry) -> Result<(Geometry, Vec<char>)> {
        self.check_geometry(g)?;
        if self.layers.is_empty() {
            return Ok((g.clone(), self.chars.clone()));
        }
        let vg = g.layered(&self.holders()?)?;
        let mut flat = self.chars.clone();
        let mut placed = vec![vec![false; g.len()]; self.layers.len()];
        for v in g.len()..vg.len() {
            let (li, k) = (vg.key[v].layer as usize - 1, vg.phys(v));
            flat.push(self.layers[li].chars[k]);
            placed[li][k] = true;
        }
        for (li, layer) in self.layers.iter().enumerate() {
            if let Some(k) = (0..g.len()).find(|&k| !placed[li][k] && layer.chars[k] != EMPTY) {
                return Err(Error::invalid(
                    "layout",
                    format!("'{}': layer {} puts '{}' on a key of the finger that holds it", self.name, li + 1, layer.chars[k]),
                ));
            }
        }
        Ok((vg, flat))
    }

    fn check_unique(&self) -> Result<()> {
        let mut seen = std::collections::BTreeSet::new();
        for &c in self.chars.iter().chain(self.layers.iter().flat_map(|l| l.chars.iter())).filter(|&&c| c != EMPTY) {
            if !seen.insert(c) {
                return Err(Error::invalid("layout", format!("'{}': symbol '{c}' appears twice", self.name)));
            }
        }
        Ok(())
    }

    pub fn check_geometry(&self, g: &Geometry) -> Result<()> {
        for chars in std::iter::once(&self.chars).chain(self.layers.iter().map(|l| &l.chars)) {
            if chars.len() != g.len() {
                return Err(Error::invalid(
                    "layout",
                    format!("'{}' has {} symbols but geometry '{}' has {} keys", self.name, chars.len(), g.name, g.len()),
                ));
            }
        }
        Ok(())
    }

    /// This layout on `g`. A layout with one symbol per key is unchanged; one with a symbol for
    /// each key without a `fill` (e.g. a 30-key layout on a board with thumb keys) gets the
    /// fills on the other keys.
    /// Layers fit the same way, with [`EMPTY`] on the filled keys. A flat layered string
    /// (longer than the board, as runs store them) is split into its layers.
    pub fn fit(&self, g: &Geometry) -> Result<Layout> {
        if self.layers.is_empty() && self.chars.len() > g.len() {
            let l = Self::from_flat(&self.name, g, self.chars.clone())?;
            return Ok(Layout { source: self.source.clone(), ..l });
        }
        let open = g.key.iter().filter(|k| k.fill.is_none()).count();
        let fit = |chars: &[char], fill: &dyn Fn(&crate::Key) -> Option<char>| -> Vec<char> {
            if chars.len() != open || chars.len() == g.len() {
                return chars.to_vec();
            }
            let mut rest = chars.iter();
            g.key.iter().map(|k| fill(k).unwrap_or_else(|| *rest.next().unwrap())).collect()
        };
        let l = Layout {
            geometry: g.name.clone(),
            chars: fit(&self.chars, &|k| k.fill_symbol()),
            layers: self.layers.iter().map(|ly| Layer { name: ly.name.clone(), chars: fit(&ly.chars, &|k| k.fill.as_ref().map(|_| EMPTY)) }).collect(),
            ..self.clone()
        };
        l.check_geometry(g)?;
        l.check_unique()?;
        Ok(l)
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

    /// Rows of each layer above the base, split like [`Layout::rows`].
    pub fn layer_rows(&self, g: &Geometry) -> Vec<Vec<String>> {
        self.layers.iter().map(|ly| Layout { chars: ly.chars.clone(), layers: Vec::new(), ..self.clone() }.rows(g)).collect()
    }

    /// Base rows followed by every layer's rows (display and run output).
    pub fn all_rows(&self, g: &Geometry) -> Vec<String> {
        let mut out = self.rows(g);
        out.extend(self.layer_rows(g).into_iter().flatten());
        out
    }

    /// Single-line form, rows joined by spaces: `qwertyuiop asdfghjkl; zxcvbnm,./`; layers
    /// follow after ` | `.
    pub fn compact(&self, g: &Geometry) -> String {
        std::iter::once(self.rows(g).join(" ")).chain(self.layer_rows(g).iter().map(|r| r.join(" "))).collect::<Vec<_>>().join(" | ")
    }

    pub fn to_toml(&self, g: &Geometry) -> String {
        let layer = self.layers.iter().zip(self.layer_rows(g)).map(|(ly, rows)| LayerFile { name: ly.name.clone(), rows }).collect();
        let f =
            LayoutFile { name: self.name.clone(), geometry: self.geometry.clone(), source: self.source.clone(), rows: self.rows(g), layer };
        toml::to_string(&f).unwrap_or_default()
    }

    /// Hash of (geometry, symbols). Identifies a layout independent of its name.
    pub fn hash(&self) -> u64 {
        let s: String = std::iter::once(self.geometry.as_str()).chain(std::iter::once("|")).collect::<String>()
            + &self.chars.iter().chain(self.layers.iter().flat_map(|l| l.chars.iter())).collect::<String>();
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
        assert!(Layout::from_str_flat("x", "g", "a{spc}␣").is_err());
    }

    #[test]
    fn tokens_and_empty_slots() {
        let l = Layout::from_str_flat("x", "g", "ab {space}{Enter} {none}{_}·").unwrap();
        assert_eq!(l.chars, vec!['a', 'b', SPACE, ENTER, EMPTY, EMPTY, EMPTY]);
        assert!(Layout::from_str_flat("x", "g", "a{shift}").is_err());
        assert_eq!(parse_symbols("{}{(}{l1}").unwrap(), vec!['{', '}', '{', '(', '}', LAYER_KEYS[0]]);
        assert_eq!(token_name(BACKSPACE), Some("bspc"));
    }

    #[test]
    fn fit_fills_thumbs() {
        let g = Geometry::from_toml(include_str!("../../../data/geometries/corne-36.toml")).unwrap();
        let l = Layout::from_toml(include_str!("../../../data/layouts/colemak-dh.toml")).unwrap();
        let f = l.fit(&g).unwrap();
        f.check_geometry(&g).unwrap();
        assert_eq!(f.rows(&g)[0], "qwfpbjluy;");
        assert_eq!(f.rows(&g)[3], "⎋␣⇥⏎⌫·");
        let back = Layout::from_toml(&f.to_toml(&g)).unwrap();
        assert_eq!(back.chars, f.chars);
        assert_eq!(f.fit(&g).unwrap(), f);
        assert!(Layout::from_str_flat("x", "g", "abc").unwrap().fit(&g).is_err());
    }
}
