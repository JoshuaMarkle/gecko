//! `interop::svg`: one keycap per key and layer, heat mapped from scorer slots back to
//! (layer, physical key), captions for layered layouts.

use std::path::Path;

use gecko_core::interop::svg::{board_svg, layer_heat, KeyColor, SvgOptions};
use gecko_core::report::analyze;
use gecko_core::store::Store;
use gecko_core::Scorer;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn keycaps(svg: &str) -> usize {
    svg.matches(r#"rx="6""#).count()
}

#[test]
fn plain_board_draws_every_key_with_a_title() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let g = store.geometry("ansi-30").unwrap();
    let lay = store.layout("qwerty", &g.name).unwrap().fit(&g).unwrap();
    let o = SvgOptions { color: KeyColor::Plain, layers: true, title: Some(("Q<W>".into(), "sub".into())) };
    let svg = board_svg(&g, &lay, &[], &o);
    assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
    assert_eq!(keycaps(&svg), 30);
    assert!(svg.contains("Q&lt;W&gt;"), "title is escaped");
    assert!(!svg.contains("Base"), "no captions without layers");
}

#[test]
fn layered_heat_follows_the_scorer_slots() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let g = store.geometry("corne-36").unwrap();
    let path = format!("{ROOT}/data/layouts/layered/corne-code-oneshot.toml");
    let lay = store.layout(&path, &g.name).unwrap().fit(&g).unwrap();
    let sc = Scorer::new(&g, &store.corpus("code").unwrap(), &store.profile("default").unwrap(), &lay).unwrap();
    let a = analyze(&sc, &sc.identity(), 0);
    let heat = layer_heat(&g, &lay, &a).unwrap();
    assert_eq!(heat.len(), lay.layers.len() + 1);
    let total: f64 = heat.iter().flatten().sum();
    assert!((total - a.heat.iter().sum::<f64>()).abs() < 1e-9, "every slot lands on one (layer, key)");
    for (l, row) in heat.iter().enumerate().skip(1) {
        for (k, &h) in row.iter().enumerate() {
            if h > 0.0 {
                assert_ne!(lay.layers[l - 1].chars[k], gecko_core::layout::EMPTY, "heat on an empty layer key");
            }
        }
    }

    let o = SvgOptions { color: KeyColor::Heat, layers: true, title: None };
    let svg = board_svg(&g, &lay, &heat, &o);
    assert_eq!(keycaps(&svg), g.len() * heat.len());
    assert!(svg.contains("Layer 1 · one-shot OS1"));
    let base = board_svg(&g, &lay, &heat, &SvgOptions { layers: false, ..o });
    assert_eq!(keycaps(&base), g.len());
}
