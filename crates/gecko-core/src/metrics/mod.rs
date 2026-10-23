//! Metric definitions and their per-geometry cost tables. IDs follow
//! `context/metrics-catalog.md`. Each linear metric is a table indexed by slot (mono),
//! slot pair (bi, skip) or slot triple (tri), in natural units (%, or a per-keystroke
//! average). The scorer folds them with profile weights; see `context/objective.md`.
//! Some tables depend on profile parameters (`Profile::sfb`, `Profile::row_offset`, `Profile::hand`), so they are built per profile.

pub mod trigram;

use serde::Serialize;

use crate::profile::{DistanceFn, SfbParams};
use crate::{Geometry, Hand, LayerMode, Profile};

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
    /// Open/close bracket pairs (`()`, `[]`, `{}`, `<>`) on the layout, weighted by their
    /// symbol frequency; a sparse list, not a corpus n-gram.
    Brackets,
    /// A cost for symbol x on slot i (a table of n × n, symbol-major), already weighted by the
    /// symbols it concerns: L01, L02.
    Place,
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
    /// Kept so old runs and profiles still score, but no bundled profile weights it and the web
    /// UI hides it (`context/metrics-catalog.md`).
    pub retired: bool,
}

const fn m(id: &'static str, name: &'static str, unit: &'static str, order: Order, dir: Dir, desc: &'static str) -> MetricDef {
    MetricDef { id, name, unit, order, dir, desc, retired: false }
}

const fn retired(d: MetricDef) -> MetricDef {
    MetricDef { retired: true, ..d }
}

/// Every metric GECKO knows, in report order.
pub static METRICS: &[MetricDef] = &[
    m("B01", "SFB", "%", Order::Bi, Dir::Min, "Same-finger bigrams: consecutive keys on one finger (different keys)."),
    m("S01", "SFS", "%", Order::Skip, Dir::Min, "Same-finger skipgrams: two keys on one finger with any one symbol between (space too)."),
    retired(m(
        "B03",
        "Finger speed",
        "u",
        Order::Bi,
        Dir::Min,
        "Retired: same-finger distance per 100 bigrams, g(distance) × finger factor (profile `sfb`). Keystroke timing shows no distance effect beyond B01.",
    )),
    m(
        "S02",
        "Far SFS",
        "%",
        Order::SkipFar,
        Dir::Min,
        "Same finger with 2–3 keys between, decayed per extra key (profile `sfb.skip_decay`).",
    ),
    retired(m("S03", "SFS distance", "u", Order::Skip, Dir::Min, "Retired: finger speed (B03) applied to skipgrams with one key between.")),
    m("B04", "LSB", "%", Order::Bi, Dir::Min, "Lateral stretch: index on a center-column key next to the middle finger."),
    m("B05", "Scissors", "%", Order::Bi, Dir::Min, "Adjacent fingers, same hand, two-row jump."),
    m(
        "B06",
        "Half scissors",
        "%",
        Order::Bi,
        Dir::Min,
        "Adjacent fingers, same hand, one-row jump with the shorter finger on the upper row.",
    ),
    m("B07", "Bigram rolls in", "%", Order::Bi, Dir::Max, "Same hand, different fingers, moving toward the index finger."),
    m("B09", "Pinky–ring pairs", "%", Order::Bi, Dir::Min, "Consecutive keys on the pinky and ring finger of one hand."),
    m(
        "B10",
        "Row jump",
        "u",
        Order::Bi,
        Dir::Min,
        "Vertical travel between consecutive same-hand keys, each measured from its finger's home key, per 100 bigrams.",
    ),
    m(
        "S05",
        "Skip stretch",
        "u",
        Order::Skip,
        Dir::Min,
        "Geometric stretch (B14) between keys 1 and 3 of a trigram, per 100 skipgrams.",
    ),
    m(
        "B14",
        "Stretch",
        "u",
        Order::Bi,
        Dir::Min,
        "Geometric stretch between same-hand fingers per 100 bigrams (Oxeylyzer 2); covers LSB, scissors, crossings.",
    ),
    m(
        "B11",
        "Movement",
        "u",
        Order::Bi,
        Dir::Min,
        "Same-hand finger-pair movement cost per 100 bigrams (KLO table, finger-length scissor rule, ring coupling).",
    ),
    m(
        "S04",
        "Skip stretch",
        "u",
        Order::Skip,
        Dir::Min,
        "Stretch (B14) between keys 1 and 3 of a trigram per 100 skipgrams: disjoint LSB / scissors (Kühlmak).",
    ),
    m(
        "M01",
        "Effort",
        "",
        Order::Mono,
        Dir::Min,
        "Average key effort per keystroke: geometry base effort plus the profile's per-finger `row_offset`, plus layer key effort and Shift key effort per Shift press.",
    ),
    m("M02", "Finger load error", "%", Order::Load, Dir::Min, "RMS gap between finger loads and the profile's target loads."),
    m("M03", "Finger balance", "%", Order::Load, Dir::Min, "Root-sum-square of non-thumb finger shares; lowest when load is even."),
    m(
        "M07",
        "Peak finger load",
        "%",
        Order::Load,
        Dir::Min,
        "Highest non-thumb finger load as % of that finger's target load (profile `finger_target`).",
    ),
    m("M04", "Left hand", "%", Order::Load, Dir::Min, "Share of keystrokes on the left hand; scored as squared distance from 50%."),
    m(
        "M09",
        "Thumb load",
        "%",
        Order::Load,
        Dir::Min,
        "Share of keystrokes on the thumbs; scored as squared distance from the profile's `thumb_target`.",
    ),
    m(
        "G01",
        "Reach",
        "",
        Order::Mono,
        Dir::Min,
        "Hand-model reach cost per keystroke: joint deviation from the natural curl, sideways splay and fingertip shortfall with the hand placed on the home keys (profile `hand`, default average).",
    ),
    m(
        "G05",
        "Press angle",
        "",
        Order::Mono,
        Dir::Min,
        "Hand-model press misalignment per keystroke: (angle / 45°)² between the fingertip's press direction and the keycap normal (fingers: the distal phalanx, within the finger's plane; thumbs: a push along the CMC→tip line).",
    ),
    m(
        "G03",
        "Wrist posture",
        "",
        Order::Mono,
        Dir::Min,
        "Hand-model wrist cost per keystroke: ulnar/radial deviation, extension and forearm pronation of the resting hand (elbows from profile `hand.body`; split halves placed to suit the arm), plus the extra deviation of sideways reaches.",
    ),
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
    m("T12", "Thumb rolls", "%", Order::Tri, Dir::Max, "Trigrams with a thumb key next to a key on the same hand (other finger)."),
    m("T13", "Thumb alternates", "%", Order::Tri, Dir::Max, "Trigrams with a thumb where every step into or out of the thumb changes hands."),
    m(
        "T09",
        "Contortions",
        "u",
        Order::Tri,
        Dir::Min,
        "Chained stretches: both steps of a trigram have geometric stretch (B14); their sum per 100 trigrams.",
    ),
    m(
        "T10",
        "Irregular trigrams",
        "%",
        Order::Tri,
        Dir::Min,
        "Both bigrams of the trigram are bad: same finger, lateral stretch (B04) or a full/half scissor (B05/B06).",
    ),
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
    m(
        "E01",
        "Typing time",
        "%",
        Order::Bi,
        Dir::Min,
        "Predicted extra time per bigram, % of a median keystroke interval: a geometric model fitted to fast touch typists (Aalto 136M keystrokes). A judge; weight 0 in the bundled profiles.",
    ),
    m("B02", "Same-key repeats", "%", Order::Bi, Dir::Info, "The same key twice in a row."),
    m(
        "Y01",
        "Layer switches",
        "%",
        Order::Bi,
        Dir::Min,
        "Layer-key presses per 100 bigrams: hold and toggle layers cost a press on entry, toggle layers another to leave, one-shot layers one per symbol; a press next to another key of the layer key's finger counts double. Plus Shift presses per 100 keystrokes (with a ⇧ key), double when Shift's finger also types the shifted key or the key before.",
    ),
    m(
        "Y02",
        "Symbol bigrams",
        "%",
        Order::SymBi,
        Dir::Min,
        "Code-symbol bigrams (`()`, `->`, `!=`, …) typed with one finger on two keys or across a layer change, as % of symbol bigrams.",
    ),
    m(
        "Y03",
        "Paired symbols",
        "%",
        Order::Brackets,
        Dir::Min,
        "Bracket pairs (`()`, `[]`, `{}`, `<>`) not adjacent or mirrored on one layer, weighted by use; reversed order counts half.",
    ),
    m(
        "Y04",
        "HRM misfires",
        "%",
        Order::Bi,
        Dir::Min,
        "Same-hand rolls out of a home-row mod key (geometry `hrm`) onto another finger, where the hold can misfire.",
    ),
    m(
        "L01",
        "Shortcut placement",
        "%",
        Order::Place,
        Dir::Min,
        "Shortcut letters Z X C V (and A S Q W at half weight) away from the left hand: 100 off it, up to 50 on it by distance from the QWERTY spot. Per shortcut letter, not per keystroke.",
    ),
    m(
        "L02",
        "Familiarity",
        "%",
        Order::Place,
        Dir::Min,
        "Distance from the reference layout (profile `familiar_to`), % of keystrokes: 0 on the reference key, 50 on another key of its finger, 75 on its hand, 100 otherwise.",
    ),
];

/// Shortcut letters scored by L01 and their weights.
pub const SHORTCUTS: [(char, f32); 8] = [('z', 1.0), ('x', 1.0), ('c', 1.0), ('v', 1.0), ('a', 0.5), ('s', 0.5), ('q', 0.5), ('w', 0.5)];

/// The QWERTY letter keys in slot order of a 3-row, 10-column board.
const QWERTY: &str = "qwertyuiopasdfghjkl;zxcvbnm,./";

/// Open/close symbol pairs scored by Y03.
pub const BRACKET_PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

pub fn def(id: &str) -> Option<&'static MetricDef> {
    METRICS.iter().find(|d| d.id == id)
}

pub fn index_of(id: &str) -> Option<usize> {
    METRICS.iter().position(|d| d.id == id)
}

/// Per-metric cost table for a geometry and profile, aligned with [`METRICS`]. `None` for
/// `Order::Load`, and for E04/E05 when the geometry has no comfort table for them.
pub fn table(g: &Geometry, p: &Profile, d: &MetricDef) -> Option<Vec<f32>> {
    let mut t = Vec::new();
    update_table(g, p, &Prep::new(g), d, &mut t, None).then_some(t)
}

/// Board-wide inputs shared by the tables of one geometry and profile, computed once per
/// build instead of once per metric: metric points and (on first use) the hand model.
pub struct Prep {
    pts: Vec<[f32; 3]>,
    hand: std::cell::OnceCell<crate::hand::HandCosts>,
}

impl Prep {
    pub fn new(g: &Geometry) -> Self {
        Prep { pts: g.metric_points(), hand: std::cell::OnceCell::new() }
    }

    /// The geometry's [`Geometry::metric_points`].
    pub fn points(&self) -> &[[f32; 3]] {
        &self.pts
    }
}

/// Fill `t` with metric `d`'s table for `g`; `false` when it has none (see [`table`]).
///
/// With `changed`, `t` already holds the table of a geometry that differs from `g` only in
/// the marked slots (their keys or [`Geometry::metric_points`]), and only pair and triple
/// entries touching a marked slot are recomputed: every pair/triple entry reads nothing but
/// its own slots, the profile and board-wide fields (layer keys, comfort tables) that the
/// caller must mark as all-changed. Mono tables are always rebuilt (O(N); the hand model and
/// home-row lookups are board-wide).
/// `prep` must be [`Prep::new`] of `g`, used with this profile only.
pub fn update_table(g: &Geometry, p: &Profile, prep: &Prep, d: &MetricDef, t: &mut Vec<f32>, changed: Option<&[bool]>) -> bool {
    let n = g.len();
    let all = vec![true; n];
    let ch = changed.unwrap_or(&all);
    let idx: Vec<usize> = (0..n).filter(|&i| ch[i]).collect();
    let fresh = changed.is_none();
    let k = &g.key;
    // Fingertip targets (3D boards: keycap tops in each hand's frame; flat boards: `pos`).
    let pts = &prep.pts;
    let spread = hand_spread(p);
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
        "M01" => mono(t, n, &|i| key_effort(g, p, i) + g.holder(i).map_or(0.0, |h| key_effort(g, p, h))),
        "G01" | "G03" | "G05" => {
            let c = prep.hand.get_or_init(|| {
                let h = p.hand.clone().unwrap_or_default();
                crate::hand::costs_with(g, &h.dims(), &h.body())
            });
            let v = match d.id {
                "G01" => &c.reach,
                "G03" => &c.wrist,
                _ => &c.align,
            };
            mono(t, n, &|i| v[i] + g.holder(i).map_or(0.0, |h| v[h]))
        }
        "M05" => mono(t, n, &|i| pct(k[i].home || k[i].row == home_row(g))),
        "M06" => mono(t, n, &|i| pct(k[i].stretch)),
        "B01" | "S01" | "S02" => pair(t, n, fresh, ch, &idx, &|i, j| pct(same_finger(i, j))),
        "B02" => pair(t, n, fresh, ch, &idx, &|i, j| pct(g.same_key(i, j))),
        "B03" | "S03" => pair(t, n, fresh, ch, &idx, &|i, j| {
            if same_finger(i, j) {
                100.0 * finger_speed_at(g, pts, &p.sfb, i, j)
            } else {
                0.0
            }
        }),
        "B14" | "S04" | "S05" => pair(t, n, fresh, ch, &idx, &|i, j| 100.0 * stretch_at(g, pts, spread, i, j)),
        "B11" => pair(t, n, fresh, ch, &idx, &|i, j| 100.0 * movement(g, i, j)),
        "E04" | "E05" => {
            let Some(c) = g.comfort_tables.get(d.id) else { return false };
            pair(t, n, fresh, ch, &idx, &|i, j| 100.0 * (1.0 - c[i * n + j]))
        }
        // Key effort reads board-wide fields (home row, home keys, layer holders): rebuild whole.
        "E01" => pair(t, n, true, &all, &(0..n).collect::<Vec<_>>(), &|i, j| {
            if g.same_key(i, j) {
                return 0.0;
            }
            let effort = key_effort(g, p, j) + g.holder(j).map_or(0.0, |h| key_effort(g, p, h));
            let fingers = !k[i].finger.is_thumb() && !k[j].finger.is_thumb();
            let sh = same_hand(i, j) && fingers && k[i].finger != k[j].finger;
            let adj = sh && adjacent(i, j);
            let pinky_ring = sh && k[i].finger.ord() + k[j].finger.ord() == 1;
            let rows = if sh { (k[i].row - k[j].row).abs() as f32 } else { 0.0 };
            100.0
                * (E01_SFB * f32::from(same_finger(i, j))
                    + E01_SAME_HAND * f32::from(sh)
                    + E01_ADJACENT * f32::from(adj)
                    + E01_ROWS * rows
                    + E01_SCISSOR * f32::from(adj && rows >= 2.0)
                    + E01_PINKY_RING * f32::from(pinky_ring)
                    + E01_EFFORT * effort)
        }),
        "B04" => pair(t, n, fresh, ch, &idx, &|i, j| {
            let lsb = |a: usize, b: usize| k[a].stretch && k[a].finger.ord() == 3 && k[b].finger.ord() == 2;
            pct(same_hand(i, j) && (lsb(i, j) || lsb(j, i)))
        }),
        "B05" => pair(t, n, fresh, ch, &idx, &|i, j| pct(adjacent(i, j) && (k[i].row - k[j].row).abs() >= 2)),
        "B06" => pair(t, n, fresh, ch, &idx, &|i, j| pct(half_scissor(g, i, j))),
        "B07" => pair(t, n, fresh, ch, &idx, &|i, j| pct(rolls(g, i, j) && k[j].finger.ord() > k[i].finger.ord())),
        "B09" => pair(t, n, fresh, ch, &idx, &|i, j| pct(rolls(g, i, j) && k[i].finger.ord() + k[j].finger.ord() == 1)),
        // Vertical travel from each finger's own home key, so a column stagger that follows
        // finger length is not a jump. Reads the home keys too: rebuild whole.
        "B10" => {
            let lift = home_lift(g, pts);
            pair(t, n, true, &all, &(0..n).collect::<Vec<_>>(), &|i, j| {
                let sh = same_hand(i, j) && !g.same_key(i, j) && !k[i].finger.is_thumb() && !k[j].finger.is_thumb();
                if sh {
                    100.0 * (lift[i] - lift[j]).abs()
                } else {
                    0.0
                }
            })
        }
        "T09" => {
            let st: Vec<f32> = (0..n * n).map(|ij| stretch_at(g, pts, spread, ij / n, ij % n)).collect();
            tri(t, n, fresh, ch, &idx, &|i, j, l| {
                let (s1, s2) = (st[i * n + j], st[j * n + l]);
                if s1 > 0.0 && s2 > 0.0 {
                    100.0 * (s1 + s2)
                } else {
                    0.0
                }
            })
        }
        "T10" => {
            let bad: Vec<bool> = (0..n * n).map(|ij| bad_bigram(g, ij / n, ij % n)).collect();
            tri(t, n, fresh, ch, &idx, &|i, j, l| pct(bad[i * n + j] && bad[j * n + l]))
        }
        "B08" => pair(t, n, fresh, ch, &idx, &|i, j| pct(!same_hand(i, j))),
        "Y01" => pair(t, n, fresh, ch, &idx, &|i, j| layer_presses(g, i, j)),
        "Y02" => pair(t, n, fresh, ch, &idx, &|i, j| {
            let oneshot = k[i].layer != 0 && layer_mode(g, k[i].layer as usize) == LayerMode::Oneshot;
            pct(same_finger(i, j) || k[i].layer != k[j].layer || oneshot)
        }),
        "Y03" => pair(t, n, fresh, ch, &idx, &|i, j| bracket_cost_at(g, pts, i, j)),
        // The mod key is still down when another finger of its hand presses: a fast roll that
        // a tap-hold resolver can read as a hold. Same-finger pairs release first.
        "Y04" => pair(t, n, fresh, ch, &idx, &|i, j| pct(g.hrm(i) && same_hand(i, j) && k[i].finger != k[j].finger)),
        id if id.starts_with('T') => {
            let Some(cls) = trigram::class_for(id) else { return false };
            tri(t, n, fresh, ch, &idx, &|i, j, l| pct(trigram::classify(g, i, j, l) == cls))
        }
        _ => false,
    }
}

// E01 coefficients, in median relative inter-key intervals, controlling log bigram frequency:
// fast touch typists (≥ 70 WPM, 18.5k people, 351 letter bigrams; cross-validated R² 0.67 with
// log frequency). `bench/cost-validity-2026-10-22/`, `context/objective.md` (E01).
const E01_SFB: f32 = 0.5522;
const E01_SAME_HAND: f32 = 0.0578;
const E01_ADJACENT: f32 = 0.0476;
const E01_ROWS: f32 = 0.0373;
const E01_SCISSOR: f32 = 0.1004;
const E01_PINKY_RING: f32 = 0.1263;
const E01_EFFORT: f32 = 0.0201;

/// The pair/triple tables of [`update_table`] that read [`Geometry::metric_points`]; every
/// other pair/triple table reads only logical key fields (finger, row, column, flags), so a
/// pose-only move leaves it unchanged.
pub fn reads_points(id: &str) -> bool {
    matches!(id, "B03" | "S03" | "B14" | "S04" | "S05" | "B10" | "Y03" | "T09")
}

/// Each slot's metric-point y minus the mean y of its finger's home keys; for a finger without
/// home keys, of its hand's home keys; raw y on a board without them.
fn home_lift(g: &Geometry, pts: &[[f32; 3]]) -> Vec<f32> {
    let mut finger = [(0.0f32, 0u32); crate::geometry::FINGERS];
    let mut hand = [(0.0f32, 0u32); 2];
    for (k, p) in g.key.iter().zip(pts) {
        if k.home {
            for e in [&mut finger[k.finger.index()], &mut hand[usize::from(k.hand() == Hand::R)]] {
                *e = (e.0 + p[1], e.1 + 1);
            }
        }
    }
    g.key
        .iter()
        .zip(pts)
        .map(|(k, p)| {
            let (f, h) = (finger[k.finger.index()], hand[usize::from(k.hand() == Hand::R)]);
            let (y, c) = if f.1 > 0 { f } else { h };
            if c > 0 {
                p[1] - y / c as f32
            } else {
                p[1]
            }
        })
        .collect()
}

/// Fill the `[N]` table `t` from `f(i)` (always every entry).
fn mono(t: &mut Vec<f32>, n: usize, f: &dyn Fn(usize) -> f32) -> bool {
    *t = (0..n).map(f).collect();
    true
}

/// Fill the `[N×N]` table `t` from `f(i, j)`: every entry when `fresh`, else only rows and
/// columns of the changed slots (`ch[i]`, listed in `idx`).
fn pair(t: &mut Vec<f32>, n: usize, fresh: bool, ch: &[bool], idx: &[usize], f: &dyn Fn(usize, usize) -> f32) -> bool {
    if fresh {
        *t = vec![0.0; n * n];
    }
    for i in 0..n {
        if ch[i] {
            for j in 0..n {
                t[i * n + j] = f(i, j);
            }
        } else {
            for &j in idx {
                t[i * n + j] = f(i, j);
            }
        }
    }
    true
}

/// [`pair`] for `[N×N×N]` tables from `f(i, j, l)`: entries touching a changed slot.
fn tri(t: &mut Vec<f32>, n: usize, fresh: bool, ch: &[bool], idx: &[usize], f: &dyn Fn(usize, usize, usize) -> f32) -> bool {
    if fresh {
        *t = vec![0.0; n * n * n];
    }
    for i in 0..n {
        for j in 0..n {
            let row = (i * n + j) * n;
            if ch[i] || ch[j] {
                for l in 0..n {
                    t[row + l] = f(i, j, l);
                }
            } else {
                for &l in idx {
                    t[row + l] = f(i, j, l);
                }
            }
        }
    }
    true
}

fn layer_mode(g: &Geometry, l: usize) -> LayerMode {
    g.layer_modes.get(l - 1).copied().unwrap_or_default()
}

/// Y01 cost of typing slot `j` right after slot `i`: 100 per layer-key press in between (a
/// toggle layer's key to leave `i`'s layer, then the key to reach `j`'s layer unless it is
/// already active, always for one-shot), plus 100 per press adjacent to another key on the
/// same finger. A hold layer is released for free; its finger's keys are not on the layer.
fn layer_presses(g: &Geometry, i: usize, j: usize) -> f32 {
    let (li, lj) = (g.key[i].layer as usize, g.key[j].layer as usize);
    let mut seq = vec![i];
    if li != 0 && li != lj && layer_mode(g, li) == LayerMode::Toggle {
        seq.push(g.layer_keys[li - 1]);
    }
    if lj != 0 && (li != lj || layer_mode(g, lj) == LayerMode::Oneshot) {
        seq.push(g.layer_keys[lj - 1]);
    }
    seq.push(j);
    let presses = seq.len() - 2;
    let sf = seq.windows(2).filter(|w| g.key[w[0]].finger == g.key[w[1]].finger && !g.same_key(w[0], w[1])).count();
    if presses == 0 {
        0.0
    } else {
        100.0 * (presses + sf) as f32
    }
}

/// Same hand, two different non-thumb fingers (a bigram roll in either direction).
fn rolls(g: &Geometry, i: usize, j: usize) -> bool {
    let (a, b) = (&g.key[i], &g.key[j]);
    a.hand() == b.hand() && a.finger != b.finger && !a.finger.is_thumb() && !b.finger.is_thumb()
}

/// Finger length rank, shortest first: pinky, index, ring, middle.
fn length_rank(ord: u8) -> u8 {
    [0, 2, 3, 1, 4][ord as usize]
}

/// B06: adjacent non-thumb fingers on one hand, rows one apart, the shorter finger on the upper row.
pub fn half_scissor(g: &Geometry, i: usize, j: usize) -> bool {
    let (a, b) = (&g.key[i], &g.key[j]);
    if !rolls(g, i, j) || a.finger.ord().abs_diff(b.finger.ord()) != 1 || (a.row - b.row).abs() != 1 {
        return false;
    }
    let (short, long) = if length_rank(a.finger.ord()) < length_rank(b.finger.ord()) { (a, b) } else { (b, a) };
    short.row < long.row
}

/// T10's "bad bigram": same finger on two keys, a lateral stretch (B04), or a full/half scissor.
pub fn bad_bigram(g: &Geometry, i: usize, j: usize) -> bool {
    let (a, b) = (&g.key[i], &g.key[j]);
    if g.same_key(i, j) || a.hand() != b.hand() || a.finger.is_thumb() || b.finger.is_thumb() {
        return false;
    }
    if a.finger == b.finger {
        return true;
    }
    let lsb = |x: &crate::geometry::Key, y: &crate::geometry::Key| x.stretch && x.finger.ord() == 3 && y.finger.ord() == 2;
    let adjacent = a.finger.ord().abs_diff(b.finger.ord()) == 1;
    lsb(a, b) || lsb(b, a) || (adjacent && (a.row - b.row).abs() >= 2) || half_scissor(g, i, j)
}

/// Y03 cost of the opener on slot `i` and the closer on slot `j`: 0 when both are on one
/// layer and either adjacent (same hand and row, neighbouring columns, opener on the left) or
/// mirrored (same row, finger and column kind on opposite hands, opener on the left hand);
/// 50 for the same shapes in reversed order; 100 otherwise.
pub fn bracket_cost(g: &Geometry, i: usize, j: usize) -> f32 {
    bracket_cost_at(g, &g.metric_points(), i, j)
}

/// [`bracket_cost`] with precomputed [`Geometry::metric_points`].
pub fn bracket_cost_at(g: &Geometry, pts: &[[f32; 3]], i: usize, j: usize) -> f32 {
    let (a, b) = (&g.key[i], &g.key[j]);
    if a.layer != b.layer || g.same_key(i, j) || a.row != b.row {
        return 100.0;
    }
    let adjacent = a.hand() == b.hand() && (a.col - b.col).abs() == 1;
    let mirrored = a.hand() != b.hand() && a.finger.ord() == b.finger.ord() && a.stretch == b.stretch;
    if !adjacent && !mirrored {
        return 100.0;
    }
    let in_order = if adjacent { pts[i][0] < pts[j][0] } else { a.hand() == Hand::L };
    if in_order {
        0.0
    } else {
        50.0
    }
}

/// Same-finger movement cost `g(d) · finger_factor` for slots `i`, `j` (B03, S03).
pub fn finger_speed(g: &Geometry, s: &SfbParams, i: usize, j: usize) -> f32 {
    finger_speed_at(g, &g.metric_points(), s, i, j)
}

/// [`finger_speed`] with precomputed [`Geometry::metric_points`].
pub fn finger_speed_at(g: &Geometry, pts: &[[f32; 3]], s: &SfbParams, i: usize, j: usize) -> f32 {
    let (a, b) = (pts[i], pts[j]);
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
/// [`SPREAD`] per finger of spread a relaxed hand covers.
pub fn stretch(g: &Geometry, i: usize, j: usize) -> f32 {
    stretch_at(g, &g.metric_points(), SPREAD, i, j)
}

/// Sideways spread per finger a relaxed average hand covers without stretching (u; Oxeylyzer 2).
pub const SPREAD: f32 = 1.35;

/// [`SPREAD`] scaled by the profile hand's span (thumb tip to pinky tip) over the average's;
/// exactly [`SPREAD`] without a `hand`, so hand-less profiles keep their scores.
pub fn hand_spread(p: &Profile) -> f32 {
    p.hand.as_ref().map_or(SPREAD, |h| SPREAD * h.dims().span / crate::hand::AVERAGE.span)
}

/// [`stretch`] with precomputed [`Geometry::metric_points`] and the relaxed per-finger `spread`
/// (u). The distance is 3D: crossings and length offsets are in-plane, but height off the hand's
/// plane counts as stretch too, or a 3D search sinks and lifts columns to hide it.
pub fn stretch_at(g: &Geometry, pts: &[[f32; 3]], spread: f32, i: usize, j: usize) -> f32 {
    let (a, b) = (&g.key[i], &g.key[j]);
    if a.finger == b.finger || a.hand() != b.hand() {
        return 0.0;
    }
    let (fa, fb) = (a.finger.index(), b.finger.index());
    let (xa, xb) = (pts[i][0], pts[j][0]);
    let (ya, yb) = (pts[i][1] + a.finger.length_offset(), pts[j][1] + b.finger.length_offset());
    let dx = (xa - xb).abs();
    let dy = (ya - yb).abs();
    let xo = finger_overlap(a.finger.ord(), b.finger.ord());
    // Fingers are indexed left to right across both hands, so the higher index should sit to the right.
    let crossed = if fa > fb { xa < xb + xo } else { xa + xo > xb };
    let dz = pts[i][2] - pts[j][2];
    let dx = if crossed { -dx } else { dx };
    let overlap = (xo - 1.3 * dx + dy / 3.0).max(0.0);
    let s = (dx * dx + dy * dy + dz * dz).sqrt() + overlap - spread * fa.abs_diff(fb) as f32;
    if s > 0.001 {
        s
    } else {
        0.0
    }
}

/// KLO's movement-pattern seed (`config/evaluation/default.yml`, `finger_switch_factor`) per
/// ordered finger pair `[from][to]`, by `Finger::ord` (pinky, ring, middle, index). Pairs KLO
/// leaves out cost 0.
#[rustfmt::skip]
const MOVE_SEED: [[f32; 4]; 4] = [
    [0.0, 4.0, 2.0, 0.0],
    [6.0, 0.0, 2.0, 0.1],
    [3.0, 5.0, 0.0, 0.6],
    [0.1, 0.3, 0.9, 0.0],
];

/// Häger-Ross & Schieber (2000) individuation index per `Finger::ord` (right hand means).
const INDIVIDUATION: [f32; 4] = [0.943, 0.907, 0.937, 0.982];

/// KLO finger lengths per `Finger::ord` for the scissor-direction rule.
const FINGER_LENGTH: [f32; 4] = [1.0, 2.0, 3.0, 2.0];

/// Movement cost per ordered finger pair: [`MOVE_SEED`], with ring-finger pairs scaled by
/// their coupling `(1 − I_a) + (1 − I_b)` relative to the mean over all six finger pairs.
fn move_pair(from: usize, to: usize) -> f32 {
    let c = |a: usize, b: usize| 2.0 - INDIVIDUATION[a] - INDIVIDUATION[b];
    let mut sum = 0.0;
    for a in 0..4 {
        for b in a + 1..4 {
            sum += c(a, b);
        }
    }
    let ring = from == 1 || to == 1;
    MOVE_SEED[from][to] * if ring { c(from, to) / (sum / 6.0) } else { 1.0 }
}

/// B11: KLO-style movement cost for two keys on the same hand and different non-thumb fingers;
/// 0 otherwise. `pair · direction · (0.1 + rows² + lateral)`: `rows` is the logical row change;
/// `direction` = 1 + 2·|Δlength| when a shorter finger goes down to a longer one or a longer
/// finger goes up to a shorter one (the scissor that hurts); `lateral` = columns spanned beyond
/// the fingers' own spacing (LSB). KLO uses `lateral_stretch_factor` 0; GECKO adds it so one
/// table covers LSB too.
pub fn movement(g: &Geometry, i: usize, j: usize) -> f32 {
    let (a, b) = (&g.key[i], &g.key[j]);
    if a.finger == b.finger || a.hand() != b.hand() || a.finger.is_thumb() || b.finger.is_thumb() {
        return 0.0;
    }
    let (fa, fb) = (a.finger.ord() as usize, b.finger.ord() as usize);
    let rows = (a.row - b.row).abs() as f32;
    let dlen = FINGER_LENGTH[fa] - FINGER_LENGTH[fb];
    let (down, up) = (b.row > a.row, b.row < a.row);
    let direction = if (down && dlen < 0.0) || (up && dlen > 0.0) { 1.0 + 2.0 * dlen.abs() } else { 1.0 };
    let lateral = ((a.col - b.col).abs() as f32 - fa.abs_diff(fb) as f32).max(0.0);
    move_pair(fa, fb) * direction * (0.1 + rows * rows + lateral)
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

/// Per-key effort including optional finger-specific travel from the nearest home key.
/// Uses logical columns/rows so split rotation and tenting do not change movement axes.
pub fn key_effort(g: &Geometry, p: &Profile, i: usize) -> f32 {
    let k = &g.key[i];
    let mut cost = k.effort + row_offset(g, p, i);
    if let Some(e) = &p.finger_effort {
        let f = e.index(k.finger.index());
        let travel = g
            .key
            .iter()
            .filter(|h| h.finger == k.finger && h.home)
            .map(|h| (k.col as f32 - h.col as f32).abs() * e.lateral[f] + (k.row as f32 - h.row as f32).abs() * e.vertical[f])
            .reduce(f32::min)
            .unwrap_or(0.0);
        cost += travel;
    }
    cost
}

/// Profile effort offset for slot `i`: `row_offset[finger][0]` above the home row, `[1]` below
/// it, 0 on the home row, for thumbs, and on geometries without home keys.
pub fn row_offset(g: &Geometry, p: &Profile, i: usize) -> f32 {
    let k = &g.key[i];
    let home = home_row(g);
    if k.finger.is_thumb() || home == i8::MIN || k.row == home {
        return 0.0;
    }
    p.row_offset[k.finger.ord() as usize][usize::from(k.row > home)]
}

/// The logical row that holds the home keys (most `home = true` keys).
fn home_row(g: &Geometry) -> i8 {
    g.key.iter().find(|k| k.home).map(|k| k.row).unwrap_or(i8::MIN)
}

/// Slot of each symbol on the QWERTY reference placed on `base` (the board without layer keys);
/// `None` for symbols QWERTY lacks, and for every symbol when the board has no 30 letter keys.
pub fn qwerty_slots(base: &Geometry, chars: &[char]) -> Vec<Option<usize>> {
    let fit = crate::Layout::from_chars("qwerty", &base.name, "", QWERTY.chars().collect()).and_then(|l| l.fit(base));
    chars
        .iter()
        .map(|&c| {
            let fit = fit.as_ref().ok()?;
            QWERTY.contains(c).then(|| fit.chars.iter().position(|&q| q == c)).flatten()
        })
        .collect()
}

/// The n × n symbol-slot table of a [`Order::Place`] metric. `qwerty[x]` is symbol x's QWERTY
/// slot; `f1` the normalized symbol frequencies. Values are in % (0–100).
pub fn place_table(id: &str, g: &Geometry, p: &Profile, chars: &[char], qwerty: &[Option<usize>], f1: &[f32]) -> Vec<f32> {
    let n = g.len();
    let mut t = vec![0.0f32; n * n];
    let k = &g.key;
    match id {
        "L01" => {
            let total: f32 = SHORTCUTS.iter().filter(|(c, _)| chars.contains(c)).map(|(_, w)| w).sum();
            for (x, c) in chars.iter().enumerate() {
                let Some((_, w)) = SHORTCUTS.iter().find(|(s, _)| s == c) else { continue };
                for i in 0..n {
                    let cost = if k[i].hand() != Hand::L {
                        100.0
                    } else {
                        let d = qwerty[x].map_or(0.0, |q| {
                            let (a, b) = (k[i].pos, k[q].pos);
                            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
                        });
                        50.0 * (d / 2.0).min(1.0)
                    };
                    t[x * n + i] = w / total * cost;
                }
            }
        }
        "L02" if p.familiar_to.is_some() => {
            for x in 0..n {
                let Some(q) = qwerty[x] else { continue };
                for i in 0..n {
                    let cost = if i == q || g.same_key(i, q) {
                        0.0
                    } else if k[i].finger == k[q].finger {
                        50.0
                    } else if k[i].hand() == k[q].hand() {
                        75.0
                    } else {
                        100.0
                    };
                    t[x * n + i] = f1[x] * cost;
                }
            }
        }
        _ => {}
    }
    t
}
