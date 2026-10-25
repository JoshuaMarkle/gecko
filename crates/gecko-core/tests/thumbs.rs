//! Thumb clusters (I2, #71): `thumb_arc` against boards whose clusters are known to fit
//! or miss the thumb's sweep, the arc's key capacity, and resizing a parametric thumb zone for
//! a key-count search. See `context/hand-model.md` and `context/algorithms/geometry-search.md`.

use std::path::Path;

use gecko_core::geomsearch::resize_thumbs;
use gecko_core::hand::{costs, thumb_slots, thumb_workspace, HandSpec, AVERAGE};
use gecko_core::store::Store;
use gecko_core::{Geometry, Hand};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

/// thumb_arc of the right hand's thumb keys, by key position `[x, y]`.
fn right_thumbs(g: &Geometry) -> Vec<([f32; 2], f32)> {
    let c = costs(g, &AVERAGE);
    g.key
        .iter()
        .enumerate()
        .filter(|(_, k)| k.finger.is_thumb() && k.phys.is_none() && k.hand() == Hand::R)
        .map(|(i, k)| ([k.pos[0], k.pos[1]], c.arc[i]))
        .collect()
}

#[test]
fn arc_fit_separates_clusters_on_and_off_the_sweep() {
    let s = store();
    // Kyria's main thumb row curves with the thumb: every key of it lies on the arc.
    let kyria = right_thumbs(&s.geometry("kyria").unwrap());
    let row: Vec<f32> = kyria.iter().filter(|(p, _)| p[0] >= 9.9).map(|e| e.1).collect();
    assert_eq!(row.len(), 4);
    assert!(row.iter().all(|&a| a < 0.5), "kyria thumb row {row:?}");
    // The Dactyl-Manuform's lower cluster keys (the known hard-to-reach ones) sit far off it,
    // its upper pair on it.
    let dactyl = right_thumbs(&s.geometry("dactyl-manuform-5x6").unwrap());
    let (upper, lower): (Vec<&([f32; 2], f32)>, Vec<_>) = dactyl.iter().partition(|(p, _)| p[1] < 5.0);
    assert!(upper.iter().all(|e| e.1 < 2.0), "dactyl upper {upper:?}");
    assert!(lower.iter().filter(|e| e.1 > 4.0).count() >= 3, "dactyl lower {lower:?}");
    // Finger keys cost nothing; mirrored halves cost the same.
    let g = s.geometry("corne-36").unwrap();
    let c = costs(&g, &AVERAGE);
    assert!(g.key.iter().zip(&c.arc).all(|(k, &a)| k.finger.is_thumb() || a == 0.0));
    let mut l: Vec<f32> = g.key.iter().zip(&c.arc).filter(|(k, _)| k.finger.is_thumb() && k.hand() == Hand::L).map(|e| *e.1).collect();
    let mut r: Vec<f32> = g.key.iter().zip(&c.arc).filter(|(k, _)| k.finger.is_thumb() && k.hand() == Hand::R).map(|e| *e.1).collect();
    l.sort_by(f32::total_cmp);
    r.sort_by(f32::total_cmp);
    assert!(l.iter().zip(&r).all(|(a, b)| (a - b).abs() < 1e-3), "{l:?} vs {r:?}");
}

#[test]
fn the_arc_holds_three_keys_for_the_average_hand() {
    assert!((thumb_workspace(&AVERAGE) - 73.1).abs() < 1e-9);
    assert_eq!(thumb_slots(&AVERAGE.scaled(0.92)), 3);
    assert_eq!(thumb_slots(&AVERAGE), 3);
    assert_eq!(thumb_slots(&AVERAGE.scaled(1.08)), 4);
}

#[test]
fn resizing_keeps_typed_and_cheap_keys_in_place() {
    let s = store();
    let (params, board) = s.param_board("corne-36").unwrap();
    let corpus = s.corpus("english").unwrap();
    let hand = HandSpec::default();
    let layout = s.layout("qwerty", "corne-36").unwrap();
    let thumbs = |g: &Geometry| -> Vec<([f32; 3], Option<String>)> {
        g.key.iter().filter(|k| k.finger.is_thumb()).map(|k| (k.pos, k.fill.clone())).collect()
    };
    let before = thumbs(&board);
    // Two keys: the space/backspace column stays, where it was.
    let two = resize_thumbs(&params, &board, &corpus, &hand, 2).unwrap().generate().unwrap();
    let after = thumbs(&two);
    assert_eq!(after.len(), 4);
    assert!(after.iter().any(|t| t.1.as_deref() == Some("space")));
    for t in &after {
        let same = before.iter().find(|b| b.1 == t.1).expect("kept key from the start board");
        assert!((0..3).all(|i| (same.0[i] - t.0[i]).abs() < 1e-4), "{t:?} moved from {same:?}");
    }
    layout.fit(&two).unwrap();
    // Four keys: one empty key added, the old ones unmoved, and a 30-key layout still fits.
    let four = resize_thumbs(&params, &board, &corpus, &hand, 4).unwrap().generate().unwrap();
    let after = thumbs(&four);
    assert_eq!(after.len(), 8);
    assert_eq!(after.iter().filter(|t| t.1.as_deref() == Some("none")).count(), 3);
    for b in &before {
        assert!(after.iter().any(|t| (0..3).all(|i| (t.0[i] - b.0[i]).abs() < 1e-4)), "{b:?} moved");
    }
    layout.fit(&four).unwrap();
    // Same count: unchanged. Out of range or no thumb zone: an error.
    assert_eq!(thumbs(&resize_thumbs(&params, &board, &corpus, &hand, 3).unwrap().generate().unwrap()), before);
    assert!(resize_thumbs(&params, &board, &corpus, &hand, 0).is_err());
    let (ortho, ortho_board) = s.param_board("ortho-30").unwrap();
    assert!(resize_thumbs(&ortho, &ortho_board, &corpus, &hand, 2).is_err());
}
