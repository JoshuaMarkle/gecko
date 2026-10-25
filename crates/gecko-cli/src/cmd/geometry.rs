//! `gecko geometry import-kle`: turn a keyboard-layout-editor.com file into a board TOML.
//! `gecko geometry optimize`: bilevel board + layout search on a parametric board (G6).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gecko_core::geomsearch::{self, GeomParams, GeomResult, GeomSpec, Mode};
use gecko_core::optimize::Control;
use gecko_core::SCHEMA;
use serde_json::json;

use crate::ctx::{threads, time_budget, Ctx, Inputs};

#[derive(Subcommand)]
pub enum GeometryCmd {
    /// Import a keyboard-layout-editor.com JSON file (or its raw data) as
    /// data/geometries/<out>.toml. Fingers, rows, home keys and efforts are guesses to review.
    ImportKle {
        file: PathBuf,
        #[arg(long)]
        out: String,
        /// Overwrite an existing geometry.
        #[arg(long)]
        force: bool,
    },
    /// Optimize a parametric board (-g <name>.param or a .param.toml path) together with its
    /// layout: CMA-ES over the free parameters, a warm-started layout search per candidate.
    /// Writes runs/<id>/ with the best board as geometry.toml and geometry.param.toml.
    Optimize(Box<OptimizeArgs>),
}

#[derive(clap::Args)]
pub struct OptimizeArgs {
    /// Free parameters: 2d, 3d, groups (stagger, splay, spread, lift, row_pitch, row_curve,
    /// col_curve, angle, tent, tilt, thumb, thumb3d) or split.<f> / <zone>.<f> /
    /// <zone>.<col>.<f>; `=lo:hi` sets bounds, e.g. "3d,tent=0:30". Free-form (I1): keys
    /// (every key's x, y, z, rx, ry, rz), keys2d (x, y, rz) or keys.<c>, with `=lo:hi` relative
    /// to each key's start, e.g. "keys,keys.z=-0.3:0.3"; start from a run's best board
    /// (-g run:<id>).
    #[arg(long, default_value = "2d")]
    free: String,
    /// bilevel (layout search per candidate), alternate (board step ↔ layout step) or board
    /// (the board for the fixed start layout).
    #[arg(long, default_value = "bilevel", value_parser = ["bilevel", "alternate", "board"])]
    mode: String,
    /// Time budget for the whole run, e.g. 30s, 5m.
    #[arg(long)]
    time: Option<String>,
    /// Board candidates to score (deterministic; overrides --time).
    #[arg(long)]
    evals: Option<u64>,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Worker threads (default: all cores).
    #[arg(long)]
    threads: Option<usize>,
    /// Symbols that keep their start position.
    #[arg(long, default_value = "")]
    pin: String,
    /// Start layout.
    #[arg(long)]
    from: Option<String>,
    /// Layout SA swaps per board candidate (bilevel).
    #[arg(long)]
    inner_evals: Option<u64>,
    /// Layout SA swaps per final polish and alternate layout step (with --evals).
    #[arg(long)]
    polish_evals: Option<u64>,
    /// CMA-ES initial step, as a fraction of each parameter's range.
    #[arg(long)]
    sigma0: Option<f64>,
    /// CMA-ES population (default 4 + 3 ln n).
    #[arg(long)]
    popsize: Option<usize>,
    /// Keycap clearance per side, u.
    #[arg(long)]
    margin: Option<f32>,
    /// Collision penalty per u of overlap, in units of |start board score| + 1.
    #[arg(long)]
    penalty: Option<f64>,
    /// Total overlap (u) past which a candidate is rejected unscored.
    #[arg(long)]
    hard_fail: Option<f64>,
    /// Alternate mode: rounds.
    #[arg(long)]
    rounds: Option<usize>,
    /// Keep the file's hand-entered key efforts instead of deriving them from positions.
    #[arg(long)]
    keep_effort: bool,
    /// Free-form: penalty per unit of roughness (squared move differences of neighbouring
    /// keys, u and radians), in units of |start board score| + 1.
    #[arg(long)]
    smooth: Option<f64>,
    /// Free-form: collision repair passes per colliding candidate (0: penalty only).
    #[arg(long)]
    repair_passes: Option<usize>,
    /// Thumb keys per thumb zone side: a count, or `auto` for as many as fit along the thumb's
    /// operational arc for the profile's hand (3 for the average hand). Shrinking keeps the keys
    /// with typed fills, then the cheapest; growing adds empty 1 u keys.
    #[arg(long)]
    thumb_keys: Option<String>,
    #[command(flatten)]
    inputs: Inputs,
}

impl GeometryCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            GeometryCmd::ImportKle { file, out, force } => import_kle(ctx, file, out, force),
            GeometryCmd::Optimize(a) => optimize(ctx, *a),
        }
    }
}

fn import_kle(ctx: &Ctx, file: PathBuf, out: String, force: bool) -> Result<()> {
    let text = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let g = gecko_core::interop::kle::import(&out, &text)?;
    let path = ctx.store.data.join("geometries").join(format!("{out}.toml"));
    if path.exists() && !force {
        bail!("{} exists (use --force to overwrite)", path.display());
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, g.to_toml()).with_context(|| format!("writing {}", path.display()))?;
    let fingers: Vec<&str> = g.key.iter().map(|k| k.finger.name()).collect();
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "geometry": g.name, "path": path, "keys": g.len(), "rows": g.row_lengths(),
            "fingers": fingers, "hash": gecko_core::hash::hex(g.hash())}));
    } else {
        println!("wrote {} ({} keys, rows {:?})", path.display(), g.len(), g.row_lengths());
        println!("fingers, rows, home keys and efforts are guesses: review the file");
    }
    Ok(())
}

fn optimize(ctx: &Ctx, a: OptimizeArgs) -> Result<()> {
    let name = a.inputs.geometry.clone().unwrap_or_else(|| ctx.store.defaults.geometry.clone());
    let mut params = ctx.store.param_geometry(&name)?;
    let mut l = ctx.load(&a.inputs)?;
    if let Some(tk) = &a.thumb_keys {
        let hand = l.profile.hand.clone().unwrap_or_default();
        let n = if tk == "auto" {
            gecko_core::hand::thumb_slots(&hand.dims())
        } else {
            tk.parse().with_context(|| format!("bad --thumb-keys '{tk}' (a count or auto)"))?
        };
        params = geomsearch::resize_thumbs(&params, &l.geometry, &l.corpus, &hand, n)?;
        params.description = format!("{} ({n} thumb keys per side)", params.description);
        l.geometry = params.generate_unchecked()?;
        ctx.store.load_comfort(&mut l.geometry)?;
        eprintln!("thumb keys: {n} per side");
    }
    let start = ctx.start(a.from.as_deref(), &l)?;
    let vars = geomsearch::parse_free(&params, &a.free)?;
    let mut s = GeomParams {
        mode: match a.mode.as_str() {
            "alternate" => Mode::Alternate,
            "board" => Mode::Board,
            _ => Mode::Bilevel,
        },
        seed: a.seed,
        threads: threads(a.threads),
        time_ms: time_budget(a.time.as_deref(), a.evals, "60s")?,
        evals: a.evals,
        keep_effort: a.keep_effort,
        ..Default::default()
    };
    fn set<T: Copy>(dst: &mut T, v: Option<T>) {
        if let Some(v) = v {
            *dst = v;
        }
    }
    set(&mut s.inner_evals, a.inner_evals);
    set(&mut s.polish_evals, a.polish_evals);
    set(&mut s.sigma0, a.sigma0);
    s.popsize = a.popsize.or(s.popsize);
    set(&mut s.margin, a.margin);
    set(&mut s.penalty, a.penalty);
    set(&mut s.hard_fail, a.hard_fail);
    set(&mut s.rounds, a.rounds);
    set(&mut s.smooth, a.smooth);
    set(&mut s.repair_passes, a.repair_passes);
    let hand = ["reach", "wrist", "press_angle", "thumb_arc"].iter().any(|m| l.profile.weights.get(*m).is_some_and(|w| *w != 0.0));
    if !hand {
        eprintln!(
            "warning: profile '{}' weights no hand-model metric (reach/wrist/press_angle/thumb_arc); board shape only moves distance metrics. Try -p hand.",
            l.profile.name
        );
    }
    let spec = GeomSpec {
        params: &params,
        board: &l.geometry,
        corpus: &l.corpus,
        profile: &l.profile,
        start: &start,
        pins: &a.pin,
        vars: &vars,
        search: s,
    };
    eprintln!(
        "geometry search on {} ({} free parameters, {} · {} threads · {})",
        params.name,
        vars.len(),
        a.mode,
        spec.search.threads,
        l.profile.name
    );
    let quiet = ctx.json;
    let mut last = std::time::Instant::now();
    let (res, dir) = geomsearch::run(&ctx.store.runs, &spec, &a.free, &Control::default(), |p| {
        if !quiet && (last.elapsed().as_millis() >= 250 || p.phase != "search") {
            last = std::time::Instant::now();
            let feasible = p.best_feasible.map_or("-".to_string(), |f| format!("{f:.4}"));
            eprint!(
                "\r  {:>6.1}s  {:<7} gen {:>4}  {:>6} boards  best {:.4}  collision-free {}   ",
                p.elapsed_ms as f64 / 1000.0,
                p.phase,
                p.generation,
                p.candidates,
                p.best,
                feasible
            );
        }
    })?;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "run": res, "dir": dir}));
        return Ok(());
    }
    eprintln!();
    print_result(&res, &dir);
    Ok(())
}

fn print_result(res: &GeomResult, dir: &std::path::Path) {
    println!(
        "run {}  {} boards, {} generations, {:.1} M layout evals in {:.1}s",
        res.id,
        res.candidates,
        res.generations,
        res.layout_evals as f64 / 1e6,
        res.elapsed_ms as f64 / 1000.0
    );
    let freeform = geomsearch::is_freeform(&res.vars);
    if freeform {
        print_key_moves(res);
    } else {
        println!("\n{:<24} {:>9} {:>9}   bounds", "parameter", "start", "best");
        for (v, b) in res.vars.iter().zip(&res.best.values) {
            let moved = if (b - v.start).abs() > 1e-6 { "" } else { "  (unchanged)" };
            println!("{:<24} {:>9.3} {:>9.3}   [{}, {}]{moved}", v.name, v.start, b, v.lo, v.hi);
        }
    }
    let gain = res.start.score - res.best.score;
    println!(
        "\nscore (same layout polish on both boards): start board {:.4} → best board {:.4} ({:+.4}, {:+.2}%)",
        res.start.score,
        res.best.score,
        -gain,
        -100.0 * gain / res.start.score.abs().max(1e-9)
    );
    if res.feasible {
        println!("collisions: none");
    } else {
        println!(
            "collisions: {} pairs, {:.3} u total; no collision-free board found (widen bounds or raise --penalty; geometry-repaired.toml, when written, has its keys pushed apart)",
            res.best.collisions, res.best.depth
        );
    }
    println!("\nstart board layout\n  {}\nbest board layout\n  {}", res.start.rows.join("\n  "), res.best.rows.join("\n  "));
    let param = if freeform { "" } else { " and geometry.param.toml" };
    println!("\nwrote {}/geometry.toml (use with -g){param}", dir.display());
}

/// Free-form runs have six variables per key: summarize the moves instead of listing them.
fn print_key_moves(res: &GeomResult) {
    let comps = gecko_core::geomsearch::freeform::COMPONENTS;
    println!("\n{} key variables (roughness {:.4}); largest moves from the start board:", res.vars.len(), res.best.rough);
    println!("  {:<4} {:>8} {:>8}  at bound", "", "max |Δ|", "mean |Δ|");
    for c in comps {
        let d: Vec<(f64, bool)> = res
            .vars
            .iter()
            .zip(&res.best.values)
            .filter(|(v, _)| v.name.rsplit('.').next() == Some(c))
            .map(|(v, &b)| ((b - v.start).abs(), b <= v.lo + 1e-6 || b >= v.hi - 1e-6))
            .collect();
        if d.is_empty() {
            continue;
        }
        let max = d.iter().map(|x| x.0).fold(0.0, f64::max);
        let mean = d.iter().map(|x| x.0).sum::<f64>() / d.len() as f64;
        let unit = if c.starts_with('r') { "°" } else { " u" };
        println!("  {c:<4} {max:>6.3}{unit} {mean:>6.3}{unit}  {}/{}", d.iter().filter(|x| x.1).count(), d.len());
    }
}
