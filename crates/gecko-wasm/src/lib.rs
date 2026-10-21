//! Browser scorer: the same gecko-core code as the CLI, so numbers in the editor always match.
//! Built with `wasm-pack build crates/gecko-wasm --target web --out-dir ../../web/src/wasm/pkg`.

use gecko_core::explore::{atlas, distance, stats};
use gecko_core::report::{analyze, swap_matrix};
use gecko_core::{Corpus, Geometry, Layout, Profile, Scorer, SCHEMA};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Engine {
    geometry: Geometry,
    corpus: Corpus,
}

fn js<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}

#[wasm_bindgen]
impl Engine {
    /// `geometry_json`: a geometry as served by `/api/geometries`; `corpus`: the `.gkc` bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(geometry_json: &str, corpus: &[u8]) -> Result<Engine, JsError> {
        let geometry: Geometry = serde_json::from_str(geometry_json).map_err(js)?;
        geometry.validate().map_err(js)?;
        let corpus = Corpus::from_bytes(corpus).map_err(js)?;
        Ok(Engine { geometry, corpus })
    }

    /// Analysis JSON for slot-ordered `chars` under `profile_json` (same shape as `POST /api/score`).
    /// A layered layout's `chars` is the flat string over its virtual slots (`Layout::flat`).
    pub fn analyze(&self, chars: &str, profile_json: &str, top: usize) -> Result<String, JsError> {
        let profile: Profile = serde_json::from_str(profile_json).map_err(js)?;
        let layout = Layout::from_str_flat("custom", &self.geometry.name, chars).and_then(|l| l.fit(&self.geometry)).map_err(js)?;
        let sc = Scorer::new(&self.geometry, &self.corpus, &profile, &layout).map_err(js)?;
        let st = sc.identity();
        let v = serde_json::json!({"schema": SCHEMA, "chars": chars, "rows": layout.all_rows(&self.geometry), "analysis": analyze(&sc, &st, top)});
        Ok(v.to_string())
    }

    /// Score change for swapping every pair of slots of `chars` (`m[i * n + j]`).
    pub fn swap_matrix(&self, chars: &str, profile_json: &str) -> Result<Vec<f32>, JsError> {
        let profile: Profile = serde_json::from_str(profile_json).map_err(js)?;
        let layout = Layout::from_str_flat("custom", &self.geometry.name, chars).and_then(|l| l.fit(&self.geometry)).map_err(js)?;
        let sc = Scorer::new(&self.geometry, &self.corpus, &profile, &layout).map_err(js)?;
        Ok(swap_matrix(&sc, &sc.identity()))
    }
}

/// An explore atlas held in WASM memory: the browser re-ranks it under edited weights and
/// computes embeddings, consensus, families and funnels with the same code as `gecko atlas`.
#[wasm_bindgen]
pub struct AtlasView {
    slots: usize,
    metrics: Vec<String>,
    syms: Vec<u8>,
    raw: Vec<f32>,
    hits: Vec<u32>,
    first_ms: Vec<u32>,
    flags: Vec<u16>,
    base: Vec<f64>,
    edges: Vec<atlas::Edge>,
    sample_raw: Vec<f32>,
    share: [f64; 2],
}

fn weights(profile_json: &str, metrics: &[String]) -> Result<stats::Weights, JsError> {
    let profile: Profile = serde_json::from_str(profile_json).map_err(js)?;
    Ok(stats::Weights::new(&profile, metrics))
}

#[wasm_bindgen]
impl AtlasView {
    /// The bytes of `optima.bin`, `edges.bin` and `samples.bin` (may be empty).
    #[wasm_bindgen(constructor)]
    pub fn new(optima: &[u8], edges: &[u8], samples: &[u8]) -> Result<AtlasView, JsError> {
        let (slots, metrics, opt) = atlas::read_optima(optima, None).map_err(js)?;
        let edges = if edges.is_empty() { Vec::new() } else { atlas::read_edges(edges, opt.len()).map_err(js)? };
        let samples = if samples.is_empty() { Vec::new() } else { atlas::read_samples(samples).map_err(js)? };
        Ok(AtlasView {
            slots,
            syms: opt.iter().flat_map(|o| o.sym.iter().copied()).collect(),
            raw: opt.iter().flat_map(|o| o.raw.iter().copied()).collect(),
            hits: opt.iter().map(|o| o.hits).collect(),
            first_ms: opt.iter().map(|o| o.first_ms).collect(),
            flags: opt.iter().map(|o| o.flags).collect(),
            base: opt.iter().map(|o| o.score).collect(),
            sample_raw: samples.iter().flat_map(|s| s.raw.iter().copied()).collect(),
            metrics,
            edges,
            share: [0.0; 2],
        })
    }

    pub fn count(&self) -> usize {
        self.hits.len()
    }
    pub fn slots(&self) -> usize {
        self.slots
    }
    /// JSON array of metric ids (the column order of `raw`).
    pub fn metric_ids(&self) -> String {
        serde_json::to_string(&self.metrics).unwrap()
    }
    /// `sym[row * slots + slot]`
    pub fn syms(&self) -> Vec<u8> {
        self.syms.clone()
    }
    /// `raw[row * metrics + k]`
    pub fn raw(&self) -> Vec<f32> {
        self.raw.clone()
    }
    pub fn hits(&self) -> Vec<u32> {
        self.hits.clone()
    }
    pub fn first_ms(&self) -> Vec<u32> {
        self.first_ms.clone()
    }
    pub fn flags(&self) -> Vec<u16> {
        self.flags.clone()
    }
    /// Scores under the run's own profile.
    pub fn base_scores(&self) -> Vec<f64> {
        self.base.clone()
    }
    /// `[from, to, count, kick << 8 | strength]` per edge.
    pub fn edges(&self) -> Vec<u32> {
        self.edges.iter().flat_map(|e| [e.from, e.to, e.count, (e.kick as u32) << 8 | e.strength as u32]).collect()
    }
    /// Every optimum re-scored under a profile (weights only; tables are the atlas's).
    pub fn rescore(&self, profile_json: &str) -> Result<Vec<f64>, JsError> {
        Ok(weights(profile_json, &self.metrics)?.score_all(&self.raw, self.metrics.len()))
    }
    /// The probe's random layouts re-scored under a profile.
    pub fn sample_scores(&self, profile_json: &str) -> Result<Vec<f64>, JsError> {
        Ok(weights(profile_json, &self.metrics)?.score_all(&self.sample_raw, self.metrics.len()))
    }
    /// The probe's random layouts' raw metric values (`[row * metrics + k]`).
    pub fn sample_raw(&self) -> Vec<f32> {
        self.sample_raw.clone()
    }
    /// 2D PCA map of every optimum (`[row * 2 + c]`) under `encoding` (`slots`, `fingers`,
    /// `metrics`); `finger[slot]` is the finger index per slot.
    pub fn embed(&mut self, encoding: &str, finger: &[u8]) -> Result<Vec<f32>, JsError> {
        let enc = stats::Encoding::parse(encoding).ok_or_else(|| JsError::new("unknown encoding"))?;
        let rows: Vec<u32> = (0..self.count() as u32).collect();
        let (x, d) = stats::features(enc, &self.syms, self.slots, &self.raw, self.metrics.len(), finger, &rows);
        let (y, share) = stats::pca2(&x, d, 3000);
        self.share = share;
        Ok(y)
    }
    /// Share of variance of the last embedding's two axes.
    pub fn embed_share(&self) -> Vec<f64> {
        self.share.to_vec()
    }
    /// `p[slot * slots + symbol]` over `rows`.
    pub fn consensus(&self, rows: &[u32]) -> Vec<f32> {
        stats::consensus(&self.syms, self.slots, rows)
    }
    pub fn entropy(&self, rows: &[u32]) -> Vec<f32> {
        stats::slot_entropy(&stats::consensus(&self.syms, self.slots, rows), self.slots)
    }
    /// Family per row of `rows` (given best first), leaders within `cut` slots.
    pub fn families(&self, rows: &[u32], cut: usize) -> Vec<u32> {
        stats::families(&self.syms, self.slots, rows, cut)
    }
    /// Funnel bottom (row) of every optimum under `scores`.
    pub fn funnels(&self, scores: &[f64]) -> Vec<u32> {
        let pairs: Vec<(u32, u32)> = self.edges.iter().map(|e| (e.from, e.to)).collect();
        stats::funnels(scores, &pairs)
    }
    pub fn fdc(&self, scores: &[f64], rows: &[u32]) -> f64 {
        stats::fdc(&self.syms, self.slots, scores, rows)
    }
    /// Hamming distance from row `i` to every row.
    pub fn hamming_to(&self, i: usize) -> Vec<u8> {
        let s = self.slots;
        let a = &self.syms[i * s..(i + 1) * s];
        self.syms.chunks_exact(s).map(|b| distance::hamming(a, b) as u8).collect()
    }
    /// Hamming distance from an arbitrary layout (`sym[slot]` in atlas symbol ids) to every row.
    pub fn hamming_from(&self, sym: &[u8]) -> Vec<u8> {
        self.syms.chunks_exact(self.slots).map(|b| distance::hamming(sym, b) as u8).collect()
    }
    /// Best layouts at least `dmin` apart (rows), best first.
    pub fn distinct(&self, scores: &[f64], dmin: usize, k: usize) -> Vec<u32> {
        stats::distinct_bests(&self.syms, self.slots, scores, dmin, k)
    }
    /// `[observed, estimated]` distinct optima among `rows` (Chao1 on hit counts).
    pub fn chao1(&self, rows: &[u32]) -> Vec<f64> {
        let hits: Vec<u32> = rows.iter().map(|&i| self.hits[i as usize]).collect();
        let (o, e) = stats::chao1(&hits);
        vec![o as f64, e]
    }
    /// Winning row for each metric column in `metrics` × weight factor (see `stats::stability`).
    pub fn stability(&self, profile_json: &str, metrics: &[u32], factors: &[f64]) -> Result<Vec<u32>, JsError> {
        let w = weights(profile_json, &self.metrics)?;
        let ks: Vec<usize> = metrics.iter().map(|&k| k as usize).collect();
        Ok(stats::stability(&w, &self.raw, self.metrics.len(), &ks, factors))
    }
}
