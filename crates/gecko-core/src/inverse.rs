//! Inverse optimization: the weights under which a given layout is (closest to) optimal.
//!
//! A designer who made a layout revealed a preference: the layout over every alternative they
//! could have chosen. [`fit_layout`] turns that into weights. Each weighted metric's weight is
//! `w₀ · e^θ` (w₀ from the base profile), with a log-normal prior θ ~ N(0, σ²), and a squared
//! hinge `½·max(0, Δ/T)²` for each competitor C the layout loses to, Δ = score(L) − score(C).
//! The minimum is the smallest change of weights that makes the layout win; T sets how much
//! residual loss is tolerated against moving the weights. Competitors the layout already beats
//! add nothing, so a layout that is optimal under the base profile keeps the base weights.
//! Competitors come from cutting planes: the layout's tightest
//! swap neighbours, then rounds of search under the current weights, adding every layout that
//! beats it, until the search finds nothing better or the rounds run out. A layout with no
//! better layout under its own weights is *self-consistent*; the gap that remains otherwise is
//! how far it is from optimal under any nearby weights. Weights are normalized to a geometric
//! mean multiplier of 1 (only ratios matter), so fitted profiles of different layouts compare
//! directly ([`cross`]). See `context/objective.md` (Inverse fit).

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;

use crate::metrics::METRICS;
use crate::optimize::{self, Algo, Control, Params};
use crate::robust::{key_distance, mirror_slots, score_of, UnitCosts};
use crate::{Corpus, Error, Geometry, Layout, Profile, Result, Scorer};

/// Settings of an inverse fit.
#[derive(Clone, Debug, Serialize)]
pub struct InverseSpec {
    /// Prior σ of each log weight multiplier (0.5 ≈ ×0.37–×2.7 at 95 %, as in `gecko sweep`).
    pub sigma: f64,
    /// Tolerance T as a share of the median |Δ| of the layout's swap neighbours under the base
    /// profile: a loss of T score points costs as much as moving one weight by σ.
    /// Smaller = the fit tries harder to make the layout win.
    pub tol: f64,
    /// Bound on |θ| (multipliers within ×e^−bound … ×e^bound).
    pub bound: f64,
    /// Search rounds (cutting planes) at most.
    pub rounds: usize,
    /// Evaluations per search.
    pub evals: u64,
    /// Searches per round (different seeds; each starts from the layout, restarts are random).
    pub seeds: u64,
    pub seed: u64,
    pub threads: usize,
    /// Swap neighbours used as competitors from the start (the tightest ones under the base profile).
    pub neighbours: usize,
    /// Metric ids to fit; empty = every metric the profile weights.
    pub fit: Vec<String>,
}

impl Default for InverseSpec {
    fn default() -> Self {
        InverseSpec {
            sigma: 0.5,
            tol: 0.05,
            bound: 3.0,
            rounds: 6,
            evals: 4_000_000,
            seeds: 2,
            seed: 1,
            threads: 1,
            neighbours: 600,
            fit: Vec::new(),
        }
    }
}

/// One fitted weight.
#[derive(Clone, Debug, Serialize)]
pub struct WeightFit {
    pub id: String,
    /// Base profile weight (spread units).
    pub base: f64,
    /// Fitted weight (spread units).
    pub fitted: f64,
    /// fitted / base.
    pub multiplier: f64,
    /// 95 % interval of the multiplier (Laplace approximation of the posterior).
    pub lo: f64,
    pub hi: f64,
    /// Posterior sd of θ / prior σ: near 1 = the layout says nothing about this weight.
    pub sd_ratio: f64,
    /// θ at the bound: the layout needs this weight more extreme than allowed.
    pub at_bound: bool,
}

/// What a search found under one weight vector.
#[derive(Clone, Debug, Serialize)]
pub struct Probe {
    /// The best layout found (slot-ordered chars) and its score under these weights.
    pub best: String,
    pub best_score: f64,
    /// The layout's own score under these weights.
    pub layout_score: f64,
    /// layout_score − best_score (≥ 0; 0 = nothing better found).
    pub gap: f64,
    /// gap as % of |best_score|.
    pub gap_pct: f64,
    /// Keys between the layout and the best (mirror-invariant).
    pub keys: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Round {
    pub round: usize,
    /// Competitors in the fit of this round.
    pub competitors: usize,
    /// Competitors the layout still loses to under the fitted weights.
    pub violated: usize,
    /// sqrt(mean θ²) of the fitted weights.
    pub log_distance: f64,
    /// The search under this round's weights.
    pub probe: Probe,
    /// Better layouts the search added.
    pub added: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct InverseFit {
    pub layout: String,
    pub chars: String,
    pub spec: InverseSpec,
    /// Tolerance T in score points.
    pub tolerance: f64,
    pub weights: Vec<WeightFit>,
    /// The full fitted profile weights (spread units; unfitted weights unchanged).
    pub profile_weights: BTreeMap<String, f32>,
    /// sqrt(mean θ²): how far the layout's implied preferences are from the base profile.
    pub log_distance: f64,
    /// Search under the base profile.
    pub base: Probe,
    /// Search under the fitted weights (the last round).
    pub fitted: Probe,
    /// No better layout found under the fitted weights.
    pub self_consistent: bool,
    /// The last round found no new competitor: more rounds would not change the fit. When this
    /// holds and the layout is not self-consistent, no weights near the base make it optimal.
    pub converged: bool,
    pub rounds: Vec<Round>,
    /// Unit costs of the layout and of the best layout found under the fitted weights (aligned
    /// with `METRICS`), for cross-scoring.
    #[serde(skip)]
    pub costs: Vec<f64>,
    #[serde(skip)]
    pub best_costs: Vec<f64>,
}

impl InverseFit {
    /// The base profile with the fitted weights.
    pub fn profile(&self, base: &Profile) -> Profile {
        let mut p = base.clone();
        p.weights = self.profile_weights.clone();
        let slug: String = self.layout.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c.to_ascii_lowercase() } else { '-' }).collect();
        p.name = format!("{slug}-fit");
        p.description = format!("Weights under which {} is closest to optimal (gecko fit-layout from {}).", self.layout, base.name);
        p
    }
}

/// A competitor layout: its chars and unit costs.
type Competitor = (String, Vec<f64>);

/// The fitting problem in θ: competitor k loses to the layout when
/// `Δ_k(θ) = Σ_i e_i·exp(θ_i)·a_k[i] + b_k ≤ 0`.
struct Problem {
    /// Effective base weight (weight / scale) per fitted metric.
    e: Vec<f64>,
    /// a_k: unit-cost differences (layout − competitor) of the fitted metrics.
    a: Vec<Vec<f64>>,
    /// b_k: the unfitted metrics' share of Δ_k.
    b: Vec<f64>,
    sigma: f64,
    temp: f64,
    bound: f64,
    /// Fix the scale with mean θ = 0 (when every weighted metric is fitted).
    centre: bool,
}

impl Problem {
    fn deltas(&self, th: &[f64]) -> Vec<f64> {
        let w: Vec<f64> = self.e.iter().zip(th).map(|(e, t)| e * t.exp()).collect();
        self.a.iter().zip(&self.b).map(|(a, b)| a.iter().zip(&w).map(|(x, y)| x * y).sum::<f64>() + b).collect()
    }

    /// Objective (prior + squared hinge on losses) and its gradient.
    fn eval(&self, th: &[f64]) -> (f64, Vec<f64>) {
        let n = th.len();
        let w: Vec<f64> = self.e.iter().zip(th).map(|(e, t)| e * t.exp()).collect();
        let s2 = self.sigma * self.sigma;
        let mut f: f64 = th.iter().map(|t| t * t).sum::<f64>() / (2.0 * s2);
        let mut g: Vec<f64> = th.iter().map(|t| t / s2).collect();
        for (a, b) in self.a.iter().zip(&self.b) {
            let d = a.iter().zip(&w).map(|(x, y)| x * y).sum::<f64>() + b;
            let v = (d / self.temp).max(0.0);
            if v == 0.0 {
                continue;
            }
            f += 0.5 * v * v;
            let s = v / self.temp;
            for i in 0..n {
                g[i] += s * w[i] * a[i];
            }
        }
        (f, g)
    }

    /// Project onto {|θ| ≤ bound} (and Σθ = 0 when centring).
    fn project(&self, th: &mut [f64]) {
        for _ in 0..50 {
            if self.centre {
                let m = th.iter().sum::<f64>() / th.len() as f64;
                th.iter_mut().for_each(|t| *t -= m);
            }
            let mut clipped = false;
            for t in th.iter_mut() {
                if t.abs() > self.bound {
                    *t = t.clamp(-self.bound, self.bound);
                    clipped = true;
                }
            }
            if !clipped || !self.centre {
                return;
            }
        }
    }

    /// Projected gradient descent with backtracking from `th`.
    fn solve(&self, mut th: Vec<f64>) -> Vec<f64> {
        self.project(&mut th);
        let (mut f, mut g) = self.eval(&th);
        let mut step = 0.1;
        for _ in 0..5000 {
            let mut moved = false;
            while step > 1e-12 {
                let mut nt: Vec<f64> = th.iter().zip(&g).map(|(t, d)| t - step * d).collect();
                self.project(&mut nt);
                let dist2: f64 = nt.iter().zip(&th).map(|(a, b)| (a - b) * (a - b)).sum();
                let (nf, ng) = self.eval(&nt);
                if nf <= f - 1e-4 * dist2 / step && dist2 > 0.0 {
                    let small = dist2.sqrt() < 1e-7;
                    th = nt;
                    f = nf;
                    g = ng;
                    step *= 1.5;
                    moved = !small;
                    break;
                }
                step *= 0.5;
            }
            if !moved {
                break;
            }
        }
        th
    }

    /// Posterior sd of each θ from the Hessian at `th` (finite differences of the gradient),
    /// restricted to Σθ = 0 when centring.
    fn posterior_sd(&self, th: &[f64]) -> Vec<f64> {
        let n = th.len();
        let h = 1e-4;
        let mut hess = vec![vec![0.0; n]; n];
        for j in 0..n {
            let (mut p, mut m) = (th.to_vec(), th.to_vec());
            p[j] += h;
            m[j] -= h;
            let (gp, gm) = (self.eval(&p).1, self.eval(&m).1);
            for i in 0..n {
                hess[i][j] = (gp[i] - gm[i]) / (2.0 * h);
            }
        }
        // Symmetrize; keep it positive definite (non-convex terms can make it indefinite).
        let s2 = self.sigma * self.sigma;
        for i in 0..n {
            for j in 0..i {
                let v = 0.5 * (hess[i][j] + hess[j][i]);
                hess[i][j] = v;
                hess[j][i] = v;
            }
        }
        if self.centre {
            // Directions along 1 carry no information: pin them with a large curvature.
            for i in 0..n {
                for j in 0..n {
                    hess[i][j] += 1e6 / n as f64;
                }
            }
        }
        let inv = invert_pd(hess, 1.0 / s2);
        (0..n).map(|i| inv[i][i].max(0.0).sqrt()).collect()
    }
}

/// Inverse of a symmetric matrix after raising its eigenvalues to at least `floor` (Jacobi).
fn invert_pd(mut a: Vec<Vec<f64>>, floor: f64) -> Vec<Vec<f64>> {
    let n = a.len();
    let mut v: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| (i == j) as u8 as f64).collect()).collect();
    for _ in 0..100 {
        let off: f64 = (0..n).flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j))).map(|(i, j)| a[i][j] * a[i][j]).sum();
        if off < 1e-18 {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k][p], v[k][q]);
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }
    let lam: Vec<f64> = (0..n).map(|i| a[i][i].max(floor)).collect();
    (0..n).map(|i| (0..n).map(|j| (0..n).map(|k| v[i][k] * v[j][k] / lam[k]).sum()).collect()).collect()
}

/// The metrics to fit: `spec.fit`, or every metric `p` weights.
fn fitted_ids(p: &Profile, spec: &InverseSpec) -> Result<Vec<usize>> {
    let ids: Vec<String> = if spec.fit.is_empty() {
        p.weights.iter().filter(|(_, &w)| w > 0.0).map(|(k, _)| k.clone()).collect()
    } else {
        spec.fit.clone()
    };
    let mut out = Vec::new();
    for id in &ids {
        let i = crate::metrics::index_of(id).ok_or_else(|| Error::invalid("fit-layout", format!("unknown metric '{id}'")))?;
        if p.weight(METRICS[i].id) <= 0.0 {
            return Err(Error::invalid("fit-layout", format!("{id} has weight 0 in {}; give it a base weight to fit it", p.name)));
        }
        out.push(i);
    }
    if out.is_empty() {
        return Err(Error::invalid("fit-layout", "no weighted metric to fit"));
    }
    Ok(out)
}

/// One search under `p`, starting from `layout`: the distinct best layouts found (score, chars).
fn search(g: &Geometry, corpus: &Corpus, p: &Profile, layout: &Layout, movable: &[usize], spec: &InverseSpec, seed: u64) -> Result<Vec<(f64, Layout)>> {
    let sc = Scorer::new(g, corpus, p, layout)?;
    let ctx = optimize::Ctx::new(&sc, movable.to_vec(), sc.identity());
    let params = Params { algo: Algo::Bls, seed, threads: spec.threads, time_ms: None, evals: Some(spec.evals), top: 16, ..Default::default() };
    let o = optimize::run(&ctx, &params, Duration::from_secs(3600), &Control::default(), |_| {});
    o.top.into_iter().map(|t| Ok((t.score, sc.layout_of(&sc.state_from_sym(t.sym), "probe")?))).collect()
}

/// Fit the weights under which `layout` is closest to optimal (module docs). `movable` slots
/// may move in the searches; `progress(round, rounds)` after each search round.
#[allow(clippy::too_many_arguments)]
pub fn fit_layout(
    g: &Geometry,
    corpus: &Corpus,
    base: &Profile,
    name: &str,
    layout: &Layout,
    movable: &[usize],
    spec: &InverseSpec,
    mut progress: impl FnMut(usize, usize),
) -> Result<InverseFit> {
    if spec.evals == 0 || spec.seeds == 0 || spec.sigma <= 0.0 || spec.tol <= 0.0 {
        return Err(Error::invalid("fit-layout", "evals, seeds, sigma and tol must be positive"));
    }
    let fit = fitted_ids(base, spec)?;
    let all_weighted = base.weights.values().filter(|&&w| w > 0.0).count();
    let mut unit = UnitCosts::new(g, corpus, base);
    let cl = unit.of(layout)?;
    let chars_of = |l: &Layout| l.chars.iter().collect::<String>();
    let (flat_g, flat_l) = layout.flat(g)?;
    let mirror = mirror_slots(&flat_g);
    let dist = |b: &Layout| -> Result<usize> { Ok(key_distance(&flat_l, &b.flat(g)?.1, mirror.as_deref())) };

    // Competitors: unit costs and chars (deduplicated by chars).
    let mut comp: Vec<Competitor> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    seen.insert(chars_of(layout));

    // Swap neighbours under the base profile; keep the tightest (smallest |Δ|).
    let sc = Scorer::new(g, corpus, base, layout)?;
    let ctx = optimize::Ctx::new(&sc, movable.to_vec(), sc.identity());
    let st0 = sc.identity();
    let m = &ctx.movable;
    let mut swaps: Vec<(f64, usize, usize)> = Vec::new();
    for i in 0..m.len() {
        for j in i + 1..m.len() {
            swaps.push((sc.delta(&st0, m[i], m[j]), m[i], m[j]));
        }
    }
    if swaps.is_empty() {
        return Err(Error::invalid("fit-layout", "fewer than two movable symbols"));
    }
    let mut mags: Vec<f64> = swaps.iter().map(|s| s.0.abs()).collect();
    mags.sort_by(f64::total_cmp);
    let tolerance = (spec.tol * mags[mags.len() / 2]).max(1e-9);
    swaps.sort_by(|x, y| x.0.abs().total_cmp(&y.0.abs()));
    for &(d, a, b) in swaps.iter().take(spec.neighbours) {
        let mut st = st0.clone();
        sc.apply(&mut st, a, b, d);
        let l = sc.layout_of(&st, "swap")?;
        if seen.insert(chars_of(&l)) {
            comp.push((chars_of(&l), unit.of(&l)?));
        }
    }

    let problem = |comp: &[Competitor]| -> Problem {
        let e: Vec<f64> = fit.iter().map(|&i| base.effective_weight(METRICS[i].id) as f64).collect();
        let fixed: Vec<usize> = (0..METRICS.len()).filter(|i| !fit.contains(i)).collect();
        let a = comp.iter().map(|(_, c)| fit.iter().map(|&i| cl[i] - c[i]).collect()).collect();
        let b = comp
            .iter()
            .map(|(_, c)| fixed.iter().map(|&i| base.effective_weight(METRICS[i].id) as f64 * (cl[i] - c[i])).sum())
            .collect();
        Problem { e, a, b, sigma: spec.sigma, temp: tolerance, bound: spec.bound, centre: fit.len() == all_weighted }
    };
    let weights_of = |th: &[f64]| -> Profile {
        let mut p = base.clone();
        for (k, &i) in fit.iter().enumerate() {
            let id = METRICS[i].id;
            p.weights.insert(id.to_string(), (base.weight(id) as f64 * th[k].exp()) as f32);
        }
        p
    };
    // Search under `p` from the layout over spec.seeds seeds: the probe, and every distinct
    // layout found that beats the layout under `p`.
    let probe = |p: &Profile, round: usize, unit: &mut UnitCosts| -> Result<(Probe, Vec<Competitor>, Vec<f64>)> {
        let own = score_of(&cl, p);
        let mut found: Vec<(f64, Layout)> = Vec::new();
        for s in 0..spec.seeds {
            found.extend(search(g, corpus, p, layout, movable, spec, spec.seed + 1000 * round as u64 + s)?);
        }
        found.sort_by(|x, y| x.0.total_cmp(&y.0));
        let tol = 1e-6 * own.abs().max(1.0);
        let mut better = Vec::new();
        let mut names = std::collections::HashSet::new();
        for (_, l) in &found {
            let c = chars_of(l);
            if names.insert(c.clone()) && c != chars_of(layout) {
                let costs = unit.of(l)?;
                if score_of(&costs, p) < own - tol {
                    better.push((c, costs));
                }
            }
        }
        let (bs, bl) = found.first().map(|(s, l)| (*s, l.clone())).unwrap_or((own, layout.clone()));
        let (best_score, best, keys, best_costs) =
            if bs < own - tol { (bs, bl.clone(), dist(&bl)?, unit.of(&bl)?) } else { (own, layout.clone(), 0, cl.clone()) };
        let gap = own - best_score;
        let pr = Probe {
            best: chars_of(&best),
            best_score,
            layout_score: own,
            gap,
            gap_pct: 100.0 * gap / best_score.abs().max(1e-9),
            keys,
        };
        Ok((pr, better, best_costs))
    };

    let (base_probe, better, _) = probe(base, 0, &mut unit)?;
    for c in better {
        if seen.insert(c.0.clone()) {
            comp.push(c);
        }
    }
    progress(0, spec.rounds);
    let mut th = vec![0.0; fit.len()];
    let mut rounds = Vec::new();
    let mut last: Option<(Probe, Vec<f64>)> = None;
    for r in 1..=spec.rounds.max(1) {
        let pb = problem(&comp);
        th = pb.solve(th);
        let violated = pb.deltas(&th).iter().filter(|&&d| d > 0.0).count();
        let (pr, better, best_costs) = probe(&weights_of(&th), r, &mut unit)?;
        let mut added = 0;
        for c in better {
            if seen.insert(c.0.clone()) {
                comp.push(c);
                added += 1;
            }
        }
        rounds.push(Round { round: r, competitors: pb.a.len(), violated, log_distance: rms(&th), probe: pr.clone(), added });
        progress(r, spec.rounds);
        last = Some((pr, best_costs));
        if added == 0 {
            break;
        }
    }
    let (fitted_probe, best_costs) = last.expect("at least one round");
    let pb = problem(&comp);
    let sd = pb.posterior_sd(&th);
    let p = weights_of(&th);
    let weights = fit
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let id = METRICS[i].id;
            let w0 = base.weight(id) as f64;
            WeightFit {
                id: id.to_string(),
                base: w0,
                fitted: w0 * th[k].exp(),
                multiplier: th[k].exp(),
                lo: (th[k] - 1.96 * sd[k]).exp(),
                hi: (th[k] + 1.96 * sd[k]).exp(),
                sd_ratio: sd[k] / spec.sigma,
                at_bound: th[k].abs() >= spec.bound - 1e-6,
            }
        })
        .collect();
    Ok(InverseFit {
        layout: name.to_string(),
        chars: chars_of(layout),
        spec: spec.clone(),
        tolerance,
        weights,
        profile_weights: p.weights.clone(),
        log_distance: rms(&th),
        base: base_probe,
        self_consistent: fitted_probe.gap <= 0.0,
        converged: rounds.last().is_some_and(|r| r.added == 0),
        fitted: fitted_probe,
        rounds,
        costs: cl,
        best_costs,
    })
}

fn rms(v: &[f64]) -> f64 {
    (v.iter().map(|x| x * x).sum::<f64>() / v.len().max(1) as f64).sqrt()
}

/// Every fitted layout scored under every fit's weights.
#[derive(Clone, Debug, Serialize)]
pub struct CrossMatrix {
    pub layouts: Vec<String>,
    /// `gap_pct[a][b]`: layout a under fit b's weights, % above the best layout known under
    /// them (fit b's search best, or any of the layouts if better). Diagonal = fit b's own gap.
    pub gap_pct: Vec<Vec<f64>>,
    /// `rank[a][b]`: rank of layout a among the layouts under fit b's weights (1 = best).
    pub rank: Vec<Vec<usize>>,
    /// `distance[a][b]`: RMS difference of the log weight multipliers of fits a and b.
    pub distance: Vec<Vec<f64>>,
}

/// Cross-score fits made with the same base profile, geometry and corpus.
pub fn cross(base: &Profile, fits: &[InverseFit]) -> CrossMatrix {
    let n = fits.len();
    let mut gap = vec![vec![0.0; n]; n];
    let mut rank = vec![vec![0; n]; n];
    for b in 0..n {
        let p = fits[b].profile(base);
        let s: Vec<f64> = fits.iter().map(|f| score_of(&f.costs, &p)).collect();
        let best = s.iter().copied().fold(score_of(&fits[b].best_costs, &p), f64::min);
        for a in 0..n {
            gap[a][b] = 100.0 * (s[a] - best) / best.abs().max(1e-9);
            rank[a][b] = 1 + s.iter().filter(|&&x| x < s[a]).count();
        }
    }
    let theta = |f: &InverseFit| -> BTreeMap<String, f64> { f.weights.iter().map(|w| (w.id.clone(), w.multiplier.ln())).collect() };
    let thetas: Vec<_> = fits.iter().map(theta).collect();
    let distance = (0..n)
        .map(|a| {
            (0..n)
                .map(|b| {
                    let d: Vec<f64> = thetas[a].iter().filter_map(|(k, v)| thetas[b].get(k).map(|w| v - w)).collect();
                    rms(&d)
                })
                .collect()
        })
        .collect();
    CrossMatrix { layouts: fits.iter().map(|f| f.layout.clone()).collect(), gap_pct: gap, rank, distance }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(a: Vec<Vec<f64>>, centre: bool) -> Problem {
        let k = a.len();
        Problem { e: vec![1.0; a[0].len()], a, b: vec![0.0; k], sigma: 0.5, temp: 0.01, bound: 3.0, centre }
    }

    #[test]
    fn no_violation_keeps_the_prior() {
        // The layout already beats every competitor by a wide margin: θ stays near 0.
        let p = problem(vec![vec![-1.0, -1.0], vec![-2.0, -0.5]], true);
        let th = p.solve(vec![0.0, 0.0]);
        assert!(th.iter().all(|t| t.abs() < 1e-3), "{th:?}");
    }

    #[test]
    fn violation_moves_the_right_weight() {
        // Competitor wins on metric 0 (a₀ > 0) and loses on metric 1: the fit must lower w₀
        // relative to w₁ until Δ = 1.0·e^θ₀ − 0.8·e^θ₁ ≤ 0.
        let p = problem(vec![vec![1.0, -0.8]], true);
        let th = p.solve(vec![0.0, 0.0]);
        assert!(th[0] < -0.05 && th[1] > 0.05, "{th:?}");
        assert!((th[0] + th[1]).abs() < 1e-9, "centred: {th:?}");
        assert!(p.deltas(&th)[0] < 0.01, "{:?}", p.deltas(&th));
        let sd = p.posterior_sd(&th);
        assert!(sd.iter().all(|s| s.is_finite() && *s >= 0.0 && *s <= 0.5 + 1e-6), "{sd:?}");
    }

    #[test]
    fn infeasible_pushes_weights_down() {
        // A competitor better on every metric cannot be beaten by any positive weights.
        let p = problem(vec![vec![1.0, 1.0]], false);
        let th = p.solve(vec![0.0, 0.0]);
        assert!(th.iter().all(|t| *t < -1.0), "{th:?}");
    }

    #[test]
    fn invert_pd_matches_a_known_inverse() {
        let inv = invert_pd(vec![vec![4.0, 1.0], vec![1.0, 3.0]], 1e-9);
        // [[4,1],[1,3]]⁻¹ = 1/11 · [[3,−1],[−1,4]].
        assert!((inv[0][0] - 3.0 / 11.0).abs() < 1e-9 && (inv[0][1] + 1.0 / 11.0).abs() < 1e-9 && (inv[1][1] - 4.0 / 11.0).abs() < 1e-9);
    }
}
