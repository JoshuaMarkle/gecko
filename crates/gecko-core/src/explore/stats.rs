//! Statistics over an atlas: re-scoring raw metric vectors under any profile, fitness–distance
//! correlation, PCA embeddings, consensus (backbone), families, funnels, coverage and weight
//! stability. Pure functions over flat arrays so the CLI and WASM share them.
//! `research/search-space-visualization.md` §3–4.

use super::distance::hamming;
use crate::metrics::{Dir, METRICS};
use crate::Profile;

/// How one raw metric value turns into a score contribution.
#[derive(Clone, Copy, Debug)]
enum Term {
    Off,
    /// `w · v` (negative w for metrics where higher is better).
    Linear(f64),
    /// M02: `w · v²` (v is the RMS load error).
    Squared(f64),
    /// M04, M09: `w · (v − target)² / 10` (v is the left-hand / thumb share, target 50 / `thumb_target`).
    Target(f64, f64),
}

/// A profile's weights aligned with an atlas's metric list; `score(raw)` equals
/// `Scorer::full` for the layout the raw vector was measured on (tested), provided the
/// profile's table parameters (`Profile::table_hash`) match the atlas.
#[derive(Clone, Debug)]
pub struct Weights {
    terms: Vec<Term>,
}

impl Weights {
    pub fn new(profile: &Profile, metric_ids: &[String]) -> Self {
        let terms = metric_ids
            .iter()
            .map(|id| {
                let Some(d) = METRICS.iter().find(|d| d.id == id) else { return Term::Off };
                let w = profile.effective_weight(id) as f64;
                if w == 0.0 {
                    return Term::Off;
                }
                match (d.dir, d.id) {
                    (Dir::Info, _) => Term::Off,
                    (_, "M02") => Term::Squared(w),
                    (_, "M04") => Term::Target(w, 50.0),
                    (_, "M09") => Term::Target(w, profile.thumb_target as f64),
                    (Dir::Min, _) => Term::Linear(w),
                    (Dir::Max, _) => Term::Linear(-w),
                }
            })
            .collect();
        Weights { terms }
    }

    /// Contribution of metric `k` with raw value `v`.
    #[inline]
    pub fn term(&self, k: usize, v: f32) -> f64 {
        let v = v as f64;
        match self.terms[k] {
            Term::Off => 0.0,
            Term::Linear(w) => w * v,
            Term::Squared(w) => w * v * v,
            Term::Target(w, t) => w * (v - t) * (v - t) / 10.0,
        }
    }

    pub fn score(&self, raw: &[f32]) -> f64 {
        raw.iter().enumerate().map(|(k, &v)| self.term(k, v)).sum()
    }

    /// Scores of `n` rows of a row-major `raw` matrix.
    pub fn score_all(&self, raw: &[f32], m: usize) -> Vec<f64> {
        if m == 0 {
            return Vec::new();
        }
        raw.chunks_exact(m).map(|r| self.score(r)).collect()
    }

    /// Metrics that carry weight.
    pub fn active(&self) -> Vec<usize> {
        (0..self.terms.len()).filter(|&k| !matches!(self.terms[k], Term::Off)).collect()
    }
}

pub fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return f64::NAN;
    }
    let (mx, my) = (x[..n].iter().sum::<f64>() / n as f64, y[..n].iter().sum::<f64>() / n as f64);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (a, b) = (x[i] - mx, y[i] - my);
        sxy += a * b;
        sxx += a * a;
        syy += b * b;
    }
    if sxx <= 0.0 || syy <= 0.0 {
        return f64::NAN;
    }
    sxy / (sxx * syy).sqrt()
}

/// Index of the lowest score.
pub fn argmin(scores: &[f64]) -> Option<usize> {
    (0..scores.len()).min_by(|&a, &b| scores[a].total_cmp(&scores[b]))
}

/// Fitness–distance correlation (Jones & Forrest): Pearson r between score and Hamming distance
/// to the best layout, over `idx` (all rows if empty). Positive = "big valley".
pub fn fdc(syms: &[u8], s: usize, scores: &[f64], idx: &[u32]) -> f64 {
    let all: Vec<u32>;
    let idx = if idx.is_empty() {
        all = (0..scores.len() as u32).collect();
        &all
    } else {
        idx
    };
    let Some(&b) = idx.iter().min_by(|&&a, &&b| scores[a as usize].total_cmp(&scores[b as usize])) else { return f64::NAN };
    let best = &syms[b as usize * s..(b as usize + 1) * s];
    let d: Vec<f64> = idx.iter().map(|&i| hamming(&syms[i as usize * s..(i as usize + 1) * s], best) as f64).collect();
    let f: Vec<f64> = idx.iter().map(|&i| scores[i as usize]).collect();
    pearson(&d, &f)
}

/// Feature encodings for embeddings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// One-hot symbol × slot (Euclidean distance = √(2·Hamming)).
    Slots,
    /// One-hot symbol × finger (layouts that put letters on the same fingers sit together).
    Fingers,
    /// Standardized raw metric vectors ("do they type alike").
    Metrics,
}

impl Encoding {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "slots" => Some(Encoding::Slots),
            "fingers" => Some(Encoding::Fingers),
            "metrics" => Some(Encoding::Metrics),
            _ => None,
        }
    }
}

/// Dense feature matrix (rows × dims) for `rows` under `enc`.
pub fn features(enc: Encoding, syms: &[u8], s: usize, raw: &[f32], m: usize, finger: &[u8], rows: &[u32]) -> (Vec<f32>, usize) {
    match enc {
        Encoding::Slots | Encoding::Fingers => {
            let groups = if enc == Encoding::Slots { s } else { finger.iter().map(|&f| f as usize + 1).max().unwrap_or(1) };
            let d = s * groups;
            let mut x = vec![0f32; rows.len() * d];
            for (r, &i) in rows.iter().enumerate() {
                let sym = &syms[i as usize * s..(i as usize + 1) * s];
                for (slot, &sy) in sym.iter().enumerate() {
                    let g = if enc == Encoding::Slots { slot } else { finger[slot] as usize };
                    x[r * d + sy as usize * groups + g] = 1.0;
                }
            }
            (x, d)
        }
        Encoding::Metrics => {
            let n = (raw.len() / m.max(1)) as f64;
            let mut mean = vec![0f64; m];
            let mut sd = vec![0f64; m];
            for row in raw.chunks_exact(m) {
                for k in 0..m {
                    mean[k] += row[k] as f64 / n;
                }
            }
            for row in raw.chunks_exact(m) {
                for k in 0..m {
                    sd[k] += (row[k] as f64 - mean[k]).powi(2) / n;
                }
            }
            let mut x = vec![0f32; rows.len() * m];
            for (r, &i) in rows.iter().enumerate() {
                for k in 0..m {
                    let s = sd[k].sqrt();
                    if s > 1e-12 {
                        x[r * m + k] = ((raw[i as usize * m + k] as f64 - mean[k]) / s) as f32;
                    }
                }
            }
            (x, m)
        }
    }
}

/// 2-component PCA of a dense row-major matrix (n × d). Components are fitted on at most
/// `max_fit` evenly strided rows (power iteration with deflation, deterministic start) and
/// every row is projected. Signs are fixed so row 0 has non-negative coordinates (a stable map).
/// Returns (coords n×2, share of variance per component).
pub fn pca2(x: &[f32], d: usize, max_fit: usize) -> (Vec<f32>, [f64; 2]) {
    let n = x.len().checked_div(d).unwrap_or(0);
    if n == 0 {
        return (Vec::new(), [0.0; 2]);
    }
    let stride = n.div_ceil(max_fit.max(1)).max(1);
    let fit: Vec<usize> = (0..n).step_by(stride).collect();
    let mut mean = vec![0f64; d];
    for &i in &fit {
        for j in 0..d {
            mean[j] += x[i * d + j] as f64;
        }
    }
    mean.iter_mut().for_each(|v| *v /= fit.len() as f64);
    let total_var: f64 = fit.iter().map(|&i| (0..d).map(|j| (x[i * d + j] as f64 - mean[j]).powi(2)).sum::<f64>()).sum::<f64>() / fit.len() as f64;
    // C·v without forming C: (1/n) Xcᵀ (Xc v).
    let cov_mul = |v: &[f64]| -> Vec<f64> {
        let mut out = vec![0f64; d];
        for &i in &fit {
            let row = &x[i * d..(i + 1) * d];
            let t: f64 = (0..d).map(|j| (row[j] as f64 - mean[j]) * v[j]).sum();
            if t != 0.0 {
                for j in 0..d {
                    out[j] += (row[j] as f64 - mean[j]) * t;
                }
            }
        }
        out.iter_mut().for_each(|o| *o /= fit.len() as f64);
        out
    };
    let mut comps: Vec<Vec<f64>> = Vec::new();
    let mut eig = [0f64; 2];
    for c in 0..2 {
        let mut v: Vec<f64> = (0..d).map(|j| 1.0 + ((j * 7919 + c * 104_729) % 101) as f64 / 101.0).collect();
        let mut lambda = 0.0;
        for _ in 0..200 {
            for p in &comps {
                let dot: f64 = v.iter().zip(p).map(|(a, b)| a * b).sum();
                v.iter_mut().zip(p).for_each(|(a, b)| *a -= dot * b);
            }
            let w = cov_mul(&v);
            let norm = w.iter().map(|a| a * a).sum::<f64>().sqrt();
            if norm < 1e-15 {
                break;
            }
            let next: Vec<f64> = w.iter().map(|a| a / norm).collect();
            let change: f64 = next.iter().zip(&v).map(|(a, b)| (a - b).abs()).sum();
            v = next;
            lambda = norm;
            if change < 1e-9 {
                break;
            }
        }
        for p in &comps {
            let dot: f64 = v.iter().zip(p).map(|(a, b)| a * b).sum();
            v.iter_mut().zip(p).for_each(|(a, b)| *a -= dot * b);
        }
        let norm = v.iter().map(|a| a * a).sum::<f64>().sqrt().max(1e-15);
        v.iter_mut().for_each(|a| *a /= norm);
        eig[c] = lambda;
        comps.push(v);
    }
    let mut out = vec![0f32; n * 2];
    for i in 0..n {
        for (c, p) in comps.iter().enumerate() {
            out[i * 2 + c] = (0..d).map(|j| (x[i * d + j] as f64 - mean[j]) * p[j]).sum::<f64>() as f32;
        }
    }
    for c in 0..2 {
        if out[c] < 0.0 {
            for i in 0..n {
                out[i * 2 + c] = -out[i * 2 + c];
            }
        }
    }
    let share = if total_var > 0.0 { [eig[0] / total_var, eig[1] / total_var] } else { [0.0; 2] };
    (out, share)
}

/// Share of `rows` that put symbol `y` on slot `x`: returns `p[slot * s + symbol]`.
pub fn consensus(syms: &[u8], s: usize, rows: &[u32]) -> Vec<f32> {
    let mut p = vec![0f32; s * s];
    if rows.is_empty() {
        return p;
    }
    for &i in rows {
        for (slot, &sy) in syms[i as usize * s..(i as usize + 1) * s].iter().enumerate() {
            p[slot * s + sy as usize] += 1.0;
        }
    }
    let n = rows.len() as f32;
    p.iter_mut().for_each(|v| *v /= n);
    p
}

/// Normalized Shannon entropy (0 = every layout agrees, 1 = uniform) per slot of a consensus matrix.
pub fn slot_entropy(p: &[f32], s: usize) -> Vec<f32> {
    let hmax = (s as f64).ln().max(1e-12);
    (0..s)
        .map(|slot| (-p[slot * s..(slot + 1) * s].iter().filter(|&&v| v > 0.0).map(|&v| v as f64 * (v as f64).ln()).sum::<f64>() / hmax) as f32)
        .collect()
}

/// Leader clustering into families: walk `rows` in the given order (best first); a layout
/// joins the first leader within Hamming distance `cut`, else becomes a new leader. Returns the
/// family index per row (families numbered in order of their leader, i.e. by best score).
pub fn families(syms: &[u8], s: usize, rows: &[u32], cut: usize) -> Vec<u32> {
    let mut leaders: Vec<u32> = Vec::new();
    let row = |i: u32| &syms[i as usize * s..(i as usize + 1) * s];
    rows.iter()
        .map(|&i| match leaders.iter().position(|&l| hamming(row(l), row(i)) <= cut) {
            Some(f) => f as u32,
            None => {
                leaders.push(i);
                (leaders.len() - 1) as u32
            }
        })
        .collect()
}

/// Greedy "next best distinct": in order of score, keep layouts at least `dmin` slots away from
/// every one kept so far, up to `k`. Returns row indices.
pub fn distinct_bests(syms: &[u8], s: usize, scores: &[f64], dmin: usize, k: usize) -> Vec<u32> {
    let mut order: Vec<u32> = (0..scores.len() as u32).collect();
    order.sort_by(|&a, &b| scores[a as usize].total_cmp(&scores[b as usize]));
    let row = |i: u32| &syms[i as usize * s..(i as usize + 1) * s];
    let mut out: Vec<u32> = Vec::new();
    for i in order {
        if out.len() >= k {
            break;
        }
        if out.iter().all(|&o| hamming(row(o), row(i)) >= dmin) {
            out.push(i);
        }
    }
    out
}

/// Funnels from LON edges (`from`, `to` row indices): every optimum follows its best improving
/// edge until a sink (no improving edge). Returns the sink row per row.
pub fn funnels(scores: &[f64], edges: &[(u32, u32)]) -> Vec<u32> {
    let n = scores.len();
    let mut next: Vec<u32> = (0..n as u32).collect();
    for &(a, b) in edges {
        let (a, b) = (a as usize, b as usize);
        if a < n && b < n && scores[b] < scores[a] - 1e-12 && scores[b] < scores[next[a] as usize] {
            next[a] = b as u32;
        }
    }
    let mut sink = vec![u32::MAX; n];
    for i in 0..n {
        let mut path = Vec::new();
        let mut j = i;
        while sink[j] == u32::MAX && next[j] as usize != j {
            path.push(j);
            j = next[j] as usize;
        }
        let root = if sink[j] == u32::MAX { j as u32 } else { sink[j] };
        sink[j] = root;
        for p in path {
            sink[p] = root;
        }
    }
    sink
}

/// Capture–recapture coverage (Chao1) from hit counts: observed classes and the estimated total.
pub fn chao1(hits: &[u32]) -> (usize, f64) {
    let s = hits.len();
    let f1 = hits.iter().filter(|&&h| h == 1).count() as f64;
    let f2 = hits.iter().filter(|&&h| h == 2).count() as f64;
    let est = if f2 > 0.0 { s as f64 + f1 * f1 / (2.0 * f2) } else { s as f64 + f1 * (f1 - 1.0).max(0.0) / 2.0 };
    (s, est)
}

/// Weight stability: for each metric `k` and each weight factor, the row that would score best
/// if that metric's weight were multiplied by the factor (all else equal). Contributions are
/// linear in each weight, so this is exact. Returns `winner[k * factors.len() + f]`.
pub fn stability(w: &Weights, raw: &[f32], m: usize, metrics: &[usize], factors: &[f64]) -> Vec<u32> {
    let n = raw.len().checked_div(m).unwrap_or(0);
    let base: Vec<f64> = w.score_all(raw, m);
    let mut out = Vec::with_capacity(metrics.len() * factors.len());
    for &k in metrics {
        let c: Vec<f64> = (0..n).map(|i| w.term(k, raw[i * m + k])).collect();
        for &f in factors {
            let mut best = (f64::INFINITY, 0u32);
            for i in 0..n {
                let v = base[i] + (f - 1.0) * c[i];
                if v < best.0 {
                    best = (v, i as u32);
                }
            }
            out.push(best.1);
        }
    }
    out
}

/// Autocorrelation ρ(0..=max_lag) of score along random walks (pooled mean and variance), and
/// the correlation length ℓ = −1 / ln ρ(1) (Weinberger).
pub fn autocorrelation(walks: &[Vec<f64>], max_lag: usize) -> (Vec<f64>, f64) {
    let all: Vec<f64> = walks.iter().flatten().copied().collect();
    if all.len() < 2 {
        return (vec![1.0], f64::NAN);
    }
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    let var = all.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / all.len() as f64;
    let rho: Vec<f64> = (0..=max_lag)
        .map(|k| {
            let (mut s, mut c) = (0.0, 0usize);
            for w in walks {
                for t in 0..w.len().saturating_sub(k) {
                    s += (w[t] - mean) * (w[t + k] - mean);
                    c += 1;
                }
            }
            if c == 0 || var <= 0.0 {
                0.0
            } else {
                s / c as f64 / var
            }
        })
        .collect();
    let r1 = rho.get(1).copied().unwrap_or(0.0);
    let ell = if r1 > 0.0 && r1 < 1.0 { -1.0 / r1.ln() } else { f64::NAN };
    (rho, ell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funnels_follow_best_improving_edge() {
        let scores = [5.0, 3.0, 4.0, 1.0, 2.0];
        // 0→1, 0→2, 1→3, 2→4; 4 has no improving edge.
        let f = funnels(&scores, &[(0, 1), (0, 2), (1, 3), (2, 4), (3, 0)]);
        assert_eq!(f, vec![3, 3, 4, 3, 4]);
    }

    #[test]
    fn leader_families_and_distinct() {
        let s = 4;
        let syms = [0u8, 1, 2, 3, /**/ 1, 0, 2, 3, /**/ 3, 2, 1, 0];
        assert_eq!(families(&syms, s, &[0, 1, 2], 2), vec![0, 0, 1]);
        assert_eq!(distinct_bests(&syms, s, &[1.0, 2.0, 3.0], 3, 5), vec![0, 2]);
        let p = consensus(&syms, s, &[0, 1]);
        assert_eq!(p[0], 0.5);
        assert_eq!(p[2 * s + 2], 1.0);
        let h = slot_entropy(&p, s);
        assert!(h[0] > 0.0 && h[2] == 0.0);
    }

    #[test]
    fn pca_separates_two_clusters() {
        // Two tight clusters along the first axis.
        let mut x = Vec::new();
        for i in 0..40 {
            let c = if i % 2 == 0 { 5.0 } else { -5.0 };
            x.extend([c + (i as f32) * 0.01, (i as f32 % 3.0) * 0.1, 0.0]);
        }
        let (y, share) = pca2(&x, 3, 1000);
        assert!(share[0] > 0.95);
        for i in 0..40 {
            assert_eq!(y[i * 2] > 0.0, i % 2 == 0);
        }
    }

    #[test]
    fn chao_and_autocorrelation() {
        assert_eq!(chao1(&[1, 1, 2, 5]), (4, 6.0));
        let walk: Vec<f64> = (0..200).map(|t| (t as f64 / 10.0).sin()).collect();
        let (rho, ell) = autocorrelation(&[walk], 5);
        assert!((rho[0] - 1.0).abs() < 1e-9 && rho[1] > 0.9 && ell > 5.0);
    }
}
