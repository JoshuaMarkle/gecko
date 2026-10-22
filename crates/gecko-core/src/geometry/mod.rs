//! Physical boards: keys with 3D position/rotation, finger assignment and base effort.
//! Format: `context/representation.md`.
//!
//! Submodules: `parametric` (zones/columns/rows → keys), `derive` (effort and stretch from
//! positions), `flat2d` (2D drawing), `collision` (2D keycap overlap), `collision3d` (3D keycap and
//! switch hulls, GJK + EPA), `pose` (key frames in 3D).

pub mod collision;
pub mod collision3d;
pub mod derive;
pub mod flat2d;
pub mod manufacture;
pub mod parametric;
pub mod pose;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{hash, Error, Result};

pub const FINGERS: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hand {
    L,
    R,
}

/// Fingers in index order LP LR LM LI LT RT RI RM RR RP.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Finger {
    LP,
    LR,
    LM,
    LI,
    LT,
    RT,
    RI,
    RM,
    RR,
    RP,
}

impl Finger {
    pub const ALL: [Finger; FINGERS] =
        [Finger::LP, Finger::LR, Finger::LM, Finger::LI, Finger::LT, Finger::RT, Finger::RI, Finger::RM, Finger::RR, Finger::RP];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn hand(self) -> Hand {
        if self.index() < 5 {
            Hand::L
        } else {
            Hand::R
        }
    }

    /// Position counted from the outside of the hand: pinky 0, ring 1, middle 2, index 3, thumb 4.
    /// A same-hand move with increasing `ord` is inward (toward the thumb).
    pub fn ord(self) -> u8 {
        let i = self.index();
        if i < 5 {
            i as u8
        } else {
            (9 - i) as u8
        }
    }

    pub fn is_thumb(self) -> bool {
        matches!(self, Finger::LT | Finger::RT)
    }

    pub fn name(self) -> &'static str {
        match self {
            Finger::LP => "LP",
            Finger::LR => "LR",
            Finger::LM => "LM",
            Finger::LI => "LI",
            Finger::LT => "LT",
            Finger::RT => "RT",
            Finger::RI => "RI",
            Finger::RM => "RM",
            Finger::RR => "RR",
            Finger::RP => "RP",
        }
    }

    /// Length offset in key units added to a key's y for this finger (longer fingers reach
    /// up more easily). Oxeylyzer 2 values; used by the B14 stretch metric.
    pub fn length_offset(self) -> f32 {
        match self.ord() {
            0 => -0.15,
            1 => 0.35,
            2 => 0.25,
            3 => -0.30,
            _ => -1.80,
        }
    }
}

fn unit_size() -> [f32; 2] {
    [1.0, 1.0]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// x, y, z in key units (1u = 19.05 mm). y grows toward the user; z up.
    pub pos: [f32; 3],
    /// Rotation about x, y, z in degrees (tilt, tent, splay). Flat boards use only z.
    #[serde(default)]
    pub rot: [f32; 3],
    #[serde(default = "unit_size")]
    pub size: [f32; 2],
    pub finger: Finger,
    pub row: i8,
    pub col: i8,
    #[serde(default)]
    pub home: bool,
    /// Index-finger stretch column (center columns on a standard board).
    #[serde(default)]
    pub stretch: bool,
    /// Home-row mod (tap-hold modifier) on this key, for Y04. Unset = the default: a home key
    /// on a finger (not thumb, not stretch column). See [`Geometry::hrm`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hrm: Option<bool>,
    /// Base effort. Optional in board files: missing (NaN) is derived from the key's distance
    /// to its finger's home key on load ([`Geometry::derive_missing`]).
    #[serde(default = "missing_effort")]
    pub effort: f32,
    /// Symbol this key gets when a layout lists only the keys without a fill (a 30-key layout
    /// on a board with thumb keys): a token name (`space`, `enter`, `bspc`, `tab`, `esc`,
    /// `none`) or a single char. Usually set on thumb and other non-alpha keys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    /// Layer of a virtual key (0 = base). Set only by [`Geometry::layered`].
    #[serde(default, skip_serializing_if = "is_zero")]
    pub layer: u8,
    /// Physical key (index into the base keys) of a virtual layer key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phys: Option<u16>,
}

fn missing_effort() -> f32 {
    f32::NAN
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

impl Key {
    pub fn hand(&self) -> Hand {
        self.finger.hand()
    }

    /// The fill as a layout symbol (`None` when unset or not a valid symbol).
    pub fn fill_symbol(&self) -> Option<char> {
        let f = self.fill.as_deref()?;
        let mut cs = f.chars();
        match (cs.next(), cs.next()) {
            (Some(c), None) => Some(c),
            _ => crate::layout::token(f),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub key: Vec<Key>,
    /// Comfort metric id (E04, E05) → key-pair comfort table in `data/comfort/<name>.csv`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub comfort: BTreeMap<String, String>,
    /// The loaded tables by metric id, `[N×N]` in 0..1 (higher = more comfortable), filled by
    /// the store from `comfort`. Serialized so the browser scorer gets them with the geometry.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub comfort_tables: BTreeMap<String, Vec<f32>>,
    /// Layered (virtual) geometries only: the physical key that holds layer `l` is
    /// `layer_keys[l - 1]`. See [`Geometry::layered`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_keys: Vec<usize>,
    /// Layered geometries only: how layer `l` is reached, `layer_modes[l - 1]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_modes: Vec<LayerMode>,
}

/// How a layer key reaches its layer. See `context/objective.md` (Y01) for the costs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerMode {
    /// Held while typing the layer's keys; the holding finger's other keys are unusable.
    #[default]
    Hold,
    /// Tapped once before each layer symbol.
    #[serde(alias = "one-shot", alias = "osl")]
    Oneshot,
    /// Tapped to enter the layer and tapped again to leave it.
    #[serde(alias = "tg")]
    Toggle,
}

impl LayerMode {
    pub const ALL: [LayerMode; 3] = [LayerMode::Hold, LayerMode::Oneshot, LayerMode::Toggle];

    pub fn name(self) -> &'static str {
        match self {
            LayerMode::Hold => "hold",
            LayerMode::Oneshot => "oneshot",
            LayerMode::Toggle => "toggle",
        }
    }
}

impl Geometry {
    /// Parse a board file. Keys without `effort` get a derived one; a board that marks no key
    /// `stretch` (true or false) gets derived stretch flags (`derive.rs`).
    pub fn from_toml(s: &str) -> Result<Self> {
        let mut g: Geometry = toml::from_str(s).map_err(|e| Error::parse("geometry", e))?;
        let raw: toml::Table = toml::from_str(s).map_err(|e| Error::parse("geometry", e))?;
        let keys = raw.get("key").and_then(|k| k.as_array());
        let stretch_given = keys.is_some_and(|ks| ks.iter().any(|k| k.get("stretch").is_some()));
        g.derive_missing(!stretch_given);
        g.validate()?;
        Ok(g)
    }

    /// TOML in the hand-written style of `data/geometries/` (defaults omitted). Base keys only.
    pub fn to_toml(&self) -> String {
        fn num(v: f32) -> String {
            let s = format!("{v}");
            if s.contains(['.', 'e', 'N', 'i']) {
                s
            } else {
                format!("{s}.0")
            }
        }
        let arr = |a: &[f32]| format!("[{}]", a.iter().map(|&v| num(v)).collect::<Vec<_>>().join(", "));
        let q = |s: &str| toml::Value::String(s.to_string()).to_string();
        let mut out = format!(
            "name = {}\ndescription = {}\n# Units: 1u = 19.05 mm. x right, y toward the user, z up.\n",
            q(&self.name),
            q(&self.description)
        );
        for k in self.key.iter().filter(|k| k.layer == 0) {
            out += &format!("\n[[key]]\npos = {}\n", arr(&k.pos));
            if k.rot != [0.0; 3] {
                out += &format!("rot = {}\n", arr(&k.rot));
            }
            if k.size != [1.0, 1.0] {
                out += &format!("size = {}\n", arr(&k.size));
            }
            out += &format!("finger = \"{}\"\nrow = {}\ncol = {}\n", k.finger.name(), k.row, k.col);
            if k.home {
                out += "home = true\n";
            }
            if k.stretch {
                out += "stretch = true\n";
            }
            out += &format!("effort = {}\n", num(k.effort));
            if let Some(f) = &k.fill {
                out += &format!("fill = {}\n", q(f));
            }
        }
        if !self.comfort.is_empty() {
            out += "\n[comfort]\n";
            for (m, t) in &self.comfort {
                out += &format!("{m} = {}\n", q(t));
            }
        }
        out
    }

    /// [`Geometry::check_keys`], and no two keys overlap: keycaps in 2D on flat boards
    /// ([`Geometry::collisions`]), keycap and switch volumes otherwise ([`Geometry::collisions_3d`]).
    pub fn validate(&self) -> Result<()> {
        self.check_keys()?;
        let hits = if self.is_flat() { self.collisions(0.0) } else { self.collisions_3d(0.0) };
        if let Some(c) = hits.first() {
            let (a, b) = (&self.key[c.a], &self.key[c.b]);
            return Err(Error::invalid(
                "geometry",
                format!(
                    "'{}': {} overlapping keycap pair(s), first keys {} (row {}, col {}) and {} (row {}, col {}) by {:.3} u",
                    self.name,
                    hits.len(),
                    c.a,
                    a.row,
                    a.col,
                    c.b,
                    b.row,
                    b.col,
                    c.depth
                ),
            ));
        }
        Ok(())
    }

    /// Structural checks: key count, comfort table sizes, unique fills. Unlike
    /// [`Geometry::validate`] it allows overlapping keys (search candidates, layer expansion).
    pub fn check_keys(&self) -> Result<()> {
        if self.key.is_empty() {
            return Err(Error::invalid("geometry", "no keys"));
        }
        if self.key.len() > crate::scorer::MAX_KEYS {
            return Err(Error::invalid(
                "geometry",
                format!("more than {} keys (layers included) is not supported", crate::scorer::MAX_KEYS),
            ));
        }
        if let Some(i) = self.key.iter().position(|k| !k.effort.is_finite()) {
            return Err(Error::invalid("geometry", format!("'{}': key {i} has no finite effort", self.name)));
        }
        if self.comfort_tables.values().any(|t| t.len() != self.len() * self.len()) {
            return Err(Error::invalid("geometry", "comfort tables must have N×N entries"));
        }
        let mut fills = std::collections::BTreeSet::new();
        for k in &self.key {
            let Some(f) = &k.fill else { continue };
            let c = k.fill_symbol().ok_or_else(|| Error::invalid("geometry", format!("'{}': unknown fill '{f}'", self.name)))?;
            if c != crate::layout::EMPTY && !fills.insert(c) {
                return Err(Error::invalid("geometry", format!("'{}': fill '{f}' appears twice", self.name)));
            }
        }
        Ok(())
    }

    /// Load a pair comfort CSV (`a_row,a_col,b_row,b_col,score,…`, `#` comments, one header
    /// line) as the table for `metric`, matching keys by (row, col). Pairs not listed score 1.0
    /// (no discomfort); same-key repeats are set to 1.0 because B02 reports them.
    pub fn load_comfort(&mut self, metric: &str, csv: &str) -> Result<()> {
        let n = self.len();
        let slot = |r: i8, c: i8| self.key.iter().position(|k| k.row == r && k.col == c);
        let mut t = vec![1.0f32; n * n];
        let bad = |line: &str| Error::invalid("comfort", format!("bad line '{line}'"));
        for line in csv.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            if f.len() < 5 {
                return Err(bad(line));
            }
            let num = |i: usize| f[i].trim().parse::<f32>().map_err(|_| bad(line));
            let (ar, ac, br, bc, score) = (num(0)? as i8, num(1)? as i8, num(2)? as i8, num(3)? as i8, num(4)?);
            if let (Some(a), Some(b)) = (slot(ar, ac), slot(br, bc)) {
                if a != b {
                    t[a * n + b] = score;
                }
            }
        }
        self.comfort_tables.insert(metric.to_string(), t);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Number of keys in each logical row, in file order. Layout rows map onto these.
    pub fn row_lengths(&self) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let mut last: Option<i8> = None;
        for k in &self.key {
            if last == Some(k.row) {
                *out.last_mut().unwrap() += 1;
            } else {
                out.push(1);
                last = Some(k.row);
            }
        }
        out
    }

    /// Physical key of slot `i` (itself unless `i` is a virtual layer key).
    pub fn phys(&self, i: usize) -> usize {
        self.key[i].phys.map_or(i, |p| p as usize)
    }

    /// Slots `i` and `j` are the same physical key (possibly on different layers).
    pub fn same_key(&self, i: usize, j: usize) -> bool {
        self.phys(i) == self.phys(j)
    }

    /// Slot `i` is a home-row mod key (Y04): a base-layer key with `hrm = true`, or, when
    /// `hrm` is unset, a home key on a finger outside the stretch columns (one per finger on
    /// the bundled boards). Layer copies never are: a layer is held, so the key has no tap-hold.
    pub fn hrm(&self, i: usize) -> bool {
        let k = &self.key[i];
        k.layer == 0 && k.hrm.unwrap_or(k.home && !k.stretch && !k.finger.is_thumb())
    }

    /// Physical key holding the layer of slot `i` (`None` on the base layer).
    pub fn holder(&self, i: usize) -> Option<usize> {
        let l = self.key[i].layer as usize;
        (l > 0).then(|| self.layer_keys[l - 1])
    }

    /// Number of physical keys (all keys unless layered).
    pub fn physical_len(&self) -> usize {
        self.key.iter().filter(|k| k.layer == 0).count()
    }

    /// The virtual geometry of a layered layout: the base keys, then for each layer `l` a copy
    /// of every usable key, with `layer = l` and `phys` set. `holders[l - 1]` is the physical
    /// key reaching layer `l` and how. A hold layer drops every key on the holding finger (it
    /// is busy holding); one-shot and toggle layers drop only the layer key itself (on a toggle
    /// layer it is the key that leaves). Every metric then sees a layered symbol at its
    /// physical key; layer-key presses are scored by Y01/Y02. Comfort tables expand by
    /// physical key; the layer key adds to effort and finger loads.
    pub fn layered(&self, holders: &[(usize, LayerMode)]) -> Result<Geometry> {
        if !self.layer_keys.is_empty() || self.key.iter().any(|k| k.layer != 0) {
            return Err(Error::invalid("geometry", "already layered"));
        }
        let mut g = self.clone();
        g.layer_keys = holders.iter().map(|h| h.0).collect();
        g.layer_modes = holders.iter().map(|h| h.1).collect();
        for (li, &(h, mode)) in holders.iter().enumerate() {
            let f = self.key.get(h).ok_or_else(|| Error::invalid("geometry", "layer key out of range"))?.finger;
            let usable = |k: usize, key: &Key| if mode == LayerMode::Hold { key.finger != f } else { k != h };
            for (k, key) in self.key.iter().enumerate().filter(|(k, key)| usable(*k, key)) {
                g.key.push(Key { layer: li as u8 + 1, phys: Some(k as u16), fill: None, ..key.clone() });
            }
        }
        let (n0, n) = (self.len(), g.len());
        for t in g.comfort_tables.values_mut() {
            let src = std::mem::take(t);
            *t = (0..n * n)
                .map(|ij| src[g.key[ij / n].phys.map_or(ij / n, |p| p as usize) * n0 + g.key[ij % n].phys.map_or(ij % n, |p| p as usize)])
                .collect();
        }
        g.check_keys()?;
        Ok(g)
    }

    /// Distance between two keys' `pos` in units (3D; equal to centre distance for equal sizes).
    pub fn dist(&self, i: usize, j: usize) -> f32 {
        let a = self.key[i].pos;
        let b = self.key[j].pos;
        let (dx, dy, dz) = (a[0] - b[0], a[1] - b[1], a[2] - b[2]);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn hash(&self) -> u64 {
        hash::fnv1a(serde_json::to_string(self).unwrap_or_default().as_bytes())
    }
}
