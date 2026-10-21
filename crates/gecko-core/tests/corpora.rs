//! Bundled corpora load and reproduce known numbers. See `context/representation.md`.

use gecko_core::report::breakdown;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn qwerty_sfb(corpus: &str) -> f64 {
    let c = Corpus::from_bytes(&std::fs::read(format!("{ROOT}/data/corpora/{corpus}.gkc")).unwrap()).unwrap();
    let g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let sc = Scorer::new(&g, &c, &p, &Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap()).unwrap();
    breakdown(&sc, &sc.identity()).metrics.iter().find(|m| m.id == "B01").unwrap().value
}

/// QWERTY SFB computed independently from cmini's raw bigram tables (folded to the charset):
/// shai 6.70 %, Monkeytype 10k 8.40 %, 450k 8.30 %, reddit 6.28 %, French (diacritics folded) 8.81 %.
#[test]
#[allow(clippy::approx_constant)] // 6.28 is reddit's SFB, not τ
fn imported_corpora_match_their_sources() {
    for (name, want) in [("shai", 6.70), ("mt-10k", 8.40), ("mt-450k", 8.30), ("reddit", 6.28), ("french", 8.81)] {
        let got = qwerty_sfb(name);
        assert!((got - want).abs() < 0.01, "{name}: QWERTY SFB {got:.3}%, expected {want}%");
    }
}

#[test]
fn every_bundled_corpus_has_skipgrams_of_each_gap() {
    for name in ["english", "shai", "mt-10k", "mt-450k", "reddit", "french"] {
        let c = Corpus::from_bytes(&std::fs::read(format!("{ROOT}/data/corpora/{name}.gkc")).unwrap()).unwrap();
        for (g, t) in [(1, &c.skip), (2, &c.skip2), (3, &c.skip3)] {
            assert!(t.iter().sum::<u64>() > 0, "{name}: skip-{g} is empty");
        }
    }
}
