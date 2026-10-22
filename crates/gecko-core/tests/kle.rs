//! KLE import/export round-trips the bundled boards. See `context/representation.md#geometry`.

use gecko_core::interop::kle;
use gecko_core::Geometry;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn geometry(name: &str) -> Geometry {
    Geometry::from_toml(&std::fs::read_to_string(format!("{ROOT}/data/geometries/{name}.toml")).unwrap()).unwrap()
}

fn close(a: &[f32], b: &[f32]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-3)
}

/// Geometry → KLE → geometry keeps the physical keys exactly and re-guesses each bundled board's
/// fingers, rows, columns and stretch keys (home keys too, except corne/sweep mark their stretch
/// keys home); ortho-30's efforts are the guess's base table.
#[test]
fn bundled_boards_round_trip_through_kle() {
    for name in ["ortho-30", "ansi-30", "corne-36", "sweep-34"] {
        let g = geometry(name);
        let text = serde_json::to_string(&kle::export(&g)).unwrap();
        let back = kle::import(name, &text).unwrap();
        assert_eq!(back.len(), g.len(), "{name}");
        assert_eq!(back.row_lengths(), g.row_lengths(), "{name}");
        for (i, (a, b)) in g.key.iter().zip(&back.key).enumerate() {
            let at = format!("{name} key {i}");
            assert!(close(&a.pos, &b.pos) && close(&a.rot, &b.rot) && close(&a.size, &b.size), "{at}: {a:?} vs {b:?}");
            assert_eq!((a.finger, a.row, a.col, a.stretch), (b.finger, b.row, b.col, b.stretch), "{at}");
            assert_eq!(a.home && !a.stretch, b.home, "{at}");
            if name == "ortho-30" {
                assert_eq!(a.effort, b.effort, "{at}");
            }
        }
        // The written TOML parses back to the same geometry, and a second KLE pass is a fixed point.
        let again = Geometry::from_toml(&back.to_toml()).unwrap();
        assert_eq!(again.hash(), back.hash(), "{name}: to_toml round-trip");
        let twice = kle::import(name, &serde_json::to_string(&kle::export(&again)).unwrap()).unwrap();
        assert_eq!(twice.hash(), back.hash(), "{name}: KLE fixed point");
    }
}
