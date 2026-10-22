//! Ranking calibration must hold out whole layout groups and never fit their labels.
use std::path::Path;

use gecko_core::calibrate::fit_preferences;
use gecko_core::interop::community::Bundle;
use gecko_core::store::Store;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

#[test]
fn held_out_labels_and_duplicates_do_not_affect_weights_or_scales() {
    let store = Store::open(Path::new(ROOT)).unwrap();
    let mut bundle = Bundle {
        schema: 1,
        source: "synthetic preferences over real layouts".into(),
        layouts: store.community().unwrap().unique().filter(|l| l.likes > 0).take(10).cloned().collect(),
        skipped: Vec::new(),
    };
    for (i, l) in bundle.layouts.iter_mut().enumerate() {
        l.likes = i as u32 + 1;
    }
    let mut duplicate = bundle.layouts[0].clone();
    duplicate.dup_of = Some(duplicate.name.clone());
    duplicate.name = "duplicate must not leak across split".into();
    duplicate.likes = 999;
    bundle.layouts.push(duplicate);
    let g = store.geometry("ansi-30").unwrap();
    let corpus = store.corpus("english").unwrap();
    let profile = store.profile("default").unwrap();
    let fit = fit_preferences(&g, &corpus, &profile, &bundle, 3).unwrap();
    assert_eq!(fit.training_layouts.len(), 8);
    assert_eq!(fit.held_out_layouts.len(), 2);
    assert_eq!(fit.training_pairs, 28);
    assert_eq!(fit.held_out_pairs, 1);
    assert!(fit.training_layouts.iter().all(|n| !fit.held_out_layouts.contains(n)));
    assert!(fit.weights.values().all(|w| w.is_finite() && *w >= 0.0));
    assert!(fit.scales.values().all(|s| s.is_finite() && *s > 0.0));
    for l in &mut bundle.layouts {
        if fit.held_out_layouts.contains(&l.name) {
            l.likes = 1000 - l.likes;
        }
        if l.dup_of.is_some() {
            l.likes = 12345;
        }
    }
    let changed = fit_preferences(&g, &corpus, &profile, &bundle, 3).unwrap();
    assert_eq!(fit.training_layouts, changed.training_layouts);
    assert_eq!(fit.weights, changed.weights);
    assert_eq!(fit.scales, changed.scales);
    assert_eq!(fit.training_loss, changed.training_loss);
    assert_eq!(fit.held_out_accuracy.unwrap() + changed.held_out_accuracy.unwrap(), 1.0);
    bundle.layouts.truncate(3);
    assert!(fit_preferences(&g, &corpus, &profile, &bundle, 3).is_err());
}
