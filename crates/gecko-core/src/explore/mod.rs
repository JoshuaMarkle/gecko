//! Search-space exploration: instead of one best layout, map the space of good layouts.
//! A landscape probe, then BLS workers with exclusion restarts feeding a shared archive of every
//! local optimum (with raw metric vectors, so any profile can re-rank it later) and the kicks
//! between them. Output is an atlas in the run directory. `context/algorithms/explore.md`,
//! `research/search-space-visualization.md`.

pub mod archive;
pub mod atlas;
pub mod distance;
pub mod probe;
pub mod qd;
pub mod stats;

use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::metrics::METRICS;
use crate::optimize::{self, Algo, Control, Ctx, ExploreProgress, Outcome, Params, Progress};
use crate::{hash, Result, Scorer, SCHEMA};
use archive::Archive;
use atlas::{Best, Counts, Coverage, ProbeSummary, Summary, CONSTRAINED};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ExploreParams {
    /// Random layouts measured by the probe.
    pub samples: usize,
    /// Random swap walks for the autocorrelation.
    pub walks: usize,
    pub walk_len: usize,
    /// Archive cap; beyond it the worst optima are dropped.
    pub max_optima: usize,
    /// How often the atlas files are rewritten during the run.
    pub snapshot_ms: u64,
    /// Coverage counts optima within this fraction of the best score.
    pub within: f64,
}

impl Default for ExploreParams {
    fn default() -> Self {
        ExploreParams { samples: 2000, walks: 64, walk_len: 400, max_optima: 100_000, snapshot_ms: 2000, within: 0.01 }
    }
}

/// The metric list every atlas uses (raw vectors are in this order).
pub fn metric_ids() -> Vec<String> {
    METRICS.iter().map(|d| d.id.to_string()).collect()
}

struct Writer<'a> {
    dir: &'a Path,
    id: &'a str,
    sc: &'a Scorer,
    symmetric: bool,
    probe: ProbeSummary,
    params: &'a Params,
}

impl Writer<'_> {
    fn write(&self, archive: &Archive, done: bool, elapsed_ms: u64, evals: u64) -> Result<Summary> {
        let snap = archive.snapshot();
        let metrics = metric_ids();
        let s = self.sc.n;
        atlas::write_atomic(&self.dir.join("optima.bin"), &atlas::write_optima(s, &metrics, &snap.optima))?;
        atlas::write_atomic(&self.dir.join("edges.bin"), &atlas::write_edges(&snap.edges))?;

        let chars = |sym: &[u8]| -> String { sym.iter().map(|&x| self.sc.chars[x as usize]).collect() };
        let free: Vec<usize> = (0..snap.optima.len()).filter(|&i| snap.optima[i].flags & CONSTRAINED == 0).collect();
        let syms: Vec<u8> = snap.optima.iter().flat_map(|o| o.sym.iter().copied()).collect();
        let scores: Vec<f64> = snap.optima.iter().map(|o| o.score).collect();
        let free_idx: Vec<u32> = free.iter().map(|&i| i as u32).collect();
        let best = free.first().map(|&i| &snap.optima[i]);
        let pairs: Vec<(u32, u32)> = snap.edges.iter().map(|e| (e.from, e.to)).collect();
        let sinks = stats::funnels(&scores, &pairs);
        let mut bottoms: Vec<u32> = free.iter().map(|&i| sinks[i]).collect();
        bottoms.sort_unstable();
        bottoms.dedup();
        let coverage = match best {
            Some(b) => {
                let lim = b.score + self.params.explore.within * b.score.abs();
                let hits: Vec<u32> = free.iter().map(|&i| &snap.optima[i]).filter(|o| o.score <= lim).map(|o| o.hits).collect();
                let (observed, estimate) = stats::chao1(&hits);
                Coverage { within: self.params.explore.within, observed, estimate }
            }
            None => Coverage::default(),
        };
        let dist = self.params.bls.exclusion_dist;
        let distinct = stats::distinct_bests(&syms, s, &scores, dist, 12)
            .into_iter()
            .map(|i| {
                let o = &snap.optima[i as usize];
                Best { score: o.score, chars: chars(&o.sym), d_best: best.map_or(0, |b| distance::hamming(&b.sym, &o.sym)) }
            })
            .collect();
        let summary = Summary {
            schema: SCHEMA,
            id: self.id.to_string(),
            done,
            elapsed_ms,
            evals,
            chars: self.sc.chars.iter().collect(),
            slots: s,
            metrics,
            symmetric: self.symmetric,
            table_hash: hash::hex(self.sc.profile.table_hash()),
            counts: Counts {
                optima: snap.optima.len(),
                constrained: snap.optima.len() - free.len(),
                hits: snap.hits,
                edges: snap.edges.len(),
                pruned: snap.pruned,
            },
            best: best.map(|b| Best { score: b.score, chars: chars(&b.sym), d_best: 0 }),
            probe: self.probe.clone(),
            fdc: stats::fdc(&syms, s, &scores, &free_idx),
            funnels: bottoms.len(),
            coverage,
            distinct_dist: dist,
            distinct,
        };
        atlas::write_atomic(&self.dir.join("atlas.json"), serde_json::to_string_pretty(&summary).unwrap().as_bytes())?;
        Ok(summary)
    }
}

/// Run an exploration: probe, then BLS with exclusion restarts on every thread for the rest of
/// the budget. Writes `samples.bin` once and `optima.bin`, `edges.bin`, `atlas.json` every
/// `snapshot_ms` and at the end. `ctx.observer` is ignored (the archive is the observer).
pub fn run(
    ctx: &Ctx,
    params: &Params,
    interval: Duration,
    control: &Control,
    dir: &Path,
    id: &str,
    mut on_progress: impl FnMut(&Progress),
) -> Result<(Outcome, Summary)> {
    let sc = ctx.scorer;
    let ep = &params.explore;
    let t0 = Instant::now();
    let start_chars: String = ctx.start.chars(sc).iter().collect();
    let phase = |phase: &str| Some(ExploreProgress { phase: phase.into(), ..Default::default() });
    on_progress(&Progress { elapsed_ms: 0, evals: 0, best: ctx.start.score, best_chars: start_chars.clone(), explore: phase("probe") });

    // Mirror canonicalization only when nothing is pinned and scores are mirror-symmetric.
    let mirror = if ctx.movable.len() == sc.n { distance::score_symmetry(sc) } else { None };
    let pr = probe::run(ctx, params.seed, params.threads, ep.samples, ep.walks, ep.walk_len, control);
    let metrics = metric_ids();
    atlas::write_atomic(&dir.join("samples.bin"), &atlas::write_samples(metrics.len(), &pr.samples))?;
    let (rho, corr_length) = stats::autocorrelation(&pr.walks, 50);
    let n = pr.samples.len().max(1) as f64;
    let mean = pr.samples.iter().map(|s| s.score).sum::<f64>() / n;
    let probe = ProbeSummary {
        samples: pr.samples.len(),
        random_mean: mean,
        random_sd: (pr.samples.iter().map(|s| (s.score - mean).powi(2)).sum::<f64>() / n).sqrt(),
        random_min: pr.samples.iter().map(|s| s.score).fold(f64::INFINITY, f64::min),
        walks: pr.walks.len(),
        walk_len: ep.walk_len,
        rho,
        corr_length,
    };
    let probe_ms = t0.elapsed().as_millis() as u64;

    let archive = Archive::new(sc, mirror.clone(), ep.max_optima, 16, params.bls.exclusion_dist);
    let sctx = ctx.clone().with_observer(&archive);
    let mut p = params.clone();
    p.algo = Algo::Explore;
    if let Some(t) = p.time_ms {
        p.time_ms = Some(t.saturating_sub(probe_ms).max(500));
    }
    let w = Writer { dir, id, sc, symmetric: mirror.is_some(), probe, params };
    let mut version = 0u64;
    let mut last_write = Instant::now();
    let snapshot_every = Duration::from_millis(ep.snapshot_ms.max(250));
    let out = optimize::run(&sctx, &p, interval, control, |pg| {
        let (elapsed_ms, evals) = (pg.elapsed_ms + probe_ms, pg.evals + pr.evals);
        if last_write.elapsed() >= snapshot_every && !archive.is_empty() && w.write(&archive, false, elapsed_ms, evals).is_ok() {
            version += 1;
            last_write = Instant::now();
        }
        let (optima, hits, edges) = archive.counts();
        let explore = Some(ExploreProgress { phase: "search".into(), optima: optima as u64, hits, edges: edges as u64, version });
        on_progress(&Progress { elapsed_ms, evals, best: pg.best, best_chars: pg.best_chars.clone(), explore });
    });
    let mut out = out;
    out.elapsed_ms += probe_ms;
    out.evals += pr.evals;
    let summary = w.write(&archive, true, out.elapsed_ms, out.evals)?;
    Ok((out, summary))
}
