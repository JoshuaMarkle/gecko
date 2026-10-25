//! The hot loop: full evaluation, swap deltas and `apply`. No allocation, no hash maps, no
//! dynamic dispatch (see `context/conventions.md`); `delta == full` is property-tested.

use super::*;

impl Scorer {
    pub fn loads(&self, pos: &[u8]) -> [f64; FINGERS] {
        let mut l = [0.0; FINGERS];
        for (x, &p) in pos.iter().enumerate() {
            let v = 100.0 * self.f1[x] as f64;
            l[self.finger[p as usize] as usize] += v;
            if self.hold[p as usize] != NO_HOLD {
                l[self.hold[p as usize] as usize] += v;
            }
        }
        l
    }

    /// Move `moved` % of keystrokes onto slot `pa`'s fingers and off slot `pb`'s (a swap).
    #[inline]
    pub(super) fn shift_load(&self, l: &mut [f64; FINGERS], pa: usize, pb: usize, moved: f64) {
        l[self.finger[pa] as usize] += moved;
        l[self.finger[pb] as usize] -= moved;
        if self.hold[pa] != NO_HOLD {
            l[self.hold[pa] as usize] += moved;
        }
        if self.hold[pb] != NO_HOLD {
            l[self.hold[pb] as usize] -= moved;
        }
    }

    /// Slots `pa` and `pb` load the same fingers (a swap leaves loads unchanged).
    #[inline]
    pub(super) fn same_load(&self, pa: usize, pb: usize) -> bool {
        self.finger[pa] == self.finger[pb] && self.hold[pa] == self.hold[pb]
    }

    pub(super) fn load_cost(&self, load: &[f64; FINGERS]) -> f64 {
        let mut c = 0.0;
        if self.w_load != 0.0 {
            c += self.w_load * load_error_cost(load, &self.target);
        }
        if self.w_hand != 0.0 {
            c += self.w_hand * hand_cost(load);
        }
        if self.w_rss != 0.0 {
            c += self.w_rss * balance_rss(load);
        }
        if self.w_peak != 0.0 {
            c += self.w_peak * peak_load(load, &self.target);
        }
        if self.w_thumb != 0.0 {
            c += self.w_thumb * thumb_cost(load, self.thumb_target);
        }
        c
    }

    /// Full evaluation from scratch. Used for verification and reports, not in the search loop.
    pub fn full(&self, st: &State) -> f64 {
        let n = self.n;
        let pos = &st.pos;
        let mut sum = 0.0f64;
        for x in 0..n {
            sum += self.f1[x] as f64 * self.c1[pos[x] as usize] as f64;
        }
        if self.has_shift {
            for x in 0..n {
                sum += self.fsh[x] as f64 * self.csh[pos[x] as usize] as f64;
            }
        }
        if self.has_place {
            for x in 0..n {
                sum += self.cp[x * n + pos[x] as usize] as f64;
            }
        }
        if self.has_shift_pair {
            for x in 0..n {
                for y in 0..n {
                    sum += self.fsb[x * n + y] as f64 * self.csb[pos[x] as usize * n + pos[y] as usize] as f64;
                }
            }
        }
        if self.has_pair {
            for x in 0..n {
                let px = pos[x] as usize * n;
                for y in 0..n {
                    let (xy, p) = (x * n + y, px + pos[y] as usize);
                    sum += self.f2[xy] as f64 * self.c2[p] as f64
                        + self.fk[xy] as f64 * self.ck[p] as f64
                        + self.fkf[xy] as f64 * self.ckf[p] as f64
                        + self.fy[xy] as f64 * self.cy[p] as f64;
                }
            }
        }
        if self.has_tri {
            for e in &self.tri_all {
                let i = (pos[e.x as usize] as usize * n + pos[e.y as usize] as usize) * n + pos[e.z as usize] as usize;
                sum += e.f as f64 * self.c3[i] as f64;
            }
        }
        for e in &self.brackets {
            sum += e.f as f64 * self.cb[e.t as usize + pos[e.x as usize] as usize * n + pos[e.y as usize] as usize] as f64;
        }
        sum + self.load_cost(&self.loads(pos))
    }

    /// Score change if symbols `a` and `b` swap slots. No allocation.
    #[inline]
    pub fn delta(&self, st: &State, a: usize, b: usize) -> f64 {
        self.delta_linear(st, a, b) + self.delta_load(st, a, b)
    }

    /// The part of [`Scorer::delta`] from the n-gram terms (everything but the finger-load
    /// terms, which are not linear in the frequencies).
    #[inline]
    pub fn delta_linear(&self, st: &State, a: usize, b: usize) -> f64 {
        if a == b {
            return 0.0;
        }
        let n = self.n;
        let pos = &st.pos;
        let (pa, pb) = (pos[a] as usize, pos[b] as usize);
        let mut old = [0u8; MAX_KEYS];
        let mut new = [0u8; MAX_KEYS];
        old[..n].copy_from_slice(pos);
        new[..n].copy_from_slice(pos);
        new[a] = pb as u8;
        new[b] = pa as u8;
        let (old, new) = (&old[..n], &new[..n]);

        let (c1, f1) = (&self.c1, &self.f1);
        let mut d = (f1[a] * (c1[pb] - c1[pa]) + f1[b] * (c1[pa] - c1[pb])) as f64;
        if self.has_shift {
            let (csh, fsh) = (&self.csh, &self.fsh);
            d += (fsh[a] * (csh[pb] - csh[pa]) + fsh[b] * (csh[pa] - csh[pb])) as f64;
        }
        if self.has_place {
            let cp = &self.cp;
            d += (cp[a * n + pb] - cp[a * n + pa] + cp[b * n + pa] - cp[b * n + pb]) as f64;
        }

        if self.has_pair || self.has_shift_pair {
            d += match (self.has_far, self.has_sym, self.has_shift_pair) {
                (false, false, false) => self.pair_delta::<false, false, false>(old, new, a, b),
                (true, false, false) => self.pair_delta::<true, false, false>(old, new, a, b),
                (false, true, false) => self.pair_delta::<false, true, false>(old, new, a, b),
                (true, true, false) => self.pair_delta::<true, true, false>(old, new, a, b),
                (false, false, true) => self.pair_delta::<false, false, true>(old, new, a, b),
                (true, false, true) => self.pair_delta::<true, false, true>(old, new, a, b),
                (false, true, true) => self.pair_delta::<false, true, true>(old, new, a, b),
                (true, true, true) => self.pair_delta::<true, true, true>(old, new, a, b),
            };
        }

        if self.has_tri {
            // Trigrams dominate the delta (hundreds per symbol). A 256-entry slot table lets
            // `u8` symbol indices skip bounds checks.
            let mut slot = [0u8; 256];
            slot[..n].copy_from_slice(new);
            let (c3, cost) = (&self.c3[..], &st.tri_cost[..]);
            let tri = |e: &TriEntry| {
                let i = (slot[e.x as usize] as usize * n + slot[e.y as usize] as usize) * n + slot[e.z as usize] as usize;
                e.f * (c3[i] - cost[e.id as usize])
            };
            let mut acc = 0.0f64;
            let ra = self.tri_start[a] as usize..self.tri_start[a + 1] as usize;
            for e in &self.tri_entries[ra] {
                acc += tri(e) as f64;
            }
            let (a8, rb) = (a as u8, self.tri_start[b] as usize..self.tri_start[b + 1] as usize);
            for e in &self.tri_entries[rb] {
                if e.x == a8 || e.y == a8 || e.z == a8 {
                    continue; // already counted with `a`
                }
                acc += tri(e) as f64;
            }
            d += acc;
        }
        for e in &self.brackets {
            let (x, y) = (e.x as usize, e.y as usize);
            if x == a || x == b || y == a || y == b {
                let cb = &self.cb[e.t as usize..];
                d += (e.f * (cb[new[x] as usize * n + new[y] as usize] - cb[old[x] as usize * n + old[y] as usize])) as f64;
            }
        }
        d
    }

    /// The finger-load part of [`Scorer::delta`] (M02, M03, M04, M07).
    #[inline]
    pub fn delta_load(&self, st: &State, a: usize, b: usize) -> f64 {
        if !self.has_load {
            return 0.0;
        }
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        if self.same_load(pa, pb) {
            return 0.0;
        }
        let mut l = st.load;
        let moved = 100.0 * (self.f1[b] as f64 - self.f1[a] as f64);
        self.shift_load(&mut l, pa, pb, moved);
        self.load_cost(&l) - self.load_cost(&st.load)
    }

    /// Change in `delta_linear(u, v)` caused by the swap of `r` and `s`, which `st` already
    /// includes. `{u, v}` and `{r, s}` must be disjoint. Only terms holding a symbol of each
    /// pair change: Taillard's O(1) QAP update for the pair channels, plus the trigrams that
    /// contain both pairs (via the pair index). Lets tabu search keep a Δ matrix up to date
    /// without recomputing every swap. No allocation.
    #[inline]
    pub fn delta_shift(&self, st: &State, r: usize, s: usize, u: usize, v: usize) -> f64 {
        let n = self.n;
        let pos = &st.pos;
        // Slots before the r/s swap.
        let (pr, ps, pu, pv) = (pos[s] as usize, pos[r] as usize, pos[u] as usize, pos[v] as usize);
        let mut d = 0.0f64;
        if self.has_pair || self.has_shift_pair {
            let chan = |f: &[f32], c: &[f32]| {
                let fwd = f[u * n + r] - f[v * n + r] - f[u * n + s] + f[v * n + s];
                let rev = f[r * n + u] - f[r * n + v] - f[s * n + u] + f[s * n + v];
                let g = c[pv * n + ps] - c[pu * n + ps] - c[pv * n + pr] + c[pu * n + pr];
                let h = c[ps * n + pv] - c[ps * n + pu] - c[pr * n + pv] + c[pr * n + pu];
                (fwd * g + rev * h) as f64
            };
            d += chan(&self.f2, &self.c2) + chan(&self.fk, &self.ck);
            if self.has_far {
                d += chan(&self.fkf, &self.ckf);
            }
            if self.has_sym {
                d += chan(&self.fy, &self.cy);
            }
            if self.has_shift_pair {
                d += chan(&self.fsb, &self.csb);
            }
        }
        if self.has_tri {
            let c3 = &self.c3;
            // Slot of symbol `z` with the r/s swap undone (`back`) and/or u/v swapped (`uv`).
            let slot = |z: usize, back: bool, uv: bool| -> usize {
                if uv && z == u {
                    pv
                } else if uv && z == v {
                    pu
                } else if back && z == r {
                    pr
                } else if back && z == s {
                    ps
                } else {
                    pos[z] as usize
                }
            };
            let cost = |e: &TriEntry, back: bool, uv: bool| {
                let (x, y, z) = (slot(e.x as usize, back, uv), slot(e.y as usize, back, uv), slot(e.z as usize, back, uv));
                c3[(x * n + y) * n + z]
            };
            let has = |e: &TriEntry, w: usize| e.x as usize == w || e.y as usize == w || e.z as usize == w;
            let list = |a: usize, b: usize| {
                let k = a.min(b) * n + a.max(b);
                &self.tri_pair[self.tri_pair_start[k] as usize..self.tri_pair_start[k + 1] as usize]
            };
            let mut acc = 0.0f64;
            // Each trigram once: skip it in a later list if an earlier one already holds it.
            for (i, (a, b)) in [(u, r), (u, s), (v, r), (v, s)].into_iter().enumerate() {
                for e in list(a, b) {
                    let seen = match i {
                        1 => has(e, r),
                        2 => has(e, u),
                        3 => has(e, u) || has(e, r),
                        _ => false,
                    };
                    if !seen {
                        acc +=
                            (e.f * ((cost(e, false, true) - cost(e, false, false)) - (cost(e, true, true) - cost(e, true, false)))) as f64;
                    }
                }
            }
            d += acc;
        }
        // Y03 and L03 are sparse, so update only pairs spanning the two swaps.
        for e in &self.brackets {
            let (x, y) = (e.x as usize, e.y as usize);
            if !((x == r || x == s) && (y == u || y == v) || (y == r || y == s) && (x == u || x == v)) {
                continue;
            }
            let slot = |z: usize, back: bool, uv: bool| {
                if uv && z == u {
                    pv
                } else if uv && z == v {
                    pu
                } else if back && z == r {
                    pr
                } else if back && z == s {
                    ps
                } else {
                    pos[z] as usize
                }
            };
            let cost = |back, uv| self.cb[e.t as usize + slot(x, back, uv) * n + slot(y, back, uv)];
            d += (e.f * ((cost(false, true) - cost(false, false)) - (cost(true, true) - cost(true, false)))) as f64;
        }
        d
    }

    /// Bigram + skipgram (+ far skipgram if `FAR`, + symbol bigram if `SYM`, + Shift pair if
    /// `SHIFT`) part of a swap delta; `old`/`new` are `pos` before/after.
    #[inline]
    pub(super) fn pair_delta<const FAR: bool, const SYM: bool, const SHIFT: bool>(
        &self,
        old: &[u8],
        new: &[u8],
        a: usize,
        b: usize,
    ) -> f64 {
        let n = self.n;
        let (c2, ck, ckf, f2, fk, fkf) = (&self.c2, &self.ck, &self.ckf, &self.f2, &self.fk, &self.fkf);
        let (cy, fy, csb, fsb) = (&self.cy, &self.fy, &self.csb, &self.fsb);
        // Change in the pair channels for frequency index `xy` moving from slot pair `o` to `nw`.
        let term = |xy: usize, o: usize, nw: usize| {
            let mut t = f2[xy] * (c2[nw] - c2[o]) + fk[xy] * (ck[nw] - ck[o]);
            if FAR {
                t += fkf[xy] * (ckf[nw] - ckf[o]);
            }
            if SYM {
                t += fy[xy] * (cy[nw] - cy[o]);
            }
            if SHIFT {
                t += fsb[xy] * (csb[nw] - csb[o]);
            }
            t
        };
        let mut acc = 0.0f64;
        for &x in &[a, b] {
            let (ox, nx) = (old[x] as usize * n, new[x] as usize * n);
            let row = x * n;
            for y in 0..n {
                acc += term(row + y, ox + old[y] as usize, nx + new[y] as usize) as f64;
            }
        }
        for x in 0..n {
            if x == a || x == b {
                continue;
            }
            let px = old[x] as usize * n;
            let row = x * n;
            for &y in &[a, b] {
                acc += term(row + y, px + old[y] as usize, px + new[y] as usize) as f64;
            }
        }
        acc
    }

    /// Apply the swap of symbols `a` and `b`, given its precomputed delta.
    #[inline]
    pub fn apply(&self, st: &mut State, a: usize, b: usize, delta: f64) {
        let (pa, pb) = (st.pos[a] as usize, st.pos[b] as usize);
        if !self.same_load(pa, pb) {
            self.shift_load(&mut st.load, pa, pb, 100.0 * (self.f1[b] as f64 - self.f1[a] as f64));
        }
        st.pos.swap(a, b);
        st.sym.swap(pa, pb);
        st.score += delta;
        if self.has_tri && a != b {
            let ra = self.tri_start[a] as usize..self.tri_start[a + 1] as usize;
            for e in &self.tri_entries[ra] {
                st.tri_cost[e.id as usize] = self.trigram_cost(&st.pos, e);
            }
            let (a8, rb) = (a as u8, self.tri_start[b] as usize..self.tri_start[b + 1] as usize);
            for e in &self.tri_entries[rb] {
                if e.x != a8 && e.y != a8 && e.z != a8 {
                    st.tri_cost[e.id as usize] = self.trigram_cost(&st.pos, e);
                }
            }
        }
    }

    #[inline]
    pub(super) fn trigram_cost(&self, pos: &[u8], e: &TriEntry) -> f32 {
        self.c3[(pos[e.x as usize] as usize * self.n + pos[e.y as usize] as usize) * self.n + pos[e.z as usize] as usize]
    }

    // ---- accessors for reports ----

    /// Unweighted Shift and Ctrl part of metric `id` (M01, Y01; 0 otherwise or without either key).
    pub fn shift_value(&self, id: &str, st: &State) -> f64 {
        let (n, pos) = (self.n, &st.pos);
        let on = |f: &[f32], t: &[f32]| (0..n).map(|y| f[y] as f64 * t[pos[y] as usize] as f64).sum::<f64>();
        let ctrl = match id {
            "M01" => on(&self.fct, &self.ctrl_effort),
            "Y01" => on(&self.fct, &self.ctrl_press),
            _ => 0.0,
        };
        if !self.has_shift {
            return ctrl;
        }
        let mono = |t: &[f32]| on(&self.fsh, t);
        ctrl + match id {
            "M01" => mono(&self.shift_effort),
            "Y01" => {
                let mut v = mono(&self.shift_press);
                for x in 0..n {
                    for y in 0..n {
                        v += self.fsb[x * n + y] as f64 * self.shift_pair[pos[x] as usize * n + pos[y] as usize] as f64;
                    }
                }
                v
            }
            _ => 0.0,
        }
    }

    /// `full` split into (mono + pair terms, trigram terms, load terms).
    pub fn parts(&self, st: &State) -> (f64, f64, f64) {
        let n = self.n;
        let pos = &st.pos;
        let tri: f64 = if self.has_tri {
            self.tri_all
                .iter()
                .map(|e| {
                    e.f as f64
                        * self.c3[(pos[e.x as usize] as usize * n + pos[e.y as usize] as usize) * n + pos[e.z as usize] as usize] as f64
                })
                .sum()
        } else {
            0.0
        };
        let load = self.load_cost(&self.loads(pos));
        (self.full(st) - tri - load, tri, load)
    }
}
