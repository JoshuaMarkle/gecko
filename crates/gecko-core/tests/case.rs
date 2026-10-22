//! J4 plate and case STL (`geometry/case.rs`, `geometry/mesh.rs`, #75): every bundled board
//! gives closed, outward shells that a slicer stitches into closed meshes, with every switch
//! cutout clear and the board above the floor.

use std::collections::HashMap;

use gecko_core::geometry::case::{CaseParams, Switch};
use gecko_core::geometry::mesh::{closed, Mesh};
use gecko_core::store::Store;
use gecko_core::Geometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const BOARDS: [&str; 9] =
    ["ansi-30", "ortho-30", "corne-36", "corne-36.param", "sweep-34", "lily58", "kyria", "ergodox", "dactyl-manuform-5x6"];

fn board(name: &str) -> Geometry {
    Store::open(std::path::Path::new(ROOT)).unwrap().geometry(name).unwrap()
}

/// Edges of the binary STL keyed by exact f32 vertices: the count not paired once each way.
/// Slicers stitch by exact coordinates, so 0 means the file loads as closed shells.
fn unpaired(stl: &[u8]) -> usize {
    let n = u32::from_le_bytes(stl[80..84].try_into().unwrap()) as usize;
    let mut e: HashMap<([u32; 3], [u32; 3]), i32> = HashMap::new();
    for i in 0..n {
        let f = |k: usize| {
            let o = 84 + 50 * i + 12 + 12 * k;
            [0, 4, 8].map(|j| u32::from_le_bytes(stl[o + j..o + j + 4].try_into().unwrap()))
        };
        let v = [f(0), f(1), f(2)];
        for k in 0..3 {
            *e.entry((v[k], v[(k + 1) % 3])).or_default() += 1;
        }
    }
    e.iter().filter(|(&(a, b), &c)| c != 1 || e.get(&(b, a)) != Some(&1)).count()
}

fn check(name: &str, m: &Mesh) {
    assert!(!m.shells.is_empty(), "{name}: empty");
    for s in &m.shells {
        assert!(closed(s), "{name}: open shell");
        assert!(s.volume() > 0.0, "{name}: inward shell");
    }
    assert!(m.bounds()[0][2] > -1e-9, "{name}: below the floor");
    assert_eq!(unpaired(&m.stl(name)), 0, "{name}: shells share vertices or edges");
}

#[test]
fn bundled_boards_give_clean_printable_parts() {
    for b in BOARDS {
        for switch in [Switch::Mx, Switch::Choc] {
            let g = board(b);
            let c = g.case(&CaseParams::for_switch(switch), 1.25).unwrap();
            let r = &c.report;
            assert!(r.blocked.is_empty(), "{b}: switches blocked at {:?}", r.blocked);
            assert_eq!(r.keys, g.physical_len());
            let split = !matches!(b, "ansi-30" | "ortho-30");
            assert_eq!(r.pieces.len(), if split { 2 } else { 1 }, "{b}");
            for (part, m) in [("plate", &c.plate), ("case", &c.case()), ("bottom", &c.bottom)] {
                check(&format!("{b} {part}"), m);
            }
            assert_eq!(c.bottom.shells.len(), r.pieces.len());
        }
    }
}

#[test]
fn flat_board_sits_at_the_clearance() {
    let p = CaseParams::default();
    let c = board("ortho-30").case(&p, 1.25).unwrap();
    // Plate top = floor + clearance + switch depth; the 10×3 grid is 10 u by 3 u of tiles.
    let b = c.plate.bounds();
    assert!((b[1][2] - (p.clearance + p.switch_depth)).abs() < 1e-9);
    assert!((b[1][2] - b[0][2] - p.plate).abs() < 1e-9);
    assert!((b[1][0] - b[0][0] - (9.0 * 19.05 + 17.4)).abs() < 1e-6);
    assert!((b[1][1] - b[0][1] - (2.0 * 19.05 + 17.4)).abs() < 1e-6);
    assert!(c.report.dropped == 0 && c.report.cap_hits.is_empty());
    // Every exterior edge of the grid has a wall: 2·10 + 2·3.
    assert_eq!(c.report.walls, 26);
}

#[test]
fn layout_is_not_mirrored() {
    // STL is right-handed: board y (toward the user) becomes −y. The left hand stays at low x
    // and a key further from the user gets a larger STL y.
    let g = board("corne-36");
    let c = g.case(&CaseParams::default(), 1.25).unwrap();
    let centre = |k: usize| {
        let b = c.plate.shells[k].bounds();
        [(b[0][0] + b[1][0]) / 2.0, (b[0][1] + b[1][1]) / 2.0]
    };
    let (top, bottom) = (g.key.iter().position(|k| k.row == 0).unwrap(), g.key.iter().position(|k| k.row == 2).unwrap());
    assert!(centre(top)[1] > centre(bottom)[1]);
    let left = g.key.iter().position(|k| k.finger == gecko_core::Finger::LP).unwrap();
    let right = g.key.iter().position(|k| k.finger == gecko_core::Finger::RP).unwrap();
    assert!(centre(left)[0] < centre(right)[0]);
}

#[test]
fn bad_params_fail() {
    let g = board("ortho-30");
    assert!(g.case(&CaseParams { plate: 0.0, ..CaseParams::default() }, 1.25).is_err());
    assert!(g.case(&CaseParams { wall: f64::NAN, ..CaseParams::default() }, 1.25).is_err());
}
