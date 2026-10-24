//! `gecko score`, `gecko compare` (with the `--corpora` robustness report), `gecko analyze`.

use anyhow::Result;
use gecko_core::report::{analyze as analyze_layout, breakdown, worst_words};
use gecko_core::robust::{self, UnitCosts};
use gecko_core::{Layout, SCHEMA};
use serde_json::json;

use crate::ctx::{score_layout, Ctx, Inputs, Loaded};
use crate::fmt::{fmt_val, layout_rows, print_breakdown};

#[derive(clap::Args)]
pub struct ScoreArgs {
    /// Layout: reference name, .toml path, run:<id>[#rank], or raw:<chars>.
    layout: String,
    #[command(flatten)]
    inputs: Inputs,
}

#[derive(clap::Args)]
pub struct CompareArgs {
    layouts: Vec<String>,
    /// Robustness report: score and rank on each of these corpora (comma-separated).
    #[arg(long, value_delimiter = ',')]
    corpora: Vec<String>,
    /// Rank confidence: how often each layout keeps its rank when every weight is multiplied by
    /// a log-normal factor (see --sigma, --draws).
    #[arg(long)]
    robust: bool,
    /// --robust: log-normal σ of the weight noise.
    #[arg(long, default_value_t = 0.5)]
    sigma: f64,
    /// --robust: number of weight draws.
    #[arg(long, default_value_t = 2000)]
    draws: usize,
    /// --robust: random seed.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    #[command(flatten)]
    inputs: Inputs,
}

#[derive(clap::Args)]
pub struct AnalyzeArgs {
    layout: String,
    #[arg(long, default_value_t = 10)]
    top: usize,
    /// Also list the worst real words per metric (frequency × cost); needs a word-import
    /// corpus (`<corpus>.words.tsv`), e.g. `-c shai`.
    #[arg(long)]
    words: bool,
    #[command(flatten)]
    inputs: Inputs,
}

pub fn score(ctx: &Ctx, a: ScoreArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let layout = ctx.layout(&a.layout, &l)?;
    let (sc, st) = score_layout(&l, &layout)?;
    let b = breakdown(&sc, &st);
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry), "magic": layout.magic,
            "shifted": layout.shifted, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name, "breakdown": b}));
    } else {
        println!(
            "{}  ({} · {} · {})\n  {}\n",
            layout.name,
            l.geometry.name,
            l.profile.name,
            l.corpus.name,
            layout_rows(&layout, &l.geometry)
        );
        print_breakdown(&b);
    }
    Ok(())
}

/// The named layouts, or every reference layout when none are named.
fn layouts_or_all(ctx: &Ctx, names: Vec<String>) -> Vec<String> {
    if names.is_empty() {
        ctx.store.list("layouts")
    } else {
        names
    }
}

pub fn compare(ctx: &Ctx, a: CompareArgs) -> Result<()> {
    if a.robust && !a.corpora.is_empty() {
        anyhow::bail!("--robust and --corpora are separate reports; use one");
    }
    if !a.corpora.is_empty() {
        return robustness(ctx, a);
    }
    if a.robust {
        return rank_confidence(ctx, a);
    }
    let l = ctx.load(&a.inputs)?;
    let mut rows = Vec::new();
    for n in &layouts_or_all(ctx, a.layouts) {
        let layout = ctx.layout(n, &l)?;
        let (sc, st) = score_layout(&l, &layout)?;
        rows.push((layout, breakdown(&sc, &st)));
    }
    rows.sort_by(|a, b| a.1.score.total_cmp(&b.1.score));
    if ctx.json {
        let v: Vec<_> =
            rows.iter().map(|(lay, b)| json!({"layout": lay.name, "rows": lay.all_rows(&l.geometry), "breakdown": b})).collect();
        ctx.print(json!({"schema": SCHEMA, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name, "layouts": v}));
        return Ok(());
    }
    let cols = ["B01", "S01", "B04", "B05", "M01", "T03", "T06", "M04"];
    print!("{:<4} {:<14} {:>8}", "#", "layout", "score");
    for c in cols {
        let d = gecko_core::metrics::def(c).unwrap();
        print!(" {:>8}", d.name.chars().take(8).collect::<String>());
    }
    println!();
    for (i, (lay, b)) in rows.iter().enumerate() {
        print!("{:<4} {:<14} {:>8.3}", i + 1, lay.name, b.score);
        for c in cols {
            let m = b.metrics.iter().find(|m| m.id == c).unwrap();
            print!(" {:>8}", fmt_val(m.value));
        }
        println!();
    }
    Ok(())
}

/// `compare --robust`: rank confidence under log-normal weight noise.
fn rank_confidence(ctx: &Ctx, a: CompareArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let names = layouts_or_all(ctx, a.layouts);
    let mut unit = UnitCosts::new(&l.geometry, &l.corpus, &l.profile);
    let costs = names.iter().map(|n| Ok(unit.of(&ctx.layout(n, &l)?)?)).collect::<Result<Vec<_>>>()?;
    let rows = robust::rank_confidence(&names, &costs, &l.profile, a.sigma, a.draws, a.seed);
    if ctx.json {
        ctx.print(json!({"schema": SCHEMA, "geometry": l.geometry.name, "profile": l.profile.name, "corpus": l.corpus.name,
            "sigma": a.sigma, "draws": a.draws, "seed": a.seed, "layouts": rows}));
        return Ok(());
    }
    println!("rank confidence: {} draws, every weight × exp(σ·z), σ = {}", a.draws, a.sigma);
    println!("{:<4} {:<16} {:>8} {:>9} {:>11} {:>10} {:>9}", "#", "layout", "score", "P(rank)", "P(>next)", "rank 5–95", "P(first)");
    for r in &rows {
        let next = r.p_beats_next.map_or("-".into(), |p| format!("{:.0}%", 100.0 * p));
        println!(
            "{:<4} {:<16} {:>8.3} {:>8.0}% {:>11} {:>10} {:>8.0}%",
            r.rank,
            r.layout,
            r.score,
            100.0 * r.p_rank,
            next,
            format!("{}–{}", r.rank_lo, r.rank_hi),
            100.0 * r.p_first
        );
    }
    println!("P(>next): share of draws in which the layout still beats the one ranked below it.");
    Ok(())
}

/// `compare --corpora`: each layout's score and rank per corpus, and how far its rank moves (L05).
fn robustness(ctx: &Ctx, a: CompareArgs) -> Result<()> {
    let corpora = &a.corpora;
    let base = ctx.load(&a.inputs)?;
    let layouts: Vec<Layout> = layouts_or_all(ctx, a.layouts).iter().map(|n| ctx.layout(n, &base)).collect::<Result<_>>()?;
    let nl = layouts.len();
    // score[c][i], sfb[c][i], rank[c][i] (1 = best)
    let mut score = vec![vec![0.0; nl]; corpora.len()];
    let mut sfb = score.clone();
    let mut rank = vec![vec![0usize; nl]; corpora.len()];
    for (ci, cn) in corpora.iter().enumerate() {
        let l = Loaded { geometry: base.geometry.clone(), profile: base.profile.clone(), corpus: ctx.store.corpus(cn)? };
        for (i, lay) in layouts.iter().enumerate() {
            let (sc, st) = score_layout(&l, lay)?;
            let b = breakdown(&sc, &st);
            score[ci][i] = b.score;
            sfb[ci][i] = b.metrics.iter().find(|m| m.id == "B01").map_or(0.0, |m| m.value);
        }
        let mut order: Vec<usize> = (0..nl).collect();
        order.sort_by(|&a, &b| score[ci][a].total_cmp(&score[ci][b]));
        for (r, &i) in order.iter().enumerate() {
            rank[ci][i] = r + 1;
        }
    }
    let nc = corpora.len() as f64;
    let mean_rank = |i: usize| rank.iter().map(|r| r[i] as f64).sum::<f64>() / nc;
    let rank_spread = |i: usize| rank.iter().map(|r| r[i]).max().unwrap() - rank.iter().map(|r| r[i]).min().unwrap();
    let mut order: Vec<usize> = (0..nl).collect();
    order.sort_by(|&a, &b| mean_rank(a).total_cmp(&mean_rank(b)).then(score[0][a].total_cmp(&score[0][b])));
    if ctx.json {
        let v: Vec<_> = order
            .iter()
            .map(|&i| {
                let per: serde_json::Map<String, serde_json::Value> = corpora
                    .iter()
                    .enumerate()
                    .map(|(c, n)| (n.clone(), json!({"score": score[c][i], "rank": rank[c][i], "sfb": sfb[c][i]})))
                    .collect();
                json!({"layout": layouts[i].name, "corpora": per, "mean_rank": mean_rank(i), "rank_spread": rank_spread(i)})
            })
            .collect();
        ctx.print(
            json!({"schema": SCHEMA, "geometry": base.geometry.name, "profile": base.profile.name, "corpora": corpora, "layouts": v}),
        );
        return Ok(());
    }
    println!("score (rank) [SFB %] per corpus, sorted by mean rank; spread = worst rank - best rank");
    print!("{:<14}", "layout");
    for c in corpora {
        print!(" {:>22}", c);
    }
    println!(" {:>9} {:>6}", "mean rank", "spread");
    for &i in &order {
        print!("{:<14}", layouts[i].name);
        for c in 0..corpora.len() {
            print!(" {:>22}", format!("{:.3} ({:>2}) [{:.2}]", score[c][i], rank[c][i], sfb[c][i]));
        }
        println!(" {:>9.1} {:>6}", mean_rank(i), rank_spread(i));
    }
    Ok(())
}

pub fn analyze(ctx: &Ctx, a: AnalyzeArgs) -> Result<()> {
    let l = ctx.load(&a.inputs)?;
    let layout = ctx.layout(&a.layout, &l)?;
    let (sc, st) = score_layout(&l, &layout)?;
    let an = analyze_layout(&sc, &st, a.top);
    let ww = if a.words { Some(worst_words(&sc, &st, &ctx.store.words(&l.corpus.name)?, a.top)) } else { None };
    if ctx.json {
        let mut v = json!({"schema": SCHEMA, "layout": layout.name, "rows": layout.all_rows(&l.geometry), "analysis": an});
        if let Some(ww) = &ww {
            v["worst_words"] = json!(ww);
        }
        ctx.print(v);
        return Ok(());
    }
    println!("{}\n  {}\n", layout.name, layout_rows(&layout, &l.geometry));
    print_breakdown(&an.breakdown);
    println!("\nfinger load %  (target)");
    for i in 0..10 {
        if an.finger_load[i] > 0.0 || an.finger_target[i] > 0.0 {
            println!(
                "  {:<3} {:>5.1}  ({:>4.1})  {}",
                an.fingers[i],
                an.finger_load[i],
                an.finger_target[i],
                "#".repeat((an.finger_load[i] * 2.0) as usize)
            );
        }
    }
    for (title, v) in [("SFB", &an.worst_sfb), ("SFS", &an.worst_sfs), ("LSB", &an.worst_lsb), ("scissors", &an.worst_scissors)] {
        let s: Vec<String> = v.iter().map(|n| format!("{} {:.2}", n.ngram, n.freq)).collect();
        println!("\nworst {title}: {}", s.join(" · "));
    }
    if let Some(ww) = &ww {
        println!("\nworst words  (word share%  — % of the metric's word cost)");
        for m in ww {
            let s: Vec<String> = m.words.iter().map(|w| format!("{} {:.1}", w.word, w.share)).collect();
            println!("  {:<16} {}", m.name, s.join(" · "));
        }
    }
    Ok(())
}
