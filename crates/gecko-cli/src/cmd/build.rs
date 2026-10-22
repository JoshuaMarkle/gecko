//! `gecko build`: physical output. `stl` writes the plate, case and bottom STL of a board
//! (`geometry/case.rs`, `context/geometry.md`).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use gecko_core::geometry::case::{CaseParams, Switch};
use gecko_core::geometry::manufacture::BuildLimits;
use gecko_core::SCHEMA;
use serde_json::json;

use crate::ctx::Ctx;

#[derive(Subcommand)]
pub enum BuildCmd {
    /// Write plate.stl, case.stl (plate + walls), bottom.stl and build.json for a board:
    /// `run:<id>` (an optimized board), a geometry name or a path.
    Stl(StlArgs),
}

#[derive(clap::Args)]
pub struct StlArgs {
    /// Board: run:<id>, a geometry name or path (default from gecko.toml).
    geometry: Option<String>,
    /// Output directory (default: runs/<id>/stl for a run, else build/<geometry>/).
    #[arg(long, short)]
    out: Option<PathBuf>,
    #[arg(long, default_value = "mx", value_parser = ["mx", "choc"])]
    switch: String,
    /// Plate thickness, mm.
    #[arg(long)]
    plate: Option<f64>,
    /// Wall thickness, mm.
    #[arg(long)]
    wall: Option<f64>,
    /// Floor to the lowest switch bottom, mm.
    #[arg(long)]
    clearance: Option<f64>,
    /// Largest gap between key edges a web spans, mm.
    #[arg(long)]
    bridge: Option<f64>,
    /// Profile whose [build] limits are checked (and whose piece_gap splits the halves).
    #[arg(long, short)]
    profile: Option<String>,
}

impl BuildCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        match self {
            BuildCmd::Stl(a) => stl(ctx, a),
        }
    }
}

fn stl(ctx: &Ctx, a: StlArgs) -> Result<()> {
    let spec = a.geometry.clone().unwrap_or_else(|| ctx.store.defaults.geometry.clone());
    let g = ctx.store.geometry(&spec)?;
    let limits = match &a.profile {
        Some(p) => ctx.store.profile(p)?.build.unwrap_or_default(),
        None => BuildLimits::default(),
    };
    let mut p = CaseParams::for_switch(if a.switch == "choc" { Switch::Choc } else { Switch::Mx });
    for (field, v) in [(&mut p.plate, a.plate), (&mut p.wall, a.wall), (&mut p.clearance, a.clearance), (&mut p.bridge, a.bridge)] {
        if let Some(v) = v {
            *field = v;
        }
    }
    let case = g.case(&p, limits.piece_gap)?;
    let manufacture = g.manufacture(&limits);
    let dir = a.out.unwrap_or_else(|| match spec.strip_prefix("run:") {
        Some(id) => ctx.store.runs.join(id).join("stl"),
        None => ctx.store.root.join("build").join(&g.name),
    });
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let header = format!("gecko {} {}", g.name, gecko_core::hash::hex(g.hash()));
    let mut files = Vec::new();
    for (name, mesh) in [("plate.stl", case.plate.clone()), ("case.stl", case.case()), ("bottom.stl", case.bottom.clone())] {
        let path = dir.join(name);
        let bytes = mesh.stl(&header);
        std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
        let b = mesh.bounds();
        files.push(json!({"file": name, "shells": mesh.shells.len(), "triangles": mesh.triangles(), "bytes": bytes.len(),
            "size_mm": [b[1][0] - b[0][0], b[1][1] - b[0][1], b[1][2] - b[0][2]]}));
    }
    let out = json!({"schema": SCHEMA, "geometry": g.name, "source": spec, "hash": gecko_core::hash::hex(g.hash()),
        "gecko_version": env!("CARGO_PKG_VERSION"), "dir": dir, "params": p, "files": files, "report": case.report,
        "manufacture": manufacture});
    std::fs::write(dir.join("build.json"), serde_json::to_string_pretty(&out)?)?;
    if ctx.json {
        ctx.print(out);
        return Ok(());
    }
    let r = &case.report;
    println!("{} → {}", g.name, dir.display());
    for f in &files {
        let s = &f["size_mm"];
        println!(
            "  {:<11} {:>4} shells {:>6} triangles  {:.0} × {:.0} × {:.0} mm",
            f["file"].as_str().unwrap(),
            f["shells"],
            f["triangles"],
            s[0].as_f64().unwrap(),
            s[1].as_f64().unwrap(),
            s[2].as_f64().unwrap()
        );
    }
    println!(
        "  {} keys, {} piece(s); {} webs, {} junctions, {} walls, {} corners; lifted {:.1} mm",
        r.keys,
        r.pieces.len(),
        r.webs,
        r.junctions,
        r.walls,
        r.corners,
        r.lift
    );
    if r.dropped > 0 {
        println!("  dropped {} connector(s) that would block a switch", r.dropped);
    }
    if !r.blocked.is_empty() {
        println!("  warning: switch holes blocked at keys {:?}", r.blocked);
    }
    if !r.cap_hits.is_empty() {
        println!("  warning: keycaps hit the case within their travel at keys {:?}", r.cap_hits);
    }
    for v in &manufacture.violations {
        println!("  build limit: {} {:.2} > {:.2} (keys {:?})", v.kind, v.value, v.limit, v.keys);
    }
    Ok(())
}
