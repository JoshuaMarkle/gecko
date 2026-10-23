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
        alt: Vec::new(),
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

#[test]
fn repair_pushes_keys_apart() {
    // Two flat keys half overlapping: each moves a quarter key apart, in the plane.
    let mut g = board(vec![key(0.0, 0.0, 0.0), key(0.5, 0.0, 0.0)]);
    let r = g.repair(0.0, 50);
    assert_eq!((r.passes, r.remaining), (1, 0));
    assert!(g.is_flat() && g.collisions(0.0).is_empty());
    assert!((g.key[0].pos[0] + 0.251).abs() < 1e-4 && (g.key[1].pos[0] - 0.751).abs() < 1e-4, "{:?}", g.key);
    assert!((r.max_shift - 0.251).abs() < 1e-4);
    // A clear board is untouched.
    let before = g.clone();
    assert_eq!(g.repair(0.0, 50), gecko_core::geometry::collision3d::Repair { passes: 0, max_shift: 0.0, remaining: 0 });
    assert_eq!(g.key, before.key);
    // Margins are honoured.
    g.repair(0.1, 50);
    assert!(g.collisions(0.1).is_empty());
}

#[test]
fn repair_clears_random_3d_boards() {
    let mut rng = Rng::new(11);
    for _ in 0..20 {
        let keys: Vec<Key> = (0..30)
            .map(|_| {
                let mut k = key(rng.below(600) as f32 / 100.0, rng.below(400) as f32 / 100.0, rng.below(60) as f32 - 30.0);
                k.pos[2] = rng.below(100) as f32 / 100.0;
                k.rot[0] = rng.below(60) as f32 - 30.0;
                k.rot[1] = rng.below(80) as f32 - 40.0;
                k
            })
            .collect();
        let mut g = board(keys);
        let rots: Vec<[f32; 3]> = g.key.iter().map(|k| k.rot).collect();
        assert!(!g.collisions_3d(0.0).is_empty());
        let r = g.repair(0.0, 500);
        assert_eq!(r.remaining, 0, "{r:?}");
        assert!(g.collisions_3d(0.0).is_empty() && g.validate().is_ok());
        assert_eq!(g.key.iter().map(|k| k.rot).collect::<Vec<_>>(), rots, "keys only translate");
    }
}

#[test]
fn repair_moves_layer_copies_with_their_key() {
    let g = board(vec![key(0.0, 0.0, 0.0), key(0.6, 0.0, 0.0), key(3.0, 0.0, 0.0)]);
    let mut layered = g.layered(&[(2, gecko_core::LayerMode::Oneshot)]).unwrap();
    layered.repair(0.0, 50);
    assert!(layered.collisions(0.0).is_empty());
    for (i, k) in layered.key.iter().enumerate() {
        if let Some(p) = k.phys {
            assert_eq!(k.pos, layered.key[p as usize].pos, "slot {i}");
        }
    }
}

#[test]
fn finger_access_blocks_buried_and_raised_keys() {
    // A key hovering over another's cap: solids clear (vertical room), but no finger reaches it.
    let mut g = board(vec![key(0.0, 0.0, 0.0), key(0.3, 0.0, 0.0)]);
    g.key[1].pos[2] = 1.0;
    let c = g.contacts_3d(0.0);
    assert_eq!(c.iter().map(|c| (c.a, c.b)).collect::<Vec<_>>(), vec![(0, 1)]);
    assert!(g.validate().unwrap_err().to_string().contains("overlapping"));
    // A key one row toward the user, raised into the finger lying back from key 0.
    let mut g = board(vec![key(0.0, 0.0, 0.0), key(0.0, 1.0, 0.0)]);
    g.key[1].pos[2] = 0.5;
    assert_eq!(g.contacts_3d(0.0).len(), 1);
    // A thumb key there has no finger behind it, so it blocks nothing.
    g.key[0].finger = Finger::LT;
    assert!(g.contacts_3d(0.0).is_empty());
    // At the same height 1.3 u ahead (a Corne's thumbs) it is clear.
    let g = board(vec![key(0.0, 0.0, 0.0), key(0.0, 1.3, 0.0)]);
    assert!(g.contacts_3d(0.0).is_empty());
}
