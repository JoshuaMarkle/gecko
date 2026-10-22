//! G6 bilevel geometry + layout search: CMA-ES over a parametric board's free parameters
//! (2D stagger/splay/thumbs, 3D curvature/lift/tent/tilt), each candidate scored by a short
//! warm-started layout search on it. See `context/algorithms/geometry-search.md`.
//!
//! - **Bilevel:** every candidate board gets a low-temperature SA + steepest descent starting
//!   from the incumbent (best so far) layout; all candidates of a generation share one inner
//!   seed (common random numbers), so their differences come from the board.
//! - **Alternate:** CMA-ES over the board with the layout fixed (a rebuild + one full score per
//!   candidate), then a full SA on the new board, for a few rounds.
//! - **Board:** the board for a fixed layout: CMA-ES with the start layout fixed for the whole
//!   budget, no layout search and no polish (the reported layout is the start layout).
//!
//! Collisions (2D on flat candidates, approximate 3D otherwise) add a penalty per unit of depth;
//! past `hard_fail` the layout search is skipped. A profile `[build]` section (G07,
//! `geometry/manufacture.rs`) adds its violations and their excess (u) to the collisions.
//! Both modes end with an equal SA polish of the best and the start board, so the reported
//! gain compares like with like.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::cmaes;
use crate::optimize::{self, steepest_descent, Algo, Control, Ctx};
use crate::report::breakdown;
use crate::rng::Rng;
use crate::run::{now_stamp, Input};
use crate::{hash, Corpus, Error, Geometry, Layout, ParametricGeometry, Profile, Result, Scorer, State, SCHEMA};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Bilevel,
    Alternate,
    Board,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GeomParams {
    pub mode: Mode,
    pub seed: u64,
    pub threads: usize,
    /// Wall-clock budget for the whole run (search + polish), ms.
    pub time_ms: Option<u64>,
    /// Board candidates to score (deterministic; overrides `time_ms`).
    pub evals: Option<u64>,
    /// Bilevel: SA swaps per candidate before its steepest descent.
    pub inner_evals: u64,
    /// Bilevel: acceptance probability of an average worsening swap at the start of the
    /// inner SA (low: it refines the warm start rather than leaving it).
    pub inner_accept0: f64,
    /// Share of a time budget for the final SA polish (half on the best board, half on the start
    /// board); the search phase also starts with a layout run of polish_frac / 2.
    pub polish_frac: f64,
    /// SA swaps per polish (and per alternate layout step) with an evaluation budget.
    pub polish_evals: u64,
    /// CMA-ES step size as a fraction of each parameter's range.
    pub sigma0: f64,
    pub popsize: Option<usize>,
    /// Keycap clearance per side, u.
    pub margin: f32,
    /// Objective added per u of total collision depth, in units of (|start score| + 1).
    pub penalty: f64,
    /// Total collision depth (u) past which a candidate is rejected without a layout search.
    pub hard_fail: f64,
    /// Alternate: rounds of (board step, layout step).
    pub rounds: usize,
    /// Keep the parameter file's hand-entered key efforts; by default they are dropped and
    /// derived from key positions on every candidate (`derive.rs`), so M01 follows moved keys.
    pub keep_effort: bool,
}

impl Default for GeomParams {
    fn default() -> Self {
        GeomParams {
            mode: Mode::Bilevel,
            seed: 1,
            threads: 1,
            time_ms: Some(60_000),
            evals: None,
            inner_evals: 20_000,
            inner_accept0: 0.05,
            polish_frac: 0.2,
            polish_evals: 1_000_000,
            sigma0: 0.1,
            popsize: None,
            margin: 0.0,
            penalty: 10.0,
            hard_fail: 2.0,
            rounds: 4,
            keep_effort: false,
        }
    }
}

/// One free parameter: `split.<field>`, `<zone>.<field>` or `<zone>.<column>.<field>`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Var {
    pub name: String,
    pub lo: f64,
    pub hi: f64,
    /// Value in the parameter file.
    pub start: f64,
}

const ZONE_FIELDS: [&str; 11] =
    ["anchor_x", "anchor_y", "rotate", "row_pitch", "arc_radius", "row_curve", "col_curve", "center_col", "tent", "tilt", "lift"];
const COLUMN_FIELDS: [&str; 4] = ["stagger", "splay", "spread", "lift"];

/// The parameter a [`Var`] name addresses.
fn field<'a>(p: &'a mut ParametricGeometry, name: &str) -> Result<&'a mut f64> {
    let bad = || Error::invalid("free parameter", format!("unknown parameter '{name}'"));
    let parts: Vec<&str> = name.split('.').collect();
    if parts[0] == "split" && parts.len() == 2 {
        let s = p.split.as_mut().ok_or_else(|| Error::invalid("free parameter", format!("'{name}': the board has no [split]")))?;
        return Ok(match parts[1] {
            "spread" => &mut s.spread,
            "angle" => &mut s.angle,
            "tent" => &mut s.tent,
            "tilt" => &mut s.tilt,
            "lift" => &mut s.lift,
            _ => return Err(bad()),
        });
    }
    let z = p.zone.iter_mut().find(|z| z.name == parts[0]).ok_or_else(bad)?;
    match parts[1..] {
        [f] => Ok(match f {
            "anchor_x" => &mut z.anchor[0],
            "anchor_y" => &mut z.anchor[1],
            "rotate" => &mut z.rotate,
            "row_pitch" => &mut z.row_pitch,
            "arc_radius" => {
                z.arc_radius.as_mut().ok_or_else(|| Error::invalid("free parameter", format!("'{name}': zone has no arc_radius")))?
            }
            "row_curve" => &mut z.row_curve,
            "col_curve" => &mut z.col_curve,
            "center_col" => &mut z.center_col,
            "tent" => &mut z.tent,
            "tilt" => &mut z.tilt,
            "lift" => &mut z.lift,
            _ => return Err(bad()),
        }),
        [c, f] => {
            let col = c.parse::<usize>().ok().and_then(|c| z.column.get_mut(c)).ok_or_else(bad)?;
            Ok(match f {
                "stagger" => &mut col.stagger,
                "splay" => &mut col.splay,
                "spread" => &mut col.spread,
                "lift" => &mut col.lift,
                _ => return Err(bad()),
            })
        }
        _ => Err(bad()),
    }
}

/// A finger zone contains its home row; other zones (thumb clusters) do not.
fn is_finger_zone(z: &crate::geometry::parametric::Zone) -> bool {
    z.rows.contains(&z.home_row)
}

/// Posture priors (H6, `research/sources/split-keyboard-ergonomics.md`): a gabled split types
/// with the least wrist extension and pronation at a slope of 0 to −4° and an opening of ≈ 15°.
/// Unlike the other defaults these bounds are absolute: a start value outside them is left to
/// the start board, and CMA-ES starts from it clamped into the prior.
pub const POSTURE_TILT: (f64, f64) = (-4.0, 0.0);
pub const POSTURE_ANGLE: (f64, f64) = (10.0, 20.0);

/// Default search range of a parameter around its start value `s`.
fn default_bounds(name: &str, thumb: bool, s: f64) -> (f64, f64) {
    match name {
        "split.tilt" => return POSTURE_TILT,
        "split.angle" => return POSTURE_ANGLE,
        _ => {}
    }
    let f = name.rsplit('.').next().unwrap_or("");
    let split = name.starts_with("split.");
    let column = name.split('.').count() == 3;
    let (lo, hi) = match f {
        "stagger" => (s - 0.75, s + 0.75),
        "splay" => (s - 15.0, s + 15.0),
        "spread" if split => (s - 2.0, s + 4.0),
        // Not below the 19.05 mm MX pitch: tighter boards shorten every distance metric, but
        // need narrow caps (Choc spacing); a file already below 1 u keeps its own pitch.
        "spread" => ((s - 0.2).max(1.0), s + 0.6),
        "row_pitch" => ((s - 0.15).max(1.0), s + 0.4),
        "anchor_x" | "anchor_y" => (s - 1.5, s + 1.5),
        "rotate" => (s - 30.0, s + 30.0),
        "arc_radius" => (s.min(s / 3.0), s.max(s * 3.0)),
        "row_curve" => (0.0, 30.0),
        "col_curve" if thumb => (-30.0, 30.0),
        "col_curve" => (0.0, 15.0),
        "center_col" => (0.0, 5.0),
        "lift" if column => (s - 0.5, s + 1.0),
        "lift" => (s - 1.0, s + 1.0),
        // Beyond ~45° a board stops being a desk keyboard; the wrist model is uncalibrated there.
        "tent" if split => (0.0, 45.0),
        "tent" | "tilt" => (s - 40.0, s + 40.0),
        _ => (s - 1.0, s + 1.0),
    };
    (lo.min(s), hi.max(s))
}

/// Expand a `--free` list into variables. Terms (comma separated, each optionally
/// `=lo:hi` to set absolute bounds on everything it expands to):
///
/// - presets `2d` (stagger, splay, spread, thumb) and `3d` (2d + row_curve, col_curve, lift, angle,
///   tent, tilt, thumb3d; split tilt and angle default to the posture priors);
/// - `stagger`, `splay`, `spread`, `lift`: every column of the finger zones;
/// - `row_pitch`, `row_curve`, `col_curve`: the finger zones;
/// - `angle`, `tent`, `tilt`: the split (both halves, mirrored);
/// - `thumb`: thumb zones' anchor, rotation and column staggers; `thumb3d`: their tent, tilt,
///   lift and column curve;
/// - explicit `split.<f>`, `<zone>.<f>` (a column field expands to every column of the zone)
///   or `<zone>.<col>.<f>`.
pub fn parse_free(p: &ParametricGeometry, spec: &str) -> Result<Vec<Var>> {
    let mut names: Vec<(String, Option<(f64, f64)>)> = Vec::new();
    let finger: Vec<&crate::geometry::parametric::Zone> = p.zone.iter().filter(|z| is_finger_zone(z)).collect();
    let thumbs: Vec<&crate::geometry::parametric::Zone> = p.zone.iter().filter(|z| !is_finger_zone(z)).collect();
    let columns = |zones: &[&crate::geometry::parametric::Zone], f: &str| -> Vec<String> {
        zones
            .iter()
            .flat_map(|z| (0..z.column.len()).filter(move |&c| f != "spread" || c > 0).map(move |c| format!("{}.{c}.{f}", z.name)))
            .collect()
    };
    let zones = |zones: &[&crate::geometry::parametric::Zone], f: &str| -> Vec<String> {
        zones.iter().filter(|z| f != "col_curve" || z.arc_radius.is_none()).map(|z| format!("{}.{f}", z.name)).collect()
    };
    let expand = |term: &str| -> Result<Vec<String>> {
        Ok(match term {
            "stagger" | "splay" | "spread" | "lift" => columns(&finger, term),
            "row_pitch" | "row_curve" | "col_curve" => zones(&finger, term),
            "angle" | "tent" | "tilt" => vec![format!("split.{term}")],
            "thumb" => {
                ["anchor_x", "anchor_y", "rotate"].iter().flat_map(|f| zones(&thumbs, f)).chain(columns(&thumbs, "stagger")).collect()
            }
            "thumb3d" => ["tent", "tilt", "lift", "col_curve"].iter().flat_map(|f| zones(&thumbs, f)).collect(),
            _ => {
                let parts: Vec<&str> = term.split('.').collect();
                match parts[..] {
                    [z, f] if z != "split" && COLUMN_FIELDS.contains(&f) && !ZONE_FIELDS.contains(&f) => {
                        let zone = p
                            .zone
                            .iter()
                            .find(|zone| zone.name == z)
                            .ok_or_else(|| Error::invalid("free parameter", format!("no zone '{z}'")))?;
                        columns(&[zone], f)
                    }
                    [z, f] if z != "split" && f == "lift" => vec![term.to_string()],
                    [_, _] | [_, _, _] => vec![term.to_string()],
                    _ => {
                        return Err(Error::invalid(
                            "free parameter",
                            format!("'{term}': use 2d, 3d, a group (stagger, splay, spread, lift, row_pitch, row_curve, col_curve, angle, tent, tilt, thumb, thumb3d) or split.<field> / <zone>.<field> / <zone>.<col>.<field>"),
                        ))
                    }
                }
            }
        })
    };
    for raw in spec.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        let (term, range) = match raw.split_once('=') {
            Some((t, r)) => {
                let (lo, hi) = r
                    .split_once(':')
                    .and_then(|(a, b)| Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?)))
                    .filter(|(lo, hi)| lo.is_finite() && hi.is_finite() && lo < hi)
                    .ok_or_else(|| Error::invalid("free parameter", format!("'{raw}': bounds must be lo:hi with lo < hi")))?;
                (t.trim(), Some((lo, hi)))
            }
            None => (raw, None),
        };
        let terms: Vec<&str> = match term {
            "2d" => vec!["stagger", "splay", "spread", "thumb"],
            "3d" => vec!["stagger", "splay", "spread", "thumb", "row_curve", "col_curve", "lift", "angle", "tent", "tilt", "thumb3d"],
            t => vec![t],
        };
        for t in terms {
            for name in expand(t)? {
                // Split fields need a split; a 3D preset on an unsplit board skips them.
                if name.starts_with("split.") && p.split.is_none() {
                    if term == "3d" {
                        continue;
                    }
                    return Err(Error::invalid("free parameter", format!("'{name}': the board has no [split]")));
                }
                match names.iter_mut().find(|(n, _)| *n == name) {
                    Some(e) => e.1 = range.or(e.1),
                    None => names.push((name, range)),
                }
            }
        }
    }
    if names.is_empty() {
        return Err(Error::invalid("free parameter", "nothing to optimize (e.g. --free 2d or --free 3d)"));
    }
    let mut probe = p.clone();
    names
        .into_iter()
        .map(|(name, range)| {
            let start = *field(&mut probe, &name)?;
            let zone = name.split('.').next().unwrap_or("");
            let thumb = p.zone.iter().any(|z| z.name == zone && !is_finger_zone(z));
            let (lo, hi) = range.unwrap_or_else(|| default_bounds(&name, thumb, start));
            Ok(Var { name, lo, hi, start })
        })
        .collect()
}

/// Everything a geometry search needs.
pub struct GeomSpec<'a> {
    pub params: &'a ParametricGeometry,
    /// Resolved board of `params` (`Store::geometry`): its comfort tables are reused.
    pub board: &'a Geometry,
    pub corpus: &'a Corpus,
    pub profile: &'a Profile,
    pub start: &'a Layout,
    pub pins: &'a str,
    pub vars: &'a [Var],
    pub search: GeomParams,
}

/// A scored board: parameter values, its layout and the collision penalty.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomPoint {
    /// Values of the free parameters, aligned with `vars`.
    pub values: Vec<f64>,
    /// Layout score on this board.
    pub score: f64,
    /// Search objective: score + collision penalty.
    pub f: f64,
    pub collisions: usize,
    /// Total collision depth, u.
    pub depth: f64,
    pub chars: String,
    pub rows: Vec<String>,
    /// Per-metric breakdown of the layout on this board.
    pub breakdown: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomProgress {
    pub elapsed_ms: u64,
    /// `search`, `layout` (alternate), `polish`.
    pub phase: String,
    pub generation: usize,
    pub candidates: u64,
    /// Best objective so far, and the best of collision-free boards.
    pub best: f64,
    pub best_feasible: Option<f64>,
    /// Best objective of this generation.
    pub generation_best: f64,
    /// The board reported so far (best collision-free, else best): free-parameter values aligned
    /// with `vars`, its layout (slot order) and keycap collisions.
    #[serde(default)]
    pub values: Vec<f64>,
    #[serde(default)]
    pub chars: String,
    #[serde(default)]
    pub collisions: usize,
}

/// One CMA-ES (re)start; restarts left without budget are not listed (their best would be
/// infinite, which JSON cannot hold).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CmaRun {
    pub popsize: usize,
    pub generations: usize,
    pub evals: usize,
    pub best: f64,
    pub stop: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomResult {
    pub schema: u32,
    pub id: String,
    pub mode: Mode,
    pub vars: Vec<Var>,
    /// The start board and the best board, each with an equal final SA polish.
    pub start: GeomPoint,
    pub best: GeomPoint,
    /// Objective of the start and best board when the search phase ended (before polish).
    pub search_start: f64,
    pub search_best: f64,
    /// Board candidates scored and layout swap evaluations spent on them.
    pub candidates: u64,
    pub generations: usize,
    pub layout_evals: u64,
    pub elapsed_ms: u64,
    pub cma_runs: Vec<CmaRun>,
    /// The best board has no keycap collisions.
    pub feasible: bool,
}

/// A candidate's evaluation.
#[derive(Clone, Debug)]
struct Cand {
    f: f64,
    collisions: usize,
    depth: f64,
    sym: Vec<u8>,
}

struct Problem<'a> {
    base: ParametricGeometry,
    vars: &'a [Var],
    comfort: BTreeMap<String, Vec<f32>>,
    scorer: Scorer,
    movable: Vec<usize>,
    p: &'a GeomParams,
    /// |start board score| + 1 (after the start board's layout search): the unit of collision penalties.
    unit: f64,
    start_score: f64,
    build: Option<crate::geometry::manufacture::BuildLimits>,
}

impl Problem<'_> {
    fn params_at(&self, x: &[f64]) -> Result<ParametricGeometry> {
        let mut p = self.base.clone();
        for (v, &value) in self.vars.iter().zip(x) {
            *field(&mut p, &v.name)? = value;
        }
        Ok(p)
    }

    fn geometry_at(&self, x: &[f64]) -> Result<Geometry> {
        let mut g = self.params_at(x)?.generate_unchecked()?;
        g.comfort_tables = self.comfort.clone();
        Ok(g)
    }

    fn collisions(&self, g: &Geometry) -> (usize, f64) {
        let c = if g.is_flat() { g.collisions(self.p.margin) } else { g.collisions_3d(self.p.margin) };
        let b = self.build.as_ref().map(|l| g.manufacture(l)).unwrap_or_default();
        (c.len() + b.violations.len(), c.iter().map(|c| c.depth as f64).sum::<f64>() + b.excess)
    }

    fn rejected(&self, collisions: usize, depth: f64, sym: &[u8]) -> Cand {
        let f = self.start_score + self.unit * (10.0 + self.p.penalty * depth);
        Cand { f, collisions, depth, sym: sym.to_vec() }
    }

    /// Score board `x`: rebuild `sc` for it, then (with `inner`) search layouts from `sym`.
    fn evaluate(&self, sc: &mut Scorer, x: &[f64], sym: &[u8], inner: bool, seed: u64) -> Cand {
        let Ok(g) = self.geometry_at(x) else { return self.rejected(0, self.p.hard_fail, sym) };
        let (collisions, depth) = self.collisions(&g);
        if depth > self.p.hard_fail || sc.set_geometry(&g).is_err() {
            return self.rejected(collisions, depth, sym);
        }
        let st = if inner {
            inner_search(sc, &self.movable, sym, self.p.inner_evals, self.p.inner_accept0, &mut Rng::new(seed))
        } else {
            sc.state_from_sym(sym.to_vec())
        };
        Cand { f: st.score + self.p.penalty * self.unit * depth, collisions, depth, sym: st.sym }
    }

    /// Score a generation in parallel (one scorer per thread); results do not depend on the
    /// thread count.
    fn evaluate_all(&self, scorers: &mut [Scorer], xs: &[Vec<f64>], sym: &[u8], inner: bool, seed: u64) -> Vec<Cand> {
        let k = scorers.len().min(xs.len()).max(1);
        let mut out: Vec<Option<Cand>> = vec![None; xs.len()];
        std::thread::scope(|scope| {
            let handles: Vec<_> = scorers
                .iter_mut()
                .take(k)
                .enumerate()
                .map(|(t, sc)| {
                    scope.spawn(move || {
                        (t..xs.len()).step_by(k).map(|i| (i, self.evaluate(sc, &xs[i], sym, inner, seed))).collect::<Vec<_>>()
                    })
                })
                .collect();
            for h in handles {
                for (i, c) in h.join().expect("candidate evaluation panicked") {
                    out[i] = Some(c);
                }
            }
        });
        out.into_iter().map(|c| c.expect("every candidate scored")).collect()
    }

    /// A full multi-threaded SA run on board `x` from layout `sym`.
    fn layout_search(&self, x: &[f64], sym: &[u8], budget: Budget, seed: u64, control: &Control) -> Result<(Cand, u64)> {
        let g = self.geometry_at(x)?;
        let mut sc = self.scorer.clone();
        sc.set_geometry(&g)?;
        let start = sc.state_from_sym(sym.to_vec());
        let ctx = Ctx::new(&sc, self.movable.clone(), start);
        let params = optimize::Params {
            algo: Algo::Sa,
            seed,
            threads: self.p.threads.max(1),
            time_ms: match budget {
                Budget::Time(ms) => Some(ms.max(1)),
                Budget::Evals(_) => None,
            },
            evals: match budget {
                Budget::Evals(e) => Some(e),
                Budget::Time(_) => None,
            },
            top: 1,
            ..Default::default()
        };
        let out = optimize::run(&ctx, &params, Duration::from_millis(500), control, |_| {});
        // SA restarts from random layouts; keep the warm start if it is still the best.
        let best = out.top.into_iter().next().map(|s| sc.state_from_sym(s.sym)).filter(|b| b.score < ctx.start.score).unwrap_or(ctx.start);
        let (collisions, depth) = self.collisions(&g);
        Ok((Cand { f: best.score + self.p.penalty * self.unit * depth, collisions, depth, sym: best.sym }, out.evals))
    }
}

#[derive(Clone, Copy)]
enum Budget {
    Time(u64),
    Evals(u64),
}

/// Warm-started layout search on one board: a short low-temperature SA from `sym`, then a
/// steepest descent from its best layout.
pub fn inner_search(sc: &Scorer, movable: &[usize], sym: &[u8], evals: u64, accept0: f64, rng: &mut Rng) -> State {
    let ctx = Ctx::new(sc, movable.to_vec(), sc.state_from_sym(sym.to_vec()));
    let mut best = ctx.start.clone();
    if evals > 0 && ctx.movable.len() >= 2 {
        let mut cur = ctx.start.clone();
        let (mut sum, mut cnt) = (0.0, 0u32);
        for _ in 0..64 {
            let (a, b) = ctx.random_pair(rng);
            let d = sc.delta(&cur, a, b);
            if d > 0.0 {
                sum += d;
                cnt += 1;
            }
        }
        let t0 = if cnt > 0 { sum / cnt as f64 } else { 1.0 } / (1.0 / accept0.clamp(1e-6, 0.999)).ln();
        let ln_ratio = (1e-3f64).ln();
        let mut t = t0;
        for i in 0..evals {
            if i % 256 == 0 {
                t = t0 * (ln_ratio * i as f64 / evals as f64).exp();
            }
            let (a, b) = ctx.random_pair(rng);
            let d = sc.delta(&cur, a, b);
            if d <= 0.0 || rng.f64() < (-d / t).exp() {
                sc.apply(&mut cur, a, b, d);
                if cur.score < best.score - 1e-12 {
                    best.clone_from(&cur);
                }
            }
        }
        // Re-score: the SA accumulated deltas.
        best = sc.state_from_sym(best.sym);
    }
    steepest_descent(&ctx, &mut best);
    sc.state_from_sym(best.sym)
}

/// Board candidates and layout evaluations so far, and the best boards found.
struct Track {
    best: (Vec<f64>, Cand),
    feasible: Option<(Vec<f64>, Cand)>,
    candidates: u64,
    generations: usize,
    layout_evals: u64,
}

impl Track {
    fn offer(&mut self, x: &[f64], c: &Cand) {
        if c.f < self.best.1.f {
            self.best = (x.to_vec(), c.clone());
        }
        if c.depth == 0.0 && self.feasible.as_ref().is_none_or(|(_, b)| c.f < b.f) {
            self.feasible = Some((x.to_vec(), c.clone()));
        }
    }
    /// The board to report: the best collision-free one, else the least bad.
    fn chosen(&self) -> &(Vec<f64>, Cand) {
        self.feasible.as_ref().unwrap_or(&self.best)
    }
}

/// Active time since a start: pauses after the start do not count.
struct Clock<'a> {
    t0: Instant,
    paused0: Duration,
    control: &'a Control,
}

impl<'a> Clock<'a> {
    fn new(control: &'a Control) -> Self {
        Clock { t0: Instant::now(), paused0: control.paused_for(), control }
    }
    fn ms(&self) -> u64 {
        self.t0.elapsed().saturating_sub(self.control.paused_for().saturating_sub(self.paused0)).as_millis() as u64
    }
}

/// Block while the run is paused (and not cancelled).
fn wait_while_paused(control: &Control) {
    while control.paused() && !control.cancelled() {
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Run the search (no files written).
pub fn search(spec: &GeomSpec, control: &Control, mut on_progress: impl FnMut(&GeomProgress)) -> Result<GeomResult> {
    let p = &spec.search;
    if spec.vars.is_empty() {
        return Err(Error::invalid("geometry search", "no free parameters"));
    }
    if !(0.0..1.0).contains(&p.polish_frac) || p.sigma0 <= 0.0 || p.hard_fail <= 0.0 || p.penalty < 0.0 {
        return Err(Error::invalid("geometry search", "need 0 ≤ polish_frac < 1, sigma0 > 0, hard_fail > 0, penalty ≥ 0"));
    }
    let clock = Clock::new(control);
    let mut base = spec.params.clone();
    if !p.keep_effort {
        base.zone.iter_mut().flat_map(|z| z.column.iter_mut()).for_each(|c| c.effort.clear());
    }
    let mut start_geometry = base.generate_unchecked()?;
    start_geometry.comfort_tables = spec.board.comfort_tables.clone();
    let scorer = Scorer::new(&start_geometry, spec.corpus, spec.profile, spec.start)?;
    let pinned = crate::layout::parse_symbols(spec.pins)?;
    if let Some(c) = pinned.iter().find(|c| !scorer.chars.contains(c)) {
        return Err(Error::invalid("pins", format!("'{c}' is not on the start layout")));
    }
    let movable: Vec<usize> = (0..scorer.n).filter(|&s| !pinned.contains(&scorer.chars[s])).collect();
    let start_score = scorer.identity().score;
    let mut pr = Problem {
        base,
        vars: spec.vars,
        comfort: spec.board.comfort_tables.clone(),
        scorer,
        movable,
        p,
        unit: start_score.abs() + 1.0,
        start_score,
        build: spec.profile.build.clone(),
    };
    pr.movable = Ctx::new(&pr.scorer, pr.movable.clone(), pr.scorer.identity()).movable;
    let threads = p.threads.max(1);
    let mut scorers: Vec<Scorer> = vec![pr.scorer.clone(); threads];
    let x_start: Vec<f64> = spec.vars.iter().map(|v| v.start).collect();
    let bounds = cmaes::Bounds { lo: spec.vars.iter().map(|v| v.lo).collect(), hi: spec.vars.iter().map(|v| v.hi).collect() };
    let x0: Vec<f64> = spec.vars.iter().map(|v| v.start.clamp(v.lo, v.hi)).collect();
    let identity: Vec<u8> = (0..pr.scorer.n as u8).collect();

    // Budgets: a time budget gives each of the three layout runs (start board before the
    // search, both boards after it) polish_frac / 2 and the board search the rest; an
    // evaluation budget, `evals` board candidates and `polish_evals` swaps per layout run.
    // Board mode searches the board only, so it has no layout runs and gets the whole budget.
    let board_only = p.mode == Mode::Board;
    let search_ms = p.time_ms.map(|ms| if board_only { ms } else { (ms as f64 * (1.0 - p.polish_frac)) as u64 });
    let polish = match (p.evals, p.time_ms) {
        (Some(_), _) | (None, None) => Budget::Evals(p.polish_evals),
        (None, Some(ms)) => Budget::Time((ms as f64 * p.polish_frac / 2.0) as u64),
    };
    let over = |clk: &Clock, limit: Option<u64>| {
        wait_while_paused(control);
        control.cancelled() || limit.is_some_and(|ms| clk.ms() >= ms)
    };

    // The start board with a full layout search, so candidates warm-start from a good layout
    // and the search measures boards rather than the layout catching up.
    // Board mode keeps the start layout: the start board is scored as it is.
    let (mut first, pre_evals) = if board_only {
        (pr.evaluate_all(&mut scorers[..1], std::slice::from_ref(&x_start), &identity, false, 0).remove(0), 0)
    } else {
        pr.layout_search(&x_start, &identity, polish, p.seed, control)?
    };
    // Penalties scale with the start board's score after its layout search (the start
    // layout's raw score can be several times larger, e.g. QWERTY), so one penalty setting
    // weighs overlap against score the same way whatever layout the run starts from.
    // A start board past hard_fail was rejected unscored and keeps the layout's score.
    if first.depth <= p.hard_fail {
        let raw = first.f - p.penalty * pr.unit * first.depth;
        pr.unit = raw.abs() + 1.0;
        pr.start_score = raw;
        first.f = raw + p.penalty * pr.unit * first.depth;
    }
    let mut tr = Track { best: (x_start.clone(), first.clone()), feasible: None, candidates: 1, generations: 0, layout_evals: pre_evals };
    tr.offer(&x_start, &first);
    let search_start = first.f;
    let mut incumbent = first.sym.clone();
    let mut cma_runs = Vec::new();
    let mut rng = Rng::new(p.seed);
    let cma = |max_evals: u64| cmaes::Params {
        sigma0: p.sigma0,
        popsize: p.popsize,
        max_evals: max_evals.min(usize::MAX as u64) as usize,
        restart_from_best: true,
        ..Default::default()
    };
    let chars = pr.scorer.chars.clone();
    let emit = |tr: &Track, phase: &str, gen_best: f64, on_progress: &mut dyn FnMut(&GeomProgress)| {
        let (x, c) = tr.chosen();
        on_progress(&GeomProgress {
            elapsed_ms: clock.ms(),
            phase: phase.into(),
            generation: tr.generations,
            candidates: tr.candidates,
            best: tr.best.1.f,
            best_feasible: tr.feasible.as_ref().map(|f| f.1.f),
            generation_best: gen_best,
            values: x.clone(),
            chars: c.sym.iter().map(|&s| chars[s as usize]).collect(),
            collisions: c.collisions,
        })
    };

    match p.mode {
        Mode::Bilevel | Mode::Board => {
            let limit = p.evals.unwrap_or(u64::MAX).saturating_sub(1);
            let out = cmaes::minimize_batch(
                |xs| {
                    if over(&clock, search_ms.filter(|_| p.evals.is_none())) {
                        return Vec::new();
                    }
                    tr.generations += 1;
                    let seed = Rng::for_worker(p.seed, tr.generations as u64).next_u64();
                    let cands = pr.evaluate_all(&mut scorers, xs, &incumbent, !board_only, seed);
                    tr.candidates += xs.len() as u64;
                    if !board_only {
                        tr.layout_evals += xs.len() as u64 * p.inner_evals;
                    }
                    for (x, c) in xs.iter().zip(&cands) {
                        tr.offer(x, c);
                    }
                    // Bilevel: warm-start the next generation from the best board's layout.
                    // Only an overlap-free board can supply it (the reported board is too).
                    if !board_only {
                        incumbent = tr.chosen().1.sym.clone();
                    }
                    let gen_best = cands.iter().map(|c| c.f).fold(f64::INFINITY, f64::min);
                    emit(&tr, "search", gen_best, &mut on_progress);
                    cands.into_iter().map(|c| c.f).collect()
                },
                &x0,
                Some(&bounds),
                &cma(limit),
                &mut rng,
            );
            cma_runs.extend(out.runs.iter().filter(|r| r.evals > 0).map(cma_run));
        }
        Mode::Alternate => {
            let rounds = p.rounds.max(1) as u64;
            let round_ms = search_ms.map(|ms| ms / rounds);
            let mut x = x0.clone();
            for round in 0..rounds {
                if over(&clock, search_ms.filter(|_| p.evals.is_none())) {
                    break;
                }
                let before = tr.best.1.f;
                // Layout step: board fixed.
                let t_round = Clock::new(control);
                let budget = match (p.evals, round_ms) {
                    (None, Some(ms)) => Budget::Time((ms / 2).max(1)),
                    _ => Budget::Evals(p.polish_evals),
                };
                let (c, evals) = pr.layout_search(&x, &incumbent, budget, p.seed.wrapping_add(round + 1), control)?;
                tr.layout_evals += evals;
                incumbent = c.sym.clone();
                tr.offer(&x, &c);
                emit(&tr, "layout", c.f, &mut on_progress);
                // Board step: layout fixed.
                let board_ms = round_ms.map(|ms| ms.saturating_sub(t_round.ms()).max(1));
                let limit = p.evals.map_or(u64::MAX, |e| (e / rounds).max(1));
                let t_board = Clock::new(control);
                let out = cmaes::minimize_batch(
                    |xs| {
                        if over(&t_board, board_ms.filter(|_| p.evals.is_none())) {
                            return Vec::new();
                        }
                        tr.generations += 1;
                        let cands = pr.evaluate_all(&mut scorers, xs, &incumbent, false, 0);
                        tr.candidates += xs.len() as u64;
                        for (x, c) in xs.iter().zip(&cands) {
                            tr.offer(x, c);
                        }
                        let gen_best = cands.iter().map(|c| c.f).fold(f64::INFINITY, f64::min);
                        emit(&tr, "search", gen_best, &mut on_progress);
                        cands.into_iter().map(|c| c.f).collect()
                    },
                    &x,
                    Some(&bounds),
                    &cma(limit),
                    &mut rng,
                );
                cma_runs.extend(out.runs.iter().filter(|r| r.evals > 0).map(cma_run));
                x = tr.chosen().0.clone();
                incumbent = tr.chosen().1.sym.clone();
                if round > 0 && before - tr.best.1.f < 1e-9 * before.abs().max(1.0) {
                    break;
                }
            }
        }
    }
    let search_best = tr.chosen().1.f;

    // Polish the chosen board and the start board equally (Board mode: the layout stays).
    let (best_x, best_c) = tr.chosen().clone();
    let ((best, e1), (start, e2)) = if board_only {
        ((best_c, 0), (first.clone(), 0))
    } else {
        (
            pr.layout_search(&best_x, &best_c.sym, polish, p.seed.wrapping_add(1000), control)?,
            pr.layout_search(&x_start, &first.sym, polish, p.seed.wrapping_add(1000), control)?,
        )
    };
    tr.layout_evals += e1 + e2;
    emit(&tr, "polish", best.f, &mut on_progress);

    let point = |x: &[f64], c: &Cand| -> Result<GeomPoint> {
        let g = pr.geometry_at(x)?;
        let mut sc = pr.scorer.clone();
        sc.set_geometry(&g)?;
        let st = sc.state_from_sym(c.sym.clone());
        let layout = sc.layout_of(&st, "")?;
        Ok(GeomPoint {
            values: x.to_vec(),
            score: st.score,
            f: c.f,
            collisions: c.collisions,
            depth: c.depth,
            chars: st.chars(&sc).iter().collect(),
            rows: layout.all_rows(&sc.base_geometry()),
            breakdown: serde_json::to_value(breakdown(&sc, &st)).unwrap(),
        })
    };
    Ok(GeomResult {
        schema: SCHEMA,
        id: String::new(),
        mode: p.mode,
        vars: spec.vars.to_vec(),
        start: point(&x_start, &start)?,
        best: point(&best_x, &best)?,
        search_start,
        search_best,
        candidates: tr.candidates,
        generations: tr.generations,
        layout_evals: tr.layout_evals,
        elapsed_ms: clock.ms(),
        cma_runs,
        feasible: best.depth == 0.0,
    })
}

fn cma_run(r: &cmaes::RunInfo) -> CmaRun {
    CmaRun { popsize: r.popsize, generations: r.generations, evals: r.evals, best: r.best, stop: format!("{:?}", r.stop).to_lowercase() }
}

/// The parameter file for a result's best board (`values` applied to `params`; efforts dropped
/// unless kept, so they re-derive).
pub fn best_params(params: &ParametricGeometry, vars: &[Var], values: &[f64], keep_effort: bool) -> Result<ParametricGeometry> {
    let mut p = params.clone();
    if !keep_effort {
        p.zone.iter_mut().flat_map(|z| z.column.iter_mut()).for_each(|c| c.effort.clear());
    }
    for (v, &value) in vars.iter().zip(values) {
        *field(&mut p, &v.name)? = value;
    }
    Ok(p)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeomRunConfig {
    pub schema: u32,
    /// `"geometry"`: tells geometry runs apart from layout runs.
    pub kind: String,
    pub id: String,
    pub created: String,
    pub gecko_version: String,
    pub params: GeomParams,
    pub free: String,
    pub vars: Vec<Var>,
    /// The start parameter file (also saved as `start.param.toml`).
    pub geometry: Input,
    pub corpus: Input,
    pub profile: Profile,
    pub profile_hash: String,
    pub start: String,
    pub pins: String,
}

/// A geometry run's directory, created with its config before the search starts.
pub struct GeomPrep {
    pub id: String,
    pub dir: PathBuf,
    pub config: GeomRunConfig,
}

/// Create `runs/<stamp>-geom-<hash>/` with `config.json` and `start.param.toml`.
pub fn prepare(runs: &Path, spec: &GeomSpec, free: &str) -> Result<GeomPrep> {
    let start_toml = toml::to_string(spec.params).map_err(|e| Error::invalid("parametric geometry", e.to_string()))?;
    let (stamp, iso) = now_stamp();
    let key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        start_toml,
        spec.corpus.hash(),
        spec.profile.hash(),
        spec.start.chars.iter().collect::<String>(),
        spec.pins,
        free,
        serde_json::to_string(&spec.search).unwrap_or_default()
    );
    let short = hash::short(hash::fnv1a(key.as_bytes()));
    let (mut id, mut k) = (format!("{stamp}-geom-{short}"), 2);
    while runs.join(&id).exists() {
        id = format!("{stamp}-geom-{short}-{k}");
        k += 1;
    }
    let dir = runs.join(&id);
    std::fs::create_dir_all(&dir)?;
    let config = GeomRunConfig {
        schema: SCHEMA,
        kind: "geometry".into(),
        id: id.clone(),
        created: iso,
        gecko_version: env!("CARGO_PKG_VERSION").to_string(),
        params: spec.search.clone(),
        free: free.to_string(),
        vars: spec.vars.to_vec(),
        geometry: Input { name: spec.params.name.clone(), hash: hash::hex(hash::fnv1a(start_toml.as_bytes())) },
        corpus: Input { name: spec.corpus.name.clone(), hash: hash::hex(spec.corpus.hash()) },
        profile: spec.profile.clone(),
        profile_hash: hash::hex(spec.profile.hash()),
        start: spec.start.chars.iter().collect(),
        pins: spec.pins.to_string(),
    };
    std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(&config).unwrap())?;
    std::fs::write(dir.join("start.param.toml"), &start_toml)?;
    Ok(GeomPrep { id, dir, config })
}

/// Name of a run's best board: `<start board>-opt-<run hash>`, so boards of different runs
/// can be told apart (and found by name) when loaded as `run:<id>`.
pub fn best_name(params_name: &str, id: &str) -> String {
    let short = id.split_once("-geom-").map_or(id, |(_, s)| s);
    format!("{params_name}-opt-{short}")
}

/// Search into a prepared directory: `progress.jsonl`, `best.json`, and the best board as
/// `geometry.param.toml` (parameters) and `geometry.toml` (keys).
pub fn execute(prep: &GeomPrep, spec: &GeomSpec, control: &Control, mut on_progress: impl FnMut(&GeomProgress)) -> Result<GeomResult> {
    let dir = &prep.dir;
    let mut log = std::io::BufWriter::new(std::fs::File::create(dir.join("progress.jsonl"))?);
    let mut res = search(spec, control, |pg| {
        let _ = writeln!(log, "{}", serde_json::to_string(pg).unwrap());
        on_progress(pg);
    })?;
    log.flush()?;
    res.id = prep.id.clone();
    let mut best = best_params(spec.params, spec.vars, &res.best.values, spec.search.keep_effort)?;
    best.name = best_name(&spec.params.name, &res.id);
    best.description = format!("{} (geometry run {})", spec.params.name, res.id);
    std::fs::write(
        dir.join("geometry.param.toml"),
        toml::to_string(&best).map_err(|e| Error::invalid("parametric geometry", e.to_string()))?,
    )?;
    let mut keys = best.generate_unchecked()?;
    keys.comfort_tables = spec.board.comfort_tables.clone();
    std::fs::write(dir.join("geometry.toml"), keys.to_toml())?;
    std::fs::write(dir.join("best.json"), serde_json::to_string_pretty(&res).unwrap())?;
    Ok(res)
}

/// Run a geometry search into `runs/<stamp>-geom-<hash>/` ([`prepare`] then [`execute`]).
pub fn run(
    runs: &Path,
    spec: &GeomSpec,
    free: &str,
    control: &Control,
    on_progress: impl FnMut(&GeomProgress),
) -> Result<(GeomResult, PathBuf)> {
    let prep = prepare(runs, spec, free)?;
    let res = execute(&prep, spec, control, on_progress)?;
    Ok((res, prep.dir))
}

/// The directory holds a geometry run (`config.json` with `kind = "geometry"`).
pub fn is_run(dir: &Path) -> bool {
    #[derive(Deserialize)]
    struct Kind {
        #[serde(default)]
        kind: String,
    }
    std::fs::read_to_string(dir.join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Kind>(&s).ok())
        .is_some_and(|k| k.kind == "geometry")
}

pub fn load_config(dir: &Path) -> Result<GeomRunConfig> {
    let s = std::fs::read_to_string(dir.join("config.json")).map_err(|e| Error::invalid("run", format!("{}: {e}", dir.display())))?;
    serde_json::from_str(&s).map_err(|e| Error::parse("config.json", e))
}

pub fn load_result(dir: &Path) -> Result<GeomResult> {
    let s = std::fs::read_to_string(dir.join("best.json"))
        .map_err(|e| Error::invalid("run", format!("{}: {e} (run not finished?)", dir.display())))?;
    serde_json::from_str(&s).map_err(|e| Error::parse("best.json", e))
}

pub fn load_progress(dir: &Path) -> Vec<GeomProgress> {
    std::fs::read_to_string(dir.join("progress.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

/// Re-run a saved `--evals` geometry run from its directory and compare with `best.json`
/// (`gecko verify`). Inputs must hash as recorded: the start parameter file
/// (`start.param.toml`), the corpus by name, the recorded profile. Nothing is written.
pub fn verify(store: &crate::store::Store, dir: &Path) -> Result<crate::run::Verification> {
    use crate::run::{Check, Verification};
    let config = load_config(dir)?;
    let recorded = load_result(dir)?;
    if config.params.evals.is_none() {
        return Err(Error::invalid("verify", format!("run {} has a time budget; only --evals runs are reproducible", config.id)));
    }
    let check = |name: &str, expected: String, actual: String| Check { name: name.into(), ok: expected == actual, expected, actual };
    let start_path = dir.join("start.param.toml");
    let start_text = std::fs::read_to_string(&start_path).map_err(|e| Error::invalid("run", format!("{}: {e}", start_path.display())))?;
    let params = ParametricGeometry::from_toml(&start_text)?;
    let board = store.geometry(&start_path.to_string_lossy())?;
    let corpus = store.corpus(&config.corpus.name)?;
    let mut checks = vec![
        check("geometry", config.geometry.hash.clone(), hash::hex(hash::fnv1a(start_text.as_bytes()))),
        check("corpus", config.corpus.hash.clone(), hash::hex(corpus.hash())),
        check("profile", config.profile_hash.clone(), hash::hex(config.profile.hash())),
    ];
    if checks.iter().any(|c| !c.ok) {
        return Ok(Verification { schema: SCHEMA, id: config.id, ok: false, checks, rerun: None });
    }
    let start = Layout::from_flat("start", &board, config.start.chars().collect())?;
    let spec = GeomSpec {
        params: &params,
        board: &board,
        corpus: &corpus,
        profile: &config.profile,
        start: &start,
        pins: &config.pins,
        vars: &config.vars,
        search: config.params.clone(),
    };
    let res = search(&spec, &Control::default(), |_| {})?;
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1.0);
    let point = |name: &str, a: &GeomPoint, b: &GeomPoint| Check {
        name: name.into(),
        expected: format!("{:.6} {} {:?}", a.score, a.chars, a.values),
        actual: format!("{:.6} {} {:?}", b.score, b.chars, b.values),
        ok: a.chars == b.chars && close(a.score, b.score) && a.values.len() == b.values.len() && a.values.iter().zip(&b.values).all(|(x, y)| close(*x, *y)),
    };
    checks.push(check("candidates", recorded.candidates.to_string(), res.candidates.to_string()));
    checks.push(point("start", &recorded.start, &res.start));
    checks.push(point("best", &recorded.best, &res.best));
    let ok = checks.iter().all(|c| c.ok);
    Ok(Verification { schema: SCHEMA, id: config.id, ok, checks, rerun: None })
}

/// The start parameter file of a geometry run (`start.param.toml`).
pub fn load_start(dir: &Path) -> Result<ParametricGeometry> {
    let s = std::fs::read_to_string(dir.join("start.param.toml")).map_err(|e| Error::invalid("run", format!("{}: {e}", dir.display())))?;
    ParametricGeometry::from_toml(&s)
}
