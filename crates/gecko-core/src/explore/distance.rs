//! Distances between layouts (`sym[slot]` arrays over the same symbol set) and mirror
//! canonicalization. `research/search-space-visualization.md` §1.

use crate::rng::Rng;
use crate::scorer::Scorer;
use crate::Geometry;

/// Slots holding different symbols.
pub fn hamming(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).filter(|(x, y)| x != y).count()
}

/// Minimum number of swaps turning `a` into `b` (Cayley distance): n − cycles(a⁻¹∘b).
pub fn swaps(a: &[u8], b: &[u8]) -> usize {
    let n = a.len();
    let mut pos_b = vec![0usize; n];
    for (slot, &s) in b.iter().enumerate() {
        pos_b[s as usize] = slot;
    }
    let mut seen = vec![false; n];
    let mut cycles = 0;
    for start in 0..n {
        if seen[start] {
            continue;
        }
        cycles += 1;
        let mut i = start;
        while !seen[i] {
            seen[i] = true;
            i = pos_b[a[i] as usize];
        }
    }
    n - cycles
}

/// Symbols typed by a different finger (`finger[slot]`).
pub fn finger_distance(a: &[u8], b: &[u8], finger: &[u8]) -> usize {
    let n = a.len();
    let mut fa = vec![0u8; n];
    for (slot, &s) in a.iter().enumerate() {
        fa[s as usize] = finger[slot];
    }
    b.iter().enumerate().filter(|&(slot, &s)| fa[s as usize] != finger[slot]).count()
}

/// Left/right mirror of a geometry: `m[slot]` is the slot at the mirrored position with the
/// mirrored finger. `None` if the board is not mirror-symmetric (e.g. row stagger).
pub fn mirror_map(g: &Geometry) -> Option<Vec<usize>> {
    let n = g.len();
    let (lo, hi) = g.key.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), k| (lo.min(k.pos[0]), hi.max(k.pos[0] + k.size[0])));
    let mid = (lo + hi) / 2.0;
    let mut m = vec![usize::MAX; n];
    for (i, k) in g.key.iter().enumerate() {
        let x = 2.0 * mid - (k.pos[0] + k.size[0]);
        let mirrored = crate::Finger::ALL[9 - k.finger.index()];
        let j = g
            .key
            .iter()
            .position(|o| (o.pos[0] - x).abs() < 1e-3 && (o.pos[1] - k.pos[1]).abs() < 1e-3 && o.finger == mirrored && o.row == k.row)?;
        m[i] = j;
    }
    Some(m)
}

/// The mirror map if mirroring provably leaves every score unchanged (checked on random layouts).
pub fn score_symmetry(sc: &Scorer) -> Option<Vec<usize>> {
    let m = mirror_map(&sc.geometry)?;
    let mut rng = Rng::new(0x5eed);
    for _ in 0..8 {
        let mut sym: Vec<u8> = (0..sc.n as u8).collect();
        rng.shuffle(&mut sym);
        let a = sc.state_from_sym(sym.clone()).score;
        let b = sc.state_from_sym(mirror(&sym, &m)).score;
        if (a - b).abs() > 1e-6 * a.abs().max(1.0) {
            return None;
        }
    }
    Some(m)
}

/// The layout with every symbol moved to its mirrored slot.
pub fn mirror(sym: &[u8], m: &[usize]) -> Vec<u8> {
    let mut out = vec![0u8; sym.len()];
    for (slot, &s) in sym.iter().enumerate() {
        out[m[slot]] = s;
    }
    out
}

/// Canonical form: the lexicographically smaller of a layout and its mirror image.
pub fn canonical(sym: &[u8], m: Option<&[usize]>) -> Vec<u8> {
    match m {
        Some(m) => {
            let r = mirror(sym, m);
            if r.as_slice() < sym {
                r
            } else {
                sym.to_vec()
            }
        }
        None => sym.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances() {
        let a = [0u8, 1, 2, 3, 4];
        assert_eq!(hamming(&a, &a), 0);
        assert_eq!(swaps(&a, &a), 0);
        let b = [1u8, 0, 2, 3, 4];
        assert_eq!((hamming(&a, &b), swaps(&a, &b)), (2, 1));
        let c = [1u8, 2, 0, 3, 4]; // 3-cycle: two swaps
        assert_eq!((hamming(&a, &c), swaps(&a, &c)), (3, 2));
        let d = [1u8, 0, 3, 2, 4];
        assert_eq!(swaps(&a, &d), 2);
        let finger = [0u8, 0, 1, 1, 2];
        assert_eq!(finger_distance(&a, &b, &finger), 0, "swap within one finger");
        assert_eq!(finger_distance(&a, &c, &finger), 2);
    }

    #[test]
    fn mirror_roundtrip() {
        let m = [4usize, 3, 2, 1, 0];
        let s = [3u8, 0, 1, 4, 2];
        assert_eq!(mirror(&mirror(&s, &m), &m), s.to_vec());
        assert_eq!(canonical(&s, Some(&m)), canonical(&mirror(&s, &m), Some(&m)));
    }
}
