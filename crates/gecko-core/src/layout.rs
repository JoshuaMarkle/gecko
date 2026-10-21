//! Layouts: one symbol per geometry slot. Text form is rows of symbols matching the geometry rows.
//!
//! Symbols are chars. Special keys (Space, Enter, …) are tokens with a reserved one-char glyph
//! so every slot stays one char in strings, JSON and the web UI; text files may spell them
//! `{space}`, `{enter}`, … ([`parse_symbols`]). [`EMPTY`] marks a slot with no symbol; it may
//! appear on any number of slots and the optimizer may move symbols onto it.
//!
//! Layers: `chars` is the base layer and `layers[l - 1].chars` layer `l`, each one symbol per
//! physical key (`sym[layer][key]`). Layer `l` is held by the base key with token [`LAYER_KEYS`]
//! `[l - 1]` (`{layer1}` = `①`); the one-shot (`{os1}` = `⓵`) and toggle (`{tg1}` = `❶`)
//! tokens reach it in those modes instead ([`LayerMode`]), so a flat string carries its layer
//! modes. TOML may also give `mode` per `[[layer]]` table. For scoring, a layered layout flattens onto the virtual
//! geometry of [`Geometry::layered`]: base keys, then each layer's keys except those on the
//! holding finger ([`Layout::flat`]); the flat char string round-trips via [`Layout::from_flat`].
//!
//! Adaptive keys (`magic`): keys whose output depends on the previous symbol, with their rules
//! ([`crate::magic`]). The scorer rewrites the corpus for them; the flat char string does not
//! carry rules, so they travel separately (`Layout::magic`, run configs).

use serde::{Deserialize, Serialize};

use crate::magic::{Magic, MagicFile, MAGIC, REPEAT};
use crate::{geometry::LayerMode, hash, Error, Geometry, Result};

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
/// Shift (`⇧`): a base key held for uppercase and shifted symbols. Fixed like layer keys; a
/// layout may have several (the scorer uses the best one per symbol). Without one, Shift
/// costs nothing.
pub const SHIFT: char = '\u{21E7}'; // ⇧
/// Base-layer keys that hold layers 1, 2, 3 (`①②③`). Never moved by the optimizer.
pub const LAYER_KEYS: [char; 3] = ['\u{2460}', '\u{2461}', '\u{2462}'];
/// One-shot layer keys for layers 1, 2, 3 (`⓵⓶⓷`).
pub const ONESHOT_KEYS: [char; 3] = ['\u{24F5}', '\u{24F6}', '\u{24F7}'];
/// Toggle layer keys for layers 1, 2, 3 (`❶❷❸`).
pub const TOGGLE_KEYS: [char; 3] = ['\u{2776}', '\u{2777}', '\u{2778}'];

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
    (SHIFT, &["shift", "sft"]),
    (LAYER_KEYS[0], &["layer1", "l1"]),
    (LAYER_KEYS[1], &["layer2", "l2"]),
    (LAYER_KEYS[2], &["layer3", "l3"]),
    (ONESHOT_KEYS[0], &["os1", "oneshot1"]),
    (ONESHOT_KEYS[1], &["os2", "oneshot2"]),
    (ONESHOT_KEYS[2], &["os3", "oneshot3"]),
    (TOGGLE_KEYS[0], &["tg1", "toggle1"]),
    (TOGGLE_KEYS[1], &["tg2", "toggle2"]),
    (TOGGLE_KEYS[2], &["tg3", "toggle3"]),
    (MAGIC, &["magic", "mgc"]),
    (REPEAT, &["repeat", "rep"]),
];

/// The token of the key reaching layer `l` (1-based) in `mode`.
pub fn layer_key(l: usize, mode: LayerMode) -> char {
    match mode {
        LayerMode::Hold => LAYER_KEYS[l - 1],
        LayerMode::Oneshot => ONESHOT_KEYS[l - 1],
        LayerMode::Toggle => TOGGLE_KEYS[l - 1],
    }
}

/// Layer (1-based) and mode of a layer-key token.
pub fn layer_key_info(c: char) -> Option<(usize, LayerMode)> {
    LayerMode::ALL.into_iter().find_map(|m| (1..=3).find(|&l| layer_key(l, m) == c).map(|l| (l, m)))
}

pub fn is_layer_key(c: char) -> bool {
    layer_key_info(c).is_some()
}

/// Base key reaching layer `l` (1-based) and its mode, if the base has one.
fn find_layer_key(base: &[char], l: usize) -> Option<(usize, LayerMode)> {
    base.iter().enumerate().find_map(|(k, &c)| layer_key_info(c).filter(|i| i.0 == l).map(|i| (k, i.1)))
}

/// Keys the optimizer never moves: layer keys and Shift.
pub fn is_fixed_key(c: char) -> bool {
    is_layer_key(c) || c == SHIFT
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
    /// Adaptive keys (`[[magic]]` tables).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub magic: Vec<MagicFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayerFile {
    #[serde(default)]
    pub name: String,
    /// How the layer is reached; sets the base layer key's token (`hold` when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<LayerMode>,
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
    /// Adaptive keys; each key's glyph is on the layout.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub magic: Vec<Magic>,
}

impl Layout {
    pub fn from_toml(s: &str) -> Result<Self> {
        let f: LayoutFile = toml::from_str(s).map_err(|e| Error::parse("layout", e))?;
        let mut l = Self::from_rows(&f.name, &f.geometry, &f.source, &f.rows)?;
        for (i, lf) in f.layer.iter().enumerate() {
            let name = if lf.name.is_empty() { format!("layer{}", i + 1) } else { lf.name.clone() };
            l.layers.push(Layer { name, chars: parse_symbols(&lf.rows.concat())? });
            if let (Some(m), Some((k, cur))) = (lf.mode, find_layer_key(&l.chars, i + 1)) {
                if cur != LayerMode::Hold && cur != m {
                    return Err(Error::invalid(
                        "layout",
                        format!("'{}': layer {} has mode {} but its key is {}", l.name, i + 1, m.name(), cur.name()),
                    ));
                }
                l.chars[k] = layer_key(i + 1, m);
            }
        }
        l.magic = f.magic.iter().map(Magic::from_file).collect::<Result<_>>()?;
        l.check_unique()?;
        l.check_magic()?;
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
        let l = Layout {
            name: name.to_string(),
            geometry: geometry.to_string(),
            source: source.to_string(),
            chars,
            layers: Vec::new(),
            magic: Vec::new(),
        };
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
        let holders: Vec<_> = (1..=LAYER_KEYS.len()).map_while(|l| find_layer_key(&flat[..n], l)).collect();
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

    /// Base keys reaching layers 1, 2, … (where the layer-key tokens sit) and their modes.
    pub fn holders(&self) -> Result<Vec<(usize, LayerMode)>> {
        (0..self.layers.len())
            .map(|i| {
                find_layer_key(&self.chars, i + 1).ok_or_else(|| {
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
                    format!("'{}': layer {} puts '{}' on a key it cannot use", self.name, li + 1, layer.chars[k]),
                ));
            }
        }
        Ok((vg, flat))
    }

    /// Symbols are unique except `·` and `⇧`; `⇧` sits on the base only.
    fn check_unique(&self) -> Result<()> {
        if self.layers.iter().any(|l| l.chars.contains(&SHIFT)) {
            return Err(Error::invalid("layout", format!("'{}': Shift (⇧) belongs on the base layer", self.name)));
        }
        let mut seen = std::collections::BTreeSet::new();
        for &c in self.chars.iter().chain(self.layers.iter().flat_map(|l| l.chars.iter())).filter(|&&c| c != EMPTY && c != SHIFT) {
            if !seen.insert(c) {
                return Err(Error::invalid("layout", format!("'{}': symbol '{c}' appears twice", self.name)));
            }
        }
        Ok(())
    }

    /// Every adaptive key is on the layout once (else its presses would be dropped).
    pub fn check_magic(&self) -> Result<()> {
        for m in &self.magic {
            m.check()?;
            if !self.chars.iter().chain(self.layers.iter().flat_map(|l| l.chars.iter())).any(|&c| c == m.key) {
                return Err(Error::invalid("layout", format!("'{}': adaptive key '{}' is not on the layout", self.name, m.key)));
            }
        }
        Ok(())
    }

    /// These adaptive keys, checked against the layout.
    pub fn with_magic(mut self, magic: Vec<Magic>) -> Result<Self> {
        self.magic = magic;
        self.check_magic()?;
        Ok(self)
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
            return Ok(Layout { source: self.source.clone(), magic: self.magic.clone(), ..l });
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
            layers: self
                .layers
                .iter()
                .map(|ly| Layer { name: ly.name.clone(), chars: fit(&ly.chars, &|k| k.fill.as_ref().map(|_| EMPTY)) })
                .collect(),
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
        let modes = self.holders().unwrap_or_default();
        let mode = |li: usize| modes.get(li).map(|h| h.1).filter(|&m| m != LayerMode::Hold);
        let layer = self
            .layers
            .iter()
            .zip(self.layer_rows(g))
            .enumerate()
            .map(|(li, (ly, rows))| LayerFile { name: ly.name.clone(), mode: mode(li), rows })
            .collect();
        let f = LayoutFile {
            name: self.name.clone(),
            geometry: self.geometry.clone(),
            source: self.source.clone(),
            rows: self.rows(g),
            layer,
            magic: self.magic.iter().map(Magic::to_file).collect(),
        };
        toml::to_string(&f).unwrap_or_default()
    }

    /// Hash of (geometry, symbols, adaptive rules). Identifies a layout independent of its name.
    pub fn hash(&self) -> u64 {
        let mut s: String = std::iter::once(self.geometry.as_str()).chain(std::iter::once("|")).collect::<String>()
            + &self.chars.iter().chain(self.layers.iter().flat_map(|l| l.chars.iter())).collect::<String>();
        for m in &self.magic {
            s += &format!("|{}{}:{}", m.key, if m.repeat { "+" } else { "" }, crate::magic::format_rules(&m.rules));
        }
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
        assert!(Layout::from_str_flat("x", "g", "a{hyper}").is_err());
        assert_eq!(Layout::from_str_flat("x", "g", "a{shift}{sft}").unwrap().chars, vec!['a', SHIFT, SHIFT]);
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
