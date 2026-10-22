//! `gecko profile normalize | fit`: re-express or fit a profile's weights.

use anyhow::{bail, Result};
use clap::Subcommand;
use gecko_core::{calibrate, Layout, Profile, SCHEMA};
use serde_json::json;

use crate::ctx::{Ctx, Inputs, Loaded};

#[derive(Subcommand)]
pub enum ProfileCmd {
    /// Measure each metric's spread over reference layouts and re-express the profile's weights
    /// in spread units (same objective, comparable weights). Prints TOML unless --out.
    Normalize {
        /// Layouts to measure spread on (default: all reference layouts).
        #[arg(long, value_delimiter = ',')]
        layouts: Vec<String>,
        /// Write data/profiles/<out>.toml.
        #[arg(long)]
        out: Option<String>,
        #[command(flatten)]
        inputs: Inputs,
    },
    /// Fit non-negative spread-unit weights to a judge metric (e.g. E05), or use
    /// --target likes for pairwise cmini ranking with 20% of layouts held out.
    Fit {
        #[arg(long, default_value = "E05")]
        target: String,
        #[arg(long, value_delimiter = ',')]
        layouts: Vec<String>,
        #[arg(long)]
        out: Option<String>,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[command(flatten)]
        inputs: Inputs,
    },
}

impl ProfileCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            ProfileCmd::Normalize { layouts, out, inputs } => normalize(ctx, layouts, out.as_deref(), &inputs),
            ProfileCmd::Fit { target, layouts, out, seed, inputs } if target.eq_ignore_ascii_case("likes") => {
                if !layouts.is_empty() {
                    bail!("--target likes uses the community bundle; --layouts is only for judge-metric fits");
                }
                fit_likes(ctx, out, seed, &inputs)
            }
            ProfileCmd::Fit { target, layouts, out, seed, inputs } => fit(ctx, &target, layouts, out, seed, &inputs),
        }
    }
}

fn normalize(ctx: &Ctx, layouts: Vec<String>, out: Option<&str>, inputs: &Inputs) -> Result<()> {
    let l = ctx.load(inputs)?;
    let (refs, basis) = reference_layouts(ctx, &l, layouts)?;
    let scales = calibrate::spread_scales(&l.geometry, &l.corpus, &l.profile, &refs)?;
    let mut p = l.profile.clone();
    for (id, w) in p.weights.iter_mut() {
        let eff = l.profile.effective_weight(id);
        *w = match scales.get(id) {
            Some(s) => round4(eff * s),
            None => eff,
        };
    }
    p.scale = scales.into_iter().filter(|(id, _)| p.weights.contains_key(id)).map(|(k, v)| (k, round4(v))).collect();
    p.scale_basis = basis;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "profile": p}));
    }
    emit_profile(ctx, &p, out)
}

fn fit_likes(ctx: &Ctx, out: Option<String>, seed: u64, inputs: &Inputs) -> Result<()> {
    let l = ctx.load(inputs)?;
    let bundle = ctx.store.community()?;
    let fit = calibrate::fit_preferences(&l.geometry, &l.corpus, &l.profile, &bundle, seed)?;
    let accuracy = fit.held_out_accuracy.map_or_else(|| "unavailable".into(), |a| format!("{:.1}%", 100.0 * a));
    let mut p = l.profile.clone();
    p.name = out.clone().unwrap_or_else(|| "fit-likes".into());
    p.description = format!(
        "Pairwise cmini likes fit, seed {seed}, {} train/{} held-out layouts; held-out accuracy {accuracy}. Opt-in popularity model.",
        fit.training_layouts.len(),
        fit.held_out_layouts.len(),
    );
    p.weights = fit.weights.clone();
    p.scale = fit.scales.clone();
    p.scale_basis = format!("cmini training layouts; seed {seed}; {}", fit.input_hashes["community"]);
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "fit": fit, "profile": p}));
    } else {
        eprintln!("likes: {} training pairs, {} held-out pairs; accuracy {accuracy}", fit.training_pairs, fit.held_out_pairs);
    }
    emit_profile(ctx, &p, out.as_deref())
}

fn fit(ctx: &Ctx, target: &str, layouts: Vec<String>, out: Option<String>, seed: u64, inputs: &Inputs) -> Result<()> {
    let l = ctx.load(inputs)?;
    let (refs, basis) = reference_layouts(ctx, &l, layouts)?;
    let fit = calibrate::fit_to(&l.geometry, &l.corpus, &l.profile, &refs, target, seed)?;
    let mut p = l.profile.clone();
    p.name = out.clone().unwrap_or_else(|| format!("fit-{}", target.to_lowercase()));
    p.description = format!("Weights fitted to predict {target} (R² = {:.3}, {} samples, seed {seed}).", fit.r2, fit.samples);
    p.weights = fit.weights.iter().map(|(k, v)| (k.clone(), round4(*v))).collect();
    p.scale = fit.scales.iter().filter(|(k, _)| p.weights.contains_key(*k)).map(|(k, v)| (k.clone(), round4(*v))).collect();
    p.scale_basis = basis;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "fit": fit, "profile": p}));
    } else {
        eprintln!("{target}: R² = {:.3} over {} sampled layouts", fit.r2, fit.samples);
    }
    emit_profile(ctx, &p, out.as_deref())
}

fn reference_layouts(ctx: &Ctx, l: &Loaded, names: Vec<String>) -> Result<(Vec<Layout>, String)> {
    let names = if names.is_empty() { ctx.store.list("layouts") } else { names };
    let layouts = names.iter().map(|n| ctx.layout(n, l)).collect::<Result<Vec<_>>>()?;
    let basis = format!("stdev over {} layouts ({}) on {} / {}", layouts.len(), names.join(", "), l.corpus.name, l.geometry.name);
    Ok((layouts, basis))
}

/// Print a profile as TOML, or write it to data/profiles/<out>.toml.
fn emit_profile(ctx: &Ctx, p: &Profile, out: Option<&str>) -> Result<()> {
    // Profile numbers are f32; print each at its shortest f32 decimal, not the widened f64.
    fn tidy(v: &mut toml::Value) {
        match v {
            toml::Value::Float(f) => *f = format!("{}", *f as f32).parse().unwrap_or(*f),
            toml::Value::Array(a) => a.iter_mut().for_each(tidy),
            toml::Value::Table(t) => t.iter_mut().for_each(|(_, v)| tidy(v)),
            _ => {}
        }
    }
    let mut v = toml::Value::try_from(p)?;
    tidy(&mut v);
    let text = toml::to_string(&v)?;
    match out {
        Some(name) => ctx.write_file(&ctx.store.data.join("profiles").join(format!("{name}.toml")), &text),
        None if !ctx.json => {
            print!("{text}");
            Ok(())
        }
        None => Ok(()),
    }
}

/// Round to 4 significant digits so generated profiles stay readable.
fn round4(v: f32) -> f32 {
    if v == 0.0 || !v.is_finite() {
        return v;
    }
    let m = 10f32.powi(3 - v.abs().log10().floor() as i32);
    (v * m).round() / m
}
