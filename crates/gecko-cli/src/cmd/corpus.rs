//! `gecko corpus build | mix | nav | import`: compile n-gram tables into data/corpora/.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gecko_core::corpus::{charset_named, fold_diacritics, WordList, WORDS_KEEP};
use gecko_core::{Corpus, SCHEMA};
use serde_json::json;

use crate::ctx::Ctx;

#[derive(Subcommand)]
pub enum CorpusCmd {
    /// Count n-grams of text files (or directories, searched recursively for files with the
    /// `--ext` extensions) into data/corpora/<out>.gkc.
    Build {
        sources: Vec<PathBuf>,
        #[arg(long, default_value = "english")]
        out: String,
        /// Symbols to count, or `default` / `code` (adds digits, all ASCII punctuation, Enter) /
        /// `nav` (code plus arrows `←↓↑→`).
        #[arg(long, default_value = "default")]
        charset: String,
        /// File extensions to read from directories, comma-separated (e.g. `rs,py,ts`).
        #[arg(long, default_value = "txt")]
        ext: String,
        /// Fold diacritics to ASCII letters first (`é` → `e`, `œ` → `oe`), for non-English text.
        #[arg(long)]
        fold: bool,
    },
    /// Weighted mix of corpora with the same charset (each normalized first), e.g.
    /// `gecko corpus mix code-rs:1 code-py:1 --out code`.
    Mix {
        /// `<corpus>[:weight]` (weight 1 when omitted).
        parts: Vec<String>,
        #[arg(long)]
        out: String,
    },
    /// Add arrow keys `←↓↑→` to a corpus from a cursor-movement model (text has none):
    /// `gecko corpus nav code --share 0.03 --out code-nav`.
    Nav {
        base: String,
        /// Arrow share of all keystrokes.
        #[arg(long, default_value_t = 0.03)]
        share: f64,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        out: String,
    },
    /// Import precomputed counts from a cmini-style directory: words.json (preferred), else
    /// monograms/bigrams/trigrams.json. Writes data/corpora/<out>.gkc.
    Import {
        dir: PathBuf,
        #[arg(long)]
        out: String,
        #[arg(long, default_value = "default")]
        charset: String,
        /// Fold diacritics to ASCII letters first (`é` → `e`, `œ` → `oe`), for non-English text.
        #[arg(long)]
        fold: bool,
    },
}

impl CorpusCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            CorpusCmd::Build { sources, out, charset, ext, fold } => build(ctx, &sources, &out, &charset, &ext, fold),
            CorpusCmd::Mix { parts, out } => {
                let loaded = parts
                    .iter()
                    .map(|p| {
                        let (name, w) = p.split_once(':').unwrap_or((p, "1"));
                        Ok((ctx.store.corpus(name)?, w.parse::<f64>().with_context(|| format!("bad weight in '{p}'"))?))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let refs: Vec<(&Corpus, f64)> = loaded.iter().map(|(c, w)| (c, *w)).collect();
                write_corpus(ctx, &Corpus::mix(&out, &refs)?, parts.len())
            }
            CorpusCmd::Nav { base, share, seed, out } => write_corpus(ctx, &ctx.store.corpus(&base)?.with_nav(&out, share, seed)?, 1),
            CorpusCmd::Import { dir, out, charset, fold } => import(ctx, &dir, &out, &charset, fold),
        }
    }
}

fn fold_if(fold: bool, s: String) -> String {
    if fold {
        fold_diacritics(&s)
    } else {
        s
    }
}

fn build(ctx: &Ctx, sources: &[PathBuf], out: &str, charset: &str, ext: &str, fold: bool) -> Result<()> {
    if sources.is_empty() {
        bail!("give at least one text file or directory");
    }
    let mut c = Corpus::new(out, charset_named(charset));
    let exts: Vec<&str> = ext.split(',').map(str::trim).collect();
    let mut files = Vec::new();
    for s in sources {
        if s.is_dir() {
            let mut v = Vec::new();
            walk(s, &exts, &mut v)?;
            v.sort();
            files.extend(v);
        } else {
            files.push(s.clone());
        }
    }
    for f in &files {
        let bytes = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
        c.add_text(&fold_if(fold, String::from_utf8_lossy(&bytes).into_owned()));
    }
    write_corpus(ctx, &c, files.len())
}

fn import(ctx: &Ctx, dir: &Path, out: &str, charset: &str, fold: bool) -> Result<()> {
    let mut c = Corpus::new(out, charset_named(charset));
    let words = dir.join("words.json");
    let files = if words.is_file() {
        let mut e = c.word_edges();
        let counts: Vec<_> = read_counts(&words)?.into_iter().map(|(w, n)| (fold_if(fold, w), n)).collect();
        for (w, n) in &counts {
            c.add_word(w, *n, &mut e);
        }
        c.add_word_boundaries(&e);
        let list = WordList::from_counts(counts, WORDS_KEEP);
        let path = ctx.store.words_path(out);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, list.to_tsv())?;
        eprintln!("wrote {} ({} words)", path.display(), list.words.len());
        1
    } else {
        let names = ["monograms.json", "bigrams.json", "trigrams.json"];
        for f in names {
            for (g, n) in read_counts(&dir.join(f))? {
                c.add_ngram(&fold_if(fold, g), n);
            }
        }
        names.len()
    };
    write_corpus(ctx, &c, files)
}

/// Files under `dir` (recursively, skipping hidden directories) with one of `exts`.
fn walk(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) -> Result<()> {
    for e in std::fs::read_dir(dir)?.flatten() {
        let p = e.path();
        if p.is_dir() {
            if !e.file_name().to_string_lossy().starts_with('.') {
                walk(&p, exts, out)?;
            }
        } else if p.extension().is_some_and(|x| exts.iter().any(|e| x == *e)) {
            out.push(p);
        }
    }
    Ok(())
}

/// A JSON object of `"gram": count` pairs.
fn read_counts(path: &Path) -> Result<Vec<(String, u64)>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&text).with_context(|| format!("{}: expected an object of counts", path.display()))?;
    map.into_iter()
        .map(|(k, v)| {
            let n = v.as_u64().or_else(|| v.as_f64().map(|f| f.round() as u64));
            n.map(|n| (k.clone(), n)).with_context(|| format!("{}: count for '{k}' is not a number", path.display()))
        })
        .collect()
}

fn write_corpus(ctx: &Ctx, c: &Corpus, files: usize) -> Result<()> {
    let path = ctx.store.corpus_path(&c.name);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, c.to_bytes())?;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "corpus": c.name, "path": path, "files": files, "chars": c.total_chars(), "hash": gecko_core::hash::hex(c.hash())}));
    } else {
        println!("wrote {} ({} files, {} chars)", path.display(), files, c.total_chars());
    }
    Ok(())
}
