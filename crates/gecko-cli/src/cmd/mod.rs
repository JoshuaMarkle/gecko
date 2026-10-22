//! One module per command group. Each owns its clap arguments and a `run` function;
//! this file only lists the commands and dispatches them.

mod bench;
mod build;
mod corpus;
mod explore;
mod geometry;
mod layouts;
mod list;
mod magic;
mod optimize;
mod profile;
mod score;
mod select;

use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;

use crate::ctx::Ctx;

#[derive(Subcommand)]
pub enum Cmd {
    /// Score one layout with a per-metric breakdown.
    Score(score::ScoreArgs),
    /// Rank several layouts side by side (all reference layouts when none are given).
    Compare(score::CompareArgs),
    /// Finger loads, heat, and the worst same-finger and stretch n-grams.
    Analyze(score::AnalyzeArgs),
    /// Search for better layouts. Always writes a run directory.
    Optimize(optimize::OptimizeArgs),
    /// Map the space of good layouts: landscape probe + breakout local search with exclusion
    /// restarts, archiving every local optimum. Writes an atlas run directory.
    Explore(explore::ExploreArgs),
    /// MAP-Elites quality-diversity map: a grid over behaviour axes (roll/alternation ratio,
    /// index load, hand balance, or any metric id), best layout per cell. Writes a qd run.
    Qd(explore::QdArgs),
    /// Landscape probe without searching: random-layout score distribution, correlation
    /// length, and where each reference layout sits among random layouts.
    Landscape(explore::LandscapeArgs),
    /// Read an explore atlas: summary, ranked optima, families and consensus. With -p, re-rank
    /// the archive under another profile (no new search).
    Atlas(explore::AtlasArgs),
    /// Rank-sum selection over many runs (Kühlmak): rank every layout of the runs per metric
    /// and sort by the sum of its ranks, a trade-off pick that needs no weights.
    Select(select::SelectArgs),
    /// Algorithm leaderboard: each algorithm over several seeds with the same budget.
    Bench(bench::BenchArgs),
    /// Re-run a saved --evals run from its config and check it reproduces best.json.
    Verify(bench::VerifyArgs),
    /// Single-thread scorer speed: full evaluations/s, swap deltas/s, scorer builds/s and
    /// one-key-moved rebuilds/s (`Scorer::set_geometry`).
    Perf(bench::PerfArgs),
    /// Search the rules of an adaptive (magic) key on a fixed layout: greedily add the
    /// `previous → output` rule that lowers the score most, then drop rules that stopped paying.
    Magic(magic::MagicArgs),
    /// Corpus tools.
    Corpus {
        #[command(subcommand)]
        cmd: corpus::CorpusCmd,
    },
    /// Geometry tools: KLE import, board + layout optimization.
    #[command(alias = "geom")]
    Geometry {
        #[command(subcommand)]
        cmd: geometry::GeometryCmd,
    },
    /// Physical output: plate and case STL.
    Build {
        #[command(subcommand)]
        cmd: build::BuildCmd,
    },
    /// Import layouts from other analyzers, export them back.
    Layouts {
        #[command(subcommand)]
        cmd: layouts::LayoutsCmd,
    },
    /// Profile calibration tools.
    Profile {
        #[command(subcommand)]
        cmd: profile::ProfileCmd,
    },
    /// List layouts, geometries, profiles, corpora, runs or metrics.
    List { what: String },
    /// Serve the web UI and JSON API.
    Serve {
        #[arg(long, default_value_t = 7777)]
        port: u16,
        /// Override the web app directory (default: embedded UI, or <root>/web/dist without embed-web).
        #[arg(long)]
        web: Option<PathBuf>,
    },
}

impl Cmd {
    pub fn run(self, ctx: Ctx) -> Result<()> {
        match self {
            Cmd::Score(a) => score::score(&ctx, a),
            Cmd::Compare(a) => score::compare(&ctx, a),
            Cmd::Analyze(a) => score::analyze(&ctx, a),
            Cmd::Optimize(a) => optimize::run(&ctx, a),
            Cmd::Explore(a) => explore::explore(&ctx, a),
            Cmd::Qd(a) => explore::qd(&ctx, a),
            Cmd::Landscape(a) => explore::landscape(&ctx, a),
            Cmd::Atlas(a) => explore::atlas(&ctx, a),
            Cmd::Select(a) => select::run(&ctx, a),
            Cmd::Bench(a) => bench::bench(&ctx, a),
            Cmd::Verify(a) => bench::verify(&ctx, a),
            Cmd::Perf(a) => bench::perf(&ctx, a),
            Cmd::Magic(a) => magic::run(&ctx, a),
            Cmd::Corpus { cmd } => cmd.run(&ctx),
            Cmd::Geometry { cmd } => cmd.run(&ctx),
            Cmd::Build { cmd } => cmd.run(&ctx),
            Cmd::Layouts { cmd } => cmd.run(&ctx),
            Cmd::Profile { cmd } => cmd.run(&ctx),
            Cmd::List { what } => list::run(&ctx, &what),
            Cmd::Serve { port, web } => crate::serve::serve(ctx.store, port, web),
        }
    }
}
