//! Metric definitions and their per-geometry cost tables. IDs follow
//! `research/metrics-catalog.md`. Each linear metric is a table indexed by slot (mono),
//! slot pair (bi, skip) or slot triple (tri), in natural units (%, or a per-keystroke
//! average). The scorer folds them with profile weights; see `context/objective.md`.

pub mod trigram;

use serde::Serialize;

use crate::Geometry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Mono,
    Bi,
    Skip,
    Tri,
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
    m("B03", "Finger travel", "u", Order::Bi, Dir::Min, "Same-finger distance per bigram, scaled by finger strength (×100)."),
    m("B04", "LSB", "%", Order::Bi, Dir::Min, "Lateral stretch: index on a center-column key next to the middle finger."),
    m("B05", "Scissors", "%", Order::Bi, Dir::Min, "Adjacent fingers, same hand, two-row jump."),
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
    m("B02", "Same-key repeats", "%", Order::Bi, Dir::Info, "The same key twice in a row."),
];

pub fn def(id: &str) -> Option<&'static MetricDef> {
    METRICS.iter().find(|d| d.id == id)
}

pub fn index_of(id: &str) -> Option<usize> {
    METRICS.iter().position(|d| d.id == id)
}

/// Per-metric cost table for a geometry, aligned with [`METRICS`]. `None` for `Order::Load`.
pub fn table(g: &Geometry, d: &MetricDef) -> Option<Vec<f32>> {
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
    let same_finger = |i: usize, j: usize| i != j && k[i].finger == k[j].finger;
    let same_hand = |i: usize, j: usize| k[i].hand() == k[j].hand();
    let adjacent = |i: usize, j: usize| same_hand(i, j) && (k[i].finger.ord() as i8 - k[j].finger.ord() as i8).abs() == 1;
    let pct = |b: bool| if b { 100.0 } else { 0.0 };
    match d.id {
        "M01" => mono(&|i| k[i].effort),
        "M05" => mono(&|i| pct(k[i].home || k[i].row == home_row(g))),
        "M06" => mono(&|i| pct(k[i].stretch)),
        "B01" | "S01" => pair(&|i, j| pct(same_finger(i, j))),
        "B02" => pair(&|i, j| pct(i == j)),
        "B03" => pair(&|i, j| if same_finger(i, j) { 100.0 * g.dist(i, j) / k[i].finger.strength() } else { 0.0 }),
        "B04" => pair(&|i, j| {
            let lsb = |a: usize, b: usize| k[a].stretch && k[a].finger.ord() == 3 && k[b].finger.ord() == 2;
            pct(same_hand(i, j) && (lsb(i, j) || lsb(j, i)))
        }),
        "B05" => pair(&|i, j| pct(adjacent(i, j) && (k[i].row - k[j].row).abs() >= 2)),
        "B08" => pair(&|i, j| pct(!same_hand(i, j))),
        id if id.starts_with('T') => Some(trigram::table(g, trigram::class_for(id)?)),
        _ => None,
    }
}

/// The logical row that holds the home keys (most `home = true` keys).
fn home_row(g: &Geometry) -> i8 {
    g.key.iter().find(|k| k.home).map(|k| k.row).unwrap_or(i8::MIN)
}
