//! The on-disk data directory (`data/`, `runs/`, `gecko.toml`) shared by the CLI and server.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::corpus::WordList;
use crate::{Corpus, Error, Geometry, Layout, Profile, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "d_data")]
    pub data: String,
    #[serde(default = "d_runs")]
    pub runs: String,
    #[serde(default = "d_geometry")]
    pub geometry: String,
    #[serde(default = "d_corpus")]
    pub corpus: String,
    #[serde(default = "d_profile")]
    pub profile: String,
    #[serde(default = "d_reference")]
    pub reference: String,
}
fn d_data() -> String {
    "data".into()
}
fn d_runs() -> String {
    "runs".into()
}
fn d_geometry() -> String {
    "ansi-30".into()
}
fn d_corpus() -> String {
    "english".into()
}
fn d_profile() -> String {
    "default".into()
}
fn d_reference() -> String {
    "qwerty".into()
}

#[derive(Clone, Debug)]
pub struct Store {
    pub root: PathBuf,
    pub data: PathBuf,
    pub runs: PathBuf,
    pub defaults: Defaults,
}

fn read(p: &Path) -> Result<String> {
    std::fs::read_to_string(p).map_err(|e| Error::invalid("file", format!("{}: {e}", p.display())))
}

impl Store {
    /// Find `gecko.toml` in `start` or any parent; fall back to `fallback` (the repo root at build time).
    pub fn discover(start: &Path, fallback: Option<&Path>) -> Result<Store> {
        let mut dir = Some(start);
        while let Some(d) = dir {
            if d.join("gecko.toml").is_file() {
                return Store::open(d);
            }
            dir = d.parent();
        }
        if let Some(f) = fallback {
            if f.join("gecko.toml").is_file() {
                return Store::open(f);
            }
        }
        Err(Error::invalid("store", "no gecko.toml found in this directory or its parents (use --root)"))
    }

    pub fn open(root: &Path) -> Result<Store> {
        let defaults: Defaults = toml::from_str(&read(&root.join("gecko.toml"))?).map_err(|e| Error::parse("gecko.toml", e))?;
        Ok(Store { root: root.to_path_buf(), data: root.join(&defaults.data), runs: root.join(&defaults.runs), defaults })
    }

    fn dir(&self, kind: &str) -> PathBuf {
        self.data.join(kind)
    }

    /// Names (file stems) of `layouts`, `geometries`, `profiles` or `corpora`.
    pub fn list(&self, kind: &str) -> Vec<String> {
        let ext = if kind == "corpora" { "gkc" } else { "toml" };
        let mut v: Vec<String> = std::fs::read_dir(self.dir(kind))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension()? != ext {
                    return None;
                }
                Some(p.file_stem()?.to_string_lossy().into_owned())
            })
            .collect();
        v.sort();
        v
    }

    pub fn geometry(&self, name: &str) -> Result<Geometry> {
        let p = self.dir("geometries").join(format!("{name}.toml"));
        let p = if p.is_file() { p } else { PathBuf::from(name) };
        let mut g = Geometry::from_toml(&read(&p)?)?;
        for (metric, table) in g.comfort.clone() {
            g.load_comfort(&metric, &read(&self.dir("comfort").join(format!("{table}.csv")))?)?;
        }
        Ok(g)
    }

    pub fn profile(&self, name: &str) -> Result<Profile> {
        let p = self.dir("profiles").join(format!("{name}.toml"));
        let p = if p.is_file() { p } else { PathBuf::from(name) };
        Profile::from_toml(&read(&p)?)
    }

    pub fn corpus_path(&self, name: &str) -> PathBuf {
        self.dir("corpora").join(format!("{name}.gkc"))
    }

    pub fn corpus(&self, name: &str) -> Result<Corpus> {
        let p = self.corpus_path(name);
        let bytes = std::fs::read(&p)
            .map_err(|e| Error::invalid("corpus", format!("{}: {e} (build it with `gecko corpus build`)", p.display())))?;
        Corpus::from_bytes(&bytes)
    }

<<<<<<< HEAD
    /// Word-count side file of a corpus (written by `gecko corpus import` from `words.json`).
    pub fn words_path(&self, name: &str) -> PathBuf {
        self.dir("corpora").join(format!("{name}.words.tsv"))
    }

    pub fn words(&self, name: &str) -> Result<WordList> {
        let p = self.words_path(name);
        let text = std::fs::read_to_string(&p).map_err(|e| {
            Error::invalid(
                "words",
                format!("{}: {e} (corpus '{name}' has no word counts; use a word-import corpus such as shai)", p.display()),
            )
        })?;
        WordList::from_tsv(&text)
    }

    /// Resolve a layout spec: a reference name, a `.toml` path, `run:<id>[#rank]` or `raw:<chars>`.
=======
    /// The imported cmini bundle (`gecko layouts import-cmini`).
    pub fn community_path(&self) -> PathBuf {
        self.dir("community").join("cmini.json")
    }

    pub fn community(&self) -> Result<crate::community::Bundle> {
        let p = self.community_path();
        if !p.is_file() {
            return Err(Error::invalid("cmini", format!("{} not found (run `gecko layouts import-cmini <cmini checkout>`)", p.display())));
        }
        crate::community::Bundle::from_json(&read(&p)?)
    }

    /// Resolve a layout spec: a reference name, a `.toml` path, `run:<id>[#rank]`, `cmini:<name>`
    /// or `raw:<chars>`.
>>>>>>> origin/fix/issue-26
    pub fn layout(&self, spec: &str, geometry: &str) -> Result<Layout> {
        if let Some(name) = spec.strip_prefix("cmini:") {
            let b = self.community()?;
            return b.get(name).ok_or_else(|| Error::invalid("layout", format!("no cmini layout '{name}'")))?.layout(geometry);
        }
        if let Some(chars) = spec.strip_prefix("raw:") {
            return Layout::from_str_flat("raw", geometry, chars);
        }
        if let Some(r) = spec.strip_prefix("run:") {
            let (id, rank) = r.split_once('#').map(|(a, b)| (a, b.parse::<usize>().unwrap_or(1))).unwrap_or((r, 1));
            let res = crate::run::load_result(&self.runs.join(id))?;
            let l = res.best.get(rank.saturating_sub(1)).ok_or_else(|| Error::invalid("layout", format!("run {id} has no rank {rank}")))?;
            return Layout::from_chars(&format!("{id}#{rank}"), &res.geometry, &format!("run:{id}"), l.chars.chars().collect());
        }
        let p = self.dir("layouts").join(format!("{spec}.toml"));
        let p = if p.is_file() { p } else { PathBuf::from(spec) };
        if !p.is_file() {
            return Err(Error::invalid("layout", format!("'{spec}' is not a known layout, file, run:<id> or raw:<chars>")));
        }
        Layout::from_toml(&read(&p)?)
    }
}
