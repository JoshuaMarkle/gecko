//! MAP-Elites quality-diversity archive (Mouret & Clune 2015): a grid over 1–3 behaviour
//! descriptors (roll/alternation ratio, index load, hand balance, or any raw metric), where each
//! cell keeps the best layout with that behaviour. Workers pick a random elite, kick it with a
//! few random swaps, run a short steepest descent and offer the result to its cell. Output is a
//! map of good but different layouts (`qd.json` + `elites.bin` in the run directory).
//! `context/algorithms/map-elites.md`.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::atlas::{self, Optimum};
use super::metric_ids;
use crate::geometry::FINGERS;
use crate::metrics::{index_of, METRICS};
pub use crate::optimize::map_elites::{MapElites, QdParams};
use crate::optimize::{
    self, steepest_descent_capped, Algo, Control, Ctx, ExploreProgress, Kick, Observer, Outcome, Params, Progress, TopK,
};
use crate::report::breakdown;
use crate::rng::Rng;
use crate::scorer::{Scorer, State};
use crate::{hash, Error, Result, SCHEMA};

/// Named descriptors besides raw metric ids: (name, description).
pub const DESCRIPTORS: &[(&str, &str)] = &[
    ("roll_ratio", "rolls / (rolls + alternates), % of T03 + T04 over T01 + T03 + T04"),
    ("index_load", "% of keystrokes on the two index fingers"),
    ("pinky_load", "% of keystrokes on the two pinkies"),
    ("hand_balance", "% of keystrokes on the left hand (M04)"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
enum Desc {
    RollRatio {
        t01: usize,
        t03: usize,
        t04: usize,
    },
    /// Share of the load on these two fingers.
    Fingers(usize, usize),
    /// A raw metric value (index into `METRICS`).
    Metric(usize),
}

impl Desc {
    fn parse(name: &str) -> Option<Desc> {
        let ix = |id: &str| index_of(id).expect("metric exists");
        Some(match name {
            "roll_ratio" => Desc::RollRatio { t01: ix("T01"), t03: ix("T03"), t04: ix("T04") },
            "index_load" => Desc::Fingers(3, 6),
            "pinky_load" => Desc::Fingers(0, 9),
            "hand_balance" => Desc::Metric(ix("M04")),
            id => Desc::Metric(index_of(id)?),
        })
    }

    /// Percent-valued descriptors are clamped to 0..100 when ranges are automatic.
    fn percent(self) -> bool {
        match self {
            Desc::Metric(k) => METRICS[k].unit == "%",
            _ => true,
        }
    }

    fn value(self, raw: &[f32], load: &[f64; FINGERS]) -> f64 {
        match self {
            Desc::RollRatio { t01, t03, t04 } => {
                let rolls = raw[t03] as f64 + raw[t04] as f64;
                100.0 * rolls / (rolls + raw[t01] as f64).max(1e-12)
            }
            Desc::Fingers(a, b) => 100.0 * (load[a] + load[b]) / load.iter().sum::<f64>().max(1e-12),
            Desc::Metric(k) => raw[k] as f64,
        }
    }
}

/// One behaviour axis as given: `name` or `name:lo:hi`.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisSpec {
    pub name: String,
    pub range: Option<(f64, f64)>,
}

/// Parse and validate an axis spec.
pub fn parse_axis(s: &str) -> Result<AxisSpec> {
    let bad = |m: String| Error::invalid("qd axis", m);
    let mut it = s.split(':');
    let name = it.next().unwrap_or("").trim().to_string();
    if Desc::parse(&name).is_none() {
        let names: Vec<&str> = DESCRIPTORS.iter().map(|d| d.0).collect();
        return Err(bad(format!("unknown descriptor '{name}' (use {} or a metric id)", names.join(", "))));
    }
    let rest: Vec<&str> = it.collect();
    let range = match rest.as_slice() {
        [] => None,
        [lo, hi] => {
            let num = |x: &str| x.trim().parse::<f64>().map_err(|_| bad(format!("'{s}': bad number '{x}'")));
            let (lo, hi) = (num(lo)?, num(hi)?);
            if hi <= lo {
                return Err(bad(format!("'{s}': need lo < hi")));
            }
            Some((lo, hi))
        }
        _ => return Err(bad(format!("'{s}': use name or name:lo:hi"))),
    };
    Ok(AxisSpec { name, range })
}

/// A resolved axis (as written to `qd.json`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Axis {
    pub name: String,
    pub lo: f64,
    pub hi: f64,
    pub bins: usize,
    /// Range came from the seed descents rather than the spec.
    pub auto: bool,
}

impl Axis {
    /// Bin of value `v`; values outside the range land in the edge bins.
    pub fn bin(&self, v: f64) -> usize {
        let t = (v - self.lo) / (self.hi - self.lo);
        ((t * self.bins as f64).floor().max(0.0) as usize).min(self.bins - 1)
    }
}

/// A layout's measurements: exact score, raw metric vector, finger loads.
struct Measured {
    sym: Vec<u8>,
    score: f64,
    raw: Vec<f32>,
    load: [f64; FINGERS],
}

fn measure(sc: &Scorer, st: &State) -> Measured {
    // Fresh loads and a from-scratch score (as `best.json` reports it): no delta drift is stored.
    let mut st = st.clone();
    st.load = sc.loads(&st.pos);
    let raw = breakdown(sc, &st).metrics.iter().map(|m| m.value as f32).collect();
    Measured { score: sc.full(&st), raw, load: st.load, sym: st.sym }
}

#[derive(Clone, Debug)]
pub struct Elite {
    pub sym: Vec<u8>,
    pub score: f64,
    pub raw: Vec<f32>,
    /// Descriptor values, one per axis.
    pub desc: Vec<f32>,
    /// Active run time when this elite entered its cell.
    pub first_ms: u32,
    /// Times this cell's elite was replaced by a better one.
    pub updates: u32,
}

#[derive(Default)]
struct Inner {
    cells: Vec<Option<Elite>>,
    filled: Vec<u32>,
    offspring: u64,
    added: u64,
    improved: u64,
}

/// The MAP-Elites grid, shared by all workers behind a mutex (touched once per offspring, never
/// in the swap loop). Cell index = Σ bin_k · bins^k (axis 0 varies fastest).
pub struct Grid<'a> {
    sc: &'a Scorer,
    descs: Vec<Desc>,
    pub axes: Vec<Axis>,
    t0: Instant,
    inner: Mutex<Inner>,
}

impl<'a> Grid<'a> {
    pub fn new(sc: &'a Scorer, specs: &[AxisSpec], ranges: &[(f64, f64)], bins: usize) -> Self {
        let bins = bins.max(1);
        let descs: Vec<Desc> = specs.iter().map(|a| Desc::parse(&a.name).expect("validated axis")).collect();
        let axes =
            specs.iter().zip(ranges).map(|(a, &(lo, hi))| Axis { name: a.name.clone(), lo, hi, bins, auto: a.range.is_none() }).collect();
        let cells = bins.pow(specs.len() as u32);
        Grid { sc, descs, axes, t0: Instant::now(), inner: Mutex::new(Inner { cells: vec![None; cells], ..Default::default() }) }
    }

    pub fn cells(&self) -> usize {
        self.inner.lock().unwrap().cells.len()
    }

    /// (filled cells, offspring offered)
    pub fn counts(&self) -> (usize, u64) {
        let g = self.inner.lock().unwrap();
        (g.filled.len(), g.offspring)
    }

    fn descriptors(&self, m: &Measured) -> Vec<f32> {
        self.descs.iter().map(|d| d.value(&m.raw, &m.load) as f32).collect()
    }

    fn cell_of(&self, desc: &[f32]) -> usize {
        let mut cell = 0;
        for (a, &v) in self.axes.iter().zip(desc).rev() {
            cell = cell * a.bins + a.bin(v as f64);
        }
        cell
    }

    /// Per-axis bins of a cell index.
    pub fn bins_of(&self, mut cell: usize) -> Vec<usize> {
        self.axes
            .iter()
            .map(|a| {
                let b = cell % a.bins;
                cell /= a.bins;
                b
            })
            .collect()
    }

    /// Offer a measured layout; returns its cell.
    fn insert(&self, m: Measured) -> usize {
        let desc = self.descriptors(&m);
        let cell = self.cell_of(&desc);
        let first_ms = self.t0.elapsed().as_millis().min(u32::MAX as u128) as u32;
        let mut g = self.inner.lock().unwrap();
        g.offspring += 1;
        match &mut g.cells[cell] {
            Some(e) if m.score < e.score - 1e-9 => {
                *e = Elite { sym: m.sym, score: m.score, raw: m.raw, desc, first_ms, updates: e.updates + 1 };
                g.improved += 1;
            }
            Some(_) => {}
            slot @ None => {
                *slot = Some(Elite { sym: m.sym, score: m.score, raw: m.raw, desc, first_ms, updates: 0 });
                g.filled.push(cell as u32);
                g.added += 1;
            }
        }
        cell
    }

    /// Filled cells as (cell, elite), best first.
    pub fn elites(&self) -> Vec<(usize, Elite)> {
        let g = self.inner.lock().unwrap();
        let mut v: Vec<(usize, Elite)> = g.filled.iter().map(|&c| (c as usize, g.cells[c as usize].clone().unwrap())).collect();
        v.sort_by(|a, b| a.1.score.total_cmp(&b.1.score).then(a.0.cmp(&b.0)));
        v
    }
}

impl Observer for Grid<'_> {
    fn optimum(&self, st: &State, _constrained: bool) -> u32 {
        self.insert(measure(self.sc, st)) as u32
    }

    fn edge(&self, _from: u32, _to: u32, _kick: Kick, _strength: u8) {}

    fn parent(&self, rng: &mut Rng) -> Option<Vec<u8>> {
        let g = self.inner.lock().unwrap();
        if g.filled.is_empty() {
            return None;
        }
        let c = g.filled[rng.below(g.filled.len())] as usize;
        g.cells[c].as_ref().map(|e| e.sym.clone())
    }
}

/// One filled cell in `qd.json`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EliteOut {
    /// Row in `elites.bin` (same order: best first).
    pub row: usize,
    pub cell: usize,
    /// Bin per axis.
    pub bins: Vec<usize>,
    /// Descriptor value per axis.
    pub desc: Vec<f32>,
    pub score: f64,
    pub chars: String,
    pub updates: u32,
}

/// `qd.json`: the grid and every elite.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QdSummary {
    pub schema: u32,
    pub id: String,
    pub done: bool,
    pub elapsed_ms: u64,
    pub evals: u64,
    /// Symbol id → char.
    pub chars: String,
    pub slots: usize,
    /// Metric ids of the raw vectors in `elites.bin`.
    pub metrics: Vec<String>,
    /// Profile table hash; re-scoring raw vectors is exact only under a matching profile.
    pub table_hash: String,
    pub axes: Vec<Axis>,
    pub cells: usize,
    pub filled: usize,
    /// filled / cells.
    pub coverage: f64,
    /// Offspring offered to the grid (incl. seeds).
    pub offspring: u64,
    /// Offspring that filled an empty cell / replaced a worse elite.
    pub added: u64,
    pub improved: u64,
    /// Mean score of the random layouts the seeds started from.
    pub random_mean: f64,
    /// QD-score: Σ over filled cells of (random_mean − score). Higher = more and better cells.
    pub qd_score: f64,
    pub best: Option<EliteOut>,
    pub elites: Vec<EliteOut>,
}

struct Writer<'a> {
    dir: &'a Path,
    id: &'a str,
    sc: &'a Scorer,
    random_mean: f64,
}

impl Writer<'_> {
    fn write(&self, grid: &Grid, done: bool, elapsed_ms: u64, evals: u64) -> Result<QdSummary> {
        let el = grid.elites();
        let metrics = metric_ids();
        let optima: Vec<Optimum> = el
            .iter()
            .map(|(_, e)| Optimum {
                sym: e.sym.clone(),
                score: e.score,
                raw: e.raw.clone(),
                hits: e.updates,
                first_ms: e.first_ms,
                flags: 0,
            })
            .collect();
        atlas::write_atomic(&self.dir.join("elites.bin"), &atlas::write_optima(self.sc.n, &metrics, &optima))?;
        let elites: Vec<EliteOut> = el
            .iter()
            .enumerate()
            .map(|(row, (cell, e))| EliteOut {
                row,
                cell: *cell,
                bins: grid.bins_of(*cell),
                desc: e.desc.clone(),
                score: e.score,
                chars: e.sym.iter().map(|&x| self.sc.chars[x as usize]).collect(),
                updates: e.updates,
            })
            .collect();
        let (cells, g) = (grid.cells(), grid.inner.lock().unwrap());
        let summary = QdSummary {
            schema: SCHEMA,
            id: self.id.to_string(),
            done,
            elapsed_ms,
            evals,
            chars: self.sc.chars.iter().collect(),
            slots: self.sc.n,
            metrics,
            table_hash: hash::hex(self.sc.profile.table_hash()),
            axes: grid.axes.clone(),
            cells,
            filled: elites.len(),
            coverage: elites.len() as f64 / cells.max(1) as f64,
            offspring: g.offspring,
            added: g.added,
            improved: g.improved,
            random_mean: self.random_mean,
            qd_score: elites.iter().map(|e| self.random_mean - e.score).sum(),
            best: elites.first().cloned(),
            elites,
        };
        drop(g);
        atlas::write_atomic(&self.dir.join("qd.json"), serde_json::to_string_pretty(&summary).unwrap().as_bytes())?;
        Ok(summary)
    }
}

/// Load `qd.json` from a run directory.
pub fn load(dir: &Path) -> Result<QdSummary> {
    let b = std::fs::read(dir.join("qd.json")).map_err(|e| Error::invalid("qd", format!("{}: {e} (not a qd run?)", dir.display())))?;
    serde_json::from_slice(&b).map_err(|e| Error::parse("qd.json", e))
}

/// Automatic axis range from seed values: their span padded by half on each side (percent
/// descriptors clamped to 0..100).
fn auto_range(vals: &[f64], percent: bool) -> (f64, f64) {
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() {
        return (0.0, 100.0);
    }
    let pad = if hi - lo > 1e-9 { 0.5 * (hi - lo) } else { 0.5 * lo.abs().max(1.0) };
    let (mut lo, mut hi) = (lo - pad, hi + pad);
    if percent {
        (lo, hi) = (lo.max(0.0), hi.min(100.0));
    }
    (lo, hi)
}

/// Run MAP-Elites: seed descents (which also fix automatic axis ranges), then mutate random
/// elites on every thread for the rest of the budget. Writes `qd.json` and `elites.bin` every
/// `snapshot_ms` and at the end. `ctx.observer` is ignored (the grid is the observer).
pub fn run(
    ctx: &Ctx,
    params: &Params,
    interval: Duration,
    control: &Control,
    dir: &Path,
    id: &str,
    mut on_progress: impl FnMut(&Progress),
) -> Result<(Outcome, QdSummary)> {
    let sc = ctx.scorer;
    let qp = &params.qd;
    let specs: Vec<AxisSpec> = qp.axes.iter().map(|a| parse_axis(a)).collect::<Result<_>>()?;
    if specs.is_empty() || specs.len() > 3 {
        return Err(Error::invalid("qd", format!("need 1 to 3 axes, got {}", specs.len())));
    }
    let t0 = Instant::now();
    let start_chars: String = ctx.start.chars(sc).iter().collect();
    let phase = |phase: &str| Some(ExploreProgress { phase: phase.into(), ..Default::default() });
    on_progress(&Progress { elapsed_ms: 0, evals: 0, best: ctx.start.score, best_chars: start_chars, explore: phase("seed") });

    // Seeds: the start layout plus random layouts, each descended to a local optimum.
    let threads = params.threads.max(1);
    let n_seeds = qp.seeds.max(1);
    let parts: Vec<(Vec<Measured>, f64, u64)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|w| {
                s.spawn(move || {
                    let mut rng = Rng::for_worker(params.seed ^ 0x51ed_0e1e, w as u64);
                    let (mut out, mut random_sum, mut evals) = (Vec::new(), 0.0, 0u64);
                    for i in (w..n_seeds).step_by(threads) {
                        if control.cancelled() {
                            break;
                        }
                        let mut st = if i == 0 { ctx.start.clone() } else { ctx.random_state(&mut rng) };
                        random_sum += if i == 0 { 0.0 } else { st.score };
                        evals += 1 + steepest_descent_capped(ctx, &mut st, usize::MAX);
                        out.push(measure(sc, &st));
                    }
                    (out, random_sum, evals)
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()).collect()
    });
    let mut seeds = Vec::new();
    let (mut random_sum, mut seed_evals) = (0.0, 0u64);
    for (m, r, e) in parts {
        seeds.extend(m);
        random_sum += r;
        seed_evals += e;
    }
    let random_mean = random_sum / (n_seeds.saturating_sub(1)).max(1) as f64;
    let descs: Vec<Desc> = specs.iter().map(|a| Desc::parse(&a.name).unwrap()).collect();
    let ranges: Vec<(f64, f64)> = specs
        .iter()
        .zip(&descs)
        .map(|(a, d)| {
            a.range.unwrap_or_else(|| auto_range(&seeds.iter().map(|m| d.value(&m.raw, &m.load)).collect::<Vec<_>>(), d.percent()))
        })
        .collect();
    let grid = Grid::new(sc, &specs, &ranges, qp.bins);
    for m in seeds {
        grid.insert(m);
    }
    let seed_ms = t0.elapsed().as_millis() as u64;

    let mut qctx = Ctx::new(sc, ctx.movable.clone(), ctx.start.clone()).with_moves(ctx.moves.clone());
    qctx.observer = Some(&grid);
    let mut p = params.clone();
    p.algo = Algo::Qd;
    if let Some(t) = p.time_ms {
        p.time_ms = Some(t.saturating_sub(seed_ms).max(500));
    }
    if let Some(e) = p.evals {
        p.evals = Some(e.saturating_sub(seed_evals).max(1));
    }
    let w = Writer { dir, id, sc, random_mean };
    let mut version = 0u64;
    let mut last_write = Instant::now();
    let snapshot_every = Duration::from_millis(qp.snapshot_ms.max(250));
    let mut out = optimize::run(&qctx, &p, interval, control, |pg| {
        let (elapsed_ms, evals) = (pg.elapsed_ms + seed_ms, pg.evals + seed_evals);
        if last_write.elapsed() >= snapshot_every && w.write(&grid, false, elapsed_ms, evals).is_ok() {
            version += 1;
            last_write = Instant::now();
        }
        let (filled, offspring) = grid.counts();
        let explore = Some(ExploreProgress { phase: "search".into(), optima: filled as u64, hits: offspring, edges: 0, version });
        on_progress(&Progress { elapsed_ms, evals, best: pg.best, best_chars: pg.best_chars.clone(), explore });
    });
    out.elapsed_ms += seed_ms;
    out.evals += seed_evals;
    // The seeds may hold the overall best (e.g. a good start layout): keep the run's top-K honest.
    let mut top = TopK::new(params.top);
    for s in &out.top {
        top.offer(s.score, &s.sym);
    }
    for (_, e) in grid.elites().iter().take(params.top) {
        top.offer(e.score, &e.sym);
    }
    out.top = top.items;
    let summary = w.write(&grid, true, out.elapsed_ms, out.evals)?;
    Ok((out, summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_parse_and_bin() {
        assert_eq!(parse_axis("roll_ratio").unwrap(), AxisSpec { name: "roll_ratio".into(), range: None });
        assert_eq!(parse_axis("B01:0.5:3").unwrap().range, Some((0.5, 3.0)));
        assert!(parse_axis("nope").is_err());
        assert!(parse_axis("index_load:5:1").is_err());
        assert!(parse_axis("index_load:5").is_err());
        let a = Axis { name: "x".into(), lo: 0.0, hi: 10.0, bins: 5, auto: false };
        assert_eq!((a.bin(-3.0), a.bin(0.0), a.bin(3.9), a.bin(9.99), a.bin(50.0)), (0, 0, 1, 4, 4));
        assert_eq!(auto_range(&[40.0, 60.0], true), (30.0, 70.0));
        assert_eq!(auto_range(&[2.0, 90.0], true), (0.0, 100.0));
    }
}
