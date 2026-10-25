//! Bundled corpora load and reproduce known numbers. See `context/representation.md`.

use gecko_core::report::breakdown;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

/// A bundled corpus, current or retired (`data/corpora/legacy/`).
fn load(name: &str) -> Corpus {
    let p = format!("{ROOT}/data/corpora/{name}.gkc");
    let p = if std::path::Path::new(&p).exists() { p } else { format!("{ROOT}/data/corpora/legacy/{name}.gkc") };
    Corpus::from_bytes(&std::fs::read(p).unwrap()).unwrap()
}

fn qwerty_sfb(corpus: &str) -> f64 {
    let c = load(corpus);
    let g = Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    let sc = Scorer::new(&g, &c, &p, &Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap()).unwrap();
    breakdown(&sc, &sc.identity()).metrics.iter().find(|m| m.id == "sfb").unwrap().value
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
        let c = load(name);
        for (g, t) in [(1, &c.skip), (2, &c.skip2), (3, &c.skip3)] {
            assert!(t.iter().sum::<u64>() > 0, "{name}: skip-{g} is empty");
        }
    }
}

/// Adding navigation/adaptive symbols must retain case data introduced by the Shift PR.
#[test]
fn corpus_transforms_preserve_shift_counts() {
    use gecko_core::magic::{self, Magic, MAGIC};
    let mut c = Corpus::new("cased", "ab");
    c.add_text("aB Ab aBB ab");
    assert!(c.has_shift());
    let extended = c.extended("nav", &gecko_core::layout::ARROWS).unwrap();
    let inert_key = Magic { key: MAGIC, ..Default::default() };
    let rewritten = magic::rewrite(&c, &[inert_key]).unwrap();
    for (got, scale) in [(&extended, 1), (&rewritten, magic::SCALE as u64)] {
        for x in 0..c.size() {
            assert_eq!(got.shift[x], c.shift[x] * scale);
            for y in 0..c.size() {
                assert_eq!(got.shift_bi[x * got.size() + y], c.shift_bi[x * c.size() + y] * scale);
            }
        }
        let back = Corpus::from_bytes(&got.to_bytes()).unwrap();
        assert_eq!(back.shift, got.shift);
        assert_eq!(back.shift_bi, got.shift_bi);
    }
}

/// Every language recipe rebuilds to the committed `.gkc` (rebuild with `gecko corpus mix --all`
/// after changing a part or the mixing code).
#[test]
fn recipes_match_their_built_corpora() {
    let store = gecko_core::store::Store::open(std::path::Path::new(ROOT)).unwrap();
    for name in store.list("corpora") {
        if store.recipe(&name).unwrap().is_none() {
            continue;
        }
        let built = store.build_recipe(&name).unwrap();
        let file = Corpus::from_bytes(&std::fs::read(store.corpus_path(&name)).unwrap()).unwrap();
        assert_eq!(built.hash(), file.hash(), "{name}: recipe and .gkc differ");
    }
}
