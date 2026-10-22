//! G04 2D keycap collision (`Geometry::collisions`, #39) and H8 3D key volumes
//! (`Geometry::collisions_3d`, #66): known overlapping and clear pairs, margins, rotated keys
//! where bounding boxes lie, the grid broad phases against brute force, and validation of the
//! bundled boards. GJK + EPA against separating axes is a unit test in `collision3d.rs`.
//! See `context/testing.md`.

use gecko_core::rng::Rng;
use gecko_core::{Finger, Geometry, Key};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn key(x: f32, y: f32, rz: f32) -> Key {
    Key {
        pos: [x, y, 0.0],
        rot: [0.0, 0.0, rz],
        size: [1.0, 1.0],
        finger: Finger::LI,
        row: 0,
        col: 0,
        home: false,
        stretch: false,
        hrm: None,
        effort: 1.0,
        fill: None,
        layer: 0,
        phys: None,
    }
}

fn board(keys: Vec<Key>) -> Geometry {
    Geometry {
        name: "test".into(),
        description: String::new(),
        key: keys,
        comfort: Default::default(),
        comfort_tables: Default::default(),
        layer_keys: Vec::new(),
        layer_modes: Vec::new(),
    }
}

fn pairs(g: &Geometry, margin: f32) -> Vec<(usize, usize)> {
    g.collisions(margin).iter().map(|c| (c.a, c.b)).collect()
}

#[test]
fn known_pairs() {
    // Side by side, touching: clear. Overlapping by 0.1 u: one collision 0.1 deep.
    assert!(board(vec![key(0.0, 0.0, 0.0), key(1.0, 0.0, 0.0)]).collisions(0.0).is_empty());
    let c = board(vec![key(0.0, 0.0, 0.0), key(0.9, 0.0, 0.0)]).collisions(0.0);
    assert_eq!(c.len(), 1);
    assert!((c[0].depth - 0.1).abs() < 1e-5, "{}", c[0].depth);
    // A margin of 0.05 u per side makes touching keys collide by 0.1 u.
    let c = board(vec![key(0.0, 0.0, 0.0), key(1.0, 0.0, 0.0)]).collisions(0.05);
    assert_eq!(c.len(), 1);
    assert!((c[0].depth - 0.1).abs() < 1e-5);
    // Diagonal neighbours only touch at a corner.
    assert!(board(vec![key(0.0, 0.0, 0.0), key(1.0, 1.0, 0.0)]).collisions(0.0).is_empty());
    // Same position: full overlap.
    assert_eq!(pairs(&board(vec![key(2.0, 2.0, 0.0), key(2.0, 2.0, 30.0)]), 0.0), vec![(0, 1)]);
}

#[test]
fn rotated_keys_use_their_own_axes() {
    // A 45° key whose bounding box overlaps the square next to it but whose diamond does not.
    let g = board(vec![key(0.0, 0.0, 0.0), key(1.2, 1.2, 45.0)]);
    assert!(g.collisions(0.0).is_empty());
    // Two 45° keys 1 u apart along x: their diamonds (half-width 0.707) do overlap.
    assert_eq!(pairs(&board(vec![key(0.0, 0.0, 45.0), key(1.0, 0.0, 45.0)]), 0.0), vec![(0, 1)]);
    // Rotated by 45° and moved 1/cos 45° apart along their own diagonal axis: touching.
    let d = std::f32::consts::SQRT_2 / 2.0;
    assert!(board(vec![key(0.0, 0.0, 45.0), key(d, d, 45.0)]).collisions(0.0).is_empty());
}

#[test]
fn grid_matches_brute_force() {
    let mut rng = Rng::new(5);
    for _ in 0..50 {
        let keys: Vec<Key> = (0..40)
            .map(|_| {
                let mut k = key(rng.below(800) as f32 / 100.0, rng.below(500) as f32 / 100.0, rng.below(90) as f32);
                k.size = [1.0 + rng.below(3) as f32 * 0.25, 1.0];
                k
            })
            .collect();
        let g = board(keys);
        for margin in [0.0, 0.1] {
            let mut want = Vec::new();
            for i in 0..g.len() {
                for j in i + 1..g.len() {
                    let pair = board(vec![g.key[i].clone(), g.key[j].clone()]);
                    if !pair.collisions(margin).is_empty() {
                        want.push((i, j));
                    }
                }
            }
            assert_eq!(pairs(&g, margin), want);
        }
    }
}

#[test]
fn bundled_boards_are_clear_and_validation_rejects_overlaps() {
    for name in ["ansi-30", "ortho-30", "corne-36", "sweep-34", "lily58", "kyria", "ergodox"] {
        let g = Geometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/{name}.toml")).unwrap()).unwrap();
        assert!(g.collisions(0.0).is_empty(), "{name}");
    }
    let mut g = board(vec![key(0.0, 0.0, 0.0), key(0.5, 0.0, 0.0)]);
    let err = g.validate().unwrap_err().to_string();
    assert!(err.contains("overlapping"), "{err}");
    assert!(g.check_keys().is_ok(), "structural checks allow overlaps");
    g.key[1].pos[0] = 1.0;
    assert!(g.validate().is_ok());
}

#[test]
fn layer_copies_and_3d_boards_are_skipped() {
    let g = board(vec![key(0.0, 0.0, 0.0), key(1.0, 0.0, 0.0)]);
    let layered = g.layered(&[(0, gecko_core::LayerMode::Oneshot)]).unwrap();
    assert!(layered.len() > 2 && layered.collisions(0.0).is_empty(), "virtual keys share their physical key");
    let mut tilted = board(vec![key(0.0, 0.0, 0.0), key(0.0, 0.0, 0.0)]);
    tilted.key[1].rot[0] = 10.0;
    assert!(tilted.collisions(0.0).is_empty(), "3D boards use collisions_3d");
    assert_eq!(tilted.collisions_3d(0.0).len(), 1);
    assert!(tilted.validate().unwrap_err().to_string().contains("overlapping"));
    tilted.key[1].pos = [1.2, 0.0, 0.0];
    assert!(tilted.validate().is_ok());
}

#[test]
fn grid_matches_brute_force_3d() {
    let mut rng = Rng::new(9);
    let mut found = 0;
    for _ in 0..30 {
        let keys: Vec<Key> = (0..40)
            .map(|_| {
                let mut k = key(rng.below(700) as f32 / 100.0, rng.below(500) as f32 / 100.0, rng.below(60) as f32 - 30.0);
                k.pos[2] = rng.below(150) as f32 / 100.0;
                k.rot[0] = rng.below(60) as f32 - 30.0;
                k.rot[1] = rng.below(80) as f32 - 40.0;
                k.size = [1.0 + rng.below(3) as f32 * 0.25, 1.0];
                k
            })
            .collect();
        let g = board(keys);
        for margin in [0.0, 0.1] {
            let mut want = Vec::new();
            for i in 0..g.len() {
                for j in i + 1..g.len() {
                    let pair = board(vec![g.key[i].clone(), g.key[j].clone()]);
                    if let Some(c) = pair.contacts_3d(margin).first() {
                        want.push((i, j, c.depth));
                    }
                }
            }
            let got: Vec<(usize, usize, f64)> = g.contacts_3d(margin).iter().map(|c| (c.a, c.b, c.depth)).collect();
            assert_eq!(got, want);
            found += got.len();
        }
    }
    assert!(found > 100, "{found}");
}
