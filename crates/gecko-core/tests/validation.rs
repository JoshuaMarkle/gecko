//! Cross-checks against external analyzers and references (roadmap A7): trigram classes vs
//! cmini's `table.json`, pair distances vs the Keyboard Layouts Doc. See `context/testing.md`.

use std::collections::BTreeMap;

use gecko_core::metrics::trigram::{classify, TriClass};
use gecko_core::{Finger, Geometry, Layout};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn ansi() -> Geometry {
    Geometry::from_toml(&read("data/geometries/ansi-30.toml")).unwrap()
}

/// GECKO classes that a cmini class may map to. cmini splits same-finger redirects
/// (`dsfb-red`) out of redirects; GECKO keeps them in redirect/bad_redirect and counts the same-finger skip
/// in sfs (decision 2026-10-21). Everything with a same-finger bigram is GECKO's `Other`.
fn expected(cmini: &str) -> &'static [TriClass] {
    match cmini {
        "alternate" => &[TriClass::Alternate],
        "dsfb-alt" => &[TriClass::AlternateSfs],
        "roll-in" => &[TriClass::RollIn],
        "roll-out" => &[TriClass::RollOut],
        "oneh-in" | "oneh-out" => &[TriClass::Onehand],
        "redirect" => &[TriClass::Redirect],
        "bad-redirect" => &[TriClass::BadRedirect],
        "dsfb-red" => &[TriClass::Redirect, TriClass::BadRedirect],
        "sfT" => &[TriClass::Sft],
        "sfb" => &[TriClass::Other],
        other => panic!("unmapped cmini class {other}"),
    }
}

/// Every non-thumb finger triple in cmini's table (pinned copy in `tests/fixtures/`) gets the
/// matching GECKO class on ansi-30. Keys are distinct, as cmini skips letter repeats (`sfR`).
#[test]
fn trigram_classes_match_cmini_table() {
    let g = ansi();
    let fixture: serde_json::Value = serde_json::from_str(&read("crates/gecko-core/tests/fixtures/cmini-table.json")).unwrap();
    let table = fixture["table"].as_object().unwrap();
    assert_eq!(table.len(), 512, "8 fingers cubed");

    let mut keys_of: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, k) in g.key.iter().enumerate() {
        keys_of.entry(k.finger.name().to_string()).or_default().push(i);
    }
    let mut mismatches = Vec::new();
    for (combo, class) in table {
        let fingers: Vec<&str> = combo.split('-').collect();
        let mut used: BTreeMap<&str, usize> = BTreeMap::new();
        let slots: Vec<usize> = fingers
            .iter()
            .map(|f| {
                let n = used.entry(f).or_default();
                *n += 1;
                keys_of[*f][*n - 1]
            })
            .collect();
        for (&s, f) in slots.iter().zip(&fingers) {
            let parsed: Finger = serde_json::from_value(serde_json::Value::String(f.to_string())).unwrap();
            assert_eq!(g.key[s].finger, parsed);
        }
        let got = classify(&g, slots[0], slots[1], slots[2]);
        let want = expected(class.as_str().unwrap());
        if !want.contains(&got) {
            mismatches.push(format!("{combo}: cmini {class}, gecko {got:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{} mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
}

/// Center-to-center distances on row-stagger quoted in the Keyboard Layouts Doc (QWERTY
/// letters on ansi-30): `rg` 1.6 u, `my` 2.66 u, `vt` 2.02 u.
#[test]
fn pair_distances_match_layouts_doc() {
    let g = ansi();
    let l = Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let slot = |c: char| l.chars.iter().position(|&x| x == c).unwrap();
    for (a, b, want) in [('r', 'g', 1.60), ('m', 'y', 2.66), ('v', 't', 2.02)] {
        let d = g.dist(slot(a), slot(b));
        assert!((d - want).abs() < 0.005, "{a}{b}: {d:.3} u, doc says {want} u");
        assert_eq!(d, g.dist(slot(b), slot(a)));
    }
}
