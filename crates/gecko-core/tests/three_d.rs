//! 3D boards: pose convention, parametric curvature/tenting, and metrics on keycap tops.

use std::path::Path;

use gecko_core::parametric::ParametricGeometry;
use gecko_core::pose::{self, contact, euler, normal, rotation};
use gecko_core::report::breakdown;
use gecko_core::store::Store;
use gecko_core::{Geometry, Hand, Scorer};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn store() -> Store {
    Store::open(Path::new(ROOT)).unwrap()
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn slot(g: &Geometry, row: i8, col: i8) -> usize {
    g.key.iter().position(|k| (k.row, k.col) == (row, col)).unwrap()
}

#[test]
fn euler_angles_round_trip_and_match_the_viewer_axes() {
    for rot in [[0.0, 0.0, 0.0], [30.0, 0.0, 0.0], [0.0, 15.0, 0.0], [12.0, -35.0, 50.0], [-16.0, 33.0, -54.0], [29.6, 8.7, -5.0]] {
        let back = euler(&rotation(rot));
        for (a, b) in rot.iter().zip(back) {
            assert!((*a as f64 - b).abs() < 1e-4, "{rot:?} -> {back:?}");
        }
    }
    // +rx turns the keycap top toward the user (+y); +ry raises the +x side (normal to -x).
    let n = pose::apply(&rotation([20.0, 0.0, 0.0]), [0.0, 0.0, 1.0]);
    assert!(n[1] > 0.3 && n[2] > 0.9);
    let n = pose::apply(&rotation([0.0, 20.0, 0.0]), [0.0, 0.0, 1.0]);
    assert!(n[0] < -0.3 && n[2] > 0.9);
}

#[test]
fn dactyl_poses_match_the_parametric_keywell() {
    let explicit = store().geometry("dactyl-manuform-5x6").unwrap();
    let text = std::fs::read_to_string(format!("{ROOT}/crates/gecko-core/tests/fixtures/dactyl-well.param.toml")).unwrap();
    let well = ParametricGeometry::from_toml(&text).unwrap().generate().unwrap();
    assert_eq!(well.len(), 48);
    for hand in [Hand::L, Hand::R] {
        // Compare relative to each half's middle-finger home key (halves sit anywhere).
        let anchor = |g: &Geometry| contact(&g.key[slot(g, 1, if hand == Hand::L { 3 } else { 8 })]);
        let (ae, aw) = (anchor(&explicit), anchor(&well));
        for k in well.key.iter().filter(|k| k.hand() == hand) {
            let e = &explicit.key[slot(&explicit, k.row, k.col)];
            assert_eq!(e.finger, k.finger);
            for (a, b) in e.rot.iter().zip(k.rot) {
                assert!((a - b).abs() < 2e-3, "rot {}/{}: {:?} vs {:?}", k.row, k.col, e.rot, k.rot);
            }
            let (pe, pw) = (contact(e), contact(k));
            for d in 0..3 {
                assert!(((pe[d] - ae[d]) - (pw[d] - aw[d])).abs() < 1e-3, "pos {}/{}", k.row, k.col);
            }
        }
    }
}

#[test]
fn dactyl_is_a_concave_tented_keywell() {
    let g = store().geometry("dactyl-manuform-5x6").unwrap();
    assert!(!g.is_flat());
    // Keycap tops of neighbouring rows are 18.4 mm apart along every column.
    for col in 0..12 {
        for row in -1..2 {
            let d = dist(contact(&g.key[slot(&g, row, col)]), contact(&g.key[slot(&g, row + 1, col)]));
            assert!((d * 19.05 - 18.4).abs() < 0.01, "col {col} rows {row}/{}: {:.3} mm", row + 1, d * 19.05);
        }
    }
    // Top row tilts toward the user, the outer rows rise above home, the halves tent inward.
    let (top, home) = (&g.key[slot(&g, 0, 3)], &g.key[slot(&g, 1, 3)]);
    assert!(top.rot[0] > 10.0 && contact(top)[2] > contact(home)[2]);
    assert!(normal(home)[0] < -0.2 && normal(&g.key[slot(&g, 1, 8)])[0] > 0.2);
    // Mirror symmetric.
    for k in g.key.iter().filter(|k| k.hand() == Hand::L) {
        let m = g.key.iter().filter(|o| o.hand() == Hand::R).find(|o| {
            let (a, b) = (contact(k), contact(o));
            (a[0] + b[0] - 17.0).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3 && (a[2] - b[2]).abs() < 1e-3
        });
        assert!(m.is_some_and(|m| (m.rot[0] - k.rot[0]).abs() < 1e-3 && (m.rot[1] + k.rot[1]).abs() < 1e-3), "{}/{}", k.row, k.col);
    }
}

#[test]
fn tenting_a_flat_board_changes_no_metric() {
    let store = store();
    let corpus = store.corpus("english").unwrap();
    let flat = store.geometry("corne-36").unwrap();
    let text = std::fs::read_to_string(format!("{ROOT}/data/geometries/corne-36.param.toml")).unwrap();
    let mut p = ParametricGeometry::from_toml(&text).unwrap();
    let split = p.split.as_mut().unwrap();
    split.tent = 25.0;
    split.lift = 1.5;
    let tented = p.generate().unwrap();
    assert!(!tented.is_flat());
    // The metrics see each half in its own plane, so a rigid tent leaves every value equal.
    for profile in ["default", "sfb"] {
        let profile = store.profile(profile).unwrap();
        let layout = store.layout("qwerty", "corne-36").unwrap().fit(&flat).unwrap();
        let a = Scorer::new(&flat, &corpus, &profile, &layout).unwrap();
        let b = Scorer::new(&tented, &corpus, &profile, &layout).unwrap();
        let (a, b) = (breakdown(&a, &a.identity()), breakdown(&b, &b.identity()));
        for (a, b) in a.metrics.iter().zip(&b.metrics) {
            assert!((a.value - b.value).abs() < 1e-3 * a.value.abs().max(1.0), "{}: {} vs {}", a.id, a.value, b.value);
        }
    }
    // Flat boards measure exactly at `pos`.
    assert!(flat.metric_points().iter().zip(&flat.key).all(|(p, k)| *p == k.pos));
}

#[test]
fn dactyl_scores_and_deltas_agree() {
    let store = store();
    let g = store.geometry("dactyl-manuform-5x6").unwrap();
    let corpus = store.corpus("english").unwrap();
    let profile = store.profile("default").unwrap();
    let layout = store.layout("qwerty", "dactyl-manuform-5x6").unwrap().fit(&g).unwrap();
    let scorer = Scorer::new(&g, &corpus, &profile, &layout).unwrap();
    let mut state = scorer.state_for(&layout).unwrap();
    for a in 0..g.len() {
        let b = (a + 7) % g.len();
        let before = scorer.full(&state);
        let delta = scorer.delta(&state, a, b);
        scorer.apply(&mut state, a, b, delta);
        assert!((scorer.full(&state) - before - delta).abs() < 1e-5 * before.abs().max(1.0));
    }
}
