//! `gecko geometry import-kle`: turn a keyboard-layout-editor.com file into a board TOML.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use gecko_core::SCHEMA;
use serde_json::json;

use crate::ctx::Ctx;

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
}

impl GeometryCmd {
    pub fn run(self, ctx: &Ctx) -> Result<()> {
        let GeometryCmd::ImportKle { file, out, force } = self;
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
}
