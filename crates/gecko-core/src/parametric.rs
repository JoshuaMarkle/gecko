//! Parametric flat boards: mirrored zones, column stagger/splay, and thumb arcs.
//! Coordinates are generated as key centers, then converted to GECKO's top-left poses.
//! See `context/geometry.md` for the transform order and `.param.toml` format.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{Error, Finger, Geometry, Hand, Key, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametricGeometry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub comfort: BTreeMap<String, String>,
    /// Shared hand transform, required by mirrored zones.
    pub split: Option<Split>,
    pub zone: Vec<Zone>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Split {
    /// Mirror plane x before adding `spread` to the right half.
    pub axis: f64,
    /// Extra separation of the right half, in u.
    #[serde(default)]
    pub spread: f64,
    /// Included angle: left rotates -angle/2, right +angle/2.
    #[serde(default)]
    pub angle: f64,
    /// Left-hand pivot in board coordinates; mirrored for the right hand.
    #[serde(default)]
    pub pivot: [f64; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub name: String,
    /// First column's first-row center, before column stagger and rotation.
    #[serde(default)]
    pub anchor: [f64; 2],
    #[serde(default)]
    pub rotate: f64,
    #[serde(default)]
    pub mirror: bool,
    /// Logical row labels, in top-to-bottom order (e.g. [0,1,2] or [3]).
    pub rows: Vec<i8>,
    #[serde(default = "one")]
    pub row_pitch: f64,
    #[serde(default = "home_row")]
    pub home_row: i8,
    /// Some existing boards tag the stretch keys as home keys too.
    #[serde(default)]
    pub home_stretch: bool,
    #[serde(default)]
    pub col_start: i8,
    /// Bend the column baseline around a circle, with arc length equal to spread.
    /// Absent = straight; negative radius bends away from the user.
    pub arc_radius: Option<f64>,
    pub column: Vec<Column>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub finger: Finger,
    /// Distance from the preceding column along the zone baseline; first is ignored.
    #[serde(default = "one")]
    pub spread: f64,
    #[serde(default)]
    pub stagger: f64,
    /// Degrees about this column's home-row key center.
    #[serde(default)]
    pub splay: f64,
    #[serde(default = "unit_size")]
    pub size: [f64; 2],
    #[serde(default)]
    pub stretch: bool,
    /// One base effort per row, in the order of `Zone::rows`.
    pub effort: Vec<f32>,
    pub fill: Option<String>,
    pub mirror_fill: Option<String>,
}

fn one() -> f64 {
    1.0
}
fn home_row() -> i8 {
    1
}
fn unit_size() -> [f64; 2] {
    [1.0, 1.0]
}
fn invalid(message: impl Into<String>) -> Error {
    Error::invalid("parametric geometry", message)
}
fn rotate([x, y]: [f64; 2], degrees: f64) -> [f64; 2] {
    let (s, c) = degrees.to_radians().sin_cos();
    [c * x - s * y, s * x + c * y]
}
fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn narrow(value: f64) -> Result<f32> {
    let value = value as f32;
    if !value.is_finite() {
        return Err(invalid("generated pose exceeds finite f32 coordinates"));
    }
    // Mirroring zero must not change a geometry's content hash to a -0 representation.
    Ok(if value == 0.0 { 0.0 } else { value })
}
fn mirrored(finger: Finger) -> Finger {
    Finger::ALL[9 - finger.index()]
}

impl ParametricGeometry {
    pub fn from_toml(text: &str) -> Result<Self> {
        let parameters: Self = toml::from_str(text).map_err(|e| Error::parse("parametric geometry", e))?;
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<()> {
        if self.zone.is_empty() {
            return Err(invalid("no zones"));
        }
        if let Some(split) = &self.split {
            if [split.axis, split.spread, split.angle, split.pivot[0], split.pivot[1]].iter().any(|v| !v.is_finite()) {
                return Err(invalid("split parameters must be finite"));
            }
        }
        let mut names = BTreeSet::new();
        let mut count = 0usize;
        for zone in &self.zone {
            let bad = |message: &str| invalid(format!("zone '{}': {message}", zone.name));
            if zone.name.is_empty() || !names.insert(&zone.name) {
                return Err(bad("zone names must be nonempty and unique"));
            }
            if zone.rows.is_empty() || zone.column.is_empty() || zone.rows.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(bad("need columns and strictly increasing row labels"));
            }
            if zone.mirror && self.split.is_none() {
                return Err(bad("mirroring requires [split]"));
            }
            let columns = zone.column.len() * if zone.mirror { 2 } else { 1 };
            if columns > crate::scorer::MAX_KEYS || zone.col_start as i16 + columns as i16 - 1 > i8::MAX as i16 {
                return Err(bad("column labels exceed i8 range"));
            }
            count += columns * zone.rows.len();
            if count > crate::scorer::MAX_KEYS {
                return Err(bad("too many keys"));
            }
            if [zone.anchor[0], zone.anchor[1], zone.rotate, zone.row_pitch].iter().any(|v| !v.is_finite()) || zone.row_pitch <= 0.0 {
                return Err(bad("anchor/rotation must be finite and row_pitch positive"));
            }
            if zone.arc_radius.is_some_and(|r| !r.is_finite() || r == 0.0) {
                return Err(bad("arc_radius must be finite and nonzero (omit it for a straight zone)"));
            }
            for (i, column) in zone.column.iter().enumerate() {
                if [column.spread, column.stagger, column.splay].iter().any(|v| !v.is_finite()) || (i > 0 && column.spread <= 0.0) {
                    return Err(bad("column spread must be positive; spread/stagger/splay must be finite"));
                }
                if column.size.iter().any(|v| !v.is_finite() || (*v as f32) <= 0.0 || *v > f32::MAX as f64) {
                    return Err(bad("key sizes must be positive finite f32 values"));
                }
                if column.effort.len() != zone.rows.len() || column.effort.iter().any(|v| !v.is_finite() || *v < 0.0) {
                    return Err(bad("need one finite nonnegative effort per row in each column"));
                }
                if zone.mirror && column.finger.hand() != Hand::L {
                    return Err(bad("mirrored columns must specify left-hand fingers"));
                }
                if !zone.mirror && column.mirror_fill.is_some() {
                    return Err(bad("mirror_fill requires a mirrored zone"));
                }
            }
        }
        Ok(())
    }

    /// Generate an ordinary, flat `Geometry`; the existing scorer and run hashes need no
    /// special path for parametric boards. `Store` subsequently resolves comfort tables.
    pub fn generate(&self) -> Result<Geometry> {
        self.validate()?;
        let mut keys = Vec::new();
        for zone in &self.zone {
            let home = zone.rows.iter().position(|row| *row == zone.home_row).unwrap_or(0) as f64;
            let mut distance = 0.0;
            for (ci, column) in zone.column.iter().enumerate() {
                if ci > 0 {
                    distance += column.spread;
                }
                let (base, tangent) = if let Some(radius) = zone.arc_radius {
                    let angle = distance / radius;
                    ([radius * angle.sin(), radius * (1.0 - angle.cos())], angle.to_degrees())
                } else {
                    ([distance, 0.0], 0.0)
                };
                let column_home = add(base, rotate([0.0, column.stagger + home * zone.row_pitch], tangent));
                for (ri, &row) in zone.rows.iter().enumerate() {
                    let local = add(column_home, rotate([0.0, (ri as f64 - home) * zone.row_pitch], tangent + column.splay));
                    let center = add(zone.anchor, rotate(local, zone.rotate));
                    let rotation = tangent + column.splay + zone.rotate;
                    for right in [false, true].into_iter().take(if zone.mirror { 2 } else { 1 }) {
                        let (mut center, mut rotation) = (center, rotation);
                        if zone.mirror {
                            let split = self.split.as_ref().expect("validated mirrored zone");
                            center = add(split.pivot, rotate([center[0] - split.pivot[0], center[1] - split.pivot[1]], -split.angle / 2.0));
                            rotation -= split.angle / 2.0;
                            if right {
                                center[0] = 2.0 * split.axis + split.spread - center[0];
                                rotation = -rotation;
                            }
                        }
                        let col = if right { 2 * zone.column.len() - 1 - ci } else { ci };
                        keys.push(Key {
                            pos: [narrow(center[0] - column.size[0] / 2.0)?, narrow(center[1] - column.size[1] / 2.0)?, 0.0],
                            rot: [0.0, 0.0, narrow(rotation)?],
                            size: [narrow(column.size[0])?, narrow(column.size[1])?],
                            finger: if right { mirrored(column.finger) } else { column.finger },
                            row,
                            col: (zone.col_start as i16 + col as i16) as i8,
                            home: row == zone.home_row && (!column.stretch || zone.home_stretch),
                            stretch: column.stretch,
                            hrm: None,
                            effort: column.effort[ri],
                            fill: if right { column.mirror_fill.clone() } else { column.fill.clone() },
                            layer: 0,
                            phys: None,
                        });
                    }
                }
            }
        }
        keys.sort_by_key(|key| (key.row, key.col));
        if keys.windows(2).any(|pair| (pair[0].row, pair[0].col) == (pair[1].row, pair[1].col)) {
            return Err(invalid("zones overlap logical (row, col) labels; assign distinct col_start values"));
        }
        let geometry = Geometry {
            name: self.name.clone(),
            description: self.description.clone(),
            key: keys,
            comfort: self.comfort.clone(),
            comfort_tables: BTreeMap::new(),
            layer_keys: Vec::new(),
            layer_modes: Vec::new(),
        };
        geometry.validate()?;
        Ok(geometry)
    }
}
