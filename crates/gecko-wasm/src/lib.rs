//! Browser scorer: the same gecko-core code as the CLI, so numbers in the editor always match.
//! Built with `wasm-pack build crates/gecko-wasm --target web --out-dir ../../web/src/wasm/pkg`.

use gecko_core::report::analyze;
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
    pub fn analyze(&self, chars: &str, profile_json: &str, top: usize) -> Result<String, JsError> {
        let profile: Profile = serde_json::from_str(profile_json).map_err(js)?;
        let layout = Layout::from_str_flat("custom", &self.geometry.name, chars).map_err(js)?;
        let sc = Scorer::new(&self.geometry, &self.corpus, &profile, &layout).map_err(js)?;
        let st = sc.identity();
        let v =
            serde_json::json!({"schema": SCHEMA, "chars": chars, "rows": layout.rows(&self.geometry), "analysis": analyze(&sc, &st, top)});
        Ok(v.to_string())
    }
}
