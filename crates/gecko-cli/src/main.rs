//! `gecko` — the agent-first CLI and `gecko serve`. See `context/cli.md`.
//!
//! Layout: `cmd/` holds one module per command group (its clap arguments and `run`), `ctx.rs` the
//! shared store/inputs context, `fmt.rs` the plain-text tables, `serve.rs` the HTTP API.

mod cmd;
mod ctx;
mod fmt;
mod serve;
mod web_assets;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use gecko_core::store::Store;

use crate::ctx::Ctx;

#[derive(Parser)]
#[command(name = "gecko", version, about = "Find the best keyboard layouts.")]
struct Cli {
    /// Repo root holding gecko.toml (default: search upward from the current directory).
    #[arg(long, global = true)]
    root: Option<PathBuf>,
    /// Print JSON (stable schema) instead of tables.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: cmd::Cmd,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = real_main(cli) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main(cli: Cli) -> Result<()> {
    let store = match &cli.root {
        Some(r) => Store::open(r)?,
        None => Store::discover(&std::env::current_dir()?, Some(&fallback_root()))?,
    };
    cli.cmd.run(Ctx { store, json: cli.json })
}

/// Where to find gecko.toml when no ancestor of the current directory has one: beside the
/// executable (portable release bundles), else the source checkout this binary was built from.
fn fallback_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .filter(|p| p.join("gecko.toml").is_file())
        .unwrap_or_else(|| PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")))
}
