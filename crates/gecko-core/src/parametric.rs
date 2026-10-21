//! Parametric boards: mirrored zones, column stagger/splay, thumb arcs, and optional 3D
//! shape (Dactyl-style row/column curvature, column lift, tent, tilt).
//! Coordinates are generated as key centers, then converted to GECKO's top-left poses.
//! See `context/geometry.md` for the transform order and `.param.toml` format.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::pose;
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
    /// Degrees; raises each half's inner edge (left half about +y through `pivot`).
    #[serde(default)]
    pub tent: f64,
    /// Degrees; positive raises the far (top-row) edge, negative is the "negative slope".
    #[serde(default)]
    pub tilt: f64,
    /// Height added to both halves, in u.
    #[serde(default)]
    pub lift: f64,
}

impl Split {
    fn is_3d(&self) -> bool {
        self.tent != 0.0 || self.tilt != 0.0 || self.lift != 0.0
    }
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
    /// Degrees between successive rows: rows sit on an arc through the home row, with the
    /// keycap tops `row_pitch` apart (a concave column, Dactyl's α). 0 = flat.
    #[serde(default)]
    pub row_curve: f64,
    /// Degrees between successive columns: columns sit on an arc with their keycap tops
    /// `spread` apart (Dactyl's β). Not combined with `arc_radius`. 0 = flat.
    #[serde(default)]
    pub col_curve: f64,
    /// Column index (may be fractional) left level by `col_curve`.
    #[serde(default)]
    pub center_col: f64,
    /// Degrees about the anchor, after `rotate`: raise the zone's +x side.
    #[serde(default)]
    pub tent: f64,
    /// Degrees about the anchor, after `tent`: raise the zone's far edge.
    #[serde(default)]
    pub tilt: f64,
    /// Height of the anchor, in u.
    #[serde(default)]
    pub lift: f64,
    pub column: Vec<Column>,
}

impl Zone {
    fn is_3d(&self) -> bool {
        self.row_curve != 0.0
            || self.col_curve != 0.0
            || self.tent != 0.0
            || self.tilt != 0.0
            || self.lift != 0.0
            || self.column.iter().any(|c| c.lift != 0.0)
    }
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
    /// Height of this column, in u (Dactyl's per-column z offset).
    #[serde(default)]
    pub lift: f64,
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
/// Per column: (baseline x, height) of its home keycap top on the `col_curve` arc. Successive
/// columns are `spread` apart along chords whose direction bisects the two columns' tilts, so
/// equal spreads lie exactly on a circle (Dactyl's column radius). Flat zones: (distance, 0).
fn column_curve(zone: &Zone) -> Vec<(f64, f64)> {
    let tilt = |i: usize| (zone.col_curve * (i as f64 - zone.center_col)).to_radians();
    let mut out = vec![(0.0, 0.0)];
    for i in 1..zone.column.len() {
        let (x, z) = out[i - 1];
        let mid = (tilt(i - 1) + tilt(i)) / 2.0;
        let s = zone.column[i].spread;
        out.push(if zone.col_curve == 0.0 { (x + s, 0.0) } else { (x + s * mid.cos(), z + s * mid.sin()) });
    }
    out
}

/// A key `k` rows from home (signed, + toward the user) on the `row_curve` arc: offset of its
/// keycap top from the home key's, and its tilt in degrees. Adjacent tops are `row_pitch` apart.
fn row_offset(zone: &Zone, k: f64) -> ([f64; 3], f64) {
    if zone.row_curve == 0.0 {
        return ([0.0, k * zone.row_pitch, 0.0], 0.0);
    }
    let r = zone.row_pitch / (2.0 * (zone.row_curve.to_radians() / 2.0).sin());
    let theta = k * zone.row_curve;
    let (s, c) = theta.to_radians().sin_cos();
    ([0.0, r * s, r * (1.0 - c)], theta)
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
            if [split.axis, split.spread, split.angle, split.pivot[0], split.pivot[1], split.tent, split.tilt, split.lift]
                .iter()
                .any(|v| !v.is_finite())
            {
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
            if [zone.row_curve, zone.col_curve, zone.center_col, zone.tent, zone.tilt, zone.lift].iter().any(|v| !v.is_finite())
                || zone.row_curve.abs() >= 60.0
                || zone.col_curve.abs() >= 60.0
            {
                return Err(bad("3D parameters must be finite; curvatures below 60° per step"));
            }
            if zone.col_curve != 0.0 && zone.arc_radius.is_some() {
                return Err(bad("col_curve and arc_radius cannot be combined"));
            }
            for (i, column) in zone.column.iter().enumerate() {
                if [column.spread, column.stagger, column.splay, column.lift].iter().any(|v| !v.is_finite())
                    || (i > 0 && column.spread <= 0.0)
                {
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
            let split = if zone.mirror { self.split.as_ref() } else { None };
            let three_d = zone.is_3d() || split.is_some_and(Split::is_3d);
            let lifts = column_curve(zone);
            let mut distance = 0.0;
            for (ci, column) in zone.column.iter().enumerate() {
                if ci > 0 {
                    distance += column.spread;
                }
                let (base, tangent) = if let Some(radius) = zone.arc_radius {
                    let angle = distance / radius;
                    ([radius * angle.sin(), radius * (1.0 - angle.cos())], angle.to_degrees())
                } else if zone.col_curve != 0.0 {
                    ([lifts[ci].0, 0.0], 0.0)
                } else {
                    ([distance, 0.0], 0.0)
                };
                let column_home = add(base, rotate([0.0, column.stagger + home * zone.row_pitch], tangent));
                for (ri, &row) in zone.rows.iter().enumerate() {
                    let local = add(column_home, rotate([0.0, (ri as f64 - home) * zone.row_pitch], tangent + column.splay));
                    let center = add(zone.anchor, rotate(local, zone.rotate));
                    let rotation = tangent + column.splay + zone.rotate;
                    // 3D boards: the keycap top centre and orientation (see `pose`).
                    let pose3 = three_d.then(|| {
                        let q =
                            pose::mul(&pose::rot_z(tangent + column.splay), &pose::rot_y(zone.col_curve * (ci as f64 - zone.center_col)));
                        let (offset, theta) = row_offset(zone, ri as f64 - home);
                        let home3 = [column_home[0], column_home[1], lifts[ci].1 + column.lift];
                        let local = pose::add(home3, pose::apply(&q, offset));
                        let z = pose::mul(&pose::mul(&pose::rot_x(zone.tilt), &pose::rot_y(zone.tent)), &pose::rot_z(zone.rotate));
                        let mut top = pose::add([zone.anchor[0], zone.anchor[1], pose::CAP_HEIGHT + zone.lift], pose::apply(&z, local));
                        let mut m = pose::mul(&z, &pose::mul(&q, &pose::rot_x(-theta)));
                        if let Some(split) = split {
                            let pivot = [split.pivot[0], split.pivot[1], pose::CAP_HEIGHT];
                            let s =
                                pose::mul(&pose::mul(&pose::rot_x(split.tilt), &pose::rot_y(split.tent)), &pose::rot_z(-split.angle / 2.0));
                            top = pose::add(
                                pose::add(pivot, pose::apply(&s, [top[0] - pivot[0], top[1] - pivot[1], top[2] - pivot[2]])),
                                [0.0, 0.0, split.lift],
                            );
                            m = pose::mul(&s, &m);
                        }
                        (top, m)
                    });
                    for right in [false, true].into_iter().take(if zone.mirror { 2 } else { 1 }) {
                        let (pos, rot) = if let Some((top, m)) = pose3 {
                            let mut pos = pose::pos_from_contact(top, &m, column.size);
                            let mut rot = pose::euler(&m);
                            if right {
                                let split = split.expect("validated mirrored zone");
                                // Mirror the key centre, then return to the top-left convention.
                                pos[0] = 2.0 * split.axis + split.spread - pos[0] - column.size[0];
                                rot = pose::mirror_rot(rot);
                            }
                            ([narrow(pos[0])?, narrow(pos[1])?, narrow(pos[2])?], [narrow(rot[0])?, narrow(rot[1])?, narrow(rot[2])?])
                        } else {
                            let (mut center, mut rotation) = (center, rotation);
                            if let Some(split) = split {
                                center =
                                    add(split.pivot, rotate([center[0] - split.pivot[0], center[1] - split.pivot[1]], -split.angle / 2.0));
                                rotation -= split.angle / 2.0;
                                if right {
                                    center[0] = 2.0 * split.axis + split.spread - center[0];
                                    rotation = -rotation;
                                }
                            }
                            (
                                [narrow(center[0] - column.size[0] / 2.0)?, narrow(center[1] - column.size[1] / 2.0)?, 0.0],
                                [0.0, 0.0, narrow(rotation)?],
                            )
                        };
                        let col = if right { 2 * zone.column.len() - 1 - ci } else { ci };
                        keys.push(Key {
                            pos,
                            rot,
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
