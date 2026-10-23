//! Non-negative pairwise logistic ranking from community preferences. Scales and fitting
//! use training layouts only; whole layouts (including duplicate/mirror groups) are held out.

use std::collections::BTreeMap;

use serde::Serialize;

use super::{costs, stdev, unit_profile};
use crate::interop::community::Bundle;
use crate::metrics::{Dir, METRICS};
use crate::{rng::Rng, Corpus, Error, Geometry, Profile, Result, Scorer};

#[derive(Clone, Debug, Serialize)]
pub struct RankingFit {
    pub target: String,
    pub seed: u64,
    pub input_hashes: BTreeMap<String, String>,
    pub regularization: f64,
    pub weights: BTreeMap<String, f32>,
    pub scales: BTreeMap<String, f32>,
    pub training_layouts: Vec<String>,
    pub held_out_layouts: Vec<String>,
    pub training_pairs: usize,
    pub held_out_pairs: usize,
    /// Pairwise accuracy, with tied predictions worth half a correct prediction.
    pub training_accuracy: f64,
    pub held_out_accuracy: Option<f64>,
    pub baseline_accuracy: Option<f64>,
    pub engram_accuracy: Option<f64>,
    pub training_loss: f64,
}

/// Fit spread-unit weights with projected gradient descent on pairwise logistic loss,
/// plus L2 regularization (0.01). Only unique, positively liked layouts supply labels;
/// pairs with tied likes carry no preference. Twenty percent of whole layouts are held
/// out before scales, pairs or weights are computed. Engram/default accuracy are baselines,
/// never held-out training labels. This is an opt-in popularity model, not a comfort claim.
pub fn fit_preferences(g: &Geometry, corpus: &Corpus, profile: &Profile, bundle: &Bundle, seed: u64) -> Result<RankingFit> {
    let mut layouts: Vec<_> = bundle.unique().filter(|l| l.likes > 0).collect();
    layouts.sort_by(|a, b| a.name.cmp(&b.name));
    if layouts.len() < 10 {
        return Err(Error::invalid("fit", "likes ranking needs at least ten distinct liked layouts"));
    }
    Rng::new(seed).shuffle(&mut layouts);
    let held = layouts.len() / 5;
    let training = &layouts[held..];
    let unit = unit_profile(profile);
    let mut rows = Vec::with_capacity(layouts.len());
    for l in &layouts {
        let l = l.layout(&g.name)?.fit(g)?;
        let sc = Scorer::new(g, corpus, &unit, &l)?;
        rows.push(costs(&sc, &sc.identity()));
    }
    let train_rows = &rows[held..];
    let mut scales = BTreeMap::new();
    let mut features = Vec::new();
    for (i, d) in METRICS.iter().enumerate() {
        let sd = stdev(train_rows.iter().map(|r| r[i]));
        // Comfort and timing (E*) stay independent judges; info and retired metrics never score.
        if d.dir != Dir::Info && !d.retired && !d.id.starts_with('E') && sd > 1e-8 {
            scales.insert(d.id.to_string(), sd as f32);
            features.push(i);
        }
    }
    if features.is_empty() {
        return Err(Error::invalid("fit", "no scored metric varies across the training layouts"));
    }
    let normalized: Vec<Vec<f64>> = rows.iter().map(|r| features.iter().map(|&i| r[i] / scales[METRICS[i].id] as f64).collect()).collect();
    let likes: Vec<_> = layouts.iter().map(|l| l.likes).collect();
    let train_pairs = preference_pairs(&likes[held..]);
    let test_pairs = preference_pairs(&likes[..held]);
    if train_pairs.is_empty() {
        return Err(Error::invalid("fit", "training layouts have no differing like counts"));
    }
    let differences: Vec<Vec<f64>> = train_pairs
        .iter()
        .map(|&(better, worse)| normalized[held + worse].iter().zip(&normalized[held + better]).map(|(a, b)| a - b).collect())
        .collect();
    let w = logistic_fit(&differences);
    let weights: BTreeMap<_, _> = features.iter().zip(w).map(|(&i, w)| (METRICS[i].id.to_string(), w as f32)).collect();
    // Evaluate the exact f32 weights that are exported to the profile.
    let w: Vec<f64> = features.iter().map(|&i| weights[METRICS[i].id] as f64).collect();
    let scores: Vec<f64> = normalized.iter().map(|r| dot(r, &w)).collect();
    let baseline: Vec<f64> =
        rows.iter().map(|r| r.iter().zip(METRICS).map(|(c, d)| c * profile.effective_weight(d.id) as f64).sum()).collect();
    let engram_index = crate::metrics::index_of("E05").unwrap();
    let engram: Vec<f64> = rows.iter().map(|r| r[engram_index]).collect();
    Ok(RankingFit {
        target: "likes".into(),
        seed,
        input_hashes: BTreeMap::from([
            ("geometry".into(), crate::hash::hex(g.hash())),
            ("corpus".into(), crate::hash::hex(corpus.hash())),
            ("profile".into(), crate::hash::hex(profile.hash())),
            ("community".into(), crate::hash::hex(crate::hash::fnv1a(bundle.to_json().as_bytes()))),
        ]),
        regularization: 0.01,
        weights,
        scales,
        training_layouts: training.iter().map(|l| l.name.clone()).collect(),
        held_out_layouts: layouts[..held].iter().map(|l| l.name.clone()).collect(),
        training_pairs: train_pairs.len(),
        held_out_pairs: test_pairs.len(),
        training_accuracy: accuracy(&scores[held..], &train_pairs).unwrap(),
        held_out_accuracy: accuracy(&scores[..held], &test_pairs),
        baseline_accuracy: accuracy(&baseline[..held], &test_pairs),
        engram_accuracy: (stdev(engram.iter().copied()) > 1e-8).then(|| accuracy(&engram[..held], &test_pairs)).flatten(),
        training_loss: differences.iter().map(|d| softplus(-dot(d, &w))).sum::<f64>() / differences.len() as f64,
    })
}

/// (preferred, other). Every different-count pair has one vote; large like counts do
/// not get interpreted as calibrated comfort labels or extra replicated observations.
fn preference_pairs(likes: &[u32]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for i in 0..likes.len() {
        for j in i + 1..likes.len() {
            match likes[i].cmp(&likes[j]) {
                std::cmp::Ordering::Greater => out.push((i, j)),
                std::cmp::Ordering::Less => out.push((j, i)),
                std::cmp::Ordering::Equal => {}
            }
        }
    }
    out
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

fn softplus(x: f64) -> f64 {
    x.max(0.0) + (-x.abs()).exp().ln_1p()
}

fn logistic_fit(differences: &[Vec<f64>]) -> Vec<f64> {
    let n = differences.len() as f64;
    let mut w = vec![0.0; differences[0].len()];
    let l2 = 0.01;
    // Global Lipschitz bound for this logistic objective, so a fixed step is stable.
    let step = 1.0 / (0.25 * differences.iter().map(|d| dot(d, d)).sum::<f64>() / n + l2);
    for _ in 0..800 {
        let mut gradient: Vec<_> = w.iter().map(|v| l2 * v).collect();
        for d in differences {
            let margin = dot(d, &w);
            let probability = if margin >= 0.0 {
                let e = (-margin).exp();
                e / (1.0 + e)
            } else {
                1.0 / (1.0 + margin.exp())
            };
            for (g, x) in gradient.iter_mut().zip(d) {
                *g -= probability * x / n;
            }
        }
        let mut changed = 0.0f64;
        for (v, g) in w.iter_mut().zip(gradient) {
            let next = (*v - step * g).max(0.0);
            changed = changed.max((next - *v).abs());
            *v = next;
        }
        if changed < 1e-8 {
            break;
        }
    }
    w
}

fn accuracy(scores: &[f64], pairs: &[(usize, usize)]) -> Option<f64> {
    if pairs.is_empty() {
        return None;
    }
    let correct: f64 = pairs
        .iter()
        .map(|&(better, worse)| match scores[better].total_cmp(&scores[worse]) {
            std::cmp::Ordering::Less => 1.0,
            std::cmp::Ordering::Equal => 0.5,
            std::cmp::Ordering::Greater => 0.0,
        })
        .sum();
    Some(correct / pairs.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_labels_and_logistic_sign_are_correct() {
        assert_eq!(preference_pairs(&[3, 1, 3]), vec![(0, 1), (2, 1)]);
        let differences = vec![vec![1.0, -1.0], vec![2.0, -2.0], vec![3.0, -3.0]];
        let w = logistic_fit(&differences);
        assert!(w[0] > 0.0 && w[1] == 0.0, "{w:?}");
        let before: f64 = differences.iter().map(|_| softplus(0.0)).sum();
        let after: f64 = differences.iter().map(|d| softplus(-dot(d, &w))).sum();
        assert!(after < before);
        assert_eq!(accuracy(&[0.0, 1.0, 0.0], &preference_pairs(&[3, 1, 3])), Some(1.0));
        assert_eq!(accuracy(&[0.0, 0.0], &[(0, 1)]), Some(0.5));
        assert!(softplus(1000.0).is_finite() && softplus(-1000.0).is_finite());
    }
}
