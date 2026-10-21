//! Metric definitions and their per-geometry cost tables. IDs follow
//! `research/metrics-catalog.md`. Each linear metric is a table indexed by slot (mono),
//! slot pair (bi, skip) or slot triple (tri), in natural units (%, or a per-keystroke
//! average). The scorer folds them with profile weights; see `context/objective.md`.
//! Some tables depend on profile parameters (`Profile::sfb`), so they are built per profile.

pub mod trigram;

use serde::Serialize;

use crate::profile::{DistanceFn, SfbParams};
use crate::{Geometry, Profile};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Mono,
    Bi,
    /// Skipgram with one key between.
    Skip,
    /// Skipgrams with two and three keys between, mixed with decaying weights (`sfb.skip_decay`).
    SkipFar,
    Tri,
    /// Bigrams of two code symbols (ASCII punctuation, e.g. `()`, `->`), normalized on their own.
    SymBi,
    /// Non-linear: computed from per-finger load accumulators.
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    /// Lower is better.
    Min,
    /// Higher is better (folded with a negative sign).
    Max,
    /// Reported only; never scored.
    Info,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct MetricDef {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub order: Order,
    pub dir: Dir,
    pub desc: &'static str,
}

const fn m(id: &'static str, name: &'static str, unit: &'static str, order: Order, dir: Dir, desc: &'static str) -> MetricDef {
    MetricDef { id, name, unit, order, dir, desc }
}

/// Every metric GECKO knows, in report order.
pub static METRICS: &[MetricDef] = &[
    m("B01", "SFB", "%", Order::Bi, Dir::Min, "Same-finger bigrams: consecutive keys on one finger (different keys)."),
    m("S01", "SFS", "%", Order::Skip, Dir::Min, "Same-finger skipgrams: keys 1 and 3 of a trigram on one finger."),
    m(
        "B03",
        "Finger speed",
        "u",
        Order::Bi,
        Dir::Min,
        "Same-finger distance per 100 bigrams: g(distance) × finger factor (profile `sfb`).",
    ),
    m(
        "S02",
        "Far SFS",
        "%",
        Order::SkipFar,
        Dir::Min,
        "Same finger with 2–3 keys between, decayed per extra key (profile `sfb.skip_decay`).",
    ),
    m("S03", "SFS distance", "u", Order::Skip, Dir::Min, "Finger speed (B03) applied to skipgrams with one key between."),
    m("B04", "LSB", "%", Order::Bi, Dir::Min, "Lateral stretch: index on a center-column key next to the middle finger."),
    m("B05", "Scissors", "%", Order::Bi, Dir::Min, "Adjacent fingers, same hand, two-row jump."),
    m(
        "B14",
        "Stretch",
        "u",
        Order::Bi,
        Dir::Min,
        "Geometric stretch between same-hand fingers per 100 bigrams (Oxeylyzer 2); covers LSB, scissors, crossings.",
    ),
    m(
        "S04",
        "Skip stretch",
        "u",
        Order::Skip,
        Dir::Min,
        "Stretch (B14) between keys 1 and 3 of a trigram per 100 skipgrams: disjoint LSB / scissors (Kühlmak).",
    ),
    m("M01", "Effort", "", Order::Mono, Dir::Min, "Average base key effort per keystroke."),
    m("M02", "Finger load error", "%", Order::Load, Dir::Min, "RMS gap between finger loads and the profile's target loads."),
    m("M04", "Left hand", "%", Order::Load, Dir::Min, "Share of keystrokes on the left hand; scored as squared distance from 50%."),
    m("M05", "Home row", "%", Order::Mono, Dir::Max, "Share of keystrokes on the home row."),
    m("M06", "Center columns", "%", Order::Mono, Dir::Min, "Share of keystrokes on the index stretch columns."),
    m("B08", "Hand alternation", "%", Order::Bi, Dir::Max, "Consecutive keys on different hands."),
    m("T01", "Alternates", "%", Order::Tri, Dir::Max, "Trigrams that alternate hands L-R-L / R-L-R (no same-finger skip)."),
    m("T02", "Alternate SFS", "%", Order::Tri, Dir::Min, "Alternating trigrams whose first and third keys share a finger."),
    m("T03", "Rolls in", "%", Order::Tri, Dir::Max, "Two same-hand keys rolling toward the thumb plus one other-hand key."),
    m("T04", "Rolls out", "%", Order::Tri, Dir::Max, "Two same-hand keys rolling away from the thumb plus one other-hand key."),
    m("T05", "One-hand rolls", "%", Order::Tri, Dir::Info, "Three same-hand keys in one direction."),
    m("T06", "Redirects", "%", Order::Tri, Dir::Min, "Three same-hand keys that change direction."),
    m("T07", "Bad redirects", "%", Order::Tri, Dir::Min, "Redirects that do not use the index finger."),
    m("T08", "SFT", "%", Order::Tri, Dir::Min, "Same-finger trigrams: all three keys on one finger."),
    m(
        "E04",
        "Engram formula",
        "%",
        Order::Bi,
        Dir::Min,
        "Bigram discomfort, mean of 1 − comfort, from Engram's hand-built pair formula (geometry `comfort.E04`).",
    ),
    m(
        "E05",
        "Engram measured",
        "%",
        Order::Bi,
        Dir::Min,
        "Bigram discomfort, mean of 1 − comfort, fitted from Engram's pairwise choices (geometry `comfort.E05`).",
    ),
    m("B02", "Same-key repeats", "%", Order::Bi, Dir::Info, "The same key twice in a row."),
    m(
        "Y01",
        "Layer switches",
        "%",
        Order::Bi,
        Dir::Min,
        "Layer-key presses per 100 bigrams (hold model: a press when the next symbol is on another non-base layer); a press right after a key on the layer key's finger counts double.",
    ),
    m(
        "Y02",
        "Symbol bigrams",
        "%",
        Order::SymBi,
        Dir::Min,
        "Code-symbol bigrams (`()`, `->`, `!=`, …) typed with one finger on two keys or across a layer change, as % of symbol bigrams.",
    ),
];

pub fn def(id: &str) -> Option<&'static MetricDef> {
    METRICS.iter().find(|d| d.id == id)
}

pub fn index_of(id: &str) -> Option<usize> {
    METRICS.iter().position(|d| d.id == id)
}

/// Per-metric cost table for a geometry and profile, aligned with [`METRICS`]. `None` for
/// `Order::Load`, and for E04/E05 when the geometry has no comfort table for them.
pub fn table(g: &Geometry, p: &Profile, d: &MetricDef) -> Option<Vec<f32>> {
    let n = g.len();
    let mono = |f: &dyn Fn(usize) -> f32| Some((0..n).map(f).collect());
    let pair = |f: &dyn Fn(usize, usize) -> f32| {
        let mut t = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                t[i * n + j] = f(i, j);
            }
        }
        Some(t)
    };
    let k = &g.key;
    // Layer copies of a key are the same physical key: no finger movement between them.
    let same_finger = |i: usize, j: usize| !g.same_key(i, j) && k[i].finger == k[j].finger;
    let same_hand = |i: usize, j: usize| k[i].hand() == k[j].hand();
    // Thumbs move independently of the finger rows, so a thumb is never part of a scissor.
    let adjacent = |i: usize, j: usize| {
        same_hand(i, j)
            && !k[i].finger.is_thumb()
            && !k[j].finger.is_thumb()
            && (k[i].finger.ord() as i8 - k[j].finger.ord() as i8).abs() == 1
    };
    let pct = |b: bool| if b { 100.0 } else { 0.0 };
    match d.id {
        "M01" => mono(&|i| k[i].effort),
        "M05" => mono(&|i| pct(k[i].home || k[i].row == home_row(g))),
        "M06" => mono(&|i| pct(k[i].stretch)),
        "B01" | "S01" | "S02" => pair(&|i, j| pct(same_finger(i, j))),
        "B02" => pair(&|i, j| pct(g.same_key(i, j))),
        "B03" | "S03" => pair(&|i, j| if same_finger(i, j) { 100.0 * finger_speed(g, &p.sfb, i, j) } else { 0.0 }),
        "B14" | "S04" => pair(&|i, j| 100.0 * stretch(g, i, j)),
        "E04" | "E05" => {
            let c = g.comfort_tables.get(d.id)?;
            pair(&|i, j| 100.0 * (1.0 - c[i * n + j]))
        }
        "B04" => pair(&|i, j| {
            let lsb = |a: usize, b: usize| k[a].stretch && k[a].finger.ord() == 3 && k[b].finger.ord() == 2;
            pct(same_hand(i, j) && (lsb(i, j) || lsb(j, i)))
        }),
        "B05" => pair(&|i, j| pct(adjacent(i, j) && (k[i].row - k[j].row).abs() >= 2)),
        "B08" => pair(&|i, j| pct(!same_hand(i, j))),
        "Y01" => pair(&|i, j| {
            let lj = k[j].layer as usize;
            if lj == 0 || k[i].layer as usize == lj {
                return 0.0;
            }
            let h = g.layer_keys[lj - 1];
            if k[i].finger == k[h].finger && g.phys(i) != h {
                200.0
            } else {
                100.0
            }
        }),
        "Y02" => pair(&|i, j| pct(same_finger(i, j) || k[i].layer != k[j].layer)),
        id if id.starts_with('T') => Some(trigram::table(g, trigram::class_for(id)?)),
        _ => None,
    }
}

/// Same-finger movement cost `g(d) · finger_factor` for slots `i`, `j` (B03, S03).
pub fn finger_speed(g: &Geometry, s: &SfbParams, i: usize, j: usize) -> f32 {
    let (a, b) = (g.key[i].pos, g.key[j].pos);
    let (dx, dy, dz) = (s.lateral * (a[0] - b[0]), a[1] - b[1], a[2] - b[2]);
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    let gd = match s.distance {
        DistanceFn::Linear => d,
        DistanceFn::Squared => d * d,
        DistanceFn::Fitts => (1.0 + d).log2(),
    };
    gd * s.finger_factor[g.key[i].finger.ord() as usize]
}

/// Oxeylyzer 2's geometric stretch (`core/src/cached_layout.rs`, commit 52b271a) for two keys
/// on the same hand and different fingers, in key units; 0 otherwise. Keys collapse to their
/// centres (edge offset 0.5); each finger's length offset shifts its keys in y; moving a finger
/// to the wrong side of its neighbour counts as a crossing. Positive only beyond the
/// `1.35 u` per finger of spread a relaxed hand covers.
pub fn stretch(g: &Geometry, i: usize, j: usize) -> f32 {
    let (a, b) = (&g.key[i], &g.key[j]);
    if a.finger == b.finger || a.hand() != b.hand() {
        return 0.0;
    }
    let (fa, fb) = (a.finger.index(), b.finger.index());
    let (xa, xb) = (a.pos[0], b.pos[0]);
    let (ya, yb) = (a.pos[1] + a.finger.length_offset(), b.pos[1] + b.finger.length_offset());
    let dx = (xa - xb).abs();
    let dy = (ya - yb).abs();
    let xo = finger_overlap(a.finger.ord(), b.finger.ord());
    // Fingers are indexed left to right across both hands, so the higher index should sit to the right.
    let crossed = if fa > fb { xa < xb + xo } else { xa + xo > xb };
    let dx = if crossed { -dx } else { dx };
    let overlap = (xo - 1.3 * dx + dy / 3.0).max(0.0);
    let s = dx.hypot(dy) + overlap - 1.35 * fa.abs_diff(fb) as f32;
    if s > 0.001 {
        s
    } else {
        0.0
    }
}

/// How far two neighbouring fingers naturally overlap sideways (Oxeylyzer 2 `x_finger_overlap`).
fn finger_overlap(oa: u8, ob: u8) -> f32 {
    match (oa.min(ob), oa.max(ob)) {
        (0, 1) => 0.8,
        (1, 2) => 0.4,
        (2, 3) => 0.1,
        (3, 4) => -2.5,
        _ => 0.0,
    }
}

/// The logical row that holds the home keys (most `home = true` keys).
fn home_row(g: &Geometry) -> i8 {
    g.key.iter().find(|k| k.home).map(|k| k.row).unwrap_or(i8::MIN)
}
