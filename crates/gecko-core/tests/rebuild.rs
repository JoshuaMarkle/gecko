//! Fast cost-table rebuild (`Scorer::set_geometry`, #32): moving keys and rebuilding only the
//! touched entries gives the same scorer as building from scratch, bit for bit, on flat, thumb,
//! 3D, layered and Shift boards, with every metric weighted. See `context/testing.md`.

use gecko_core::layout::{EMPTY, ESC, SHIFT};
use gecko_core::metrics::METRICS;
use gecko_core::rng::Rng;
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer};

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

fn all_weights() -> Profile {
    let mut p = Profile::from_toml(&read("data/profiles/default.toml")).unwrap();
    p.familiar_to = Some("qwerty".into());
    for (i, d) in METRICS.iter().enumerate() {
        p.weights.insert(d.id.to_string(), 0.1 + i as f32 * 0.07);
    }
    p
}

/// Random words over `charset`, every fifth capitalized (Shift counts).
fn corpus(charset: &str, seed: u64) -> Corpus {
    let cs: Vec<char> = charset.chars().collect();
    let mut rng = Rng::new(seed);
    let mut text = String::new();
    for w in 0..8_000 {
        for k in 0..1 + rng.below(6) {
            let c = cs[(rng.below(cs.len()) * rng.below(cs.len())) / cs.len()];
            text.push(if w % 5 == 0 && k == 0 { c.to_ascii_uppercase() } else { c });
        }
        text.push(' ');
    }
    let mut c = Corpus::new("synthetic", charset);
    c.add_text(&text);
    c
}

/// (board, corpus, layout) cases covering flat, thumbs + Shift, 3D and layered scorers.
fn cases() -> Vec<(Geometry, Corpus, Layout)> {
    let qwerty = || Layout::from_toml(&read("data/layouts/qwerty.toml")).unwrap();
    let text = corpus(gecko_core::corpus::DEFAULT_CHARSET, 3);
    let code = corpus(gecko_core::corpus::CODE_CHARSET, 5);
    let mut out = vec![(board("ansi-30"), text.clone(), qwerty())];
    let corne = board("corne-36");
    let mut shifted = qwerty().fit(&corne).unwrap();
    for c in shifted.chars.iter_mut().filter(|c| **c == ESC || **c == EMPTY) {
        *c = SHIFT;
    }
    out.push((corne.clone(), text.clone(), shifted));
    let dactyl = board("dactyl-manuform-5x6");
    out.push((dactyl.clone(), text, qwerty().fit(&dactyl).unwrap()));
    let layered = Layout::from_toml(&read("data/layouts/layered/corne-code.toml")).unwrap().fit(&corne).unwrap();
    out.push((corne, code, layered));
    out
}

/// Move `k` random keys: x/y always, z and tilt/tent on 3D boards, sometimes effort.
fn nudge(g: &mut Geometry, rng: &mut Rng, k: usize) {
    let three_d = !g.is_flat();
    let r = |rng: &mut Rng| (rng.below(2001) as f32 - 1000.0) / 4000.0;
    for _ in 0..k {
        let i = rng.below(g.len());
        let key = &mut g.key[i];
        key.pos[0] += r(rng);
        key.pos[1] += r(rng);
        if three_d {
            key.pos[2] += r(rng);
            key.rot[0] += 10.0 * r(rng);
            key.rot[1] += 10.0 * r(rng);
        }
        if rng.below(4) == 0 {
            key.effort += r(rng).abs();
        }
    }
}

/// Every unfolded table and the full score of a few random layouts agree exactly.
fn assert_same(a: &Scorer, b: &Scorer, rng: &mut Rng, what: &str) {
    for (mi, d) in METRICS.iter().enumerate() {
        assert_eq!(a.table(mi), b.table(mi), "{what}: {} table", d.id);
    }
    for _ in 0..8 {
        let mut sym: Vec<u8> = (0..a.n as u8).collect();
        for i in (1..sym.len()).rev() {
            sym.swap(i, rng.below(i + 1));
        }
        let (sa, sb) = (a.state_from_sym(sym.clone()), b.state_from_sym(sym));
        assert_eq!(a.full(&sa).to_bits(), b.full(&sb).to_bits(), "{what}: full score");
        assert_eq!(sa.score.to_bits(), sb.score.to_bits(), "{what}: state score");
    }
}

#[test]
fn rebuild_equals_fresh_build() {
    let p = all_weights();
    let mut rng = Rng::new(7);
    for (g0, c, l) in cases() {
        let mut sc = Scorer::new(&g0, &c, &p, &l).unwrap();
        let mut g = g0.clone();
        // Chained moves on one scorer, so stale entries would accumulate.
        for step in 0..6 {
            nudge(&mut g, &mut rng, 1 + step % 3);
            sc.set_geometry(&g).unwrap();
            let fresh = Scorer::new(&g, &c, &p, &l).unwrap();
            assert_same(&sc, &fresh, &mut rng, &format!("{} step {step}", g.name));
        }
    }
}

#[test]
fn rebuild_keeps_refolded_weights() {
    let (g0, c, l) = cases().swap_remove(1);
    let p = all_weights();
    let weights = [("B01", 2.0), ("T08", 1.0), ("M01", 0.5), ("Y01", 0.3), ("M02", 1.0)];
    let mut sc = Scorer::new(&g0, &c, &p, &l).unwrap().with_weights(&weights).unwrap();
    let mut g = g0.clone();
    nudge(&mut g, &mut Rng::new(3), 3);
    sc.set_geometry(&g).unwrap();
    let fresh = Scorer::new(&g, &c, &p, &l).unwrap().with_weights(&weights).unwrap();
    assert_same(&sc, &fresh, &mut Rng::new(4), "with_weights");
}

#[test]
fn changed_slots_are_counted() {
    let (g0, c, l) = cases().swap_remove(0);
    let mut sc = Scorer::new(&g0, &c, &all_weights(), &l).unwrap();
    assert_eq!(sc.set_geometry(&g0).unwrap(), 0);
    let mut g = g0.clone();
    g.key[4].pos[0] += 0.1;
    assert_eq!(sc.set_geometry(&g).unwrap(), 1, "a flat board: only the moved key");

    // 3D: the moved key's hand re-fits its plane, so that hand's keys all change.
    let (d0, c, l) = cases().swap_remove(2);
    let mut sc = Scorer::new(&d0, &c, &all_weights(), &l).unwrap();
    let mut d = d0.clone();
    d.key[0].pos[2] += 0.2;
    let hand = d.key[0].hand();
    let changed = sc.set_geometry(&d).unwrap();
    assert!(changed > 1 && changed <= d.key.iter().filter(|k| k.hand() == hand).count(), "{changed}");
}

#[test]
fn delta_matches_full_after_rebuild() {
    let (g0, c, l) = cases().swap_remove(3);
    let mut sc = Scorer::new(&g0, &c, &all_weights(), &l).unwrap();
    let mut st = sc.identity();
    let mut g = g0.clone();
    let mut rng = Rng::new(11);
    nudge(&mut g, &mut rng, 4);
    sc.set_geometry(&g).unwrap();
    sc.refresh(&mut st);
    assert_eq!(st.score, sc.full(&st));
    for _ in 0..200 {
        let (a, b) = (rng.below(sc.n), rng.below(sc.n));
        let before = sc.full(&st);
        let d = sc.delta(&st, a, b);
        sc.apply(&mut st, a, b, d);
        assert!((sc.full(&st) - before - d).abs() < 1e-5 * before.abs().max(1.0));
    }
}

#[test]
fn different_keys_are_rejected() {
    let (g0, c, l) = cases().swap_remove(0);
    let mut sc = Scorer::new(&g0, &c, &all_weights(), &l).unwrap();
    let mut g = g0.clone();
    g.key.pop();
    assert!(sc.set_geometry(&g).is_err());
}
