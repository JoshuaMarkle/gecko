//! Generate a dependency-free, deterministic index of the production UI for embed-web.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    println!("cargo:rerun-if-changed={}", dir.display());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            collect(&entry.path(), files)?;
        } else if ty.is_file() {
            files.push(entry.path());
        } else {
            return Err(std::io::Error::other(format!("unsupported web asset: {}", entry.path().display())));
        }
    }
    Ok(())
}

fn main() {
    if env::var_os("CARGO_FEATURE_EMBED_WEB").is_none() {
        return;
    }
    let dist = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../web/dist");
    assert!(
        dist.join("index.html").is_file(),
        "embed-web needs web/dist/index.html; run `cd web && npm ci && npm run wasm && npm run build` first"
    );
    let mut files = Vec::new();
    collect(&dist, &mut files).expect("read web/dist for embed-web");
    let mut assets: Vec<_> = files
        .into_iter()
        .map(|file| {
            let name = file.strip_prefix(&dist).unwrap().to_str().expect("UTF-8 web asset name").replace('\\', "/");
            (name, file)
        })
        .collect();
    // Lookup uses normalized URL names, whose order must also match on Windows.
    assets.sort_by(|a, b| a.0.cmp(&b.0));
    let mut source = String::from("pub const ASSETS: &[(&str, &[u8])] = &[\n");
    for (name, file) in assets {
        println!("cargo:rerun-if-changed={}", file.display());
        source += &format!("({name:?}, include_bytes!({:?})),\n", file.to_str().unwrap());
    }
    source += "];\n";
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("web_assets.rs");
    fs::write(output, source).expect("write embedded web index");
}
