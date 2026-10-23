//! `Geometry::flat2d`: flat boards unchanged, a rigidly tilted board recovered, and a curved,
//! tented split board drawn without overlaps and with row/column order kept.

use gecko_core::geometry::flat2d::overlaps;
use gecko_core::{Finger, Flat, Geometry, Key};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn geometry(name: &str) -> Geometry {
    Geometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/{name}.toml")).unwrap()).unwrap()
}

fn center(f: &Flat, k: &Key) -> [f32; 2] {
    [f.pos[0] + k.size[0] / 2.0, f.pos[1] + k.size[1] / 2.0]
}

fn assert_clean(g: &Geometry, flat: &[Flat]) {
    assert_eq!(flat.len(), g.len());
    for i in 0..g.len() {
        for j in i + 1..g.len() {
            assert!(
                !overlaps((&flat[i], g.key[i].size), (&flat[j], g.key[j].size)),
                "{}: keys {i} and {j} overlap: {:?} {:?}",
                g.name,
                flat[i],
                flat[j]
            );
        }
    }
    // Row/column order: along a row (same hand) x grows with col; down a column y grows with row.
    for i in 0..g.len() {
        for j in 0..g.len() {
            let (a, b) = (&g.key[i], &g.key[j]);
            let (ca, cb) = (center(&flat[i], a), center(&flat[j], b));
            if a.hand() == b.hand() && a.row == b.row && a.col < b.col {
                assert!(ca[0] < cb[0], "{}: row {} col order broken at {i},{j}", g.name, a.row);
            }
            if a.hand() == b.hand() && a.col == b.col && a.row < b.row {
                assert!(ca[1] < cb[1], "{}: col {} row order broken at {i},{j}", g.name, a.col);
            }
        }
    }
}

#[test]
fn flat_boards_unchanged() {
    for name in ["ansi-30", "ortho-30", "corne-36", "sweep-34"] {
        let g = geometry(name);
        assert!(g.is_flat());
        let flat = g.flat2d();
        for (f, k) in flat.iter().zip(&g.key) {
            assert_eq!(f.pos, [k.pos[0], k.pos[1]], "{name}");
            assert_eq!(f.rot, k.rot[2], "{name}");
        }
    }
}

#[test]
fn tilted_flat_board_is_recovered() {
    let g0 = geometry("ortho-30");
    let mut g = g0.clone();
    let a = 25f32.to_radians();
    for k in &mut g.key {
        let cy = k.pos[1] + k.size[1] / 2.0;
        k.pos[1] = cy * a.cos() - k.size[1] / 2.0;
        k.pos[2] = cy * a.sin();
        k.rot[0] = 25.0;
    }
    assert!(!g.is_flat());
    let flat = g.flat2d();
    let o = (flat[0].pos, g0.key[0].pos);
    for (f, k) in flat.iter().zip(&g0.key) {
        for d in 0..2 {
            assert!(((f.pos[d] - o.0[d]) - (k.pos[d] - o.1[d])).abs() < 1e-3, "{f:?} vs {:?}", k.pos);
        }
        assert!(f.rot.abs() < 1e-3);
    }
}

/// A Dactyl-like split board: per half 3 curved rows × 5 columns (column well, 40° per row)
/// plus 3 thumb keys tucked under the bottom row, each half tented 25°.
fn curved() -> Geometry {
    let mut key = Vec::new();
    let fingers = [Finger::LP, Finger::LR, Finger::LM, Finger::LI, Finger::LI, Finger::RI, Finger::RI, Finger::RM, Finger::RR, Finger::RP];
    let well = 1.05 / (2.0 * 20f32.to_radians().sin());
    let tent = 25f32;
    for half in 0..2 {
        let (side, x0) = if half == 0 { (-1.0f32, 0.0f32) } else { (1.0, 7.0) };
        let mut push = |x: f32, y: f32, z: f32, rx: f32, row: i8, col: i8, finger: Finger| {
            // Tent: the half rotates about its inner edge (x = 5 left, x = 7 right), outer side down.
            let (pivot, t) = (if half == 0 { 5.0 } else { 7.0 }, tent.to_radians());
            let dist = (x + 0.5 - pivot).abs();
            let (cx, cz) = (pivot + side * dist * t.cos(), z - dist * t.sin());
            key.push(Key {
                pos: [cx - 0.5, y - 0.5, cz],
                rot: [rx, -side * tent, 0.0],
                size: [1.0, 1.0],
                finger,
                row,
                col,
                home: row == 1,
                stretch: false,
                effort: 1.0,
                fill: None,
                hrm: None,
                layer: 0,
                phys: None,
                alt: Vec::new(),
            });
        };
        for row in 0..3i8 {
            for c in 0..5i8 {
                let phi = (row as f32 - 1.0) * 40f32.to_radians();
                let col = c + 5 * half as i8;
                push(
                    x0 + c as f32,
                    well * phi.sin(),
                    well * (1.0 - phi.cos()),
                    -(row as f32 - 1.0) * 40.0,
                    row,
                    col,
                    fingers[col as usize],
                );
            }
        }
        // Thumbs: lower in z, overlapping the bottom row once projected.
        let thumb = if half == 0 { Finger::LT } else { Finger::RT };
        let tx = if half == 0 { 2.5 } else { 7.5 };
        for t in 0..3i8 {
            push(tx + t as f32 * 1.02, well * 40f32.to_radians().sin() + 0.8, -1.2, 0.0, 3, t + 10 * half as i8, thumb);
        }
    }
    Geometry {
        name: "curved-test".into(),
        description: String::new(),
        key,
        comfort: Default::default(),
        comfort_tables: Default::default(),
        layer_modes: Vec::new(),
        layer_keys: Vec::new(),
    }
}

#[test]
fn curved_board_has_no_overlaps() {
    let g = curved();
    // Structural checks only: this synthetic well's switch bodies graze (0.009 u in 3D).
    g.check_keys().unwrap();
    assert!(!g.is_flat());
    let flat = g.flat2d();
    assert_clean(&g, &flat);
    // Halves stay apart, left of right.
    let max_l = (0..g.len()).filter(|&i| g.key[i].finger.index() < 5).map(|i| flat[i].pos[0]).fold(f32::MIN, f32::max);
    let min_r = (0..g.len()).filter(|&i| g.key[i].finger.index() >= 5).map(|i| flat[i].pos[0]).fold(f32::MAX, f32::min);
    assert!(max_l < min_r);
}

#[test]
fn layered_keys_copy_their_physical_key() {
    let g = curved().layered(&[(15, gecko_core::LayerMode::Hold)]).unwrap();
    let flat = g.flat2d();
    for i in 0..g.len() {
        assert_eq!(flat[i], flat[g.phys(i)]);
    }
}
