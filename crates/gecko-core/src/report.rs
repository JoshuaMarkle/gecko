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
            (Order::Bi | Order::Skip, Some(t)) => {
                let f = if d.order == Order::Bi { sc.f2() } else { sc.fk() };
                let mut v = 0.0;
                for x in 0..n {
                    for y in 0..n {
                        v += f[x * n + y] as f64 * t[pos[x] as usize * n + pos[y] as usize] as f64;
                    }
                }
                v
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
        let w = weight as f64;
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
    }
}
