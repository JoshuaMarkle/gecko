//! Profile calibration: spread-normalized weights and weights fitted to a judge metric.
//! See `context/objective.md` (Normalization and profiles, Calibration).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::metrics::{Dir, METRICS};
use crate::report::breakdown;
use crate::rng::Rng;
use crate::{Corpus, Error, Geometry, Layout, Profile, Result, Scorer, State};

/// `profile` with every scored metric at weight 1 and no scales, so a breakdown's `weighted`
/// column is each metric's raw cost (the quantity a weight multiplies).
fn unit_profile(profile: &Profile) -> Profile {
    let mut p = profile.clone();
    p.scale.clear();
    p.weights = METRICS.iter().filter(|d| d.dir != Dir::Info).map(|d| (d.id.to_string(), 1.0)).collect();
    p
}

/// Raw cost of every metric (aligned with `METRICS`) for `st`.
fn costs(sc: &Scorer, st: &State) -> Vec<f64> {
    breakdown(sc, st).metrics.iter().map(|m| m.weighted).collect()
}

/// Population standard deviation of each metric's raw cost over `layouts` (M02/M04/M09 use their
/// squared costs, like the scorer). Metrics with zero spread are left out.
pub fn spread_scales(g: &Geometry, corpus: &Corpus, profile: &Profile, layouts: &[Layout]) -> Result<BTreeMap<String, f32>> {
    let unit = unit_profile(profile);
    let mut rows = Vec::new();
    for l in layouts {
        let sc = Scorer::new(g, corpus, &unit, l)?;
        rows.push(costs(&sc, &sc.identity()));
    }
    let mut out = BTreeMap::new();
    for (i, d) in METRICS.iter().enumerate() {
        let sd = stdev(rows.iter().map(|r| r[i]));
        if sd > 1e-9 {
            out.insert(d.id.to_string(), sd as f32);
        }
    }
    Ok(out)
}

fn stdev(xs: impl Iterator<Item = f64> + Clone) -> f64 {
    let n = xs.clone().count() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mean = xs.clone().sum::<f64>() / n;
    (xs.map(|x| (x - mean) * (x - mean)).sum::<f64>() / n).sqrt()
}

/// Ranks (1-based) with ties sharing their average rank.
fn ranks(xs: &[f64]) -> Vec<f64> {
    let mut idx: Vec<usize> = (0..xs.len()).collect();
    idx.sort_by(|&a, &b| xs[a].total_cmp(&xs[b]));
    let mut r = vec![0.0; xs.len()];
    let mut i = 0;
    while i < idx.len() {
        let mut j = i;
        while j + 1 < idx.len() && xs[idx[j + 1]] == xs[idx[i]] {
            j += 1;
        }
        for &k in &idx[i..=j] {
            r[k] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

/// Spearman rank correlation (Pearson on tie-averaged ranks). NaN when either side is constant.
pub fn spearman(x: &[f64], y: &[f64]) -> f64 {
    let (rx, ry) = (ranks(x), ranks(y));
    let n = rx.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in rx.iter().zip(&ry) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    sxy / (sxx * syy).sqrt()
}

#[derive(Clone, Debug, Serialize)]
pub struct Fit {
    pub target: String,
    /// Fitted weight per feature metric, in spread units (use with `scales`).
    pub weights: BTreeMap<String, f32>,
    pub scales: BTreeMap<String, f32>,
    /// Share of the target's variance over the samples explained by the fit.
    pub r2: f64,
    pub samples: usize,
}

/// Fit non-negative weights (in spread units) so the weighted sum of the other scored metrics
/// predicts the `target` metric's cost, over layouts sampled around `layouts`: each one plus
/// copies scrambled by 1–32 random swaps. Answers "which of our metrics agree with this judge,
/// and how much". Comfort metrics (E*) are never features.
pub fn fit_to(g: &Geometry, corpus: &Corpus, profile: &Profile, layouts: &[Layout], target: &str, seed: u64) -> Result<Fit> {
    let ti = crate::metrics::index_of(target).ok_or_else(|| Error::invalid("fit", format!("unknown metric '{target}'")))?;
    let scales = spread_scales(g, corpus, profile, layouts)?;
    if !scales.contains_key(target) {
        return Err(Error::invalid("fit", format!("{target} has no spread on this geometry (no comfort table?)")));
    }
    let feats: Vec<usize> = METRICS
        .iter()
        .enumerate()
        .filter(|(i, d)| *i != ti && !d.id.starts_with('E') && scales.contains_key(d.id))
        .map(|(i, _)| i)
        .collect();
    let unit = unit_profile(profile);
    let mut rng = Rng::new(seed);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for l in layouts {
        let sc = Scorer::new(g, corpus, &unit, l)?;
        for swaps in [0usize, 1, 2, 4, 8, 16, 32] {
            for _ in 0..if swaps == 0 { 1 } else { 12 } {
                let mut st = sc.identity();
                for _ in 0..swaps {
                    let (a, b) = (rng.below(sc.n), rng.below(sc.n));
                    let d = sc.delta(&st, a, b);
                    sc.apply(&mut st, a, b, d);
                }
                let c = costs(&sc, &st);
                xs.push(feats.iter().map(|&i| c[i] / scales[METRICS[i].id] as f64).collect::<Vec<f64>>());
                ys.push(c[ti]);
            }
        }
    }
    let (w, r2) = nnls(&xs, &ys);
    let weights = feats.iter().zip(&w).filter(|(_, &v)| v > 1e-6).map(|(&i, &v)| (METRICS[i].id.to_string(), v as f32)).collect();
    Ok(Fit { target: target.to_string(), weights, scales, r2, samples: ys.len() })
}

/// Least squares `y ≈ b + X·w` with `w ≥ 0` (free intercept), by cyclic coordinate descent on
/// centred data. Returns `(w, R²)`.
fn nnls(xs: &[Vec<f64>], ys: &[f64]) -> (Vec<f64>, f64) {
    let (n, k) = (ys.len() as f64, xs.first().map_or(0, |r| r.len()));
    let my = ys.iter().sum::<f64>() / n;
    let mx: Vec<f64> = (0..k).map(|j| xs.iter().map(|r| r[j]).sum::<f64>() / n).collect();
    let x: Vec<Vec<f64>> = xs.iter().map(|r| r.iter().zip(&mx).map(|(v, m)| v - m).collect()).collect();
    let y: Vec<f64> = ys.iter().map(|v| v - my).collect();
    let norm: Vec<f64> = (0..k).map(|j| x.iter().map(|r| r[j] * r[j]).sum()).collect();
    let mut w = vec![0.0; k];
    let mut resid = y.clone();
    for _ in 0..500 {
        let mut moved = 0.0f64;
        for j in 0..k {
            if norm[j] < 1e-12 {
                continue;
            }
            let g: f64 = x.iter().zip(&resid).map(|(r, e)| r[j] * e).sum();
            let nw = (w[j] + g / norm[j]).max(0.0);
            let dw = nw - w[j];
            if dw != 0.0 {
                for (r, e) in x.iter().zip(resid.iter_mut()) {
                    *e -= dw * r[j];
                }
                w[j] = nw;
                moved = moved.max(dw.abs());
            }
        }
        if moved < 1e-9 {
            break;
        }
    }
    let ss_res: f64 = resid.iter().map(|e| e * e).sum();
    let ss_tot: f64 = y.iter().map(|e| e * e).sum();
    (w, if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 })
}

#[cfg(test)]
mod tests {
    use super::{nnls, spearman};

    #[test]
    fn spearman_handles_ties_and_order() {
        assert!((spearman(&[1.0, 2.0, 3.0, 4.0], &[10.0, 20.0, 30.0, 400.0]) - 1.0).abs() < 1e-12);
        assert!((spearman(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-12);
        // Ranks with ties: x = [1, 2.5, 2.5, 4], y = [1, 2, 3, 4] → ρ = 4.5 / sqrt(4.5 · 5).
        let r = spearman(&[1.0, 2.0, 2.0, 3.0], &[1.0, 2.0, 3.0, 4.0]);
        assert!((r - 4.5 / (4.5f64 * 5.0).sqrt()).abs() < 1e-12, "{r}");
        assert!(spearman(&[1.0, 1.0], &[1.0, 2.0]).is_nan());
    }

    #[test]
    fn nnls_recovers_nonnegative_weights() {
        // y = 3 + 2·x0 + 0·x1, with x1 anti-correlated noise that a free fit would use negatively.
        let xs: Vec<Vec<f64>> = (0..50).map(|i| vec![i as f64 % 7.0, (i * 3 % 5) as f64]).collect();
        let ys: Vec<f64> = xs.iter().map(|r| 3.0 + 2.0 * r[0]).collect();
        let (w, r2) = nnls(&xs, &ys);
        assert!((w[0] - 2.0).abs() < 1e-6 && w[1].abs() < 1e-6 && r2 > 0.999999, "{w:?} {r2}");
    }
}
