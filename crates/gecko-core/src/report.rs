//! Per-metric breakdowns and analysis (finger loads, heat, worst n-grams and words). JSON shapes here are
//! part of the stable `--json` schema; see `context/cli.md`.

use serde::Serialize;

use crate::corpus::{fold_char, WordList};
use crate::geometry::{Finger, FINGERS};
use crate::layout::SPACE;
use crate::metrics::{Dir, Order, METRICS};
use crate::scorer::{balance_rss, hand_cost, load_error_cost, peak_load, Scorer, State};

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
                "M03" => balance_rss(&st.load),
                "M07" => peak_load(&st.load, sc.target()),
                _ => 0.0,
            },
            _ => 0.0,
        } + sc.shift_value(d.id, st);
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

/// One word's cost under one metric.
#[derive(Clone, Debug, Serialize)]
pub struct WordCost {
    pub word: String,
    /// % of all word occurrences.
    pub freq: f64,
    /// Per occurrence, summed over the word's n-grams: offending n-grams for % metrics (2 = two
    /// SFBs), distance for `u` metrics, raw effort for M01.
    pub cost: f64,
    /// % of this metric's word-attributed total (Σ freq × cost over every word).
    pub share: f64,
    /// The word's n-grams with a non-zero cost, in typing order (a far skipgram once per gap).
    pub ngrams: Vec<String>,
}

/// The words that cost most on one metric, by frequency × cost.
#[derive(Clone, Debug, Serialize)]
pub struct MetricWords {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub words: Vec<WordCost>,
}

/// Worst words per lower-is-better metric (cost tables only; load metrics have no per-word
/// cost). Each word is typed as `␣word␣` when `␣` is on the layout, else as the bare word;
/// a symbol not on the layout breaks n-grams, as in the corpus. N-grams spanning two words
/// belong to neither. Metrics no word touches are left out.
pub fn worst_words(sc: &Scorer, st: &State, words: &WordList, top: usize) -> Vec<MetricWords> {
    let n = sc.n;
    let space = sc.chars.iter().position(|&c| c == SPACE);
    let decay = sc.profile.sfb.skip_decay as f64;
    let total = words.total.max(1) as f64;
    // Each word as slots (None = break), with its symbols for n-gram labels.
    let typed: Vec<Vec<Option<(usize, char)>>> = words
        .words
        .iter()
        .map(|(w, _)| {
            let sym = |c: char| sc.chars.iter().position(|&x| x == c).map(|s| (st.pos[s] as usize, c));
            let mut v = vec![space.map(|s| (st.pos[s] as usize, SPACE))];
            v.extend(w.chars().map(|c| sym(fold_char(c))));
            v.push(v[0]);
            if space.is_none() {
                v.remove(0);
                v.pop();
            }
            v
        })
        .collect();
    let mut out = Vec::new();
    for (mi, d) in METRICS.iter().enumerate() {
        let Some(t) = sc.table(mi) else { continue };
        if d.dir != Dir::Min || d.order == Order::Load {
            continue;
        }
        // Gaps between the n-gram's first and last symbol, with their weight.
        let gaps: &[(usize, f64)] = match d.order {
            Order::Mono => &[(0, 1.0)],
            Order::Bi | Order::SymBi => &[(1, 1.0)],
            Order::Skip => &[(2, 1.0)],
            Order::SkipFar => &[(3, decay), (4, decay * decay)],
            Order::Tri => &[(2, 1.0)],
            Order::Load => continue,
        };
        // % and u tables hold 100 × the per-n-gram cost (metric values read per 100 n-grams).
        let scale = if d.unit.is_empty() { 1.0 } else { 0.01 };
        let mut rows: Vec<WordCost> = Vec::new();
        let mut sum = 0.0;
        for (ks, (w, count)) in typed.iter().zip(&words.words) {
            let mut cost = 0.0;
            let mut ngrams = Vec::new();
            for &(g, wt) in gaps {
                for i in 0..ks.len().saturating_sub(g) {
                    let (Some((a, ca)), Some((b, cb))) = (ks[i], ks[i + g]) else { continue };
                    let (c, label) = match d.order {
                        Order::Mono => (t[a], ca.to_string()),
                        Order::Tri => {
                            let Some((m, cm)) = ks[i + 1] else { continue };
                            (t[(a * n + m) * n + b], [ca, cm, cb].iter().collect())
                        }
                        Order::SymBi if !(ca.is_ascii_punctuation() && cb.is_ascii_punctuation()) => continue,
                        _ => (t[a * n + b], [ca, cb].iter().collect()),
                    };
                    if c != 0.0 {
                        cost += wt * scale * c as f64;
                        ngrams.push(label);
                    }
                }
            }
            if cost > 0.0 {
                let freq = 100.0 * *count as f64 / total;
                sum += freq * cost;
                rows.push(WordCost { word: w.clone(), freq, cost, share: 0.0, ngrams });
            }
        }
        if rows.is_empty() || sum <= 0.0 {
            continue;
        }
        rows.sort_by(|a, b| (b.freq * b.cost).total_cmp(&(a.freq * a.cost)).then_with(|| a.word.cmp(&b.word)));
        rows.truncate(top);
        for r in &mut rows {
            r.share = 100.0 * r.freq * r.cost / sum;
        }
        out.push(MetricWords { id: d.id, name: d.name, unit: d.unit, words: rows });
    }
    out
}
