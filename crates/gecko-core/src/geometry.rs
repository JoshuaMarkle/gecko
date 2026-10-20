//! Physical boards: keys with 3D position/rotation, finger assignment and base effort.
//! Format: `context/representation.md`.

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

    /// Relative strength used to scale same-finger travel (index strongest).
    pub fn strength(self) -> f32 {
        match self.ord() {
            0 => 0.5,
            1 => 0.65,
            2 => 0.9,
            3 => 1.0,
            _ => 1.0,
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
