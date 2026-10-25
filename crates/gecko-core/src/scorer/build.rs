//! Building a scorer: fold metric tables × profile weights into the flat tables the hot loop
//! reads, re-weight them, and re-cost them for a moved geometry.

use super::*;

impl Scorer {
    /// Build a scorer for the symbol set of `layout` (which must fit `geometry`; layers are
    /// flattened onto the virtual geometry). With adaptive keys the corpus is first rewritten
    /// as typed with them.
    pub fn new(geometry: &Geometry, corpus: &Corpus, profile: &Profile, layout: &Layout) -> Result<Self> {
        let board = geometry;
        let keyed;
        let corpus = if layout.shifted.is_empty() {
            corpus
        } else {
            layout.check_shifted()?;
            let pairs: Vec<(char, char)> = layout.shifted.iter().map(|p| crate::layout::pair_chars(p)).collect();
            keyed = corpus.keyed(&pairs)?;
            &keyed
        };
        let rewritten;
        let corpus = if layout.magic.is_empty() {
            corpus
        } else {
            layout.check_magic()?;
            rewritten = crate::magic::rewrite(corpus, &layout.magic)?;
            &rewritten
        };
        let (vg, chars) = layout.flat(geometry)?;
        let geometry = &vg;
        let n = geometry.len();
        if n > MAX_KEYS {
            return Err(Error::invalid("geometry", format!("at most {MAX_KEYS} keys")));
        }
        let cidx: Vec<Option<usize>> = chars.iter().map(|&c| corpus.index_of(c)).collect();
        let cs = corpus.size();
        let norm = |v: &mut Vec<f32>| {
            let t: f64 = v.iter().map(|&x| x as f64).sum();
            if t > 0.0 {
                v.iter_mut().for_each(|x| *x = (*x as f64 / t) as f32);
            }
        };
        let mut f1 = vec![0f32; n];
        let mut f2 = vec![0f32; n * n];
        let mut fk = vec![0f32; n * n];
        let (mut fk2, mut fk3) = (vec![0f32; n * n], vec![0f32; n * n]);
        let mut f3 = vec![0f32; n * n * n];
        for x in 0..n {
            let Some(cx) = cidx[x] else { continue };
            f1[x] = corpus.mono[cx] as f32;
            for y in 0..n {
                let Some(cy) = cidx[y] else { continue };
                f2[x * n + y] = corpus.bi[cx * cs + cy] as f32;
                fk[x * n + y] = corpus.skip[cx * cs + cy] as f32;
                fk2[x * n + y] = corpus.skip2[cx * cs + cy] as f32;
                fk3[x * n + y] = corpus.skip3[cx * cs + cy] as f32;
                for z in 0..n {
                    let Some(cz) = cidx[z] else { continue };
                    f3[(x * n + y) * n + z] = corpus.tri[(cx * cs + cy) * cs + cz] as f32;
                }
            }
        }
        let is_sym: Vec<bool> = chars.iter().map(|c| c.is_ascii_punctuation()).collect();
        let mut fy: Vec<f32> = (0..n * n).map(|xy| if is_sym[xy / n] && is_sym[xy % n] { f2[xy] } else { 0.0 }).collect();
        for v in [&mut f1, &mut f2, &mut fk, &mut fk2, &mut fk3, &mut fy, &mut f3] {
            norm(v);
        }
        // Far skipgrams: gap 2 × decay + gap 3 × decay², each normalized like gap 1, so sfs_far reads
        // as "% of skipgram volume, decayed" relative to sfs.
        let decay = profile.sfb.skip_decay;
        let fkf: Vec<f32> = fk2.iter().zip(&fk3).map(|(a, b)| decay * a + decay * decay * b).collect();
        let shift_keys: Vec<usize> = (0..n).filter(|&i| chars[i] == crate::layout::SHIFT).collect();
        let (mut fsh, mut fsb) = (vec![0f32; n], vec![0f32; n * n]);
        let mono_total: f64 = (0..n).filter_map(|x| cidx[x]).map(|cx| corpus.mono[cx] as f64).sum();
        if !shift_keys.is_empty() && mono_total > 0.0 {
            for x in 0..n {
                let Some(cx) = cidx[x] else { continue };
                fsh[x] = (corpus.shift[cx] as f64 / mono_total) as f32;
                for y in 0..n {
                    let Some(cy) = cidx[y] else { continue };
                    fsb[x * n + y] = (corpus.shift_bi[cx * cs + cy] as f64 / mono_total) as f32;
                }
            }
        }
        let (shift_effort, shift_press, shift_pair) = shift_tables(geometry, &shift_keys);
        let fct = ctrl_freqs(&chars, profile);
        let (ctrl_effort, ctrl_press) = ctrl_tables(geometry, &chars);
        truncate_coverage(&mut f3, &chars, profile.trigram_coverage as f64);
        norm(&mut f3);

        // brackets weights: each bracket pair on the layout by the corpus frequency of its two symbols.
        let mut bracket_all = Vec::new();
        for (o, c) in metrics::BRACKET_PAIRS {
            let (Some(x), Some(y)) = (chars.iter().position(|&ch| ch == o), chars.iter().position(|&ch| ch == c)) else {
                continue;
            };
            let f = cidx[x].map_or(0, |i| corpus.mono[i]) as f64 + cidx[y].map_or(0, |i| corpus.mono[i]) as f64;
            if f > 0.0 {
                bracket_all.push((x as u8, y as u8, f));
            }
        }
        let bracket_total: f64 = bracket_all.iter().map(|e| e.2).sum();
        let mut bracket_all: Vec<PairEntry> =
            bracket_all.into_iter().map(|(x, y, f)| PairEntry { x, y, f: (f / bracket_total) as f32, t: 0 }).collect();
        // vim: the policy's motion-key pairs on the layout, equal weights.
        if let Some(pairs) = profile.vim.as_deref().and_then(metrics::vim_pairs) {
            let at = |c: char| chars.iter().position(|&ch| ch == c);
            let on: Vec<(usize, usize)> = pairs.iter().filter_map(|&(a, b)| Some((at(a)?, at(b)?))).collect();
            let t = (metrics::sparse_block("vim") * n * n) as u32;
            bracket_all.extend(on.iter().map(|&(x, y)| PairEntry { x: x as u8, y: y as u8, f: 1.0 / on.len() as f32, t }));
        }
        // sym_groups: the profile's related-symbol pairs on the layout, equal weights.
        if let Some(pairs) = profile.sym_groups.as_deref().and_then(metrics::sym_group_pairs) {
            let at = |c: char| chars.iter().position(|&ch| ch == c);
            let on: Vec<(usize, usize)> = pairs.iter().filter_map(|&(a, b)| Some((at(a)?, at(b)?))).collect();
            let t = (metrics::sparse_block("sym_groups") * n * n) as u32;
            bracket_all.extend(on.iter().map(|&(x, y)| PairEntry { x: x as u8, y: y as u8, f: 1.0 / on.len() as f32, t }));
        }

        let qwerty = metrics::qwerty_slots(board, &chars);
        let prep = metrics::Prep::new(geometry);
        let tables: Vec<Option<Vec<f32>>> = METRICS
            .iter()
            .map(|d| match d.order {
                Order::Place => Some(metrics::place_table(d.id, geometry, profile, &chars, &qwerty, &f1)),
                _ => {
                    let mut t = Vec::new();
                    metrics::update_table(geometry, profile, &prep, d, &mut t, None).then_some(t)
                }
            })
            .collect();
        let mut c1 = vec![0f32; n];
        let mut c2 = vec![0f32; n * n];
        let mut ck = vec![0f32; n * n];
        let mut ckf = vec![0f32; n * n];
        let mut cy = vec![0f32; n * n];
        let mut c3 = vec![0f32; n * n * n];
        let mut cb = vec![0f32; metrics::SPARSE.len() * n * n];
        let mut cp = vec![0f32; n * n];
        let (mut has_pair, mut has_far, mut has_sym, mut has_tri) = (false, false, false, false);
        let has_shift = fsh.iter().any(|&f| f > 0.0);
        let w_of = |id: &str| if has_shift { profile.effective_weight(id) } else { 0.0 };
        let (w_effort, w_y01) = (w_of("effort"), w_of("layer_switch"));
        let csh: Vec<f32> = shift_effort.iter().zip(&shift_press).map(|(e, p)| w_effort * e + w_y01 * p).collect();
        let csb: Vec<f32> = shift_pair.iter().map(|p| w_y01 * p).collect();
        let has_shift_pair = has_shift && csb.iter().any(|&c| c != 0.0) && fsb.iter().any(|&f| f > 0.0);
        let (mut w_load, mut w_hand, mut w_rss, mut w_peak) = (0.0, 0.0, 0.0, 0.0);
        let mut w_thumb = 0.0;
        let mut fold_w = vec![0f32; METRICS.len()];
        for (mi, (d, t)) in METRICS.iter().zip(&tables).enumerate() {
            let w = profile.effective_weight(d.id);
            let signed = match d.dir {
                Dir::Min => w,
                Dir::Max => -w,
                Dir::Info => 0.0,
            };
            if signed == 0.0 {
                continue;
            }
            match (d.order, t) {
                (Order::Load, _) => match d.id {
                    "finger_target" => w_load = w as f64,
                    "hand_balance" => w_hand = w as f64,
                    "finger_evenness" => w_rss = w as f64,
                    "finger_peak" => w_peak = w as f64,
                    "thumb_load" => w_thumb = w as f64,
                    _ => {}
                },
                (order, Some(t)) => {
                    let block = metrics::sparse_block(d.id) * n * n;
                    let dst: &mut [f32] = match order {
                        Order::Mono => &mut c1,
                        Order::Bi => {
                            has_pair = true;
                            &mut c2
                        }
                        Order::Skip => {
                            has_pair = true;
                            &mut ck
                        }
                        Order::SkipFar => {
                            has_pair = true;
                            has_far = true;
                            &mut ckf
                        }
                        Order::SymBi => {
                            has_pair = true;
                            has_sym = true;
                            &mut cy
                        }
                        Order::Tri => {
                            has_tri = true;
                            &mut c3
                        }
                        Order::Pairs => &mut cb[block..block + n * n],
                        Order::Place => &mut cp,
                        Order::Load => unreachable!(),
                    };
                    fold_w[mi] = signed;
                    dst.iter_mut().zip(t).for_each(|(c, v)| *c += signed * v);
                }
                (_, None) => {}
            }
        }
        let mut tri_all = Vec::new();
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let f = f3[(x * n + y) * n + z];
                    if f > 0.0 {
                        tri_all.push(TriEntry { id: tri_all.len() as u32, x: x as u8, y: y as u8, z: z as u8, f });
                    }
                }
            }
        }
        let (tri_start, tri_entries, tri_pair_start, tri_pair) = index_trigrams(n, &tri_all);
        let ctrl_w = (profile.effective_weight("effort"), profile.effective_weight("layer_switch"));
        add_ctrl(&mut cp, &fct, &ctrl_effort, &ctrl_press, ctrl_w);
        let has_place = cp.iter().any(|&c| c != 0.0);
        let brackets = sparse_used(&bracket_all, &cb, n);
        if let Some(h) = &profile.hand {
            h.validate()?;
        }
        if let Some(e) = &profile.finger_effort {
            e.validate()?;
        }
        let (target, thumb_target) = finger_targets(profile, geometry);
        Ok(Scorer {
            n,
            chars,
            geometry: geometry.clone(),
            profile: profile.clone(),
            corpus_name: corpus.name.clone(),
            magic: layout.magic.clone(),
            shifted: layout.shifted.clone(),
            f1,
            f2,
            fk,
            fkf,
            fy,
            f3,
            c1,
            c2,
            ck,
            ckf,
            cy,
            c3,
            fsh,
            fsb,
            csh,
            csb,
            shift_effort,
            shift_press,
            shift_pair,
            has_shift,
            has_shift_pair,
            has_pair,
            has_far,
            has_sym,
            has_tri,
            brackets,
            bracket_all,
            cb,
            cp,
            has_place,
            tri_start,
            tri_entries,
            tri_all,
            tri_pair_start,
            tri_pair,
            finger: geometry.key.iter().map(|k| k.finger.index() as u8).collect(),
            hold: (0..n).map(|i| geometry.holder(i).map_or(NO_HOLD, |h| geometry.key[h].finger.index() as u8)).collect(),
            w_load,
            w_hand,
            w_rss,
            w_peak,
            has_load: w_load != 0.0 || w_hand != 0.0 || w_rss != 0.0 || w_peak != 0.0 || w_thumb != 0.0,
            w_thumb,
            thumb_target,
            target,
            tables,
            fold_w,
            shift_w: (w_effort, w_y01),
            fct,
            ctrl_effort,
            ctrl_press,
            ctrl_w,
        })
    }

    /// Re-fold existing metric tables with new nonnegative weights in natural cost units.
    /// Geometry, corpus, table parameters and symbol ids are unchanged. Construct fresh states
    /// with the returned scorer: a state's cached costs belong to its constructing scorer.
    pub fn with_weights(&self, weights: &[(&str, f32)]) -> Result<Self> {
        let mut out = self.clone();
        let n = out.n;
        out.profile.weights.clear();
        out.profile.scale.clear();
        for &(id, weight) in weights {
            let mi = metrics::index_of(id).ok_or_else(|| Error::invalid("objectives", format!("unknown metric '{id}'")))?;
            if METRICS[mi].dir == Dir::Info || !weight.is_finite() || weight < 0.0 {
                return Err(Error::invalid("objectives", format!("'{id}' needs a scored metric and a finite nonnegative weight")));
            }
            out.profile.weights.insert(METRICS[mi].id.to_owned(), weight);
        }
        for table in [&mut out.c1, &mut out.c2, &mut out.ck, &mut out.ckf, &mut out.cy, &mut out.c3, &mut out.cb, &mut out.cp] {
            table.fill(0.0);
        }
        (out.w_load, out.w_hand, out.w_rss, out.w_peak, out.w_thumb) = (0.0, 0.0, 0.0, 0.0, 0.0);
        (out.has_pair, out.has_far, out.has_sym, out.has_tri) = (false, false, false, false);
        out.fold_w.fill(0.0);
        for (mi, (d, table)) in METRICS.iter().zip(&out.tables).enumerate() {
            let w = out.profile.weight(d.id);
            if w == 0.0 {
                continue;
            }
            let signed = if d.dir == Dir::Max { -w } else { w };
            match (d.order, table) {
                (Order::Load, _) => match d.id {
                    "finger_target" => out.w_load = w as f64,
                    "hand_balance" => out.w_hand = w as f64,
                    "finger_evenness" => out.w_rss = w as f64,
                    "finger_peak" => out.w_peak = w as f64,
                    "thumb_load" => out.w_thumb = w as f64,
                    _ => {}
                },
                (order, Some(table)) => {
                    let block = metrics::sparse_block(d.id) * n * n;
                    let target: &mut [f32] = match order {
                        Order::Mono => &mut out.c1,
                        Order::Bi => {
                            out.has_pair = true;
                            &mut out.c2
                        }
                        Order::Skip => {
                            out.has_pair = true;
                            &mut out.ck
                        }
                        Order::SkipFar => {
                            out.has_pair = true;
                            out.has_far = true;
                            &mut out.ckf
                        }
                        Order::SymBi => {
                            out.has_pair = true;
                            out.has_sym = true;
                            &mut out.cy
                        }
                        Order::Tri => {
                            out.has_tri = true;
                            &mut out.c3
                        }
                        Order::Pairs => &mut out.cb[block..block + n * n],
                        Order::Place => &mut out.cp,
                        Order::Load => unreachable!(),
                    };
                    out.fold_w[mi] = signed;
                    for (target, value) in target.iter_mut().zip(table) {
                        *target += signed * value;
                    }
                }
                _ => {}
            }
        }
        out.ctrl_w = (out.profile.weight("effort"), out.profile.weight("layer_switch"));
        add_ctrl(&mut out.cp, &out.fct, &out.ctrl_effort, &out.ctrl_press, out.ctrl_w);
        out.has_place = out.cp.iter().any(|&c| c != 0.0);
        out.has_load = [out.w_load, out.w_hand, out.w_rss, out.w_peak, out.w_thumb].iter().any(|&w| w != 0.0);
        out.brackets = sparse_used(&out.bracket_all, &out.cb, n);
        let (effort, layer) = (out.profile.weight("effort"), out.profile.weight("layer_switch"));
        out.shift_w = (effort, layer);
        for ((cost, e), p) in out.csh.iter_mut().zip(&out.shift_effort).zip(&out.shift_press) {
            *cost = effort * e + layer * p;
        }
        for (cost, p) in out.csb.iter_mut().zip(&out.shift_pair) {
            *cost = layer * p;
        }
        out.has_shift_pair = out.has_shift && out.csb.iter().any(|&c| c != 0.0) && out.fsb.iter().any(|&f| f > 0.0);
        Ok(out)
    }

    /// One metric's actual cost at unit weight (max metrics are negated; load targets retain
    /// their nonlinear penalty). Raw report values may differ, e.g. hand_balance reports left share.
    pub fn objective(&self, id: &str) -> Result<Self> {
        self.with_weights(&[(id, 1.0)])
    }

    /// This scorer for a moved board: [`Scorer::set_geometry`] on a copy.
    pub fn with_geometry(&self, board: &Geometry) -> Result<Self> {
        let mut out = self.clone();
        out.set_geometry(board)?;
        Ok(out)
    }

    /// Non-zero trigrams this scorer evaluates.
    pub fn trigram_count(&self) -> usize {
        self.tri_all.len()
    }

    /// A cheaper copy for search (adaptive fidelity, B8): only the `keep` most frequent
    /// trigrams, not renormalized, so its scores are the full score minus the dropped
    /// trigrams' share. Everything else is unchanged. Its states belong to it alone.
    pub fn coarse(&self, keep: usize) -> Self {
        let mut out = self.clone();
        if keep >= self.tri_all.len() {
            return out;
        }
        let key = |t: &TriEntry| [t.x, t.y, t.z].map(|s| self.chars[s as usize]);
        let mut idx: Vec<usize> = (0..self.tri_all.len()).collect();
        // Ties by characters, not symbol index: the kept set must not depend on symbol order.
        idx.sort_by(|&a, &b| {
            let (ta, tb) = (&self.tri_all[a], &self.tri_all[b]);
            tb.f.total_cmp(&ta.f).then_with(|| key(ta).cmp(&key(tb))).then(a.cmp(&b))
        });
        idx.truncate(keep);
        idx.sort_unstable();
        out.tri_all = idx.iter().enumerate().map(|(id, &i)| TriEntry { id: id as u32, ..self.tri_all[i] }).collect();
        (out.tri_start, out.tri_entries, out.tri_pair_start, out.tri_pair) = index_trigrams(self.n, &out.tri_all);
        out
    }

    /// Re-cost the scorer for `board`, the physical board with keys moved (pose, size, effort,
    /// any key field) but the same keys in the same slots. Overlapping keys are allowed (a
    /// search scores them and penalizes [`Geometry::collisions`] itself). Corpus tables, symbol ids and
    /// weights are kept. Only pair/triple table entries that touch a changed slot are
    /// recomputed and refolded: O(k·N) pair and O(k·N²) trigram entries for k changed slots,
    /// instead of every metric's N² and N³ table. A slot changes when its key or its
    /// [`Geometry::metric_points`] entry does (on a 3D board, moving one key re-fits its hand's
    /// plane, so every key of that hand changes); a changed layer key or comfort table changes
    /// them all. The result equals [`Scorer::new`] on `board` bit for bit. Returns the number
    /// of changed slots. Existing states hold stale costs: [`Scorer::refresh`] them.
    pub fn set_geometry(&mut self, board: &Geometry) -> Result<usize> {
        let n = self.n;
        let vg = if self.geometry.layer_keys.is_empty() {
            board.check_keys()?;
            board.clone()
        } else {
            let holders: Vec<_> = self.geometry.layer_keys.iter().copied().zip(self.geometry.layer_modes.iter().copied()).collect();
            board.layered(&holders)?
        };
        let same_slots = vg.len() == n && vg.key.iter().zip(&self.geometry.key).all(|(a, b)| a.layer == b.layer && a.phys == b.phys);
        if !same_slots || !vg.comfort_tables.keys().eq(self.geometry.comfort_tables.keys()) {
            return Err(Error::invalid("geometry", "keys or comfort tables differ from the scorer's board; build a new scorer"));
        }
        let prep = metrics::Prep::new(&vg);
        let (old, old_pts, new_pts) = (&self.geometry.key, self.geometry.metric_points(), prep.points());
        // Two masks: a slot's logical fields (finger, row, home, …) changed, or anything a
        // geometric table reads (its key or metric point) did. A pose-only move leaves the
        // logical tables (trigram classes, SFB, scissors, …) untouched.
        // `o` with `k`'s pose equals `k`: only the pose changed.
        let pose_free = |k: &crate::geometry::Key, o: &crate::geometry::Key| {
            *k == crate::geometry::Key { pos: k.pos, rot: k.rot, size: k.size, ..o.clone() }
        };
        let mut logical: Vec<bool> = (0..n).map(|i| !pose_free(&vg.key[i], &old[i])).collect();
        let mut moved: Vec<bool> = (0..n).map(|i| vg.key[i] != old[i] || old_pts[i] != new_pts[i]).collect();
        if vg.layer_keys.iter().any(|&h| moved[h]) || vg.comfort_tables != self.geometry.comfort_tables {
            logical.fill(true);
            moved.fill(true);
        }
        // row_jump measures from each finger's (or hand's) home keys: a moved home key moves its hand.
        // Key effort (effort, typing_time) reads the home row and home keys board-wide.
        let home_moved: Vec<crate::Hand> =
            (0..n).filter(|&i| moved[i] && (vg.key[i].home || old[i].home)).map(|i| vg.key[i].hand()).collect();
        for i in 0..n {
            if home_moved.contains(&vg.key[i].hand()) {
                moved[i] = true;
            }
        }
        if (0..n).any(|i| logical[i] && (vg.key[i].home || old[i].home)) {
            logical.fill(true);
        }
        self.geometry = vg;
        let count = moved.iter().filter(|&&c| c).count();
        if count == 0 {
            return Ok(0);
        }
        let any_logical = logical.contains(&true);
        let mask = |id: &str| if metrics::reads_points(id) { &moved } else { &logical };
        let geometry = &self.geometry;
        let qwerty = metrics::qwerty_slots(board, &self.chars);
        for (d, t) in METRICS.iter().zip(&mut self.tables) {
            match (d.order, t) {
                (Order::Load, _) | (_, None) => {}
                (Order::Place, Some(t)) => *t = metrics::place_table(d.id, geometry, &self.profile, &self.chars, &qwerty, &self.f1),
                (Order::Mono, Some(t)) => {
                    metrics::update_table(geometry, &self.profile, &prep, d, t, None);
                }
                (_, Some(t)) => {
                    if any_logical || metrics::reads_points(d.id) {
                        metrics::update_table(geometry, &self.profile, &prep, d, t, Some(mask(d.id)));
                    }
                }
            }
        }

        // Refold exactly as `new` does (0, then + w·t in `METRICS` order), so results match.
        // Per order, only entries a folded table may have changed in.
        let (tables, fold_w) = (&self.tables, &self.fold_w);
        // `block`: only the sparse metrics of that cost-table block (`metrics::SPARSE`).
        let srcs_in = |order: Order, block: Option<usize>| -> (Vec<(&[f32], f32)>, &Vec<bool>) {
            let used: Vec<_> = METRICS
                .iter()
                .zip(tables)
                .zip(fold_w)
                .filter(|((d, t), &w)| {
                    d.order == order && w != 0.0 && t.is_some() && block.is_none_or(|b| metrics::sparse_block(d.id) == b)
                })
                .collect();
            let mask = if used.iter().any(|((d, _), _)| metrics::reads_points(d.id)) { &moved } else { &logical };
            (used.into_iter().map(|((_, t), &w)| (t.as_deref().unwrap(), w)).collect(), mask)
        };
        let srcs = |order: Order| srcs_in(order, None);
        let fold = |dst: &mut [f32], srcs: &[(&[f32], f32)], e: usize| {
            let mut c = 0.0f32;
            for (t, w) in srcs {
                c += w * t[e];
            }
            dst[e] = c;
        };
        for (dst, order) in [(&mut self.c1, Order::Mono), (&mut self.cp, Order::Place)] {
            let (s, _) = srcs(order);
            (0..dst.len()).for_each(|e| fold(dst, &s, e));
        }
        for (dst, srcs) in [
            (&mut self.c2, Order::Bi),
            (&mut self.ck, Order::Skip),
            (&mut self.ckf, Order::SkipFar),
            (&mut self.cy, Order::SymBi),
        ]
        .into_iter()
        .map(|(dst, order)| (&mut dst[..], srcs(order)))
        .chain(self.cb.chunks_mut(n * n).enumerate().map(|(b, dst)| (dst, srcs_in(Order::Pairs, Some(b)))))
        {
            let (s, ch) = srcs;
            let idx: Vec<usize> = (0..n).filter(|&i| ch[i]).collect();
            for i in 0..n {
                if ch[i] {
                    (i * n..(i + 1) * n).for_each(|e| fold(dst, &s, e));
                } else {
                    idx.iter().for_each(|&j| fold(dst, &s, i * n + j));
                }
            }
        }
        let (s, ch) = srcs(Order::Tri);
        let idx: Vec<usize> = (0..n).filter(|&i| ch[i]).collect();
        if !idx.is_empty() {
            for ij in 0..n * n {
                let row = ij * n;
                if ch[ij / n] || ch[ij % n] {
                    (row..row + n).for_each(|e| fold(&mut self.c3, &s, e));
                } else {
                    idx.iter().for_each(|&l| fold(&mut self.c3, &s, row + l));
                }
            }
        }

        let geometry = &self.geometry;
        self.finger = geometry.key.iter().map(|k| k.finger.index() as u8).collect();
        self.hold = (0..n).map(|i| geometry.holder(i).map_or(NO_HOLD, |h| geometry.key[h].finger.index() as u8)).collect();
        let shift_keys: Vec<usize> = (0..n).filter(|&i| self.chars[i] == crate::layout::SHIFT).collect();
        (self.shift_effort, self.shift_press, self.shift_pair) = shift_tables(geometry, &shift_keys);
        let (w_effort, w_y01) = self.shift_w;
        self.csh = self.shift_effort.iter().zip(&self.shift_press).map(|(e, p)| w_effort * e + w_y01 * p).collect();
        self.csb = self.shift_pair.iter().map(|p| w_y01 * p).collect();
        self.has_shift_pair = self.has_shift && self.csb.iter().any(|&c| c != 0.0) && self.fsb.iter().any(|&f| f > 0.0);
        (self.ctrl_effort, self.ctrl_press) = ctrl_tables(geometry, &self.chars);
        add_ctrl(&mut self.cp, &self.fct, &self.ctrl_effort, &self.ctrl_press, self.ctrl_w);
        self.has_place = self.cp.iter().any(|&c| c != 0.0);
        self.brackets = sparse_used(&self.bracket_all, &self.cb, n);
        (self.target, self.thumb_target) = finger_targets(&self.profile, geometry);
        Ok(count)
    }
}

/// The sparse entries whose cost-table block has any non-zero cost (weighted metrics only).
fn sparse_used(all: &[PairEntry], cb: &[f32], n: usize) -> Vec<PairEntry> {
    let live = |t: u32| cb[t as usize..t as usize + n * n].iter().any(|&c| c != 0.0);
    all.iter().filter(|e| live(e.t)).copied().collect()
}

/// Unfolded Shift cost tables for the `⇧` slots `keys` (all zero without one): the Shift key
/// that types slot j is the best of `keys` by (on j's finger, on j's hand, effort, index).
/// Per press before slot j: that key's effort (effort) and layer_switch = 100, 200 when it is on j's finger
/// (held while j presses). Per press between slots i and j: layer_switch + 100 when i is a different key
/// on that Shift key's finger (same finger into Shift).
fn shift_tables(g: &Geometry, keys: &[usize]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let n = g.len();
    let (mut effort, mut press, mut pair) = (vec![0f32; n], vec![0f32; n], vec![0f32; n * n]);
    if keys.is_empty() {
        return (effort, press, pair);
    }
    let k = &g.key;
    for j in 0..n {
        let rank = |s: usize| (k[s].finger == k[j].finger, k[s].hand() == k[j].hand());
        let s = keys.iter().copied().min_by(|&a, &b| rank(a).cmp(&rank(b)).then(k[a].effort.total_cmp(&k[b].effort))).unwrap();
        effort[j] = k[s].effort;
        press[j] = if k[s].finger == k[j].finger { 200.0 } else { 100.0 };
        for i in 0..n {
            if k[i].finger == k[s].finger && g.phys(i) != s {
                pair[i * n + j] = 100.0;
            }
        }
    }
    (effort, press, pair)
}

/// Ctrl shortcut presses per keystroke before each symbol: profile `ctrl` spread over the
/// letters of [`crate::profile::CTRL_SHORTCUTS`] on the layout (renormalized). All zero
/// without a `⎈` key.
fn ctrl_freqs(chars: &[char], profile: &Profile) -> Vec<f32> {
    let mut f = vec![0f32; chars.len()];
    if !chars.contains(&crate::layout::CTRL) || profile.ctrl <= 0.0 {
        return f;
    }
    let share = |c: char| crate::profile::CTRL_SHORTCUTS.iter().find(|s| s.0 == c).map_or(0.0, |s| s.1);
    let total: f32 = chars.iter().map(|&c| share(c)).sum();
    if total > 0.0 {
        for (v, &c) in f.iter_mut().zip(chars) {
            *v = profile.ctrl * share(c) / total;
        }
    }
    f
}

/// Unfolded Ctrl tables per slot j: the effort of the `⎈` key that presses shortcuts on j
/// (chosen like Shift's) and layer_switch = 100, 200 on j's finger. Zero without a `⎈` key.
fn ctrl_tables(g: &Geometry, chars: &[char]) -> (Vec<f32>, Vec<f32>) {
    let keys: Vec<usize> = (0..chars.len()).filter(|&i| chars[i] == crate::layout::CTRL).collect();
    let (effort, press, _) = shift_tables(g, &keys);
    (effort, press)
}

/// Fold Ctrl presses into the symbol-on-slot table: `cp[x, j] += fct[x] · (w_M01 · effort[j]
/// + w_Y01 · press[j])`.
fn add_ctrl(cp: &mut [f32], fct: &[f32], effort: &[f32], press: &[f32], (w_effort, w_y01): (f32, f32)) {
    let n = fct.len();
    for (x, &f) in fct.iter().enumerate().filter(|e| *e.1 > 0.0) {
        for j in 0..n {
            cp[x * n + j] += f * (w_effort * effort[j] + w_y01 * press[j]);
        }
    }
}

/// Target finger loads (%) and the thumb target: the profile's `finger_target`, or with
/// `finger_effort` each finger present on `g` by inverse typing cost, normalized to 100.
fn finger_targets(profile: &Profile, g: &Geometry) -> ([f64; FINGERS], f64) {
    let mut target = [0.0; FINGERS];
    for (t, &v) in target.iter_mut().zip(&profile.finger_target) {
        *t = v as f64;
    }
    if let Some(e) = &profile.finger_effort {
        let mut present = [false; FINGERS];
        for k in &g.key {
            present[k.finger.index()] = true;
        }
        for i in 0..FINGERS {
            target[i] = if present[i] { 1.0 / e.typing[e.index(i)] as f64 } else { 0.0 };
        }
        let total: f64 = target.iter().sum();
        if total > 0.0 {
            for t in &mut target {
                *t *= 100.0 / total;
            }
        }
    }
    let thumb_target = if profile.finger_effort.is_some() { target[4] + target[5] } else { profile.thumb_target as f64 };
    (target, thumb_target)
}

/// The sparse trigram indexes over `tri_all`: per symbol (CSR `tri_start`/`tri_entries`) and per
/// unordered symbol pair (CSR `tri_pair_start`/`tri_pair`, key `x * n + y` with `x < y`).
fn index_trigrams(n: usize, tri_all: &[TriEntry]) -> (Vec<u32>, Vec<TriEntry>, Vec<u32>, Vec<TriEntry>) {
    let mut tri_start = vec![0u32; n + 1];
    let mut tri_entries = Vec::new();
    for sym in 0..n {
        tri_start[sym] = tri_entries.len() as u32;
        let s8 = sym as u8;
        tri_entries.extend(tri_all.iter().filter(|e| e.x == s8 || e.y == s8 || e.z == s8));
    }
    tri_start[n] = tri_entries.len() as u32;
    let pairs_of = |e: &TriEntry| {
        let mut v = [(e.x, e.y), (e.x, e.z), (e.y, e.z)].map(|(a, b)| (a.min(b) as usize, a.max(b) as usize));
        v.sort_unstable();
        let mut out = [usize::MAX; 3];
        for (i, &(a, b)) in v.iter().enumerate() {
            if a != b && (i == 0 || v[i - 1] != (a, b)) {
                out[i] = a * n + b;
            }
        }
        out
    };
    let mut tri_pair_start = vec![0u32; n * n + 1];
    for e in tri_all {
        for k in pairs_of(e).into_iter().filter(|&k| k != usize::MAX) {
            tri_pair_start[k + 1] += 1;
        }
    }
    for k in 0..n * n {
        tri_pair_start[k + 1] += tri_pair_start[k];
    }
    let mut tri_pair = vec![TriEntry { id: 0, x: 0, y: 0, z: 0, f: 0.0 }; tri_pair_start[n * n] as usize];
    let mut fill = tri_pair_start.clone();
    for e in tri_all {
        for k in pairs_of(e).into_iter().filter(|&k| k != usize::MAX) {
            tri_pair[fill[k] as usize] = *e;
            fill[k] += 1;
        }
    }
    (tri_start, tri_entries, tri_pair_start, tri_pair)
}

/// Zero the least frequent entries of the trigram table `f` (indexed by the symbols `chars`)
/// so the rest cover `coverage` of the total. Ties break by characters, not symbol index, so
/// the kept set does not depend on the layout's symbol order (the search's start layout).
fn truncate_coverage(f: &mut [f32], chars: &[char], coverage: f64) {
    if coverage >= 1.0 {
        return;
    }
    let n = chars.len();
    let key = |i: usize| [chars[i / (n * n)], chars[i / n % n], chars[i % n]];
    let mut idx: Vec<usize> = (0..f.len()).filter(|&i| f[i] > 0.0).collect();
    idx.sort_by(|&a, &b| f[b].total_cmp(&f[a]).then_with(|| key(a).cmp(&key(b))).then(a.cmp(&b)));
    let total: f64 = idx.iter().map(|&i| f[i] as f64).sum();
    let mut acc = 0.0;
    let mut keep = idx.len();
    for (k, &i) in idx.iter().enumerate() {
        acc += f[i] as f64;
        if acc >= coverage * total {
            keep = k + 1;
            break;
        }
    }
    for &i in &idx[keep..] {
        f[i] = 0.0;
    }
}
