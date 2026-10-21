//! Per-metric breakdowns and analysis (finger loads, heat, worst n-grams). JSON shapes here are
//! part of the stable `--json` schema; see `context/cli.md`.

use serde::Serialize;

use crate::geometry::{Finger, FINGERS};
use crate::metrics::{Dir, Order, METRICS};
use crate::scorer::{hand_cost, load_error_cost, Scorer, State};

#[derive(Clone, Debug, Serialize)]
pub struct MetricValue {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub dir: Dir,
    /// Natural units (%, avg effort, …).
    pub value: f64,
    pub weight: f32,
    /// Contribution to the score (lower is better).
    pub weighted: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Breakdown {
    pub score: f64,
    pub metrics: Vec<MetricValue>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ngram {
    pub ngram: String,
    /// % of all n-grams of that order on this layout.
    pub freq: f64,
}

/// One directional pair (bigram or skipgram) with its class on this layout, for drawing
/// every pair at once. `kind`: `sfb` | `lsb` | `scissor` | `same_hand` | `alternate` for
/// bigrams; `sfs` | `same_hand` | `alternate` for skipgrams. Same-key pairs are left out.
#[derive(Clone, Debug, Serialize)]
pub struct Pair {
    pub ngram: String,
    /// % of all n-grams of that order on this layout.
    pub freq: f64,
    pub kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct Analysis {
    pub breakdown: Breakdown,
    /// % of keystrokes per finger, LP LR LM LI LT RT RI RM RR RP.
    pub finger_load: [f64; FINGERS],
    pub finger_target: [f64; FINGERS],
    pub fingers: [&'static str; FINGERS],
    /// % of keystrokes per slot (heatmap).
    pub heat: Vec<f64>,
    pub worst_sfb: Vec<Ngram>,
    pub worst_sfs: Vec<Ngram>,
    pub worst_lsb: Vec<Ngram>,
    pub worst_scissors: Vec<Ngram>,
    /// Every bigram with nonzero frequency, most frequent first.
    pub bigrams: Vec<Pair>,
    /// Every skipgram (keys 1 and 3 of a trigram) with nonzero frequency, most frequent first.
    pub skipgrams: Vec<Pair>,
}

/// Per-metric values for `st`. `Σ weighted == scorer.full(st)` (tested).
pub fn breakdown(sc: &Scorer, st: &State) -> Breakdown {
    let n = sc.n;
    let pos = &st.pos;
    let mut metrics = Vec::with_capacity(METRICS.len());
    let mut score = 0.0;
    for (mi, d) in METRICS.iter().enumerate() {
        let value = match (d.order, sc.table(mi)) {
            (Order::Mono, Some(t)) => (0..n).map(|x| sc.f1()[x] as f64 * t[pos[x] as usize] as f64).sum(),
            (Order::Bi | Order::Skip | Order::SkipFar | Order::SymBi, Some(t)) => {
                let f = match d.order {
                    Order::Bi => sc.f2(),
                    Order::Skip => sc.fk(),
                    Order::SymBi => sc.fy(),
                    _ => sc.fkf(),
                };
                let mut v = 0.0;
                for x in 0..n {
                    for y in 0..n {
                        v += f[x * n + y] as f64 * t[pos[x] as usize * n + pos[y] as usize] as f64;
                    }
                }
                v
            }
            (Order::Brackets, Some(t)) => {
                sc.bracket_pairs().iter().map(|e| e.f as f64 * t[pos[e.x as usize] as usize * n + pos[e.y as usize] as usize] as f64).sum()
            }
            (Order::Tri, Some(t)) => {
                let f = sc.f3();
                let mut v = 0.0;
                for x in 0..n {
                    for y in 0..n {
                        let fb = (x * n + y) * n;
                        let tb = (pos[x] as usize * n + pos[y] as usize) * n;
                        for z in 0..n {
                            v += f[fb + z] as f64 * t[tb + pos[z] as usize] as f64;
                        }
                    }
                }
                v
            }
            (Order::Load, _) => match d.id {
                "M02" => load_error_cost(&st.load, sc.target()).sqrt(),
                "M04" => 100.0 * st.load[..5].iter().sum::<f64>() / st.load.iter().sum::<f64>().max(1e-12),
                _ => 0.0,
            },
            _ => 0.0,
        };
        let weight = sc.profile.weight(d.id);
        let w = sc.profile.effective_weight(d.id) as f64;
        let weighted = match (d.dir, d.id) {
            (Dir::Info, _) => 0.0,
            (_, "M02") => w * load_error_cost(&st.load, sc.target()),
            (_, "M04") => w * hand_cost(&st.load),
            (Dir::Min, _) => w * value,
            (Dir::Max, _) => -w * value,
        };
        score += weighted;
        metrics.push(MetricValue { id: d.id, name: d.name, unit: d.unit, dir: d.dir, value, weight, weighted });
    }
    Breakdown { score, metrics }
}

fn worst_pairs(sc: &Scorer, st: &State, f: &[f32], t: &[f32], top: usize) -> Vec<Ngram> {
    let n = sc.n;
    let mut v: Vec<Ngram> = Vec::new();
    for x in 0..n {
        for y in 0..n {
            let c = t[st.pos[x] as usize * n + st.pos[y] as usize];
            let fr = f[x * n + y];
            if c > 0.0 && fr > 0.0 {
                v.push(Ngram { ngram: [sc.chars[x], sc.chars[y]].iter().collect(), freq: 100.0 * fr as f64 });
            }
        }
    }
    v.sort_by(|a, b| b.freq.total_cmp(&a.freq));
    v.truncate(top);
    v
}

/// All pairs of `f` on `st`, classed by the first matching `(table, kind)` in `classes`, else
/// `alternate` / `same_hand` from `alt` (the B08 alternation table).
fn all_pairs(sc: &Scorer, st: &State, f: &[f32], classes: &[(&[f32], &'static str)], alt: &[f32]) -> Vec<Pair> {
    let n = sc.n;
    let mut v = Vec::new();
    for x in 0..n {
        for y in 0..n {
            let fr = f[x * n + y];
            let ij = st.pos[x] as usize * n + st.pos[y] as usize;
            if x == y || fr <= 0.0 {
                continue;
            }
            let fallback = if alt[ij] > 0.0 { "alternate" } else { "same_hand" };
            let kind = classes.iter().find(|(t, _)| t[ij] > 0.0).map_or(fallback, |&(_, k)| k);
            v.push(Pair { ngram: [sc.chars[x], sc.chars[y]].iter().collect(), freq: 100.0 * fr as f64, kind });
        }
    }
    v.sort_by(|a, b| b.freq.total_cmp(&a.freq));
    v
}

/// Score change for swapping the symbols on every pair of slots: `m[i * n + j]`, symmetric,
/// zero on the diagonal (one 435-swap neighbourhood scan for 30 keys).
pub fn swap_matrix(sc: &Scorer, st: &State) -> Vec<f32> {
    let n = sc.n;
    let mut m = vec![0f32; n * n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = sc.delta(st, st.sym[i] as usize, st.sym[j] as usize) as f32;
            m[i * n + j] = d;
            m[j * n + i] = d;
        }
    }
    m
}

pub fn analyze(sc: &Scorer, st: &State, top: usize) -> Analysis {
    let id = |s: &str| METRICS.iter().position(|d| d.id == s).unwrap();
    let t = |s: &str| sc.table(id(s)).unwrap();
    let mut heat = vec![0.0; sc.n];
    for x in 0..sc.n {
        heat[st.pos[x] as usize] = 100.0 * sc.f1()[x] as f64;
    }
    let mut fingers = [""; FINGERS];
    for (i, f) in Finger::ALL.iter().enumerate() {
        fingers[i] = f.name();
    }
    Analysis {
        breakdown: breakdown(sc, st),
        finger_load: st.load,
        finger_target: *sc.target(),
        fingers,
        heat,
        worst_sfb: worst_pairs(sc, st, sc.f2(), t("B01"), top),
        worst_sfs: worst_pairs(sc, st, sc.fk(), t("S01"), top),
        worst_lsb: worst_pairs(sc, st, sc.f2(), t("B04"), top),
        worst_scissors: worst_pairs(sc, st, sc.f2(), t("B05"), top),
        bigrams: all_pairs(sc, st, sc.f2(), &[(t("B01"), "sfb"), (t("B04"), "lsb"), (t("B05"), "scissor")], t("B08")),
        skipgrams: all_pairs(sc, st, sc.fk(), &[(t("S01"), "sfs")], t("B08")),
    }
}
