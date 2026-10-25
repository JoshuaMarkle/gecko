//! `gecko corpus build | mix | nav | edits | import | info | vim`: compile n-gram tables into data/corpora/.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gecko_core::corpus::{charset_named, fold_diacritics, strip_markup, WordList, WORDS_KEEP};
use gecko_core::recipe::Recipe;
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
        /// Hard-wrapped prose: a line break is a space, a blank line one Enter (`⏎`).
        #[arg(long)]
        paragraphs: bool,
        /// Remove Project Gutenberg typesetting: `_emphasis_`, `[Illustration]`, `*  *  *` lines.
        #[arg(long)]
        strip_markup: bool,
        /// Read more source paths from this file, one per line.
        #[arg(long)]
        list: Option<PathBuf>,
    },
    /// Weighted mix of corpora (each normalized first; the charset is their union), e.g.
    /// `gecko corpus mix code-rs:1 code-py:1 --out code`. Writes the recipe
    /// data/corpora/<out>.mix.toml beside the corpus; without parts, rebuilds <out> from its
    /// recipe; `--all` rebuilds every recipe.
    Mix {
        /// `<corpus>[:weight]` (weight 1 when omitted).
        parts: Vec<String>,
        #[arg(long, required_unless_present = "all")]
        out: Option<String>,
        /// Backspace share of all keystrokes to add after mixing (`corpus edits`).
        #[arg(long)]
        edits: Option<f64>,
        /// Arrow share of all keystrokes to add after mixing (`corpus nav`).
        #[arg(long)]
        nav: Option<f64>,
        #[arg(long)]
        description: Option<String>,
        /// Rebuild every recipe in data/corpora (parts before the languages that use them).
        #[arg(long, conflicts_with_all = ["parts", "out"])]
        all: bool,
    },
    /// Vim keystrokes: a synthetic editing session (normal-mode commands, short inserts typed
    /// from `--text`, `⎋`, `:` commands), or with `--log`, real keystroke logs recorded with
    /// `nvim -w <file>` (every mode, as typed).
    Vim {
        #[arg(long, default_value = "vim")]
        out: String,
        /// Text corpus the inserts and search patterns are typed from.
        #[arg(long, default_value = "prose-web")]
        text: String,
        #[arg(long, default_value_t = 2_000_000)]
        keystrokes: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// `-w` keystroke logs to count instead of the model.
        #[arg(long, num_args = 1..)]
        log: Vec<PathBuf>,
    },
    /// Symbol coverage of a corpus: shares by class (letters, space, punctuation, digits,
    /// special keys), Shift presses, printable ASCII symbols that are missing or rare, and the
    /// parts of a recipe.
    Info {
        name: String,
        /// Symbols under this share (per mille of keystrokes) are listed as rare.
        #[arg(long, default_value_t = 0.05)]
        rare: f64,
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
    /// Add Backspace `⌫` to a corpus from a typing-error model fitted on the Aalto 136M
    /// keystrokes study (text has none): `gecko corpus edits english-full --out english-full-bspc`.
    Edits {
        base: String,
        /// Backspace share of all keystrokes (Aalto typists: 0.056).
        #[arg(long, default_value_t = 0.056)]
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
            CorpusCmd::Build { mut sources, out, charset, ext, fold, paragraphs, strip_markup, list } => {
                if let Some(l) = list {
                    let text = std::fs::read_to_string(&l).with_context(|| format!("reading {}", l.display()))?;
                    sources.extend(text.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from));
                }
                build(ctx, &sources, &out, &charset, &ext, fold, paragraphs, strip_markup)
            }
            CorpusCmd::Mix { all: true, .. } => {
                for name in recipe_order(ctx)? {
                    mix(ctx, &name, &[], None, None, None)?;
                }
                Ok(())
            }
            CorpusCmd::Mix { parts, out, edits, nav, description, .. } => mix(ctx, &out.unwrap(), &parts, edits, nav, description),
            CorpusCmd::Info { name, rare } => info(ctx, &name, rare),
            CorpusCmd::Vim { out, log, .. } if !log.is_empty() => {
                let mut c = Corpus::new(&out, &gecko_core::vim::log_charset());
                for f in &log {
                    c.add_text(&gecko_core::vim::log_text(&std::fs::read(f).with_context(|| format!("reading {}", f.display()))?));
                }
                write_corpus(ctx, &c, log.len())
            }
            CorpusCmd::Vim { out, text, keystrokes, seed, .. } => {
                write_corpus(ctx, &gecko_core::vim::synthetic(&out, &ctx.store.corpus(&text)?, keystrokes, seed)?, 1)
            }
            CorpusCmd::Nav { base, share, seed, out } => write_corpus(ctx, &ctx.store.corpus(&base)?.with_nav(&out, share, seed)?, 1),
            CorpusCmd::Edits { base, share, seed, out } => write_corpus(ctx, &ctx.store.corpus(&base)?.with_edits(&out, share, seed)?, 1),
            CorpusCmd::Import { dir, out, charset, fold } => import(ctx, &dir, &out, &charset, fold),
        }
    }
}

/// Write the recipe (when parts or options are given) and build it.
fn mix(ctx: &Ctx, out: &str, parts: &[String], edits: Option<f64>, nav: Option<f64>, description: Option<String>) -> Result<()> {
    let mut r = ctx.store.recipe(out)?.unwrap_or_default();
    if !parts.is_empty() {
        r.parts = Recipe::parse_parts(parts)?;
    }
    r.edits = edits.unwrap_or(r.edits);
    r.nav = nav.unwrap_or(r.nav);
    r.description = description.unwrap_or(r.description);
    if r.parts.is_empty() {
        bail!("no recipe data/corpora/{out}.mix.toml; give the parts to mix");
    }
    r.check()?;
    let loaded = r.parts.iter().map(|p| ctx.store.corpus(&p.corpus)).collect::<gecko_core::Result<Vec<_>>>()?;
    let c = r.build(out, &loaded)?;
    let path = ctx.store.recipe_path(out);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, r.to_toml())?;
    write_corpus(ctx, &c, r.parts.len())
}

/// Every recipe, each after the recipes it mixes.
fn recipe_order(ctx: &Ctx) -> Result<Vec<String>> {
    let names: Vec<String> = ctx.store.list("corpora").into_iter().filter(|n| ctx.store.recipe_path(n).exists()).collect();
    let mut done: Vec<String> = Vec::new();
    while done.len() < names.len() {
        let before = done.len();
        for n in &names {
            let r = ctx.store.recipe(n)?.unwrap();
            if !done.contains(n) && r.parts.iter().all(|p| !names.contains(&p.corpus) || done.contains(&p.corpus)) {
                done.push(n.clone());
            }
        }
        if done.len() == before {
            bail!("recipes mix each other in a cycle");
        }
    }
    Ok(done)
}

/// Class of a symbol for coverage reports.
pub fn class_of(ch: char) -> &'static str {
    match ch {
        'a'..='z' => "letters",
        '0'..='9' => "digits",
        '\u{2423}' => "space",
        c if c.is_ascii_punctuation() => "punctuation",
        _ => "special",
    }
}

fn info(ctx: &Ctx, name: &str, rare: f64) -> Result<()> {
    let c = ctx.store.corpus(name)?;
    let tot = c.total_chars().max(1) as f64;
    let share = |i: usize| 1000.0 * c.mono[i] as f64 / tot;
    let mut classes: Vec<(&str, f64)> = Vec::new();
    for (i, &ch) in c.charset.iter().enumerate() {
        let k = class_of(ch);
        match classes.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1 += share(i),
            None => classes.push((k, share(i))),
        }
    }
    let printable: Vec<char> = ('!'..='~').filter(|c| !c.is_ascii_uppercase()).collect();
    // A shifted symbol off the charset is typed on its US key with Shift (tokenizer).
    let on_key = |ch: char| gecko_core::corpus::US_SHIFT.iter().find(|p| p.0 == ch).map(|p| p.1).filter(|b| c.index_of(*b).is_some());
    let missing: String = printable.iter().filter(|&&ch| c.index_of(ch).is_none() && on_key(ch).is_none()).collect();
    let folded: String = printable.iter().filter(|&&ch| c.index_of(ch).is_none() && on_key(ch).is_some()).collect();
    let rare_syms: Vec<(char, f64)> =
        c.charset.iter().enumerate().filter(|&(i, _)| share(i) < rare).map(|(i, &ch)| (ch, share(i))).collect();
    let shift = 100.0 * c.shift.iter().sum::<u64>() as f64 / tot;
    let recipe = ctx.store.recipe(name)?;
    let meta = ctx.store.corpus_index()?.remove(name).unwrap_or_default();
    let mut syms: Vec<(char, f64)> = c.charset.iter().enumerate().map(|(i, &ch)| (ch, share(i))).collect();
    syms.sort_by(|a, b| b.1.total_cmp(&a.1));
    if ctx.json {
        ctx.print(json!({
            "schema": SCHEMA, "corpus": name, "kind": meta.kind, "description": meta.description, "source": meta.source,
            "chars": c.total_chars(), "hash": gecko_core::hash::hex(c.hash()), "charset": c.charset.iter().collect::<String>(),
            "classes_per_mille": classes.iter().map(|(k, v)| json!({"class": k, "share": v})).collect::<Vec<_>>(),
            "shift_per_100": shift, "missing": missing, "shifted_on_key": folded,
            "rare": rare_syms.iter().map(|(ch, v)| json!({"symbol": ch.to_string(), "share": v})).collect::<Vec<_>>(),
            "symbols_per_mille": syms.iter().map(|(ch, v)| json!({"symbol": ch.to_string(), "share": v})).collect::<Vec<_>>(),
            "recipe": recipe,
        }));
        return Ok(());
    }
    println!("{name}: {} keystrokes, {} symbols, hash {}", c.total_chars(), c.size(), gecko_core::hash::hex(c.hash()));
    if !meta.description.is_empty() {
        println!("  {}", meta.description);
    }
    if let Some(r) = &recipe {
        let parts: Vec<String> = r.parts.iter().zip(r.shares()).map(|(p, s)| format!("{} {:.0}%", p.corpus, 100.0 * s)).collect();
        println!(
            "  recipe: {}{}{}",
            parts.join(" + "),
            if r.edits > 0.0 { format!(", ⌫ {:.1}%", 100.0 * r.edits) } else { String::new() },
            if r.nav > 0.0 { format!(", arrows {:.1}%", 100.0 * r.nav) } else { String::new() }
        );
    }
    let cls: Vec<String> = classes.iter().map(|(k, v)| format!("{k} {:.1}%", v / 10.0)).collect();
    println!("  classes: {}", cls.join(", "));
    println!("  Shift presses: {shift:.2} per 100 keystrokes{}", if c.has_shift() { "" } else { " (no case counts)" });
    println!("  missing ASCII: {}", if missing.is_empty() { "none".into() } else { missing });
    if !folded.is_empty() {
        println!("  typed as Shift + US key (not separate symbols): {folded}");
    }
    if !rare_syms.is_empty() {
        let r: Vec<String> = rare_syms.iter().map(|(ch, v)| format!("{ch} {v:.3}")).collect();
        println!("  rare (< {rare} ‰): {}", r.join("  "));
    }
    let top: Vec<String> = syms.iter().map(|(ch, v)| format!("{ch} {v:.1}")).collect();
    println!("  per mille: {}", top.join("  "));
    Ok(())
}

fn fold_if(fold: bool, s: String) -> String {
    if fold {
        fold_diacritics(&s)
    } else {
        s
    }
}

#[allow(clippy::too_many_arguments)]
fn build(ctx: &Ctx, sources: &[PathBuf], out: &str, charset: &str, ext: &str, fold: bool, paragraphs: bool, strip: bool) -> Result<()> {
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
    let mut seen = std::collections::HashSet::new();
    let mut dups = 0;
    for f in &files {
        let bytes = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
        // Vendored copies and repeated versions of one file count once.
        if !seen.insert(gecko_core::hash::fnv1a(&bytes)) {
            dups += 1;
            continue;
        }
        let mut text = fold_if(fold, String::from_utf8_lossy(&bytes).into_owned());
        if strip {
            text = strip_markup(&text);
        }
        if paragraphs {
            c.add_prose(&text);
        } else {
            c.add_text(&text);
        }
    }
    if dups > 0 {
        eprintln!("skipped {dups} duplicate files");
    }
    write_corpus(ctx, &c, files.len() - dups)
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
    std::fs::write(&path, c.to_file_bytes())?;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "corpus": c.name, "path": path, "files": files, "chars": c.total_chars(), "hash": gecko_core::hash::hex(c.hash())}));
    } else {
        println!("wrote {} ({} files, {} chars)", path.display(), files, c.total_chars());
    }
    Ok(())
}
