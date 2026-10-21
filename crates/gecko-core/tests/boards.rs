//! Bundled physical-layout provenance, alpha-core fitting and scoring on larger split boards.

use std::collections::BTreeSet;
use std::path::Path;

use gecko_core::store::Store;
use gecko_core::{kle, Finger, Geometry, Layout, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const BOARDS: [(&str, usize, usize); 6] =
    [("corne-36", 36, 6), ("sweep-34", 34, 4), ("lily58", 58, 10), ("kyria", 50, 14), ("ergodox", 76, 12), ("dactyl-manuform-5x6", 64, 12)];

fn geometry(name: &str) -> Geometry {
    Store::open(Path::new(ROOT)).unwrap().geometry(name).unwrap()
}

fn close(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-4)
}

#[test]
fn bundled_splits_match_pinned_kle_sources() {
    for (name, count, _) in BOARDS {
        let g = geometry(name);
        let source = std::fs::read_to_string(format!("{ROOT}/data/geometries/sources/{name}.kle.json")).unwrap();
        let (_, physical) = kle::parse(&source).unwrap();
        assert_eq!(physical.len(), count, "{name}");
        assert_eq!(g.len(), count, "{name}");
        // Match by center because reviewed row/finger annotations intentionally change import order.
        for k in physical {
            let center = k.center();
            let matches: Vec<_> =
                g.key.iter().filter(|p| close(&[p.pos[0] + p.size[0] / 2.0, p.pos[1] + p.size[1] / 2.0], &center)).collect();
            assert_eq!(matches.len(), 1, "{name}: source key {}", k.label);
            let p = matches[0];
            assert!(close(&p.size, &[k.w, k.h]), "{name}: size {}", k.label);
            assert!(close(&p.rot, &[0.0, 0.0, k.r]), "{name}: rotation {}", k.label);
        }
        let back = Geometry::from_toml(&g.to_toml()).unwrap();
        assert_eq!(back.hash(), g.hash(), "{name}: TOML round trip");
        let exported = serde_json::to_string(&kle::export(&g)).unwrap();
        let (_, back) = kle::parse(&exported).unwrap();
        assert_eq!(back.len(), count);
    }
}

#[test]
fn bundled_splits_have_reviewed_alpha_cores_and_fit_every_reference() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let expected = [Finger::LP, Finger::LR, Finger::LM, Finger::LI, Finger::LI, Finger::RI, Finger::RI, Finger::RM, Finger::RR, Finger::RP];
    for (name, _, thumbs) in BOARDS {
        let g = geometry(name);
        assert_eq!(g.key.iter().filter(|k| k.finger.is_thumb()).count(), thumbs, "{name}");
        let mut slots = BTreeSet::new();
        for k in &g.key {
            assert!(slots.insert((k.row, k.col)), "{name}: duplicate row/col");
            assert!(k.pos.iter().chain(&k.rot).chain(&k.size).all(|n| n.is_finite()));
            assert!(k.effort.is_finite() && k.effort > 0.0);
            assert!(k.fill.is_none() || k.fill_symbol().is_some());
        }
        for row in 0..3 {
            let alpha: Vec<_> = g.key.iter().filter(|k| k.row == row && k.fill.is_none()).collect();
            assert_eq!(alpha.len(), 10, "{name}: row {row}");
            for (col, (k, finger)) in alpha.into_iter().zip(expected).enumerate() {
                assert_eq!(k.finger, finger, "{name}: row {row} alpha {col}");
                assert_eq!(k.home, row == 1);
                assert_eq!(k.stretch, matches!(col, 4 | 5));
            }
        }
        assert_eq!(g.key.iter().filter(|k| k.fill.is_none()).count(), 30);
        for reference in store.list("layouts") {
            let layout = store.layout(&reference, name).unwrap().fit(&g).unwrap();
            assert_eq!(layout.chars.len(), g.len(), "{name}: {reference}");
            layout.check_geometry(&g).unwrap();
        }
    }
}

#[test]
fn corne_and_sweep_stagger_is_sourced_instead_of_rounded() {
    for (name, expected) in [("corne-36", [0.3, 0.1, 0.0, 0.1, 0.2]), ("sweep-34", [0.93, 0.31, 0.0, 0.28, 0.42])] {
        let g = geometry(name);
        for row in 0..3 {
            for (col, stagger) in expected.iter().enumerate() {
                assert!((g.key[row * 10 + col].pos[1] - row as f32 - stagger).abs() < 1e-4, "{name}: row {row} col {col}");
            }
        }
    }
}

#[test]
fn full_board_scores_and_deltas_include_empty_slots_and_thumbs() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let corpus = store.corpus("english").unwrap();
    let mut profile = store.profile("default").unwrap();
    for (i, metric) in gecko_core::metrics::METRICS.iter().enumerate() {
        profile.weights.insert(metric.id.to_owned(), 0.1 + i as f32 * 0.07);
    }
    for (name, _, _) in BOARDS {
        let g = geometry(name);
        let layout =
            Layout::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/layouts/qwerty.toml")).unwrap()).unwrap().fit(&g).unwrap();
        let scorer = Scorer::new(&g, &corpus, &profile, &layout).unwrap();
        let mut state = scorer.state_for(&layout).unwrap();
        assert!(state.score.is_finite(), "{name}");
        // Visit every symbol, including ErgoDox's symbols above the 64-bit boundary.
        for a in 0..g.len() {
            let b = (a + 17) % g.len();
            let before = scorer.full(&state);
            let delta = scorer.delta(&state, a, b);
            scorer.apply(&mut state, a, b, delta);
            let after = scorer.full(&state);
            assert!((after - before - delta).abs() < 1e-5 * before.abs().max(1.0), "{name}: swap {a}/{b}");
        }
    }
}
