//! `gecko explore`, `gecko landscape`, `gecko atlas`: map the space of good layouts, probe the
//! landscape, and read an atlas back (optionally re-ranked under another profile).
//! See `context/cli.md` and `context/algorithms/explore.md`.

use anyhow::{bail, Result};
use gecko_core::explore::atlas::{self, Summary};
use gecko_core::explore::{distance, probe, stats};
use gecko_core::optimize::{Algo, Control, Ctx as SearchCtx, Params};
use gecko_core::run::{self, RunSpec};
use gecko_core::{Scorer, SCHEMA};
use serde_json::json;

use super::{parse_duration, Ctx, Inputs};

pub struct ExploreArgs {
    pub time: Option<String>,
    pub evals: Option<u64>,
    pub seed: u64,
    pub threads: Option<usize>,
    pub pin: String,
    pub from: Option<String>,
    pub samples: Option<usize>,
    pub max_optima: Option<usize>,
    pub inputs: Inputs,
}

fn cores() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

pub fn explore(ctx: &Ctx, a: ExploreArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let start = ctx.layout(a.from.as_deref().unwrap_or(&ctx.store.defaults.reference), &l)?;
    let mut params = Params {
        algo: Algo::Explore,
        seed: a.seed,
        threads: a.threads.unwrap_or_else(cores),
        time_ms: if a.evals.is_some() { None } else { Some(parse_duration(a.time.as_deref().unwrap_or("60s"))?) },
        evals: a.evals,
        top: 10,
        ..Default::default()
    };
    if let Some(s) = a.samples {
        params.explore.samples = s;
    }
    if let Some(m) = a.max_optima {
        params.explore.max_optima = m;
    }
    let spec = RunSpec { geometry: &l.geometry, corpus: &l.corpus, profile: &l.profile, start: &start, pins: &a.pin, params };
    let prep = run::prepare(&ctx.store.runs, &spec)?;
    eprintln!("explore {}  ({} threads · {})", prep.config.id, spec.params.threads, l.profile.name);
    let quiet = ctx.json;
    run::execute(&prep, &spec, &Control::default(), |p| {
        if !quiet {
            let e = p.explore.clone().unwrap_or_default();
            eprint!(
                "\r  {:>6.1}s  {:<6} {:>8} optima  {:>9} hits  best {:.4}   ",
                p.elapsed_ms as f64 / 1000.0,
                e.phase,
                e.optima,
                e.hits,
                p.best
            );
        }
    })?;
    if !quiet {
        eprintln!();
    }
    let (summary, _) = atlas::load(&prep.dir, Some(0))?;
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "run": prep.config.id, "atlas": summary}));
    } else {
        print_summary(&summary, &l.geometry);
    }
    Ok(())
}

/// Slot-ordered symbols split into the geometry's rows (symbols may repeat).
fn rows(chars: &str, g: &gecko_core::Geometry) -> String {
    let mut it = chars.chars();
    g.row_lengths().iter().map(|&n| it.by_ref().take(n).collect::<String>()).collect::<Vec<_>>().join(" ")
}

fn print_summary(s: &Summary, g: &gecko_core::Geometry) {
    println!("atlas {}  {} evals in {:.1}s", s.id, s.evals, s.elapsed_ms as f64 / 1000.0);
    println!(
        "  optima {} (+{} constrained)  hits {}  edges {}  funnels {}{}",
        s.counts.optima - s.counts.constrained,
        s.counts.constrained,
        s.counts.hits,
        s.counts.edges,
        s.funnels,
        if s.symmetric { "  (mirror-deduplicated)" } else { "" }
    );
    println!(
        "  random layouts {:.2} ± {:.2} (min {:.2})  correlation length {:.1} swaps  FDC {:.3}",
        s.probe.random_mean, s.probe.random_sd, s.probe.random_min, s.probe.corr_length, s.fdc
    );
    println!(
        "  within {:.1}% of best: {} optima seen, ~{:.0} estimated (Chao1)",
        100.0 * s.coverage.within,
        s.coverage.observed,
        s.coverage.estimate
    );
    println!("\ndistinct best layouts (≥ {} slots apart)", s.distinct_dist);
    for (i, b) in s.distinct.iter().enumerate() {
        println!("#{:<2} {:>8.3}  d={:<2}  {}", i + 1, b.score, b.d_best, rows(&b.chars, g));
    }
}

pub fn landscape(ctx: &Ctx, inputs: &Inputs, from: Option<&str>, samples: usize, walks: usize, walk_len: usize, seed: u64) -> Result<()> {
    let l = ctx.load(inputs)?;
    let start = ctx.layout(from.unwrap_or(&ctx.store.defaults.reference), &l)?;
    let sc = Scorer::new(&l.geometry, &l.corpus, &l.profile, &start)?;
    let sctx = SearchCtx::new(&sc, (0..sc.n).collect(), sc.identity());
    let p = probe::run(&sctx, seed, cores(), samples, walks, walk_len, &Control::default());
    let (rho, ell) = stats::autocorrelation(&p.walks, 20);
    let mut scores: Vec<f64> = p.samples.iter().map(|s| s.score).collect();
    scores.sort_by(f64::total_cmp);
    let n = scores.len().max(1) as f64;
    let mean = scores.iter().sum::<f64>() / n;
    let sd = (scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n).sqrt();
    let q = |f: f64| scores.get(((scores.len() as f64 - 1.0) * f).round() as usize).copied().unwrap_or(f64::NAN);
    let mut refs = Vec::new();
    for name in ctx.store.list("layouts") {
        let Ok(lay) = ctx.layout(&name, &l) else { continue };
        let Ok(st) = sc.state_for(&lay) else { continue };
        // Share of random layouts that score better (lower).
        let pct = 100.0 * scores.partition_point(|&s| s < st.score) as f64 / n;
        refs.push((name, st.score, pct, (st.score - mean) / sd.max(1e-12)));
    }
    refs.sort_by(|a, b| a.1.total_cmp(&b.1));
    let symmetric = distance::score_symmetry(&sc).is_some();
    if ctx.json {
        let r: Vec<_> = refs.iter().map(|(n, s, p, z)| json!({"layout": n, "score": s, "percentile": p, "z": z})).collect();
        ctx.print(json!({"schema": SCHEMA, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name,
            "samples": scores.len(), "random": {"mean": mean, "sd": sd, "min": q(0.0), "p01": q(0.01), "median": q(0.5)},
            "walks": p.walks.len(), "walk_len": walk_len, "rho": rho, "corr_length": ell, "mirror_symmetric": symmetric, "references": r}));
    } else {
        println!("{} random layouts ({} · {} · {})", scores.len(), l.geometry.name, l.profile.name, l.corpus.name);
        println!("  score {mean:.3} ± {sd:.3}   min {:.3}  1% {:.3}  median {:.3}", q(0.0), q(0.01), q(0.5));
        println!("  correlation length {ell:.2} swaps  (ρ(1) {:.3}, ρ(5) {:.3}, {} walks × {walk_len})", rho[1], rho[5.min(rho.len() - 1)], p.walks.len());
        println!("  mirror-symmetric scores: {}", if symmetric { "yes" } else { "no" });
        println!("\n{:<14} {:>9} {:>11} {:>8}", "layout", "score", "% random <", "z");
        for (name, s, pct, z) in &refs {
            println!("{name:<14} {s:>9.3} {pct:>10.2}% {z:>8.2}");
        }
    }
    Ok(())
}

pub fn atlas_cmd(ctx: &Ctx, id: &str, inputs: &Inputs, top: usize, cut: usize, family_pool: usize) -> Result<()> {
    let dir = ctx.store.runs.join(id);
    let config = run::load_config(&dir)?;
    let (summary, at) = atlas::load(&dir, None)?;
    let g = ctx.store.geometry(&config.geometry.name)?;
    let profile = match &inputs.profile {
        Some(p) => ctx.store.profile(p)?,
        None => config.profile.clone(),
    };
    if profile.table_hash() != config.profile.table_hash() {
        eprintln!("warning: {} differs from the atlas profile in targets/sfb/trigram coverage; re-scoring uses its weights only", profile.name);
    }
    let w = stats::Weights::new(&profile, &at.metrics);
    let s = at.slots;
    let m = at.metrics.len();
    let raw: Vec<f32> = at.optima.iter().flat_map(|o| o.raw.iter().copied()).collect();
    let scores = w.score_all(&raw, m);
    let syms: Vec<u8> = at.optima.iter().flat_map(|o| o.sym.iter().copied()).collect();
    let free: Vec<u32> = (0..at.optima.len() as u32).filter(|&i| at.optima[i as usize].flags & atlas::CONSTRAINED == 0).collect();
    let mut order = free.clone();
    order.sort_by(|&a, &b| scores[a as usize].total_cmp(&scores[b as usize]));
    if order.is_empty() {
        bail!("atlas {id} has no optima yet");
    }
    let alphabet: Vec<char> = summary.chars.chars().collect();
    let chars = |i: u32| -> String { syms[i as usize * s..(i as usize + 1) * s].iter().map(|&x| alphabet[x as usize]).collect() };
    let best = order[0];
    let d = |i: u32| distance::hamming(&syms[i as usize * s..(i as usize + 1) * s], &syms[best as usize * s..(best as usize + 1) * s]);
    let pool: Vec<u32> = order.iter().copied().take(family_pool).collect();
    let fam = stats::families(&syms, s, &pool, cut);
    let nfam = fam.iter().max().map_or(0, |&f| f as usize + 1);
    let families: Vec<_> = (0..nfam)
        .map(|f| {
            let members: Vec<u32> = pool.iter().zip(&fam).filter(|(_, &x)| x as usize == f).map(|(&i, _)| i).collect();
            let lead = members[0];
            json!({"family": f + 1, "size": members.len(), "best": scores[lead as usize], "worst": scores[*members.last().unwrap() as usize],
                "d_best": d(lead), "chars": chars(lead)})
        })
        .collect();
    let cons_rows: Vec<u32> = order.iter().copied().take(top.max(1) * 10).collect();
    let cons = stats::consensus(&syms, s, &cons_rows);
    let ent = stats::slot_entropy(&cons, s);
    let modal: Vec<(char, f32)> = (0..s)
        .map(|slot| {
            let row = &cons[slot * s..(slot + 1) * s];
            let (sym, &p) = row.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
            (alphabet[sym], p)
        })
        .collect();
    let pairs: Vec<(u32, u32)> = at.edges.iter().map(|e| (e.from, e.to)).collect();
    let sinks = stats::funnels(&scores, &pairs);
    let mut bottoms: Vec<u32> = free.iter().map(|&i| sinks[i as usize]).collect();
    bottoms.sort_unstable();
    bottoms.dedup();
    let fdc = stats::fdc(&syms, s, &scores, &free);
    if ctx.json {
        let ranked: Vec<_> = order
            .iter()
            .take(top)
            .enumerate()
            .map(|(r, &i)| json!({"rank": r + 1, "score": scores[i as usize], "hits": at.optima[i as usize].hits, "d_best": d(i), "chars": chars(i)}))
            .collect();
        ctx.print(json!({"schema": SCHEMA, "atlas": summary, "profile": profile.name, "fdc": fdc, "funnels": bottoms.len(),
            "ranked": ranked, "families": families, "family_cut": cut,
            "consensus": {"layouts": cons_rows.len(), "modal": modal.iter().map(|x| x.0).collect::<String>(),
                "share": modal.iter().map(|x| x.1).collect::<Vec<_>>(), "entropy": ent}}));
        return Ok(());
    }
    print_summary(&summary, &g);
    println!("\nranked under {} (FDC {fdc:.3}, {} funnels)", profile.name, bottoms.len());
    println!("{:<4} {:>8} {:>6} {:>4}  layout", "#", "score", "hits", "d");
    for (r, &i) in order.iter().take(top).enumerate() {
        println!("{:<4} {:>8.3} {:>6} {:>4}  {}", r + 1, scores[i as usize], at.optima[i as usize].hits, d(i), rows(&chars(i), &g));
    }
    println!("\nfamilies among the best {} (leader clustering, ≤ {cut} slots from the leader)", pool.len());
    for f in families.iter().take(12) {
        println!(
            "F{:<3} {:>4} layouts  {:>8.3} – {:<8.3} d={:<3} {}",
            f["family"],
            f["size"],
            f["best"].as_f64().unwrap(),
            f["worst"].as_f64().unwrap(),
            f["d_best"],
            rows(f["chars"].as_str().unwrap(), &g)
        );
    }
    println!("\nconsensus of the best {} (modal symbol; digit = share in tenths, · = all agree)", cons_rows.len());
    let modal_chars: String = modal.iter().map(|x| x.0).collect();
    let shares: String = modal.iter().map(|x| if x.1 >= 0.999 { '·' } else { char::from(b'0' + (x.1 * 10.0).floor().min(9.0) as u8) }).collect();
    let (a, b) = (rows(&modal_chars, &g), rows(&shares, &g));
    for (x, y) in a.split(' ').zip(b.split(' ')) {
        println!("  {}    {}", x.chars().map(|c| format!("{c} ")).collect::<String>(), y.chars().map(|c| format!("{c} ")).collect::<String>());
    }
    Ok(())
}
