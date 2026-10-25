//! The objective as quadratic-assignment terms, for lower bounds (`crate::bound`).

use super::*;

impl Scorer {
    /// The objective as quadratic-assignment terms for lower bounds (`crate::bound`).
    /// With `trigrams`, each trigram is relaxed to pair terms that never exceed its cost.
    /// `None` if a load term has a negative weight (no simple bound on it).
    pub(crate) fn qap_terms(&self, trigrams: bool) -> Option<QapTerms> {
        if [self.w_load, self.w_hand, self.w_rss, self.w_peak, self.w_thumb].iter().any(|&w| w < 0.0) {
            return None;
        }
        let n = self.n;
        let mut mono = vec![0.0f64; n * n];
        for x in 0..n {
            for i in 0..n {
                let mut v = self.f1[x] as f64 * self.c1[i] as f64;
                if self.has_shift {
                    v += self.fsh[x] as f64 * self.csh[i] as f64;
                }
                v += self.cp[x * n + i] as f64;
                mono[x * n + i] = v;
            }
        }
        let mut chans: Vec<(Vec<f32>, Vec<f32>)> = Vec::new();
        if self.has_pair {
            chans.push((self.f2.clone(), self.c2.clone()));
            chans.push((self.fk.clone(), self.ck.clone()));
            if self.has_far {
                chans.push((self.fkf.clone(), self.ckf.clone()));
            }
            if self.has_sym {
                chans.push((self.fy.clone(), self.cy.clone()));
            }
        }
        if self.has_shift_pair {
            chans.push((self.fsb.clone(), self.csb.clone()));
        }
        for b in 0..crate::metrics::SPARSE.len() {
            let off = (b * n * n) as u32;
            if !self.brackets.iter().any(|e| e.t == off) {
                continue;
            }
            let mut fb = vec![0.0f32; n * n];
            for e in self.brackets.iter().filter(|e| e.t == off) {
                fb[e.x as usize * n + e.y as usize] += e.f;
            }
            chans.push((fb, self.cb[off as usize..off as usize + n * n].to_vec()));
        }
        if trigrams && self.has_tri {
            let c3 = |i: usize, j: usize, k: usize| self.c3[(i * n + j) * n + k];
            // Distinct x y z: c3[i,j,k] ≥ ⅓(min_k c3 + min_i c3 + min_j c3), the third key on
            // any slot other than the two fixed ones. Repeated symbols: exact pair/mono terms.
            let mut tabs: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.0f32; n * n]);
            for i in 0..n {
                for j in 0..n {
                    let (mut a, mut b, mut c) = (f32::INFINITY, f32::INFINITY, f32::INFINITY);
                    for k in 0..n {
                        if k != i && k != j {
                            a = a.min(c3(i, j, k)); // (x, y) at (i, j)
                            b = b.min(c3(k, i, j)); // (y, z) at (i, j)
                            c = c.min(c3(i, k, j)); // (x, z) at (i, j)
                        }
                    }
                    let ij = i * n + j;
                    let fin = |v: f32| if v.is_finite() { v } else { 0.0 };
                    tabs[0][ij] = fin(a);
                    tabs[1][ij] = fin(b);
                    tabs[2][ij] = fin(c);
                    tabs[3][ij] = c3(i, j, i); // x = z: pair (x, y)
                    tabs[4][ij] = c3(i, i, j); // x = y: pair (x, z)
                    tabs[5][ij] = c3(i, j, j); // y = z: pair (x, y)
                }
            }
            let mut flows: [Vec<f32>; 6] = std::array::from_fn(|_| vec![0.0f32; n * n]);
            for e in &self.tri_all {
                let (x, y, z) = (e.x as usize, e.y as usize, e.z as usize);
                match (x == y, y == z, x == z) {
                    (true, true, _) => {
                        for i in 0..n {
                            mono[x * n + i] += e.f as f64 * c3(i, i, i) as f64;
                        }
                    }
                    (false, false, true) => flows[3][x * n + y] += e.f,
                    (true, false, false) => flows[4][x * n + z] += e.f,
                    (false, true, false) => flows[5][x * n + y] += e.f,
                    _ => {
                        let t = e.f / 3.0;
                        flows[0][x * n + y] += t;
                        flows[1][y * n + z] += t;
                        flows[2][x * n + z] += t;
                    }
                }
            }
            for (f, c) in flows.into_iter().zip(tabs) {
                if f.iter().any(|&v| v != 0.0) {
                    chans.push((f, c));
                }
            }
        }
        // Same-symbol pairs (x = x) sit on one slot: fold them into the mono terms.
        for (f, c) in &chans {
            for x in 0..n {
                let fx = f[x * n + x] as f64;
                if fx != 0.0 {
                    for i in 0..n {
                        mono[x * n + i] += fx * c[i * n + i] as f64;
                    }
                }
            }
        }
        Some(QapTerms { n, mono, chans })
    }
}

/// The objective as `Σ mono[x, p(x)] + Σ_{x≠y} Σ_ch F[x, y]·C[p(x), p(y)]` (see
/// [`Scorer::qap_terms`]); load terms (all weights ≥ 0) are dropped, which only lowers it.
pub(crate) struct QapTerms {
    pub n: usize,
    /// `mono[x * n + i]`: symbol x on slot i, including same-symbol pair terms.
    pub mono: Vec<f64>,
    /// (flow `F[x * n + y]`, cost `C[i * n + j]`) per pair channel.
    pub chans: Vec<(Vec<f32>, Vec<f32>)>,
}
