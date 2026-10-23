//! Research tool for the cost-function analysis (not part of the gecko repo).
//! features: unit feature vectors per layout; tables: per-metric cost tables + slot info;
//! opt: SA under many weight vectors, best layout + its features.
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{anyhow, Result};
use gecko_core::metrics::{self, Dir, Order, METRICS};
use gecko_core::optimize::{self, Algo, Control, Params};
use gecko_core::report::breakdown;
use gecko_core::store::Store;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};
use serde_json::{json, Value};

fn arg(args: &[String], k: &str, d: &str) -> String {
    args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned().unwrap_or_else(|| d.to_string())
}

/// `p` with every scorable metric at weight 1, scale 1: breakdown's `weighted` is then the
/// unit contribution u_m (signed: Max metrics negative), so score(w) = Σ (w_m/scale_m)·u_m.
fn unit(p: &Profile) -> Profile {
    let mut u = p.clone();
    u.weights = METRICS.iter().filter(|d| d.dir != Dir::Info).map(|d| (d.id.to_string(), 1.0)).collect();
    u.scale = BTreeMap::new();
    u
}

fn feats(g: &Geometry, c: &Corpus, u: &Profile, p: &Profile, l: &Layout) -> Result<Value> {
    let sc = Scorer::new(g, c, u, l)?;
    let st = sc.identity();
    let b = breakdown(&sc, &st);
    let mut f = serde_json::Map::new();
    let mut raw = serde_json::Map::new();
    for m in &b.metrics {
        if m.dir != Dir::Info {
            f.insert(m.id.into(), json!(m.weighted));
        }
        raw.insert(m.id.into(), json!(m.value));
    }
    let scp = Scorer::new(g, c, p, l)?;
    let score = scp.full(&scp.identity());
    Ok(json!({"name": l.name, "chars": l.chars.iter().collect::<String>(), "score": score, "u": f, "raw": raw}))
}

fn load(args: &[String]) -> Result<(Store, Geometry, Corpus, Profile)> {
    let st = Store::open(Path::new(&arg(args, "--root", "/home/josh/projects/gecko")))?;
    let g = st.geometry(&arg(args, "-g", "ansi-30"))?;
    let c = st.corpus(&arg(args, "-c", "english"))?;
    let pn = arg(args, "-p", "default");
    let p = if pn.ends_with(".toml") { Profile::from_toml(&std::fs::read_to_string(&pn)?)? } else { st.profile(&pn)? };
    Ok((st, g, c, p))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).ok_or_else(|| anyhow!("usage: costlab features|tables|opt ..."))?.clone();
    let (st, g, c, p) = load(&args)?;
    let out = std::io::stdout();
    let mut out = out.lock();
    match cmd.as_str() {
        // stdin: one layout spec per line (reference name, cmini:<name>, raw:..., run:<id>)
        "features" => {
            let u = unit(&p);
            for line in std::io::stdin().lock().lines() {
                let spec = line?;
                let spec = spec.trim();
                if spec.is_empty() {
                    continue;
                }
                let r = st.layout(spec, &g.name).and_then(|l| l.fit(&g)).map_err(anyhow::Error::from).and_then(|mut l| {
                    l.name = spec.to_string();
                    feats(&g, &c, &u, &p, &l)
                });
                match r {
                    Ok(v) => writeln!(out, "{v}")?,
                    Err(e) => eprintln!("skip {spec}: {e}"),
                }
            }
        }
        "tables" => {
            let slots: Vec<Value> = g
                .key
                .iter()
                .map(|k| json!({"finger": k.finger.index(), "row": k.row, "col": k.col, "x": k.pos[0], "y": k.pos[1], "home": k.home, "effort": k.effort}))
                .collect();
            let mut t = serde_json::Map::new();
            for d in METRICS {
                if matches!(d.order, Order::Mono | Order::Bi | Order::Skip | Order::SkipFar) {
                    if let Some(v) = metrics::table(&g, &p, d) {
                        t.insert(d.id.into(), json!({"order": d.order, "dir": d.dir, "v": v}));
                    }
                }
            }
            writeln!(out, "{}", json!({"slots": slots, "tables": t}))?;
        }
        // stdin: JSONL {"id": .., "weights": {..}} (spread units; scales from -p). Starts from qwerty.
        "opt" => {
            let evals: u64 = arg(&args, "--evals", "20000000").parse()?;
            let seeds: u64 = arg(&args, "--seeds", "1").parse()?;
            let threads: usize = arg(&args, "--threads", "20").parse()?;
            let u = unit(&p);
            let start = st.layout("qwerty", &g.name)?.fit(&g)?;
            for line in std::io::stdin().lock().lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let v: Value = serde_json::from_str(&line)?;
                let mut q = p.clone();
                if let Some(w) = v.get("weights").and_then(|w| w.as_object()) {
                    q.weights = w.iter().map(|(k, x)| (k.clone(), x.as_f64().unwrap_or(0.0) as f32)).collect();
                }
                for s in 0..seeds {
                    let sc = Scorer::new(&g, &c, &q, &start)?;
                    let ctx = optimize::Ctx::new(&sc, (0..sc.n).collect(), sc.identity());
                    let params = Params { algo: Algo::Sa, seed: 1 + s, threads, time_ms: None, evals: Some(evals), top: 1, ..Default::default() };
                    let o = optimize::run(&ctx, &params, std::time::Duration::from_secs(3600), &Control::default(), |_| {});
                    let best = &o.top[0];
                    let st2 = sc.state_from_sym(best.sym.clone());
                    let mut l = sc.layout_of(&st2, &format!("{}#{}", v["id"].as_str().unwrap_or("?"), s))?;
                    l.geometry = g.name.clone();
                    let mut f = feats(&g, &c, &u, &q, &l)?;
                    f["id"] = v["id"].clone();
                    f["seed"] = json!(1 + s);
                    f["opt_score"] = json!(best.score);
                    writeln!(out, "{f}")?;
                    out.flush()?;
                }
            }
        }
        _ => return Err(anyhow!("unknown command {cmd}")),
    }
    Ok(())
}
