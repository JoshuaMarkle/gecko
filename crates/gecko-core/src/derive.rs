//! G4: per-key costs from key positions, so a new board needs no hand-entered `effort` or
//! `stretch` (`Geometry::derive_missing`). A 2D/3D reach proxy until the hand model's costs
//! (G01) replace hand-entered effort entirely. Fitted to the bundled boards; see
//! `context/geometry.md#derived-effort-and-stretch-g4`.

use crate::geometry::{Geometry, Hand};

/// Effort on a finger's home key, by `Finger::ord` (pinky, ring, middle, index, thumb): the
/// bundled boards' hand-entered values.
pub const HOME_EFFORT: [f32; 5] = [1.6, 1.3, 1.1, 1.0, 1.0];

/// Effort per key unit of distance from the finger's home key, by `Finger::ord`. Least-squares
/// fit to the hand-entered efforts of the seven bundled flat boards (r = 0.93, RMSE 0.28;
/// a quadratic term gained nothing). Thumbs: halfway between their 1.0 home and the 1.5 the
/// bundled splits give one key out.
pub const TRAVEL: [f32; 5] = [1.14, 1.15, 1.03, 1.08, 0.5];

/// Where a thumb rests when no thumb key is marked `home`: from the index home key, this far
/// inward (toward the other hand) and toward the user, in key units (median of the bundled
/// splits' cheapest thumb keys).
pub const THUMB_REST: [f32; 2] = [0.5, 2.3];

/// An index key further inward than this from the index's home key (key units, measured
/// between key centres) is on the stretch column. Between ANSI's bottom-row `V` (0.5 u, not a
/// stretch key) and its top-row `T` (0.75 u, a stretch key).
pub const STRETCH_OFFSET: f32 = 0.625;

impl Geometry {
    /// Fill what the board file left out: every key's effort when it is NaN (missing), and
    /// with `stretch` every key's stretch flag. Stretch first, since effort is measured from
    /// the non-stretch home keys.
    pub fn derive_missing(&mut self, stretch: bool) {
        if stretch {
            let flags = self.derived_stretch();
            for (k, s) in self.key.iter_mut().zip(flags) {
                k.stretch = s;
            }
        }
        if self.key.iter().any(|k| k.effort.is_nan()) {
            let efforts = self.derived_efforts();
            for (k, e) in self.key.iter_mut().zip(efforts) {
                if k.effort.is_nan() {
                    k.effort = e;
                }
            }
        }
    }

    /// Key centres in each hand's measuring frame ([`Geometry::metric_points`] plus half the
    /// key size), so 3D boards measure along their curved surface.
    fn centres(&self) -> Vec<[f32; 2]> {
        self.metric_points().iter().zip(&self.key).map(|(p, k)| [p[0] + k.size[0] / 2.0, p[1] + k.size[1] / 2.0]).collect()
    }

    /// The home key a finger's distances are measured from: its home key that is not a
    /// stretch key, else any of its home keys (the outermost for the index).
    fn home_of(&self, finger: crate::Finger, c: &[[f32; 2]]) -> Option<usize> {
        let homes: Vec<usize> =
            (0..self.len()).filter(|&i| self.key[i].phys.is_none() && self.key[i].finger == finger && self.key[i].home).collect();
        let pick = |v: &[usize]| v.iter().copied().min_by(|&a, &b| inward(self, a, c).total_cmp(&inward(self, b, c)));
        let plain: Vec<usize> = homes.iter().copied().filter(|&i| !self.key[i].stretch).collect();
        pick(&plain).or_else(|| pick(&homes))
    }

    /// Effort per key from its distance to its finger's home key:
    /// `HOME_EFFORT + TRAVEL · d` (d in key units). A finger without a home key measures from
    /// the nearest home key of its hand; a thumb without one from [`THUMB_REST`], with the
    /// thumb key nearest that point at 1.0. Layer copies take their physical key's value.
    pub fn derived_efforts(&self) -> Vec<f32> {
        let c = self.centres();
        let dist = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        let mut out = vec![0.0f32; self.len()];
        for i in 0..self.len() {
            let k = &self.key[self.phys(i)];
            let ord = k.finger.ord() as usize;
            let home = self.home_of(k.finger, &c).map(|h| dist(c[self.phys(i)], c[h]));
            let d = home.or_else(|| {
                if k.finger.is_thumb() {
                    let rest = self.thumb_rest(k.hand(), &c)?;
                    let thumbs = (0..self.len()).filter(|&j| self.key[j].phys.is_none() && self.key[j].finger == k.finger);
                    let nearest = thumbs.map(|j| dist(c[j], rest)).fold(f32::INFINITY, f32::min);
                    Some(dist(c[self.phys(i)], rest) - nearest)
                } else {
                    (0..self.len())
                        .filter(|&j| self.key[j].phys.is_none() && self.key[j].home && self.key[j].hand() == k.hand())
                        .map(|j| dist(c[self.phys(i)], c[j]))
                        .reduce(f32::min)
                }
            });
            out[i] = HOME_EFFORT[ord] + TRAVEL[ord] * d.unwrap_or(0.0).max(0.0);
        }
        out
    }

    fn thumb_rest(&self, hand: Hand, c: &[[f32; 2]]) -> Option<[f32; 2]> {
        let index = if hand == Hand::L { crate::Finger::LI } else { crate::Finger::RI };
        let h = c[self.home_of(index, c)?];
        let s = if hand == Hand::L { 1.0 } else { -1.0 };
        Some([h[0] + s * THUMB_REST[0], h[1] + THUMB_REST[1]])
    }

    /// Stretch flags from position: index keys more than [`STRETCH_OFFSET`] inward of the
    /// index's home key. Other fingers, and an index without a home key, have none.
    pub fn derived_stretch(&self) -> Vec<bool> {
        let c = self.centres();
        (0..self.len())
            .map(|i| {
                let k = &self.key[self.phys(i)];
                if k.finger.ord() != 3 || k.finger.is_thumb() {
                    return false;
                }
                let homes = (0..self.len()).filter(|&j| self.key[j].phys.is_none() && self.key[j].finger == k.finger && self.key[j].home);
                let Some(h) = homes.min_by(|&a, &b| inward(self, a, &c).total_cmp(&inward(self, b, &c))) else { return false };
                inward(self, self.phys(i), &c) - inward(self, h, &c) > STRETCH_OFFSET
            })
            .collect()
    }
}

/// How far key `i` sits toward the other hand (x for the left hand, −x for the right).
fn inward(g: &Geometry, i: usize, c: &[[f32; 2]]) -> f32 {
    if g.key[i].hand() == Hand::L {
        c[i][0]
    } else {
        -c[i][0]
    }
}
