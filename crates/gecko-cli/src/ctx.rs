//! Shared command context: the data store, the `--json` switch, the common `-g/-p/-c` inputs, and
//! small parsing helpers used by several commands.

use anyhow::{bail, Context, Result};
use gecko_core::optimize::moves::MoveParams;
use gecko_core::store::Store;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer, State};

#[derive(clap::Args, Clone)]
pub struct Inputs {
    /// Geometry name or path (default from gecko.toml).
    #[arg(long, short)]
    pub geometry: Option<String>,
    /// Profile name or path.
    #[arg(long, short)]
    pub profile: Option<String>,
    /// Compiled corpus name.
    #[arg(long, short)]
    pub corpus: Option<String>,
}

pub struct Ctx {
    pub store: Store,
    pub json: bool,
}

/// The geometry, profile and corpus named by an [`Inputs`] (or the gecko.toml defaults).
pub struct Loaded {
    pub geometry: Geometry,
    pub profile: Profile,
    pub corpus: Corpus,
}

impl Ctx {
    pub fn load(&self, i: &Inputs) -> Result<Loaded> {
        let d = &self.store.defaults;
        Ok(Loaded {
            geometry: self.store.geometry(i.geometry.as_deref().unwrap_or(&d.geometry))?,
            profile: self.store.profile(i.profile.as_deref().unwrap_or(&d.profile))?,
            corpus: self.store.corpus(i.corpus.as_deref().unwrap_or(&d.corpus))?,
        })
    }

    pub fn layout(&self, spec: &str, l: &Loaded) -> Result<Layout> {
        Ok(self.store.layout(spec, &l.geometry.name)?.fit(&l.geometry)?)
    }

    /// The start layout for a search: `--from`, else the gecko.toml reference layout.
    pub fn start(&self, from: Option<&str>, l: &Loaded) -> Result<Layout> {
        self.layout(from.unwrap_or(&self.store.defaults.reference), l)
    }

    pub fn print(&self, v: serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
    }

    /// Write `text` to `path`, creating its directory, and say so unless `--json`.
    pub fn write_file(&self, path: &std::path::Path, text: &str) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        if !self.json {
            println!("wrote {}", path.display());
        }
        Ok(())
    }
}

pub fn score_layout(l: &Loaded, layout: &Layout) -> Result<(Scorer, State)> {
    let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, layout)?;
    let st = sc.identity();
    Ok((sc, st))
}

/// Worker threads: `--threads`, else all cores.
/// Worker threads: `n`, or all cores when unset or 0.
pub fn threads(n: Option<usize>) -> usize {
    match n {
        Some(n) if n > 0 => n,
        _ => std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
    }
}

/// Milliseconds from `30s`, `2m`, `500ms`, `1h` (a bare number is seconds).
pub fn parse_duration(s: &str) -> Result<u64> {
    let s = s.trim();
    let (num, mul) = if let Some(v) = s.strip_suffix("ms") {
        (v, 1.0)
    } else if let Some(v) = s.strip_suffix('s') {
        (v, 1000.0)
    } else if let Some(v) = s.strip_suffix('m') {
        (v, 60_000.0)
    } else if let Some(v) = s.strip_suffix('h') {
        (v, 3_600_000.0)
    } else {
        (s, 1000.0)
    };
    let v: f64 = num.parse().with_context(|| format!("bad duration '{s}' (use e.g. 30s, 2m, 500ms)"))?;
    if !v.is_finite() || v < 0.0 {
        bail!("bad duration '{s}' (must be a finite, non-negative time)");
    }
    Ok((v * mul) as u64)
}

/// The time budget of a search: none when an evaluation budget is given, else `time` or `default`.
pub fn time_budget(time: Option<&str>, evals: Option<u64>, default: &str) -> Result<Option<u64>> {
    match evals {
        Some(0) => bail!("evaluation budget must be positive"),
        Some(_) => return Ok(None),
        None => {}
    }
    let ms = parse_duration(time.unwrap_or(default))?;
    if ms == 0 {
        bail!("time budget must be positive (got '{}')", time.unwrap_or(default));
    }
    Ok(Some(ms))
}

pub fn parse_moves(s: &str) -> Result<MoveParams> {
    MoveParams::parse(s).with_context(|| format!("unknown moves '{s}' (comma list of swap, columns, cycle3, depth2, all)"))
}

/// Short commit hash of `root`, with `-dirty` if the work tree has changes; "unknown" without git.
pub fn git_commit(root: &std::path::Path) -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    match git(&["rev-parse", "--short", "HEAD"]) {
        Some(c) if git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty()) => format!("{c}-dirty"),
        Some(c) => c,
        None => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("500ms").unwrap(), 500);
        assert_eq!(parse_duration("30s").unwrap(), 30_000);
        assert_eq!(parse_duration("2m").unwrap(), 120_000);
        assert_eq!(parse_duration("1h").unwrap(), 3_600_000);
        assert_eq!(parse_duration("1.5").unwrap(), 1500);
        assert!(parse_duration("fast").is_err());
        assert!(parse_duration("-5s").is_err());
        assert!(parse_duration("nan").is_err());
        assert!(parse_duration("inf").is_err());
        assert!(time_budget(Some("0s"), None, "60s").is_err());
        assert!(time_budget(None, Some(0), "60s").is_err());
        assert_eq!(threads(Some(3)), 3);
        assert!(threads(Some(0)) >= 1);
        assert_eq!(time_budget(Some("1s"), Some(10), "60s").unwrap(), None);
        assert_eq!(time_budget(None, None, "60s").unwrap(), Some(60_000));
    }
}
