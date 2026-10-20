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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub id: String,
    pub created: String,
    pub algo: String,
    pub profile: String,
    pub corpus: String,
    pub geometry: String,
    pub best_score: Option<f64>,
    pub best: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub status: String,
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
}

pub struct Prepared {
    pub config: RunConfig,
    pub dir: PathBuf,
}

/// Create the run directory and write `config.json`.
pub fn prepare(runs: &Path, spec: &RunSpec) -> Result<Prepared> {
    let (stamp, iso) = now_stamp();
    let key = format!(
        "{}|{}|{}|{}|{}|{}",
        spec.geometry.hash(),
        spec.corpus.hash(),
        spec.profile.hash(),
        spec.start.chars.iter().collect::<String>(),
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
    };
    std::fs::write(dir.join("config.json"), serde_json::to_string_pretty(&config).unwrap())?;
    Ok(Prepared { config, dir })
}

/// Build the scorer/context, run the optimizer, and write `progress.jsonl` + `best.json`.
pub fn execute(prep: &Prepared, spec: &RunSpec, control: &Control, mut on_progress: impl FnMut(&Progress)) -> Result<RunResult> {
    let scorer = Scorer::new(spec.geometry, spec.corpus, spec.profile, spec.start)?;
    let start = scorer.identity();
    let pinned: Vec<char> = spec.pins.chars().filter(|c| !c.is_whitespace()).collect();
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
            let l = Layout { name: String::new(), geometry: spec.geometry.name.clone(), source: String::new(), chars: chars.clone() };
            Ranked {
                rank: i + 1,
                score: st.score,
                chars: chars.iter().collect(),
                rows: l.rows(spec.geometry),
                hash: hash::short(l.hash()),
                breakdown: serde_json::to_value(breakdown(&scorer, &st)).unwrap(),
            }
        })
        .collect();
    let res = RunResult {
        schema: SCHEMA,
        id: prep.config.id.clone(),
        geometry: spec.geometry.name.clone(),
        evals: out.evals,
        elapsed_ms: out.elapsed_ms,
        evals_per_sec: out.evals_per_sec,
        params: out.params.clone(),
        best,
    };
    std::fs::write(prep.dir.join("best.json"), serde_json::to_string_pretty(&res).unwrap())?;
    Ok(res)
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

/// All runs, newest first.
pub fn list(runs: &Path) -> Vec<RunSummary> {
    let mut v: Vec<RunSummary> = std::fs::read_dir(runs)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let dir = e.path();
            let c = load_config(&dir).ok()?;
            let r = load_result(&dir).ok();
            Some(RunSummary {
                id: c.id,
                created: c.created,
                algo: c.params.algo.name().to_string(),
                profile: c.profile.name,
                corpus: c.corpus.name,
                geometry: c.geometry.name,
                best_score: r.as_ref().and_then(|r| r.best.first().map(|b| b.score)),
                best: r.as_ref().and_then(|r| r.best.first().map(|b| b.rows.join(" "))),
                elapsed_ms: r.as_ref().map(|r| r.elapsed_ms),
                status: if r.is_some() { "done".into() } else { "incomplete".into() },
            })
        })
        .collect();
    v.sort_by(|a, b| b.id.cmp(&a.id));
    v
}
