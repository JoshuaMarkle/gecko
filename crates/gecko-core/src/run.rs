//! Run directories: `runs/<timestamp>-<algo>-<hash>/` with `config.json`, `best.json`,
//! `progress.jsonl`. Everything needed to reproduce a run is in `config.json`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::optimize::{self, Control, Ctx, Params, Progress};
use crate::report::breakdown;
use crate::{hash, Corpus, Error, Geometry, Layout, Profile, Result, Scorer, SCHEMA};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    pub hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunConfig {
    pub schema: u32,
    pub id: String,
    pub created: String,
    pub gecko_version: String,
    pub params: Params,
    pub geometry: Input,
    pub corpus: Input,
    pub profile: Profile,
    pub profile_hash: String,
    pub start: String,
    /// Pinned symbols (they keep their start slot).
    pub pins: String,
    /// Adaptive keys of the start layout (they hold for every result).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub magic: Vec<crate::magic::Magic>,
    /// Corpus the results are also scored on but the search never sees (L06).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout: Option<Input>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ranked {
    pub rank: usize,
    pub score: f64,
    /// Slot-ordered symbols.
    pub chars: String,
    pub rows: Vec<String>,
    pub hash: String,
    pub breakdown: serde_json::Value,
    /// Score on the run's holdout corpus (same profile and board).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParetoRanked {
    #[serde(flatten)]
    pub layout: Ranked,
    pub raw: Vec<f64>,
    pub costs: Vec<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunResult {
    pub schema: u32,
    pub id: String,
    pub geometry: String,
    pub evals: u64,
    pub elapsed_ms: u64,
    pub evals_per_sec: f64,
    /// Parameters actually used (auto values such as SA steps resolved).
    #[serde(default)]
    pub params: Params,
    pub best: Vec<Ranked>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<optimize::nsga2::Objective>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub front: Vec<ParetoRanked>,
    /// Gilmore–Lawler lower bound and optimality gap of `best[0]` (`crate::bound`); absent
    /// when skipped (too many symbols, negative load weights).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound: Option<crate::bound::BoundReport>,
    /// How the results do on the holdout corpus (L06); absent without `--holdout`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holdout: Option<HoldoutReport>,
}

/// L06 held-out corpus generalization: the start layout and the best result scored on the
/// search corpus and on a corpus the search never saw.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoldoutReport {
    pub corpus: String,
    pub start: f64,
    pub best: f64,
    pub start_search: f64,
    pub best_search: f64,
    /// Share of the search corpus's gain over the start that holds on the holdout corpus:
    /// (start − best) / (start_search − best_search). 1 = fully generalizes; below 1 the
    /// search fitted the search corpus; 0 when the search gained nothing.
    pub retained: f64,
    /// Spearman rank correlation of the ranked results' scores on the two corpora (≥ 3 results).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank_corr: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub created: String,
    pub algo: String,
    pub profile: String,
    pub corpus: String,
    pub geometry: String,
    /// Input hashes: runs with equal hashes searched the same problem (names alone do not say so).
    pub geometry_hash: String,
    pub corpus_hash: String,
    pub profile_hash: String,
    pub best_score: Option<f64>,
    pub best: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub status: String,
    /// `layout`, or `geometry` for a board + layout search (`geomsearch`; `algo` is then
    /// `geometry` and `geometry` names the best board, loadable as `run:<id>`).
    #[serde(default)]
    pub kind: String,
}

/// UTC timestamp `YYYYMMDD-HHMMSS` and ISO form, without a date crate.
pub fn now_stamp() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem / 60) % 60, rem % 60);
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    (format!("{y:04}{mo:02}{d:02}-{h:02}{m:02}{s:02}"), format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z"))
}

/// Everything needed to start a run.
pub struct RunSpec<'a> {
    pub geometry: &'a Geometry,
    pub corpus: &'a Corpus,
    pub profile: &'a Profile,
    pub start: &'a Layout,
    pub pins: &'a str,
    pub params: Params,
    /// Corpus to score the results on without searching on it (L06).
    pub holdout: Option<&'a Corpus>,
}

pub struct Prepared {
    pub config: RunConfig,
    pub dir: PathBuf,
}

/// Create the run directory and write `config.json`.
pub fn prepare(runs: &Path, spec: &RunSpec) -> Result<Prepared> {
    if matches!(spec.params.algo, optimize::Algo::Nsga2 | optimize::Algo::Rwsa) {
        optimize::nsga2::validate(&spec.params.nsga2)?;
    }
    if spec.holdout.is_some_and(|h| h.hash() == spec.corpus.hash()) {
        return Err(Error::invalid("holdout", "the holdout corpus must differ from the search corpus"));
    }
    let (stamp, iso) = now_stamp();
    let key = format!(
        "{}|{}|{}|{}|{}|{}",
        spec.geometry.hash(),
        spec.corpus.hash(),
        spec.profile.hash(),
        spec.start.chars.iter().collect::<String>()
            + &crate::magic::format_rules(&spec.start.magic.iter().flat_map(|m| m.rules.clone()).collect::<Vec<_>>()),
        spec.pins,
        serde_json::to_string(&spec.params).unwrap_or_default()
    );
    let mut id = format!("{stamp}-{}-{}", spec.params.algo.name(), hash::short(hash::fnv1a(key.as_bytes())));
    let mut dir = runs.join(&id);
    let mut k = 2;
    while dir.exists() {
        id = format!("{stamp}-{}-{}-{k}", spec.params.algo.name(), hash::short(hash::fnv1a(key.as_bytes())));
        dir = runs.join(&id);
        k += 1;
    }
    std::fs::create_dir_all(&dir)?;
    let config = RunConfig {
        schema: SCHEMA,
        id: id.clone(),
        created: iso,
        gecko_version: env!("CARGO_PKG_VERSION").to_string(),
        params: spec.params.clone(),
        geometry: Input { name: spec.geometry.name.clone(), hash: hash::hex(spec.geometry.hash()) },
        corpus: Input { name: spec.corpus.name.clone(), hash: hash::hex(spec.corpus.hash()) },
        profile: spec.profile.clone(),
        profile_hash: hash::hex(spec.profile.hash()),
        start: spec.start.chars.iter().collect(),
        pins: spec.pins.to_string(),
        magic: spec.start.magic.clone(),
        holdout: spec.holdout.map(|h| Input { name: h.name.clone(), hash: hash::hex(h.hash()) }),
    };
    std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(&config).unwrap())?;
    Ok(Prepared { config, dir })
}

/// Build the scorer/context, run the optimizer, and write `progress.jsonl` + `best.json`.
pub fn execute(prep: &Prepared, spec: &RunSpec, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Result<RunResult> {
    let scorer = Scorer::new(spec.geometry, spec.corpus, spec.profile, spec.start)?;
    // Same start layout, so the same symbol ids: states carry over between the two scorers.
    let held = spec.holdout.map(|h| Scorer::new(spec.geometry, h, spec.profile, spec.start)).transpose()?;
    let start = scorer.identity();
    let pinned: Vec<char> = crate::layout::parse_symbols(spec.pins)?;
    for c in &pinned {
        if !scorer.chars.contains(c) {
            return Err(Error::invalid("pins", format!("'{c}' is not on the start layout")));
        }
    }
    let movable: Vec<usize> = (0..scorer.n).filter(|&s| !pinned.contains(&scorer.chars[s])).collect();
    let ctx = Ctx::new(&scorer, movable, start);
    let mut log = std::io::BufWriter::new(std::fs::File::create(prep.dir.join("progress.jsonl"))?);
    let mut emit = |p: &Progress| {
        let _ = writeln!(log, "{}", serde_json::to_string(p).unwrap());
        on_progress(p);
    };
    let interval = Duration::from_millis(250);
    let out = if spec.params.algo == optimize::Algo::Explore {
        crate::explore::run(&ctx, &spec.params, interval, control, &prep.dir, &prep.config.id, &mut emit)?.0
    } else if spec.params.algo == optimize::Algo::Qd {
        crate::explore::qd::run(&ctx, &spec.params, interval, control, &prep.dir, &prep.config.id, &mut emit)?.0
    } else {
        optimize::run(&ctx, &spec.params, interval, control, &mut emit)
    };
    log.flush()?;
    drop(log);
    let best = out
        .top
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let st = scorer.state_from_sym(s.sym.clone());
            let chars: Vec<char> = st.chars(&scorer);
            let l = Layout::from_flat("", spec.geometry, chars.clone())?.with_magic(spec.start.magic.clone())?;
            Ok(Ranked {
                rank: i + 1,
                score: st.score,
                chars: chars.iter().collect(),
                rows: l.all_rows(spec.geometry),
                hash: hash::short(l.hash()),
                breakdown: serde_json::to_value(breakdown(&scorer, &st)).unwrap(),
                holdout: held.as_ref().map(|h| h.state_from_sym(s.sym.clone()).score),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let holdout = held.as_ref().zip(spec.holdout).zip(best.first()).map(|((h, corpus), first)| {
        let (start_search, start) = (scorer.identity().score, h.identity().score);
        let (best_search, best_held) = (first.score, first.holdout.unwrap_or(f64::NAN));
        let gain = start_search - best_search;
        let scores = |f: &dyn Fn(&Ranked) -> f64| best.iter().map(f).collect::<Vec<f64>>();
        HoldoutReport {
            corpus: corpus.name.clone(),
            start,
            best: best_held,
            start_search,
            best_search,
            retained: if gain > 0.0 { (start - best_held) / gain } else { 0.0 },
            rank_corr: (best.len() >= 3).then(|| spearman(&scores(&|r| r.score), &scores(&|r| r.holdout.unwrap_or(f64::NAN)))),
        }
    });
    let front = out
        .front
        .iter()
        .enumerate()
        .map(|(i, point)| {
            let st = scorer.state_from_sym(point.sym.clone());
            let chars = st.chars(&scorer);
            let layout = Layout::from_flat("", spec.geometry, chars.clone())?.with_magic(spec.start.magic.clone())?;
            Ok(ParetoRanked {
                layout: Ranked {
                    rank: i + 1,
                    score: st.score,
                    chars: chars.iter().collect(),
                    rows: layout.all_rows(spec.geometry),
                    hash: hash::short(layout.hash()),
                    breakdown: serde_json::to_value(breakdown(&scorer, &st)).unwrap(),
                    holdout: None,
                },
                raw: point.raw.clone(),
                costs: point.costs.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let bound = out
        .top
        .first()
        .and_then(|s| crate::bound::report(&scorer, &ctx.movable, &scorer.state_from_sym(s.sym.clone()), crate::bound::ASCENT_ITERS));
    let res = RunResult {
        schema: SCHEMA,
        id: prep.config.id.clone(),
        geometry: spec.geometry.name.clone(),
        evals: out.evals,
        elapsed_ms: out.elapsed_ms,
        evals_per_sec: out.evals_per_sec,
        params: out.params.clone(),
        best,
        objectives: out.objectives.clone(),
        front,
        bound,
        holdout,
    };
    std::fs::write(prep.dir.join("best.json"), serde_json::to_string_pretty(&res).unwrap())?;
    Ok(res)
}

/// Spearman rank correlation (ties get their average rank).
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let ranks = |v: &[f64]| {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&i, &j| v[i].total_cmp(&v[j]));
        let mut r = vec![0.0; v.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
                j += 1;
            }
            for &k in &idx[i..=j] {
                r[k] = (i + j) as f64 / 2.0;
            }
            i = j + 1;
        }
        r
    };
    let (ra, rb) = (ranks(a), ranks(b));
    let n = a.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let cov: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum();
    let vb: f64 = rb.iter().map(|y| (y - mb).powi(2)).sum();
    if va == 0.0 || vb == 0.0 {
        0.0
    } else {
        cov / (va * vb).sqrt()
    }
}

pub fn load_config(dir: &Path) -> Result<RunConfig> {
    let s = std::fs::read_to_string(dir.join("config.json")).map_err(|e| Error::invalid("run", format!("{}: {e}", dir.display())))?;
    serde_json::from_str(&s).map_err(|e| Error::parse("config.json", e))
}

pub fn load_result(dir: &Path) -> Result<RunResult> {
    let s = std::fs::read_to_string(dir.join("best.json"))
        .map_err(|e| Error::invalid("run", format!("{}: {e} (run not finished?)", dir.display())))?;
    serde_json::from_str(&s).map_err(|e| Error::parse("best.json", e))
}

pub fn load_progress(dir: &Path) -> Vec<Progress> {
    std::fs::read_to_string(dir.join("progress.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

fn geometry_summary(dir: &Path) -> Option<RunSummary> {
    let c = crate::geomsearch::load_config(dir).ok()?;
    let r = crate::geomsearch::load_result(dir).ok();
    Some(RunSummary {
        geometry: if r.is_some() { crate::geomsearch::best_name(&c.geometry.name, &c.id) } else { c.geometry.name },
        // The start board's parameter file.
        geometry_hash: c.geometry.hash,
        corpus_hash: c.corpus.hash,
        profile_hash: c.profile_hash,
        id: c.id,
        created: c.created,
        algo: "geometry".into(),
        profile: c.profile.name,
        corpus: c.corpus.name,
        best_score: r.as_ref().map(|r| r.best.score),
        best: r.as_ref().map(|r| r.best.rows.join(" ")),
        elapsed_ms: r.as_ref().map(|r| r.elapsed_ms),
        status: if r.is_some() { "done".into() } else { "incomplete".into() },
        kind: "geometry".into(),
    })
}

/// All runs (layout and geometry), newest first.
pub fn list(runs: &Path) -> Vec<RunSummary> {
    let mut v: Vec<RunSummary> = std::fs::read_dir(runs)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let dir = e.path();
            if crate::geomsearch::is_run(&dir) {
                return geometry_summary(&dir);
            }
            let c = load_config(&dir).ok()?;
            let r = load_result(&dir).ok();
            Some(RunSummary {
                id: c.id,
                created: c.created,
                algo: c.params.algo.name().to_string(),
                profile: c.profile.name,
                corpus: c.corpus.name,
                geometry: c.geometry.name,
                geometry_hash: c.geometry.hash,
                corpus_hash: c.corpus.hash,
                profile_hash: c.profile_hash,
                best_score: r.as_ref().and_then(|r| r.best.first().map(|b| b.score)),
                best: r.as_ref().and_then(|r| r.best.first().map(|b| b.rows.join(" "))),
                elapsed_ms: r.as_ref().map(|r| r.elapsed_ms),
                status: if r.is_some() { "done".into() } else { "incomplete".into() },
                kind: "layout".into(),
            })
        })
        .collect();
    v.sort_by(|a, b| b.id.cmp(&a.id));
    v
}

/// One recorded-vs-recomputed comparison in a [`Verification`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub expected: String,
    pub actual: String,
    pub ok: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verification {
    pub schema: u32,
    pub id: String,
    pub ok: bool,
    /// Input hashes, eval count, then each ranked layout (`best#<rank>`).
    pub checks: Vec<Check>,
    /// The re-run's result (not written to the run directory).
    pub rerun: Option<RunResult>,
}

fn check(name: &str, expected: impl ToString, actual: impl ToString) -> Check {
    let (expected, actual) = (expected.to_string(), actual.to_string());
    Check { name: name.to_string(), ok: expected == actual, expected, actual }
}

/// Re-run a saved `--evals` run from its `config.json` and compare against its `best.json`.
/// Inputs are resolved by name from `store` and must hash as recorded; if any differs the
/// optimizer is not re-run. The re-run writes to a temporary directory, never to the run.
pub fn verify(store: &crate::store::Store, dir: &Path) -> Result<Verification> {
    if crate::geomsearch::is_run(dir) {
        return crate::geomsearch::verify(store, dir);
    }
    let config = load_config(dir)?;
    let recorded = load_result(dir)?;
    if config.params.evals.is_none() {
        return Err(Error::invalid("verify", format!("run {} has a time budget; only --evals runs are reproducible", config.id)));
    }
    let geometry = store.geometry(&config.geometry.name)?;
    let corpus = store.corpus(&config.corpus.name)?;
    let holdout = config.holdout.as_ref().map(|h| store.corpus(&h.name)).transpose()?;
    let mut checks = vec![
        check("geometry", &config.geometry.hash, hash::hex(geometry.hash())),
        check("corpus", &config.corpus.hash, hash::hex(corpus.hash())),
        check("profile", &config.profile_hash, hash::hex(config.profile.hash())),
    ];
    if let (Some(h), Some(c)) = (&config.holdout, &holdout) {
        checks.push(check("holdout", &h.hash, hash::hex(c.hash())));
    }
    if checks.iter().any(|c| !c.ok) {
        return Ok(Verification { schema: SCHEMA, id: config.id, ok: false, checks, rerun: None });
    }
    let start = Layout::from_flat("start", &geometry, config.start.chars().collect())?;
    let spec = RunSpec {
        geometry: &geometry,
        corpus: &corpus,
        profile: &config.profile,
        start: &start,
        pins: &config.pins,
        params: config.params.clone(),
        holdout: holdout.as_ref(),
    };
    let tmp = std::env::temp_dir().join(format!("gecko-verify-{}-{}", std::process::id(), config.id));
    std::fs::create_dir_all(&tmp)?;
    let prep = Prepared { config: config.clone(), dir: tmp.clone() };
    let res = execute(&prep, &spec, &Control::default(), |_| {});
    let _ = std::fs::remove_dir_all(&tmp);
    let res = res?;
    checks.push(check("evals", recorded.evals, res.evals));
    checks.push(check("ranked", recorded.best.len(), res.best.len()));
    for (a, b) in recorded.best.iter().zip(&res.best) {
        // Same chars and score to 1e-9 relative (the score is recomputed from the layout).
        let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1.0);
        let same = a.chars == b.chars && close(a.score, b.score) && a.holdout.zip(b.holdout).is_none_or(|(x, y)| close(x, y));
        checks.push(Check {
            name: format!("best#{}", a.rank),
            expected: format!("{:.6} {}", a.score, a.chars),
            actual: format!("{:.6} {}", b.score, b.chars),
            ok: same,
        });
    }
    checks.push(check("objectives", serde_json::to_string(&recorded.objectives).unwrap(), serde_json::to_string(&res.objectives).unwrap()));
    checks.push(check("front", recorded.front.len(), res.front.len()));
    for (a, b) in recorded.front.iter().zip(&res.front) {
        let close =
            |xs: &[f64], ys: &[f64]| xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| (x - y).abs() <= 1e-9 * x.abs().max(1.0));
        checks.push(Check {
            name: format!("front#{}", a.layout.rank),
            expected: serde_json::to_string(a).unwrap(),
            actual: serde_json::to_string(b).unwrap(),
            ok: a.layout.chars == b.layout.chars
                && close(&a.raw, &b.raw)
                && close(&a.costs, &b.costs)
                && close(&[a.layout.score], &[b.layout.score]),
        });
    }
    let ok = checks.iter().all(|c| c.ok);
    Ok(Verification { schema: SCHEMA, id: config.id, ok, checks, rerun: Some(res) })
}
