//! `gecko keyset` and `optimize --keyset`: build the symbol set and start layout from the board
//! and corpus (alpha only, alpha + Shift-paired symbols, or alpha + a symbol layer), and search
//! the keys of the fixed modifiers (Shift, Ctrl, layer key).

use anyhow::Result;
use gecko_core::keyset::{self, Keyset, PlaceOpts, Placement, Spec};
use gecko_core::{Layout, SCHEMA};
use serde_json::json;

use crate::ctx::{score_layout, threads, Ctx, Inputs, Loaded};
use crate::fmt::layout_rows;

/// Keyset options shared by `gecko keyset` and `gecko optimize`.
#[derive(clap::Args, Clone, Default)]
pub struct KeysetOpts {
    /// Symbols to fill: pair keys (`shift`) or symbols on base and layer (`layer`). Default: every free key.
    #[arg(long)]
    pub symbols: Option<usize>,
    /// Shift pairs (`shift`): natural (US: -_ =+ [{ ;: '" ,< .> /?), frequency (by rank), or
    /// pairs like "-_ =+ ({ )}".
    #[arg(long, default_value = "natural")]
    pub pairs: String,
    /// Count digits as symbols (default: in `layer` mode only).
    #[arg(long, overrides_with = "no_digits")]
    pub digits: bool,
    #[arg(long)]
    pub no_digits: bool,
    /// Special keys, comma-separated: space,bspc,enter,shift,ctrl,tab,esc (repeat shift for one
    /// per hand). Default: Space, Backspace, Enter, Shift, Ctrl (+ Tab, Esc when keys are left)
    /// on boards with thumb keys, none otherwise.
    #[arg(long)]
    pub keys: Option<String>,
    /// How the symbol layer is reached (`layer`): hold, oneshot or toggle.
    #[arg(long, default_value = "hold")]
    pub layer_mode: String,
    /// Skip the search for the keys of Shift, Ctrl and the layer key.
    #[arg(long)]
    pub no_place: bool,
    /// Let letters and symbols take thumb keys (and Space, Backspace, Enter leave them).
    /// Default: a thumb zone (profile `thumb_zone`, L04 weight 100) keeps them apart.
    #[arg(long)]
    pub thumb_letters: bool,
    /// Placement search: SA evaluations for its first optimization and after each accepted move.
    #[arg(long, default_value_t = 2_000_000)]
    pub place_evals: u64,
}

impl KeysetOpts {
    pub fn spec(&self, mode: &str) -> Result<Spec> {
        let digits = if self.digits {
            Some(true)
        } else if self.no_digits {
            Some(false)
        } else {
            None
        };
        Ok(keyset::spec_from(mode, self.symbols, &self.pairs, digits, self.keys.as_deref(), &self.layer_mode, self.thumb_letters)?)
    }

    /// Build the keyset, set the profile's thumb zone (unless `--thumb-letters`) and, unless
    /// `--no-place` or it has no fixed keys, place them.
    pub fn start(
        &self,
        ctx: &Ctx,
        l: &mut Loaded,
        mode: &str,
        pins: &str,
        seed: u64,
        threads: usize,
    ) -> Result<(Keyset, Option<Placement>)> {
        let ks = keyset::build(&l.geometry, &l.corpus, &self.spec(mode)?)?;
        let notes = keyset::apply_profile(&l.geometry, &ks.layout, &mut l.profile, self.thumb_letters);
        if !ctx.json && !notes.is_empty() {
            eprintln!("profile {}: {}", l.profile.name, notes.join(", "));
        }
        let has_fixed = ks.layout.chars.iter().any(|&c| gecko_core::layout::is_fixed_key(c));
        if self.no_place || !has_fixed {
            return Ok((ks, None));
        }
        if !ctx.json {
            eprintln!("placing Shift/Ctrl/layer keys ({} SA evals per settle)…", self.place_evals);
        }
        let opts =
            PlaceOpts { evals: self.place_evals, seed, threads, pins: gecko_core::layout::parse_symbols(pins)?, ..Default::default() };
        let p = keyset::place_fixed(&l.geometry, &l.corpus, &l.profile, &ks.layout, &opts)?;
        Ok((ks, Some(p)))
    }
}

#[derive(clap::Args)]
pub struct KeysetArgs {
    /// alpha (letters only; free keys stay empty), shift (letters + symbol keys with Shift
    /// pairs) or layer (letters + a symbol layer).
    mode: String,
    #[command(flatten)]
    opts: KeysetOpts,
    /// Search the fixed keys' places (off by default here; `optimize --keyset` does it).
    #[arg(long)]
    place: bool,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[arg(long)]
    threads: Option<usize>,
    /// Write the start layout: a name (data/layouts/keyset/<name>.toml) or a .toml path.
    #[arg(long)]
    out: Option<String>,
    #[command(flatten)]
    inputs: Inputs,
}

pub fn run(ctx: &Ctx, a: KeysetArgs) -> Result<()> {
    let mut l = ctx.load(&a.inputs)?;
    let opts = KeysetOpts { no_place: !a.place, ..a.opts.clone() };
    let (ks, placed) = opts.start(ctx, &mut l, &a.mode, "", a.seed, threads(a.threads))?;
    let mut layout = placed.as_ref().map_or_else(|| ks.layout.clone(), |p| p.layout.clone());
    layout.name = format!("keyset-{}", ks.mode.name());
    let (_, st) = score_layout(&l, &layout)?;
    if let Some(out) = &a.out {
        let path = if out.ends_with(".toml") || out.contains('/') {
            std::path::PathBuf::from(out)
        } else {
            ctx.store.data.join("layouts/keyset").join(format!("{out}.toml"))
        };
        ctx.write_file(&path, &layout.to_toml(&l.geometry))?;
    }
    if ctx.json {
        ctx.print(json!({
            "schema": SCHEMA, "mode": ks.mode, "geometry": l.geometry.name, "corpus": l.corpus.name,
            "rows": layout.all_rows(&l.geometry), "chars": flat(&layout, &l)?, "shifted": layout.shifted,
            "keys": ks.keys, "left_out": ks.left_out, "coverage": ks.coverage, "empty": ks.empty,
            "score": st.score, "placement": placed,
        }));
        return Ok(());
    }
    print_keyset(&ks, &layout, &l, placed.as_ref());
    println!("score {:.3}  ({} · {})", st.score, l.profile.name, l.corpus.name);
    Ok(())
}

fn flat(layout: &Layout, l: &Loaded) -> Result<String> {
    Ok(layout.flat(&l.geometry)?.1.iter().collect())
}

/// Human summary of a keyset: the symbols it places, what it leaves out, and the start layout.
pub fn print_keyset(ks: &Keyset, layout: &Layout, l: &Loaded, placed: Option<&Placement>) {
    let fmt = |kind: &str| {
        ks.keys
            .iter()
            .filter(|k| k.kind == kind)
            .map(|k| format!("{}{}", k.symbol, k.shifted.map_or(String::new(), |s| s.to_string())))
            .collect::<Vec<_>>()
            .join(" ")
    };
    println!(
        "keyset {} on {} ({}): {:.1}% of keystrokes typeable, {} empty keys",
        ks.mode.name(),
        l.geometry.name,
        l.corpus.name,
        ks.coverage,
        ks.empty
    );
    for (kind, label) in [("special", "keys"), ("fixed", "fixed"), ("symbol", "symbols"), ("layer", "layer")] {
        let s = fmt(kind);
        if !s.is_empty() {
            println!("  {label:<8} {s}");
        }
    }
    if !ks.left_out.is_empty() {
        let top: Vec<String> = ks
            .left_out
            .iter()
            .take(12)
            .map(|k| format!("{}{} {:.2}%", k.symbol, k.shifted.map_or(String::new(), |s| s.to_string()), k.share))
            .collect();
        println!("  left out {}{}", top.join(", "), if ks.left_out.len() > 12 { ", …" } else { "" });
    }
    if let Some(p) = placed {
        println!("  placement: {} candidates, {} move(s), score {:.3} → {:.3}", p.tried, p.moves.len(), p.start_score, p.score);
        for m in &p.moves {
            println!("    {} key {} → key {}  ({:.3})", m.key, m.from, m.to, m.score);
        }
    }
    println!("  {}", layout_rows(layout, &l.geometry));
}
