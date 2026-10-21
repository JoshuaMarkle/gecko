//! Generate fresh CLI expectations and resolved WASM inputs for `npm run test:parity`.
use std::path::Path;
use std::process::Command;

use anyhow::{ensure, Context, Result};
use gecko_core::store::Store;
use serde_json::{json, Value};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(args.len() == 3, "usage: wasm_parity_fixture <repo-root> <gecko-binary>");
    let store = Store::open(Path::new(&args[1]))?;
    let geometry = "ansi-30";
    let corpus = "english";
    let references = store.list("layouts");
    ensure!(!references.is_empty(), "no reference layouts found");
    let reference_names = references.iter().map(|name| Ok(store.layout(name, geometry)?.name)).collect::<Result<Vec<_>>>()?;
    let mut cases = Vec::new();
    for profile in store.list("profiles") {
        let output = Command::new(&args[2])
            .arg("--root")
            .arg(&store.root)
            .args(["compare", "--json", "--geometry", geometry, "--corpus", corpus, "--profile", &profile])
            .output()
            .context("running gecko compare --json")?;
        ensure!(output.status.success(), "CLI compare failed: {}", String::from_utf8_lossy(&output.stderr));
        let expected: Value = serde_json::from_slice(&output.stdout).context("parsing CLI compare JSON")?;
        cases.push(json!({"profile": store.profile(&profile)?, "expected": expected}));
    }
    ensure!(!cases.is_empty(), "no profiles found");
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "geometry": store.geometry(geometry)?,
            "corpus_path": store.corpus_path(corpus),
            "references": reference_names,
            "cases": cases,
        }))?
    );
    Ok(())
}
