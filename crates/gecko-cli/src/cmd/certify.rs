//! `gecko certify`: exact optimum over a few free symbols of a run's best layout (B7).

use std::time::Duration;

use anyhow::{bail, Result};
use gecko_core::certify::{certify_run, FreeSpec, Kind, Limits};
use serde_json::json;

use crate::ctx::{parse_duration, Ctx};

#[derive(clap::Args)]
pub struct CertifyArgs {
    /// The run whose best layout is certified (`run:<id>` or `<id>`).
    run: String,
    /// Free the k most frequent unpinned symbols; every other symbol stays on its slot.
    #[arg(long, default_value_t = 10)]
    free: usize,
    /// Free exactly these symbols instead (e.g. `nsrhtceaio`); overrides --free.
    #[arg(long)]
    symbols: Option<String>,
    /// `full`: the real objective. `pair`: only the mono + bigram/skipgram part (not the score).
    #[arg(long, default_value = "full")]
    kind: String,
    /// Stop after this long (`0` = no limit); the result is then a best-so-far and a lower bound.
    #[arg(long, default_value = "60s")]
    time: String,
    /// Stop after this many search nodes.
    #[arg(long)]
    nodes: Option<u64>,
}

pub fn run(ctx: &Ctx, a: CertifyArgs) -> Result<()> {
    let id = a.run.strip_prefix("run:").unwrap_or(&a.run);
    if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
        bail!("bad run id '{id}'");
    }
    let dir = ctx.store.runs.join(id);
    let kind = Kind::parse(&a.kind)?;
    let free = match &a.symbols {
        Some(s) => FreeSpec::Symbols(gecko_core::layout::parse_symbols(s)?),
        None => FreeSpec::Count(a.free),
    };
    let ms = parse_duration(&a.time)?;
    let limits = Limits { time: (ms > 0).then(|| Duration::from_millis(ms)), nodes: a.nodes };
    let r = certify_run(&ctx.store, &dir, &free, kind, limits)?;
    let path = dir.join(format!("certify-{}.json", kind.name()));
    ctx.write_file(&path, &serde_json::to_string_pretty(&r)?)?;
    if ctx.json {
        ctx.print(json!(r));
        return Ok(());
    }
    let c = &r.cert;
    println!("certify {}  ({} · {} · {})", r.run, r.geometry.name, r.corpus.name, r.profile);
    println!("kind  {}: {}", kind.name(), r.covers);
    println!("free  {} ({} symbols, the rest stay on their slots)", c.free, c.free.chars().count());
    let what = if kind == Kind::Pair { "pair part" } else { "score" };
    match (&c.stopped, c.improved) {
        (None, false) => println!("OPTIMAL ({}): no placement of the free symbols beats the run's {what} {:.4}", kind.name(), c.start),
        (None, true) => {
            println!("IMPROVED ({}): {what} {:.4} -> {:.4} (the exact optimum)", kind.name(), c.start, c.optimum);
            println!("  {}", r.rows.join("\n  "));
        }
        (Some(why), _) => {
            let gap = 100.0 * (c.optimum - c.lower) / c.optimum.abs().max(1e-12);
            println!(
                "NOT CERTIFIED (stopped: {why}): best {what} {:.4} (start {:.4}), lower bound {:.4}, gap {gap:.2} %",
                c.optimum, c.start, c.lower
            );
        }
    }
    println!(
        "root bound {:.4} · {} nodes · {} leaves of {:.0} · {:.2} % pruned · {:.1} s",
        c.root_bound,
        c.nodes,
        c.leaves,
        c.space,
        c.pruned_pct,
        c.elapsed_ms as f64 / 1000.0
    );
    Ok(())
}
