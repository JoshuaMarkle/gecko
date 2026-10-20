//! Physical boards: keys with 3D position/rotation, finger assignment and base effort.
//! Format: `context/representation.md`.

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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub effort: f32,
}

impl Key {
    pub fn hand(&self) -> Hand {
        self.finger.hand()
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
}

impl Geometry {
    pub fn from_toml(s: &str) -> Result<Self> {
        let g: Geometry = toml::from_str(s).map_err(|e| Error::parse("geometry", e))?;
        g.validate()?;
        Ok(g)
    }

    pub fn validate(&self) -> Result<()> {
        if self.key.is_empty() {
            return Err(Error::invalid("geometry", "no keys"));
        }
        if self.key.len() > 64 {
            return Err(Error::invalid("geometry", "more than 64 keys is not supported yet"));
        }
        if self.comfort_tables.values().any(|t| t.len() != self.len() * self.len()) {
            return Err(Error::invalid("geometry", "comfort tables must have N×N entries"));
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

    /// Planar distance between key centers in units.
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
