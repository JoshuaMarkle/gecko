//! C11 alternative fingering: board options, incremental re-costing on reassignment, and the
//! fingering search. See `context/objective.md` (Fingering search).

use gecko_core::fingering::{self, FingeringParams};
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Finger, Geometry, Layout, Profile, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{p}")).unwrap()
}

fn board(name: &str) -> Geometry {
    let mut g = Geometry::from_toml(&read(&format!("data/geometries/{name}.toml"))).unwrap();
    for (metric, table) in g.comfort.clone() {
        g.load_comfort(&metric, &read(&format!("data/comfort/{table}.csv"))).unwrap();
    }
    g
}

fn inputs() -> (Corpus, Profile, Layout) {
    let c = Corpus::from_bytes(&std::fs::read(format!("{ROOT}/data/corpora/english.gkc")).unwrap()).unwrap();
    let p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    (c, p, Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap())
}

#[test]
fn alt_board_choosing_every_option_is_the_angle_mod() {
    let (alt, angle) = (board("ansi-30-alt"), board("ansi-30-angle"));
    let mut choice = vec![0u8; alt.len()];
    for (i, k) in alt.key.iter().enumerate() {
        if !k.alt.is_empty() {
            choice[i] = 1;
        }
    }
    assert_eq!(choice.iter().filter(|&&c| c == 1).count(), 3);
    let g = alt.with_fingering(&choice).unwrap();
    for (i, (a, b)) in g.key.iter().zip(&angle.key).enumerate() {
        assert_eq!(a.finger, b.finger, "key {i}");
        // V keeps its standard effort (the angle board raises it; options are per key).
        if !(a.row == 2 && a.col == 3) {
            assert_eq!(a.effort, b.effort, "key {i}");
        }
    }
    // The option set is kept: the original finger becomes an alternative.
    for i in 0..alt.len() {
        let mut a = alt.finger_options(i);
        let mut b = g.finger_options(i);
        a.sort_by_key(|o| o.finger.index());
        b.sort_by_key(|o| o.finger.index());
        assert_eq!(a, b);
    }
    // TOML round trip keeps the alternatives.
    let back = Geometry::from_toml(&g.to_toml()).unwrap();
    assert!(back.key.iter().zip(&g.key).all(|(a, b)| a.alt == b.alt && a.finger == b.finger));
    assert!(alt.with_fingering(&[2]).is_err());
}

#[test]
fn alt_options_validate_and_derive_effort() {
    let key = |extra: &str, home: bool| {
        format!("[[key]]\npos = [0.0, 0.0, 0.0]\nfinger = \"LI\"\nrow = 1\ncol = 3\nhome = {home}\neffort = 1.0\n\n[[key]]\npos = [1.0, 1.0, 0.0]\nfinger = \"LM\"\nrow = 1\ncol = 2\nhome = true\neffort = 1.0\n\n[[key]]\npos = [0.0, 2.0, 0.0]\nfinger = \"LI\"\nrow = 2\ncol = 3\n{extra}\n")
    };
    let g = Geometry::from_toml(&format!("name = \"t\"\n{}", key("alt = [{ finger = \"LM\" }]", true))).unwrap();
    let e = g.key[2].alt[0].effort;
    assert!(e.is_finite() && e > 0.0 && e != g.key[2].effort, "derived alt effort {e}");
    assert!(Geometry::from_toml(&format!("name = \"t\"\n{}", key("alt = [{ finger = \"LI\", effort = 1.0 }]", true))).is_err());
    let home_alt = "[[key]]\npos = [0.0, 0.0, 0.0]\nfinger = \"LI\"\nrow = 1\ncol = 3\nhome = true\neffort = 1.0\nalt = [{ finger = \"LM\", effort = 1.0 }]\n";
    assert!(Geometry::from_toml(&format!("name = \"t\"\n{home_alt}")).is_err());
}

#[test]
fn reassignment_recosts_like_a_new_scorer() {
    let (c, mut p, l) = inputs();
    for d in gecko_core::metrics::METRICS {
        p.weights.entry(d.id.to_string()).or_insert(0.05);
    }
    let alt = board("ansi-30-alt");
    let base = Scorer::new(&alt, &c, &p, &l).unwrap();
    let i = alt.key.iter().position(|k| !k.alt.is_empty()).unwrap();
    let mut choice = vec![0u8; alt.len()];
    choice[i] = 1;
    let g = alt.with_fingering(&choice).unwrap();
    let fast = base.with_geometry(&g).unwrap();
    let fresh = Scorer::new(&g, &c, &p, &l).unwrap();
    let mut rng = Rng::new(5);
    for _ in 0..50 {
        let mut sym: Vec<u8> = (0..fast.n as u8).collect();
        rng.shuffle(&mut sym);
        assert_eq!(fast.state_from_sym(sym.clone()).score, fresh.state_from_sym(sym).score);
    }
}

#[test]
fn search_takes_improving_reassignments_only() {
    let (c, p, l) = inputs();
    let alt = board("ansi-30-alt");
    let sc = Scorer::new(&alt, &c, &p, &l).unwrap();
    let movable: Vec<usize> = (0..sc.n).collect();
    let params = FingeringParams { search: true, evals: 50_000 };
    let run = |seed| fingering::search(&sc, &alt, &movable, &sc.identity().sym, &params, &mut Rng::new(seed)).unwrap();
    let out = run(1);
    let r = &out.report;
    assert!(r.score <= r.base_score);
    assert!(r.candidates >= 3);
    assert_eq!(r.changes.len(), r.choice.iter().filter(|&&c| c != 0).count());
    assert!(r.changes.iter().all(|c| c.from != c.to && c.to != Finger::LP));
    assert_eq!(out.scorer.state_from_sym(out.best.sym.clone()).score, out.best.score);
    let again = run(1);
    assert_eq!(again.report, out.report, "deterministic for a seed");
}

