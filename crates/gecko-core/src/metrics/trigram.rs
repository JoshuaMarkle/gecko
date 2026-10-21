//! Trigram classification (T01–T08). Every slot triple gets exactly one class, so class
//! shares sum to 100% of trigrams and reports stay easy to read.

use serde::Serialize;

use crate::geometry::Finger;
use crate::Geometry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TriClass {
    Alternate,
    AlternateSfs,
    RollIn,
    RollOut,
    Onehand,
    Redirect,
    BadRedirect,
    Sft,
    /// A thumb key next to a key on the same hand (other finger), e.g. index → space.
    ThumbRoll,
    /// Every step into or out of a thumb key changes hands, e.g. left key → right thumb → left key.
    ThumbAlt,
    /// Contains a same-finger bigram (thumbs included) or a repeated key.
    Other,
}

pub fn class_for(id: &str) -> Option<TriClass> {
    Some(match id {
        "T01" => TriClass::Alternate,
        "T02" => TriClass::AlternateSfs,
        "T03" => TriClass::RollIn,
        "T04" => TriClass::RollOut,
        "T05" => TriClass::Onehand,
        "T06" => TriClass::Redirect,
        "T07" => TriClass::BadRedirect,
        "T08" => TriClass::Sft,
        "T12" => TriClass::ThumbRoll,
        "T13" => TriClass::ThumbAlt,
        _ => return None,
    })
}

pub fn classify(g: &Geometry, i: usize, j: usize, k: usize) -> TriClass {
    let (a, b, c) = (&g.key[i], &g.key[j], &g.key[k]);
    if g.same_key(i, j) || g.same_key(j, k) {
        return TriClass::Other;
    }
    if a.finger.is_thumb() || b.finger.is_thumb() || c.finger.is_thumb() {
        return thumb_class(a.finger, b.finger, c.finger);
    }
    let (ha, hb, hc) = (a.hand(), b.hand(), c.hand());
    let (fa, fb, fc) = (a.finger, b.finger, c.finger);
    if ha == hb && hb == hc {
        if fa == fb && fb == fc {
            return TriClass::Sft;
        }
        if fa == fb || fb == fc {
            return TriClass::Other;
        }
        let (oa, ob, oc) = (fa.ord(), fb.ord(), fc.ord());
        if (oa < ob && ob < oc) || (oa > ob && ob > oc) {
            return TriClass::Onehand;
        }
        // Direction changes (includes fa == fc).
        if oa != 3 && ob != 3 && oc != 3 {
            return TriClass::BadRedirect;
        }
        return TriClass::Redirect;
    }
    if ha != hb && hb != hc {
        return if fa == fc && i != k { TriClass::AlternateSfs } else { TriClass::Alternate };
    }
    // Roll: exactly one same-hand consecutive pair.
    let (x, y) = if ha == hb { (fa, fb) } else { (fb, fc) };
    if x == y {
        return TriClass::Other;
    }
    if y.ord() > x.ord() {
        TriClass::RollIn
    } else {
        TriClass::RollOut
    }
}

/// Class of a trigram with at least one thumb key (no repeated keys). Only the steps that touch
/// a thumb decide roll vs alternate; any same-finger step makes it `Other`, as without thumbs.
fn thumb_class(fa: Finger, fb: Finger, fc: Finger) -> TriClass {
    if fa == fb || fb == fc {
        return TriClass::Other;
    }
    let roll = [(fa, fb), (fb, fc)].iter().any(|&(x, y)| (x.is_thumb() || y.is_thumb()) && x.hand() == y.hand());
    if roll {
        TriClass::ThumbRoll
    } else {
        TriClass::ThumbAlt
    }
}

/// `[N×N×N]` table: 100 where the triple has class `cls`, else 0.
pub fn table(g: &Geometry, cls: TriClass) -> Vec<f32> {
    let n = g.len();
    let mut t = vec![0.0; n * n * n];
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                if classify(g, i, j, k) == cls {
                    t[(i * n + j) * n + k] = 100.0;
                }
            }
        }
    }
    t
}
