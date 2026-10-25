//! `gecko fit-layout`: the weights under which each layout is closest to optimal, and every
//! layout scored under every other layout's weights (`context/cli.md`, `context/objective.md`).

use anyhow::Result;
use gecko_core::inverse::{cross, fit_layout, InverseFit, InverseSpec};
use gecko_core::{hash, SCHEMA};
use serde_json::json;

use crate::ctx::{threads, Ctx, Inputs};

#[derive(clap::Args)]
pub struct FitArgs {
    /// Layout specs (reference name, .toml, run:<id>[#rank], cmini:<name>, raw:<chars>).
    #[arg(required = true)]
    layouts: Vec<String>,
    /// Prior σ of each log weight multiplier (0.5 ≈ ×0.37–×2.7 at 95 %).
    #[arg(long, default_value_t = 0.5)]
    sigma: f64,
    /// Tolerance as a share of the median |Δ| of the layout's swaps under the base profile.
    #[arg(long, default_value_t = 0.05)]
    tol: f64,
    /// Bound on each log multiplier (3 = ×0.05–×20).
    #[arg(long, default_value_t = 3.0)]
    bound: f64,
    /// Search rounds (cutting planes) at most.
    #[arg(long, default_value_t = 6)]
    rounds: usize,
    /// BLS evaluations per search.
    #[arg(long, default_value_t = 4_000_000)]
    evals: u64,
    /// Searches per round.
    #[arg(long, default_value_t = 2)]
    seeds: u64,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Worker threads per search (default: all cores).
    #[arg(long)]
    threads: Option<usize>,
    /// Swap neighbours used as competitors from the start.
    #[arg(long, default_value_t = 600)]
    neighbours: usize,
    /// Metrics to fit, comma-separated (default: every weighted metric).
    #[arg(long, default_value = "")]
    fit: String,
    /// Display names for the layouts, comma-separated in the same order (default: the specs).
    #[arg(long, default_value = "")]
    names: String,
    /// Symbols that keep their position in every search, e.g. ",./;".
    #[arg(long, default_value = "")]
    pin: String,
    /// Write the full report as JSON to this file.
    #[arg(long)]
    out: Option<std::path::PathBuf>,
    /// Write each fitted profile as <dir>/<layout>-fit.toml.
    #[arg(long)]
    profiles: Option<std::path::PathBuf>,
    #[command(flatten)]
    inputs: Inputs,
}

pub fn run(ctx: &Ctx, a: FitArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let spec = InverseSpec {
        sigma: a.sigma,
        tol: a.tol,
        bound: a.bound,
        rounds: a.rounds,
        evals: a.evals,
        seeds: a.seeds,
        seed: a.seed,
        threads: threads(a.threads),
        neighbours: a.neighbours,
        fit: a.fit.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
    };
    let quiet = ctx.json;
    let names: Vec<&str> = a.names.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    if !names.is_empty() && names.len() != a.layouts.len() {
        anyhow::bail!("--names has {} names for {} layouts", names.len(), a.layouts.len());
    }
    let mut fits: Vec<InverseFit> = Vec::new();
    for (k, layout_spec) in a.layouts.iter().enumerate() {
        let spec_name = names.get(k).copied().unwrap_or(layout_spec);
        let layout = ctx.layout(layout_spec, &l)?;
        let flat = layout.flat(&l.geometry)?.1;
        let movable: Vec<usize> = (0..flat.len()).filter(|&s| !a.pin.contains(flat[s])).collect();
        let f = fit_layout(&l.geometry, &l.corpus, &l.profile, spec_name, &layout, &movable, &spec, |r, n| {
            if !quiet {
                eprint!("\r  {spec_name}: round {r}/{n}   ");
            }
        })?;
        if !quiet {
            eprintln!();
        }
        if let Some(dir) = &a.profiles {
            let p = f.profile(&l.profile);
            ctx.write_file(&dir.join(format!("{}.toml", p.name)), &super::profile::profile_toml(&p)?)?;
        }
        fits.push(f);
    }
    let matrix = (fits.len() > 1).then(|| cross(&l.profile, &fits));
    let inputs = json!({
        "geometry": {"name": l.geometry.name, "hash": hash::hex(l.geometry.hash())},
        "corpus": {"name": l.corpus.name, "hash": hash::hex(l.corpus.hash())},
        "profile": {"name": l.profile.name, "hash": hash::hex(l.profile.hash())},
        "pins": a.pin,
    });
    let report = json!({"schema": SCHEMA, "inputs": inputs, "fits": fits, "cross": matrix});
    if let Some(path) = &a.out {
        ctx.write_file(path, &serde_json::to_string_pretty(&report)?)?;
    }
    if ctx.json {
        ctx.print(report);
        return Ok(());
    }
    for f in &fits {
        print_fit(f, &l.geometry);
    }
    if let Some(m) = &matrix {
        let w = m.layouts.iter().map(|s| s.len()).max().unwrap_or(6).clamp(6, 16);
        println!("\neach layout (row) under each fit's weights (column): % above the best layout known under them");
        print!("{:<w$}", "");
        for n in &m.layouts {
            print!(" {:>w$}", trunc(n, w));
        }
        println!();
        for (i, n) in m.layouts.iter().enumerate() {
            print!("{:<w$}", trunc(n, w));
            for j in 0..m.layouts.len() {
                print!(" {:>w$}", format!("{:.1} ({})", m.gap_pct[i][j], m.rank[i][j]));
            }
            println!();
        }
        println!("(rank among these layouts in brackets)");
    }
    Ok(())
}

fn trunc(s: &str, w: usize) -> String {
    s.chars().take(w).collect()
}

fn print_fit(f: &InverseFit, g: &gecko_core::Geometry) {
    println!("\n{}  {}", f.layout, crate::fmt::chars_rows(&f.chars, g));
    let line = |tag: &str, p: &gecko_core::inverse::Probe| {
        let what = if p.gap > 0.0 { format!("better layout {:.2} % lower, {} keys away", p.gap_pct, p.keys) } else { "nothing better found".into() };
        println!("  {tag:<15} score {:.3}  {what}", p.layout_score);
    };
    line("base profile:", &f.base);
    line("fitted weights:", &f.fitted);
    if f.fitted.gap > 0.0 {
        println!("    best found: {}", crate::fmt::chars_rows(&f.fitted.best, g));
    }
    println!(
        "  {} after {} rounds; log distance from the base profile {:.2} (tolerance {:.4})",
        if f.self_consistent {
            "self-consistent"
        } else if f.converged {
            "not optimal under any nearby weights"
        } else {
            "still finding better layouts (raise --rounds)"
        },
        f.rounds.len(),
        f.log_distance,
        f.tolerance
    );
    let mut w = f.weights.clone();
    w.sort_by(|x, y| y.multiplier.ln().abs().total_cmp(&x.multiplier.ln().abs()));
    println!("  {:<14} {:>8} {:>8} {:>7}  {:>15}", "weight", "base", "fitted", "×", "95 % interval");
    for x in &w {
        let flag = if x.at_bound {
            "  at bound"
        } else if x.sd_ratio > 0.8 {
            "  (prior)"
        } else {
            ""
        };
        println!("  {:<14} {:>8.4} {:>8.4} {:>7.2}  {:>6.2} – {:<6.2}{flag}", x.id, x.base, x.fitted, x.multiplier, x.lo, x.hi);
    }
}
