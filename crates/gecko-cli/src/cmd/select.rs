//! `gecko select`: rank-sum selection over many runs (B9, `gecko_core::select`).

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use gecko_core::{run, select, SCHEMA};
use serde_json::json;

use crate::ctx::{Ctx, Inputs};
use crate::fmt::{chars_rows, fmt_val};

#[derive(clap::Args)]
pub struct SelectArgs {
    /// Run ids (or `run:<id>`): optimize, NSGA-II / RWSA, explore (atlas) or qd runs.
    #[arg(required = true)]
    runs: Vec<String>,
    /// Metrics to rank on, comma-separated (default: every metric the profile weights).
    #[arg(long, value_delimiter = ',')]
    metrics: Vec<String>,
    /// Layouts to show.
    #[arg(long, default_value_t = 10)]
    top: usize,
    /// Best optima / elites taken from each atlas or qd run.
    #[arg(long, default_value_t = 300)]
    pool: usize,
    /// Board, corpus and profile to score on (default: the runs' board, the first run's corpus
    /// and profile).
    #[command(flatten)]
    inputs: Inputs,
}

pub fn run(ctx: &Ctx, a: SelectArgs) -> Result<()> {
    let st = &ctx.store;
    let (cands, sources) = select::gather(&st.runs, &a.runs, a.pool)?;
    if cands.is_empty() {
        bail!("the runs have no layouts");
    }
    let first = run::load_config(&st.runs.join(&sources[0].id))?;
    let mut boards = BTreeMap::new();
    for s in &sources {
        if !boards.contains_key(&s.geometry) {
            boards.insert(s.geometry.clone(), st.geometry(&s.geometry)?);
        }
    }
    let g = match &a.inputs.geometry {
        Some(name) => st.geometry(name)?,
        None if boards.len() == 1 => boards.values().next().unwrap().clone(),
        None => bail!("the runs use different boards ({}); pass -g", boards.keys().cloned().collect::<Vec<_>>().join(", ")),
    };
    let corpus = st.corpus(a.inputs.corpus.as_deref().unwrap_or(&first.corpus.name))?;
    let profile = match &a.inputs.profile {
        Some(p) => st.profile(p)?,
        None => first.profile.clone(),
    };
    if a.inputs.corpus.is_none() && sources.iter().any(|s| s.corpus != corpus.name) {
        eprintln!("warning: the runs use different corpora; scoring on {} (pass -c)", corpus.name);
    }
    if a.inputs.profile.is_none() && sources.iter().any(|s| s.profile != profile.name) {
        eprintln!("warning: the runs use different profiles; scoring under {} (pass -p)", profile.name);
    }
    let defs = select::metric_cols(&a.metrics, &profile)?;
    let (metrics, ranked) = select::select(&cands, &boards, &g, &corpus, &profile, &defs)?;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "geometry": g.name, "corpus": corpus.name, "profile": profile.name,
            "metrics": metrics, "runs": sources, "candidates": ranked.len(),
            "ranked": ranked.iter().take(a.top).collect::<Vec<_>>()}));
        return Ok(());
    }
    println!(
        "{} layouts from {} runs on {} / {} / {}, ranked on {}",
        ranked.len(),
        sources.len(),
        g.name,
        corpus.name,
        profile.name,
        metrics.iter().map(|m| m.id).collect::<Vec<_>>().join(" ")
    );
    print!("{:<4} {:>7} {:>8}", "#", "ranks", "score");
    for m in &metrics {
        print!(" {:>7}", format!("{}{}", m.id, if m.dir == "max" { "+" } else { "" }));
    }
    println!("  layout  (source)");
    for s in ranked.iter().take(a.top) {
        print!("{:<4} {:>7.1} {:>8.3}", s.rank, s.rank_sum, s.score);
        for v in &s.values {
            print!(" {:>7}", fmt_val(*v));
        }
        let more = if s.sources.len() > 1 { format!(" +{}", s.sources.len() - 1) } else { String::new() };
        println!("  {}  ({}{more})", chars_rows(&s.chars, &g), s.sources[0]);
    }
    Ok(())
}
