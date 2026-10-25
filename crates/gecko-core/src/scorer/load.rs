//! Finger-load costs (finger_target, finger_evenness, hand_balance, finger_peak, thumb_load) as plain functions of a load vector.

use crate::geometry::FINGERS;

const NON_THUMB: [usize; 8] = [0, 1, 2, 3, 6, 7, 8, 9];

/// Non-thumb finger loads as % of non-thumb keystrokes (indexed like `load`; thumbs 0).
fn non_thumb_shares(load: &[f64; FINGERS]) -> [f64; FINGERS] {
    let total: f64 = NON_THUMB.iter().map(|&f| load[f]).sum();
    let k = if total > 0.0 { 100.0 / total } else { 0.0 };
    let mut s = [0.0; FINGERS];
    for f in NON_THUMB {
        s[f] = k * load[f];
    }
    s
}

/// finger_evenness cost: root-sum-square of non-thumb finger shares (%). 35.4 when all eight are even.
pub fn balance_rss(load: &[f64; FINGERS]) -> f64 {
    non_thumb_shares(load).iter().map(|s| s * s).sum::<f64>().sqrt()
}

/// finger_peak cost: the highest non-thumb finger share as % of its target share (fingers with a
/// target of 0 are skipped). 100 = no finger above its target.
pub fn peak_load(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
    let s = non_thumb_shares(load);
    NON_THUMB.iter().filter(|&&f| target[f] > 0.0).map(|&f| 100.0 * s[f] / target[f]).fold(0.0, f64::max)
}

/// finger_target cost: mean squared gap (%²) between finger loads and targets over non-thumb fingers.
/// Loads are taken as shares of non-thumb keystrokes, so thumb keys (Space) don't skew them.
pub fn load_error_cost(load: &[f64; FINGERS], target: &[f64; FINGERS]) -> f64 {
    if load[4] == 0.0 && load[5] == 0.0 {
        return NON_THUMB.iter().map(|&f| (load[f] - target[f]).powi(2)).sum::<f64>() / 8.0;
    }
    let total: f64 = NON_THUMB.iter().map(|&f| load[f]).sum();
    let k = if total > 0.0 { 100.0 / total } else { 0.0 };
    let mut s = 0.0;
    for f in NON_THUMB {
        let e = k * load[f] - target[f];
        s += e * e;
    }
    s / 8.0
}

/// hand_balance value: left-hand share in % of keystrokes.
pub fn left_share(load: &[f64; FINGERS]) -> f64 {
    100.0 * load[..5].iter().sum::<f64>() / load.iter().sum::<f64>().max(1e-12)
}

/// thumb_load value: thumb share (both thumbs) in % of keystrokes.
pub fn thumb_share(load: &[f64; FINGERS]) -> f64 {
    100.0 * (load[4] + load[5]) / load.iter().sum::<f64>().max(1e-12)
}

/// Squared distance of a share from its target, scaled by 1/10 (hand_balance, thumb_load).
pub fn target_cost(share: f64, target: f64) -> f64 {
    (share - target) * (share - target) / 10.0
}

/// hand_balance cost: squared distance of the left-hand share from 50%, scaled by 1/10.
pub fn hand_cost(load: &[f64; FINGERS]) -> f64 {
    if load.iter().sum::<f64>() <= 0.0 {
        return 0.0;
    }
    target_cost(left_share(load), 50.0)
}

/// thumb_load cost: squared distance of the thumb share from `target`, scaled by 1/10.
pub fn thumb_cost(load: &[f64; FINGERS], target: f64) -> f64 {
    if load.iter().sum::<f64>() <= 0.0 {
        return 0.0;
    }
    target_cost(thumb_share(load), target)
}
