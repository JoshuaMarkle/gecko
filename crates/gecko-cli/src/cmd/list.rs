//! `gecko list layouts | geometries | profiles | corpora | runs | metrics`.

use anyhow::{bail, Result};
use gecko_core::metrics::METRICS;
use gecko_core::{run, SCHEMA};
use serde_json::json;

use crate::ctx::Ctx;

pub fn run(ctx: &Ctx, what: &str) -> Result<()> {
    let items: serde_json::Value = match what {
        "layouts" | "geometries" | "profiles" | "corpora" => json!(ctx.store.list(what)),
        "runs" => json!(run::list(&ctx.store.runs)),
        "metrics" => json!(METRICS),
        _ => bail!("list what? layouts | geometries | profiles | corpora | runs | metrics"),
    };
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "kind": what, "items": items}));
        return Ok(());
    }
    match what {
        "runs" => {
            for r in run::list(&ctx.store.runs) {
                let score = r.best_score.map(|s| format!("{s:.3}")).unwrap_or("-".into());
                println!("{:<34} {:<4} {:<9} {:>9} {}", r.id, r.algo, r.profile, score, r.best.unwrap_or_default());
            }
        }
        "metrics" => {
            for m in METRICS {
                let tag = if m.retired { " (retired)" } else { "" };
                println!("{:<15} {:<4} {:<18} {:?}{tag}  {}", m.id, m.legacy, m.name, m.dir, m.desc);
            }
        }
        _ => {
            for i in items.as_array().unwrap() {
                println!("{}", i.as_str().unwrap());
            }
        }
    }
    Ok(())
}
