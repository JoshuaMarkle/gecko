//! CMA-ES for continuous parameters (geometry search, G5): (μ/μ_w, λ)-CMA-ES after Hansen's
//! tutorial (arXiv:1604.00772) with IPOP restarts (λ doubles on each restart) and box bounds.
//! Seeded through `rng::Rng`, so runs reproduce on every platform. See
//! `context/algorithms/geometry-search.md`.
//!
//! With bounds, the search runs in the unit box (x = lo + u·(hi − lo)) and `sigma0` is a
//! fraction of each range; a sample outside is scored at its nearest in-box point plus a
//! penalty growing with the squared distance, while the update uses the sample itself.

use crate::rng::Rng;

#[derive(Clone, Debug)]
pub struct Params {
    /// Initial step size: a fraction of each bound's width, or absolute without bounds.
    pub sigma0: f64,
    /// Population size of the first run (`None`: 4 + ⌊3 ln n⌋); doubled on each restart.
    pub popsize: Option<usize>,
    /// Total evaluation budget over all restarts.
    pub max_evals: usize,
    /// Restarts after the first run (IPOP); 0 = a single run.
    pub restarts: usize,
    /// A run stops when the best values of recent generations and the current population
    /// all lie within this range.
    pub tol_fun: f64,
    /// A run stops when every coordinate's step is below `tol_x · sigma0`.
    pub tol_x: f64,
    /// Stop as soon as a value at or below this is found.
    pub target: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { sigma0: 0.3, popsize: None, max_evals: 10_000, restarts: 9, tol_fun: 1e-12, tol_x: 1e-11, target: f64::NEG_INFINITY }
    }
}

#[derive(Clone, Debug)]
pub struct Bounds {
    pub lo: Vec<f64>,
    pub hi: Vec<f64>,
}

/// Why a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    Budget,
    Target,
    TolFun,
    TolX,
    Condition,
    Stagnation,
}

#[derive(Clone, Debug)]
pub struct RunInfo {
    pub popsize: usize,
    pub generations: usize,
    pub evals: usize,
    pub best: f64,
    pub stop: Stop,
}

#[derive(Clone, Debug)]
pub struct Outcome {
    /// Best point (in bounds) and its value.
    pub x: Vec<f64>,
    pub f: f64,
    pub evals: usize,
    pub runs: Vec<RunInfo>,
    /// (evaluations so far, best value so far) after each generation.
    pub history: Vec<(usize, f64)>,
}

/// Minimize `f` from `x0` (see [`minimize_batch`]).
pub fn minimize(mut f: impl FnMut(&[f64]) -> f64, x0: &[f64], bounds: Option<&Bounds>, p: &Params, rng: &mut Rng) -> Outcome {
    minimize_batch(|xs| xs.iter().map(|x| f(x)).collect(), x0, bounds, p, rng)
}

/// Minimize with a batch evaluator: `eval` scores one generation (all points in bounds) at
/// once, so a caller can score candidates in parallel. The first run starts at `x0`; IPOP
/// restarts start at a uniform point in the bounds (at `x0` without bounds) with twice the
/// previous population.
pub fn minimize_batch(
    mut eval: impl FnMut(&[Vec<f64>]) -> Vec<f64>,
    x0: &[f64],
    bounds: Option<&Bounds>,
    p: &Params,
    rng: &mut Rng,
) -> Outcome {
    let n = x0.len();
    assert!(n > 0, "CMA-ES needs at least one dimension");
    if let Some(b) = bounds {
        assert!(b.lo.len() == n && b.hi.len() == n && b.lo.iter().zip(&b.hi).all(|(l, h)| l < h), "bad bounds");
    }
    // Search space: the unit box with bounds, else the problem space.
    let to_x = |u: &[f64]| -> Vec<f64> {
        match bounds {
            Some(b) => u.iter().enumerate().map(|(i, &v)| b.lo[i] + v.clamp(0.0, 1.0) * (b.hi[i] - b.lo[i])).collect(),
            None => u.to_vec(),
        }
    };
    let to_u = |x: &[f64]| -> Vec<f64> {
        match bounds {
            Some(b) => x.iter().enumerate().map(|(i, &v)| ((v - b.lo[i]) / (b.hi[i] - b.lo[i])).clamp(0.0, 1.0)).collect(),
            None => x.to_vec(),
        }
    };
    let mut out = Outcome { x: to_x(&to_u(x0)), f: f64::INFINITY, evals: 0, runs: Vec::new(), history: Vec::new() };
    let mut lambda = p.popsize.unwrap_or(4 + (3.0 * (n as f64).ln()).floor() as usize).max(2);
    for run in 0..=p.restarts {
        if out.evals >= p.max_evals || out.f <= p.target {
            break;
        }
        let start = if run == 0 || bounds.is_none() { to_u(x0) } else { (0..n).map(|_| rng.f64()).collect() };
        let mut es = Cma::new(start, p.sigma0, lambda);
        let info = es.run(&mut eval, &to_x, bounds.is_some(), p, rng, &mut out);
        out.runs.push(info);
        lambda *= 2;
    }
    out
}

/// One CMA-ES run's state, in search space.
struct Cma {
    n: usize,
    lambda: usize,
    mu: usize,
    w: Vec<f64>,
    mueff: f64,
    cc: f64,
    cs: f64,
    c1: f64,
    cmu: f64,
    ds: f64,
    chin: f64,
    sigma0: f64,
    m: Vec<f64>,
    sigma: f64,
    pc: Vec<f64>,
    ps: Vec<f64>,
    /// Covariance (row-major n × n), its eigenvectors (columns of `b`) and sqrt eigenvalues.
    c: Vec<f64>,
    b: Vec<f64>,
    d: Vec<f64>,
}

impl Cma {
    fn new(m: Vec<f64>, sigma: f64, lambda: usize) -> Self {
        let n = m.len();
        let nf = n as f64;
        let mu = lambda / 2;
        let mut w: Vec<f64> = (1..=mu).map(|i| ((lambda as f64 + 1.0) / 2.0).ln() - (i as f64).ln()).collect();
        let sw: f64 = w.iter().sum();
        w.iter_mut().for_each(|v| *v /= sw);
        let mueff = 1.0 / w.iter().map(|v| v * v).sum::<f64>();
        let cc = (4.0 + mueff / nf) / (nf + 4.0 + 2.0 * mueff / nf);
        let cs = (mueff + 2.0) / (nf + mueff + 5.0);
        let c1 = 2.0 / ((nf + 1.3).powi(2) + mueff);
        let cmu = (1.0 - c1).min(2.0 * (mueff - 2.0 + 1.0 / mueff) / ((nf + 2.0).powi(2) + mueff));
        let ds = 1.0 + 2.0 * (((mueff - 1.0) / (nf + 1.0)).sqrt() - 1.0).max(0.0) + cs;
        let chin = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));
        let mut eye = vec![0.0; n * n];
        (0..n).for_each(|i| eye[i * n + i] = 1.0);
        Cma {
            n,
            lambda,
            mu,
            w,
            mueff,
            cc,
            cs,
            c1,
            cmu,
            ds,
            chin,
            sigma0: sigma,
            m,
            sigma,
            pc: vec![0.0; n],
            ps: vec![0.0; n],
            c: eye.clone(),
            b: eye,
            d: vec![1.0; n],
        }
    }

    fn run(
        &mut self,
        eval: &mut impl FnMut(&[Vec<f64>]) -> Vec<f64>,
        to_x: &impl Fn(&[f64]) -> Vec<f64>,
        bounded: bool,
        p: &Params,
        rng: &mut Rng,
        out: &mut Outcome,
    ) -> RunInfo {
        let n = self.n;
        let nf = n as f64;
        let eigen_every = ((1.0 / (10.0 * nf * (self.c1 + self.cmu))).floor() as usize).max(1);
        // Recent generations' best values (TolFun) and medians (stagnation).
        let hist_len = 10 + (30.0 * nf / self.lambda as f64).ceil() as usize;
        let mut recent: Vec<f64> = Vec::new();
        let mut medians: Vec<f64> = Vec::new();
        let (mut gen, mut evals, mut best) = (0usize, 0usize, f64::INFINITY);
        let stop = loop {
            if out.evals + self.lambda > p.max_evals {
                break Stop::Budget;
            }
            // Sample: y = B·D·z, u = m + σ·y.
            let ys: Vec<Vec<f64>> = (0..self.lambda)
                .map(|_| {
                    let z: Vec<f64> = (0..n).map(|_| normal(rng)).collect();
                    let dz: Vec<f64> = (0..n).map(|j| self.d[j] * z[j]).collect();
                    (0..n).map(|i| (0..n).map(|j| self.b[i * n + j] * dz[j]).sum()).collect()
                })
                .collect();
            let us: Vec<Vec<f64>> = ys.iter().map(|y| (0..n).map(|i| self.m[i] + self.sigma * y[i]).collect()).collect();
            let xs: Vec<Vec<f64>> = us.iter().map(|u| to_x(u)).collect();
            let fs = eval(&xs);
            assert_eq!(fs.len(), xs.len(), "the evaluator must score every point");
            evals += xs.len();
            out.evals += xs.len();
            for (x, &v) in xs.iter().zip(&fs) {
                if v < out.f {
                    out.f = v;
                    out.x = x.clone();
                }
                best = best.min(v);
            }
            out.history.push((out.evals, out.f));
            // Out-of-box samples rank behind their in-box point by the squared distance,
            // scaled to the population's values.
            let fit: Vec<f64> = if bounded {
                let scale = 1.0 + fs.iter().map(|v| v.abs()).fold(0.0, f64::max);
                us.iter().zip(&fs).map(|(u, &v)| v + scale * u.iter().map(|&c| (c - c.clamp(0.0, 1.0)).powi(2)).sum::<f64>()).collect()
            } else {
                fs.clone()
            };
            let mut order: Vec<usize> = (0..self.lambda).collect();
            order.sort_by(|&a, &b| fit[a].total_cmp(&fit[b]).then(a.cmp(&b)));
            gen += 1;

            // Mean, evolution paths, covariance, step size.
            let yw: Vec<f64> = (0..n).map(|i| (0..self.mu).map(|k| self.w[k] * ys[order[k]][i]).sum()).collect();
            for i in 0..n {
                self.m[i] += self.sigma * yw[i];
            }
            // C^(-1/2)·yw = B·D⁻¹·Bᵀ·yw.
            let bt: Vec<f64> = (0..n).map(|j| (0..n).map(|i| self.b[i * n + j] * yw[i]).sum::<f64>() / self.d[j]).collect();
            let inv: Vec<f64> = (0..n).map(|i| (0..n).map(|j| self.b[i * n + j] * bt[j]).sum()).collect();
            let ks = (self.cs * (2.0 - self.cs) * self.mueff).sqrt();
            for i in 0..n {
                self.ps[i] = (1.0 - self.cs) * self.ps[i] + ks * inv[i];
            }
            let ps_norm = self.ps.iter().map(|v| v * v).sum::<f64>().sqrt();
            let hs = ps_norm / (1.0 - (1.0 - self.cs).powi(2 * gen as i32)).sqrt() / self.chin < 1.4 + 2.0 / (nf + 1.0);
            let kc = (self.cc * (2.0 - self.cc) * self.mueff).sqrt();
            for i in 0..n {
                self.pc[i] = (1.0 - self.cc) * self.pc[i] + if hs { kc * yw[i] } else { 0.0 };
            }
            let dh = if hs { 0.0 } else { self.c1 * self.cc * (2.0 - self.cc) };
            let keep = 1.0 - self.c1 - self.cmu + dh;
            for i in 0..n {
                for j in 0..=i {
                    let rank_mu: f64 = (0..self.mu).map(|k| self.w[k] * ys[order[k]][i] * ys[order[k]][j]).sum();
                    let v = keep * self.c[i * n + j] + self.c1 * self.pc[i] * self.pc[j] + self.cmu * rank_mu;
                    self.c[i * n + j] = v;
                    self.c[j * n + i] = v;
                }
            }
            self.sigma *= ((self.cs / self.ds) * (ps_norm / self.chin - 1.0)).min(1.0).exp();
            if gen % eigen_every == 0 {
                let (vals, vecs) = eigh(&self.c, n);
                self.b = vecs;
                self.d = vals.iter().map(|&v| v.max(1e-300).sqrt()).collect();
            }

            if best <= p.target {
                break Stop::Target;
            }
            let mut sorted = fs.clone();
            sorted.sort_by(f64::total_cmp);
            recent.push(sorted[0]);
            medians.push(sorted[sorted.len() / 2]);
            if recent.len() > hist_len {
                recent.remove(0);
            }
            let span = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max) - v.iter().copied().fold(f64::INFINITY, f64::min);
            if recent.len() == hist_len && span(&recent).max(span(&sorted)) < p.tol_fun {
                break Stop::TolFun;
            }
            let dmax = self.d.iter().copied().fold(0.0, f64::max);
            let dmin = self.d.iter().copied().fold(f64::INFINITY, f64::min);
            let step = (0..n).map(|i| self.sigma * self.c[i * n + i].sqrt().max(self.pc[i].abs())).fold(0.0, f64::max);
            if step < p.tol_x * self.sigma0 {
                break Stop::TolX;
            }
            if dmax * dmax > 1e14 * dmin * dmin {
                break Stop::Condition;
            }
            // Stagnation: the median of the last 20% of generations no better than of the
            // 20% before, over at least 120 + 30n/λ generations.
            let window = 120 + (30.0 * nf / self.lambda as f64) as usize;
            if medians.len() > window {
                let k = (medians.len() / 5).max(1);
                let med = |v: &[f64]| {
                    let mut s = v.to_vec();
                    s.sort_by(f64::total_cmp);
                    s[s.len() / 2]
                };
                let (old, new) = (med(&medians[medians.len() - 2 * k..medians.len() - k]), med(&medians[medians.len() - k..]));
                if new >= old {
                    break Stop::Stagnation;
                }
            }
        };
        RunInfo { popsize: self.lambda, generations: gen, evals, best, stop }
    }
}

/// Standard normal sample (Box–Muller).
fn normal(rng: &mut Rng) -> f64 {
    let u1 = 1.0 - rng.f64();
    let u2 = rng.f64();
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

/// Eigen-decomposition of a symmetric n × n matrix (row-major) by cyclic Jacobi rotations:
/// eigenvalues and eigenvectors as the columns of the returned matrix.
fn eigh(a: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut a = a.to_vec();
    let mut v = vec![0.0; n * n];
    (0..n).for_each(|i| v[i * n + i] = 1.0);
    for _ in 0..100 {
        let off: f64 = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).map(|(i, j)| a[i * n + j].powi(2)).sum();
        let diag: f64 = (0..n).map(|i| a[i * n + i].powi(2)).sum();
        if off <= 1e-30 * diag.max(1e-300) {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = a[p * n + q];
                if apq.abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k * n + p], a[k * n + q]);
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p * n + k], a[q * n + k]);
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|i| a[i * n + i]).collect(), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eigh_reconstructs() {
        let n = 4;
        let mut rng = Rng::new(3);
        let r: Vec<f64> = (0..n * n).map(|_| rng.f64() - 0.5).collect();
        // A = R·Rᵀ (symmetric positive semi-definite).
        let a: Vec<f64> = (0..n * n).map(|ij| (0..n).map(|k| r[(ij / n) * n + k] * r[(ij % n) * n + k]).sum()).collect();
        let (vals, vecs) = eigh(&a, n);
        for i in 0..n {
            for j in 0..n {
                let back: f64 = (0..n).map(|k| vecs[i * n + k] * vals[k] * vecs[j * n + k]).sum();
                assert!((back - a[i * n + j]).abs() < 1e-12);
            }
        }
    }
}
