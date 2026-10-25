//! Reference snapshots: score and weighted metric values of every reference layout under each
//! bundled profile on `english` (ansi-30). Any change to a score or metric shows up as a
//! snapshot diff and must be accepted with `cargo insta review`. See `context/testing.md`.

use std::fmt::Write;

use gecko_core::report::breakdown;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

const LAYOUTS: [&str; 10] = ["canary", "colemak", "colemak-dh", "dvorak", "graphite", "halmak", "qwerty", "semimak", "sturdy", "workman"];

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn ansi() -> Geometry {
    let mut g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    for (metric, table) in g.comfort.clone() {
        g.load_comfort(&metric, &read(&format!("data/comfort/{table}.csv"))).unwrap();
    }
    g
}

/// One block per layout: total score, then each weighted metric's value and contribution.
/// Rounded so the snapshot is stable across platforms and summation order.
fn table(profile: &str) -> String {
    let g = ansi();
    let c = Corpus::from_bytes(&std::fs::read(format!("{ROOT}/data/corpora/english.gkc")).unwrap()).unwrap();
    let p = Profile::from_toml(&read(&format!("data/profiles/{profile}.toml"))).unwrap();
    let mut out = String::new();
    for name in LAYOUTS {
        let l = Layout::from_toml(&read(&format!("data/layouts/{name}.toml"))).unwrap();
        let sc = Scorer::new(&g, &c, &p, &l).unwrap();
        let b = breakdown(&sc, &sc.identity());
        writeln!(out, "{name}  score {:.4}", b.score).unwrap();
        for m in b.metrics.iter().filter(|m| m.weight != 0.0) {
            writeln!(out, "  {:<15} {:<28} {:>10.4} {:<4} x {:>6.3} = {:>10.4}", m.id, m.name, m.value, m.unit, m.weight, m.weighted).unwrap();
        }
    }
    out
}

#[test]
fn reference_default() {
    insta::assert_snapshot!(table("default"));
}

#[test]
fn reference_sfb() {
    insta::assert_snapshot!(table("sfb"));
}

#[test]
fn reference_engram() {
    insta::assert_snapshot!(table("engram"));
}

#[test]
fn reference_code() {
    insta::assert_snapshot!(table("code"));
}
