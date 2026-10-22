//! `gecko magic`: search the rules of an adaptive (magic) key on a fixed layout.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use gecko_core::magic::{self, Magic, SearchOpts, MAGIC};
use gecko_core::report::breakdown;
use gecko_core::SCHEMA;
use serde_json::json;

use crate::ctx::{score_layout, Ctx, Inputs};
use crate::fmt::{layout_rows, print_breakdown};

#[derive(clap::Args)]
pub struct MagicArgs {
    /// Layout with a `[[magic]]` key (or use --on).
    layout: String,
    /// Which adaptive key (1 = the first `[[magic]]`).
    #[arg(long, default_value_t = 1)]
    key: usize,
    #[arg(long, default_value_t = 32)]
    max_rules: usize,
    /// Accept a rule only when it lowers the score by more than this.
    #[arg(long, default_value_t = 0.01)]
    min_gain: f64,
    /// Extend the key's existing rules instead of starting from none.
    #[arg(long)]
    keep: bool,
    /// Put a new magic key (`★`) on this symbol's key; the symbol is dropped.
    #[arg(long)]
    on: Option<String>,
    /// Write the result to data/layouts/<out>.toml (or to a path ending in .toml).
    #[arg(long)]
    out: Option<String>,
    #[command(flatten)]
    inputs: Inputs,
}

pub fn run(ctx: &Ctx, a: MagicArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let mut layout = ctx.layout(&a.layout, &l)?;
    if let Some(sym) = &a.on {
        let sym = gecko_core::layout::parse_symbols(sym)?;
        let [c] = sym[..] else { bail!("--on takes one symbol") };
        let slot = layout.chars.iter().position(|&x| x == c).with_context(|| format!("'{c}' is not on the base layer"))?;
        layout.chars[slot] = MAGIC;
        let mut keys = layout.magic.clone();
        keys.push(Magic { key: MAGIC, ..Default::default() });
        layout = layout.with_magic(keys)?;
        layout.name = format!("{} + magic", layout.name);
    }
    let opts = SearchOpts { key: a.key.saturating_sub(1), max_rules: a.max_rules, keep: a.keep, min_gain: a.min_gain };
    let t0 = std::time::Instant::now();
    let r = magic::search_rules(&l.geometry, &l.corpus, &l.profile, &layout, &opts)?;
    let ms = t0.elapsed().as_millis() as u64;
    let (sc, st) = score_layout(&l, &r.layout)?;
    let b = breakdown(&sc, &st);
    if let Some(name) = &a.out {
        let path = if name.ends_with(".toml") { PathBuf::from(name) } else { ctx.store.data.join("layouts").join(format!("{name}.toml")) };
        ctx.write_file(&path, &r.layout.to_toml(&l.geometry))?;
    }
    let m = &r.layout.magic[opts.key];
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "layout": r.layout.name, "rows": r.layout.all_rows(&l.geometry), "key": m.key,
            "rules": magic::format_rules(&m.rules), "start_score": r.start_score, "score": r.score,
            "steps": r.steps, "evals": r.evals, "elapsed_ms": ms, "magic": r.layout.magic, "breakdown": b,
            "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name}));
        return Ok(());
    }
    println!("{}  ({} · {} · {})  {} evals in {} ms", r.layout.name, l.geometry.name, l.profile.name, l.corpus.name, r.evals, ms);
    for s in &r.steps {
        println!("  {:<8} {:>9.3}", s.rule, s.score);
    }
    println!("score {:.3} -> {:.3}\n  {}\n", r.start_score, r.score, layout_rows(&r.layout, &l.geometry));
    print_breakdown(&b);
    Ok(())
}
