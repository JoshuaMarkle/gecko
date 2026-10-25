//! `gecko serve`: the built web app plus a JSON API over `data/` and `runs/`, with live run
//! progress over SSE. JSON shapes match the CLI's `--json` output. See `context/web.md`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use gecko_core::geomsearch::{self, GeomParams, GeomSpec};
use gecko_core::keyset;
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::analyze;
use gecko_core::run::{self, RunSpec};
use gecko_core::store::Store;
use gecko_core::{Corpus, Layout, Profile, Scorer, SCHEMA};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct Msg {
    event: &'static str,
    data: String,
}

struct LiveRun {
    tx: broadcast::Sender<Msg>,
    history: Mutex<Vec<Msg>>,
    done: AtomicBool,
    control: Control,
}

impl LiveRun {
    fn send(&self, event: &'static str, data: String) {
        let m = Msg { event, data };
        let mut h = self.history.lock().unwrap();
        // Keep the replay short: only the latest progress event matters for late subscribers.
        if event == "progress" {
            h.retain(|x| x.event != "progress");
        }
        h.push(m.clone());
        let _ = self.tx.send(m);
    }
}

struct App {
    store: Store,
    live: Mutex<HashMap<String, Arc<LiveRun>>>,
}

type AppState = Arc<App>;

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"schema": SCHEMA, "error": self.1}))).into_response()
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(StatusCode::BAD_REQUEST, e.to_string())
    }
}

type ApiResult = std::result::Result<Json<Value>, ApiError>;

fn router(store: Store, web: Option<PathBuf>) -> Router {
    let ui = crate::web_assets::router(&store.root, web);
    let app: AppState = Arc::new(App { store, live: Mutex::new(HashMap::new()) });
    let api = Router::new()
        .route("/meta", get(meta))
        .route("/layouts", get(layouts))
        .route("/geometries", get(geometries))
        .route("/geometries/{id}/free", get(geometry_free))
        .route("/profiles", get(profiles))
        .route("/corpora", get(corpora))
        .route("/corpora/mix", post(save_mix))
        .route("/corpora/{name}", get(corpus_bin))
        .route("/corpora/{name}/composition", get(corpus_composition))
        .route("/corpora/{name}/sources", get(corpus_sources))
        .route("/corpora/{name}/sources/{file}", get(corpus_source))
        .route("/score", post(score))
        .route("/runs", get(list_runs).post(start_run))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/events", get(run_events))
        .route("/runs/{id}/cancel", post(cancel_run))
        .route("/runs/{id}/pause", post(pause_run))
        .route("/runs/{id}/resume", post(resume_run))
        .route("/runs/{id}/atlas", get(atlas_summary))
        .route("/runs/{id}/qd", get(qd_summary))
        .route("/runs/{id}/atlas/{file}", get(atlas_file))
        .route("/swap-matrix", post(swap_matrix))
        .route("/path", post(path))
        .route("/bench", get(list_bench))
        .route("/bench/{id}", get(get_bench))
        .route("/", any(|| async { ApiError(StatusCode::NOT_FOUND, "unknown API route".into()) }))
        .fallback(|| async { ApiError(StatusCode::NOT_FOUND, "unknown API route".into()) })
        .with_state(app);
    Router::new()
        .nest("/api", api)
        .route("/api/", any(|| async { ApiError(StatusCode::NOT_FOUND, "unknown API route".into()) }))
        .fallback_service(ui)
}

pub fn serve(store: Store, port: u16, web: Option<PathBuf>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    rt.block_on(async move {
        let router = router(store, web);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        eprintln!("gecko serve: http://{}", listener.local_addr()?);
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = tokio::signal::ctrl_c().await;
            })
            .await?;
        Ok(())
    })
}

async fn meta(State(app): State<AppState>) -> ApiResult {
    Ok(Json(json!({
        "schema": SCHEMA,
        "version": env!("CARGO_PKG_VERSION"),
        "defaults": app.store.defaults,
        "algos": Algo::ALL.iter().map(|a| a.name()).collect::<Vec<_>>(),
        "metrics": gecko_core::metrics::METRICS,
        "threads": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
    })))
}

async fn layouts(State(app): State<AppState>) -> ApiResult {
    let mut out = Vec::new();
    // Layered layouts live in `layouts/layered/` (kept out of `compare`); their `chars` is the
    // flat string over the virtual layered slots.
    let layered = app.store.list("layouts/layered").into_iter().map(|id| format!("layered/{id}"));
    for id in app.store.list("layouts").into_iter().chain(layered) {
        let l = app.store.layout(&id, "")?;
        let chars: String = match app.store.geometry(&l.geometry).and_then(|g| l.flat(&g)) {
            Ok((_, flat)) if !l.layers.is_empty() => flat.iter().collect(),
            _ => l.chars.iter().collect(),
        };
        out.push(json!({"id": id, "name": l.name, "geometry": l.geometry, "source": l.source, "chars": chars, "layers": l.layers.iter().map(|ly| &ly.name).collect::<Vec<_>>()}));
    }
    Ok(Json(json!(out)))
}

/// Every board in `data/geometries/`, then the best board of each finished geometry run
/// (`run:<id>`, with `run`). Boards with a parameter file (`<id>.param.toml`, or a run's
/// `geometry.param.toml`) carry it as `param`: they can be geometry-searched.
async fn geometries(State(app): State<AppState>) -> ApiResult {
    let mut out = Vec::new();
    let runs =
        run::list(&app.store.runs).into_iter().filter(|r| r.kind == "geometry" && r.status == "done").map(|r| format!("run:{}", r.id));
    for id in app.store.list("geometries").into_iter().chain(runs) {
        let Ok(g) = app.store.geometry(&id) else { continue };
        let mut v = serde_json::to_value(&g)?;
        v["id"] = json!(id);
        v["hash"] = json!(gecko_core::hash::hex(g.hash()));
        v["flat"] = serde_json::to_value(g.flat2d())?;
        if let Ok(p) = app.store.param_geometry(&id) {
            v["param"] = serde_json::to_value(p)?;
        }
        if let Some(run) = id.strip_prefix("run:") {
            v["run"] = json!(run);
        }
        out.push(v);
    }
    Ok(Json(json!(out)))
}

#[derive(Deserialize)]
struct FreeReq {
    #[serde(default = "two_d")]
    spec: String,
}
fn two_d() -> String {
    "2d".into()
}

/// Free parameters (`geomsearch::parse_free`) of a parametric board: names, bounds, start values.
async fn geometry_free(State(app): State<AppState>, Path(id): Path<String>, Query(q): Query<FreeReq>) -> ApiResult {
    let p = app.store.param_geometry(store_name("geometry", &id)?)?;
    Ok(Json(json!({"schema": SCHEMA, "vars": geomsearch::parse_free(&p, &q.spec)?})))
}

async fn profiles(State(app): State<AppState>) -> ApiResult {
    let mut out = Vec::new();
    for id in app.store.list("profiles") {
        let p = app.store.profile(&id)?;
        let mut v = serde_json::to_value(&p)?;
        v["id"] = json!(id);
        out.push(v);
    }
    Ok(Json(json!(out)))
}

async fn corpora(State(app): State<AppState>) -> ApiResult {
    let mut out = Vec::new();
    let index = app.store.corpus_index()?;
    for id in app.store.list("corpora") {
        let c = app.store.corpus(&id)?;
        let recipe = app.store.recipe(&id)?;
        let meta = index.get(&id).cloned().unwrap_or_default();
        let kind = if meta.kind.is_empty() {
            if recipe.is_some() {
                "language"
            } else {
                "component"
            }
        } else {
            &meta.kind
        };
        let description = match (&recipe, meta.description.is_empty()) {
            (Some(r), true) => r.description.clone(),
            _ => meta.description.clone(),
        };
        out.push(json!({
            "id": id, "name": c.name, "chars": c.total_chars(), "charset": c.charset.iter().collect::<String>(),
            "hash": gecko_core::hash::hex(c.hash()), "kind": kind, "description": description, "source": meta.source,
            "shift": c.has_shift(),
            "recipe": recipe.map(|r| json!({"parts": r.parts.iter().zip(r.shares()).map(|(p, s)| json!({"corpus": p.corpus, "weight": p.weight, "share": s})).collect::<Vec<_>>(), "edits": r.edits, "nav": r.nav})),
        }));
    }
    Ok(Json(json!(out)))
}

/// Per-symbol keystroke shares of a recipe's parts (each over its own keystrokes) and of the
/// built language, for the Language tab's composition chart. A corpus without a recipe is one
/// part of itself.
async fn corpus_composition(State(app): State<AppState>, Path(name): Path<String>) -> ApiResult {
    safe_name(&name)?;
    let shares = |c: &Corpus| -> Value {
        let tot = c.total_chars().max(1) as f64;
        Value::Object(c.charset.iter().zip(&c.mono).map(|(ch, &n)| (ch.to_string(), json!(n as f64 / tot))).collect())
    };
    let built = app.store.corpus(&name)?;
    let (parts, edits, nav) = match app.store.recipe(&name)? {
        Some(r) => {
            let mut parts = Vec::new();
            for (p, share) in r.parts.iter().zip(r.shares()) {
                let c = app.store.corpus(&p.corpus)?;
                parts.push(json!({"corpus": p.corpus, "weight": p.weight, "share": share, "mono": shares(&c), "recipe": app.store.recipe_path(&p.corpus).exists()}));
            }
            (parts, r.edits, r.nav)
        }
        None => (vec![json!({"corpus": name, "weight": 1.0, "share": 1.0, "mono": shares(&built), "recipe": false})], 0.0, 0.0),
    };
    Ok(Json(json!({"schema": SCHEMA, "corpus": name, "parts": parts, "edits": edits, "nav": nav, "mono": shares(&built)})))
}

#[derive(Deserialize)]
struct MixReq {
    name: String,
    parts: Vec<gecko_core::recipe::Part>,
    #[serde(default)]
    edits: f64,
    #[serde(default)]
    nav: f64,
    #[serde(default)]
    description: String,
}

/// Save a language recipe (`data/corpora/<name>.mix.toml`) and build its `.gkc`. New names, or
/// existing recipes; a corpus built from text is never overwritten.
async fn save_mix(State(app): State<AppState>, Json(req): Json<MixReq>) -> ApiResult {
    let ok = !req.name.is_empty() && req.name.len() <= 64 && req.name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(ApiError(StatusCode::BAD_REQUEST, "name: letters, digits, '-' and '_' only".into()));
    }
    if app.store.corpus_path(&req.name).exists() && !app.store.recipe_path(&req.name).exists() {
        return Err(ApiError(StatusCode::CONFLICT, format!("'{}' is a built corpus, not a recipe; choose another name", req.name)));
    }
    if app.store.corpus_index()?.contains_key(&req.name) {
        return Err(ApiError(StatusCode::CONFLICT, format!("'{}' is a bundled language; save the mix under another name", req.name)));
    }
    let r = gecko_core::recipe::Recipe { description: req.description, edits: req.edits, nav: req.nav, parts: req.parts };
    r.check()?;
    let parts = r
        .parts
        .iter()
        .map(|p| store_name("corpus", &p.corpus).map_err(|e| e.1).and_then(|n| app.store.corpus(n).map_err(|e| e.to_string())))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| ApiError(StatusCode::BAD_REQUEST, e))?;
    if r.parts.iter().any(|p| p.corpus == req.name) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "a recipe cannot mix itself".into()));
    }
    let c = r.build(&req.name, &parts)?;
    std::fs::write(app.store.recipe_path(&req.name), r.to_toml())?;
    std::fs::write(app.store.corpus_path(&req.name), c.to_file_bytes())?;
    Ok(Json(json!({"schema": SCHEMA, "id": req.name, "hash": gecko_core::hash::hex(c.hash()), "chars": c.total_chars()})))
}

async fn corpus_bin(State(app): State<AppState>, Path(name): Path<String>) -> std::result::Result<Response, ApiError> {
    if name.contains('/') || name.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "bad corpus name".into()));
    }
    let path = app.store.corpus_path(&name);
    let bytes = if !path.exists() && app.store.recipe_path(&name).exists() {
        app.store.corpus(&name)?.to_file_bytes()
    } else {
        std::fs::read(path).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?
    };
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}

/// Source texts of a corpus: `data/corpora/src/<name>/`, or for `english` the loose files in
/// `data/corpora/src/` (the Gutenberg novels it was built from). Sorted by file name.
fn source_files(app: &App, name: &str) -> Vec<PathBuf> {
    let src = app.store.corpus_path(name).with_file_name("src");
    let dir = src.join(name);
    let dir = if dir.is_dir() {
        dir
    } else if name == "english" {
        src
    } else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()).collect();
    out.sort();
    out
}

fn safe_name(s: &str) -> std::result::Result<(), ApiError> {
    if s.contains('/') || s.contains('\\') || s.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "bad name".into()));
    }
    Ok(())
}

async fn corpus_sources(State(app): State<AppState>, Path(name): Path<String>) -> ApiResult {
    safe_name(&name)?;
    let files: Vec<Value> = source_files(&app, &name)
        .iter()
        .map(|p| json!({"name": p.file_name().unwrap_or_default().to_string_lossy(), "bytes": p.metadata().map(|m| m.len()).unwrap_or(0)}))
        .collect();
    Ok(Json(json!(files)))
}

async fn corpus_source(State(app): State<AppState>, Path((name, file)): Path<(String, String)>) -> std::result::Result<Response, ApiError> {
    safe_name(&name)?;
    safe_name(&file)?;
    let path = source_files(&app, &name).into_iter().find(|p| p.file_name().is_some_and(|f| f.to_string_lossy() == file));
    let path = path.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, format!("no source '{file}' for corpus '{name}'")))?;
    let text = std::fs::read(&path).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], String::from_utf8_lossy(&text).into_owned()).into_response())
}

/// Shared input for scoring and runs: names default to gecko.toml; `profile_override` wins over `profile`.
#[derive(Deserialize, Default)]
#[serde(default)]
struct InputsReq {
    geometry: Option<String>,
    corpus: Option<String>,
    profile: Option<String>,
    profile_override: Option<Profile>,
    /// Slot-ordered symbols; or `layout` (any CLI layout spec).
    chars: Option<String>,
    layout: Option<String>,
    /// Shift pairs of `chars` (`["-_", "[{"]`), as a keyset run's results carry them.
    #[serde(default)]
    shifted: Vec<String>,
}

struct Resolved {
    geometry: gecko_core::Geometry,
    corpus: gecko_core::Corpus,
    profile: Profile,
    layout: Layout,
}

/// A store entry name from a request. The CLI also accepts file paths; the API must not read
/// files outside the store, so names (and `run:<id>` ids) may not hold path separators or `..`.
fn store_name<'a>(what: &str, name: &'a str) -> std::result::Result<&'a str, ApiError> {
    let body = name.strip_prefix("run:").unwrap_or(name);
    if body.is_empty() || body.contains(['/', '\\']) || body.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, format!("bad {what} name '{name}'")));
    }
    Ok(name)
}

fn resolve(store: &Store, r: &InputsReq) -> std::result::Result<Resolved, ApiError> {
    let d = &store.defaults;
    let geometry = store.geometry(store_name("geometry", r.geometry.as_deref().unwrap_or(&d.geometry))?)?;
    let corpus = store.corpus(store_name("corpus", r.corpus.as_deref().unwrap_or(&d.corpus))?)?;
    let profile = match &r.profile_override {
        Some(p) => p.clone(),
        None => store.profile(store_name("profile", r.profile.as_deref().unwrap_or(&d.profile))?)?,
    };
    let layout = match (&r.chars, &r.layout) {
        (Some(c), _) => Layout::from_str_flat("custom", &geometry.name, c)?,
        // `raw:` and `cmini:` specs are symbols / bundle names, never paths.
        (None, Some(spec)) if spec.starts_with("raw:") || spec.starts_with("cmini:") => store.layout(spec, &geometry.name)?,
        (None, Some(spec)) => store.layout(store_name("layout", spec)?, &geometry.name)?,
        (None, None) => store.layout(&d.reference, &geometry.name)?,
    }
    .fit(&geometry)?;
    let layout = if r.shifted.is_empty() { layout } else { layout.with_shifted(r.shifted.clone())? };
    Ok(Resolved { geometry, corpus, profile, layout })
}

#[derive(Deserialize)]
struct ScoreReq {
    #[serde(flatten)]
    inputs: InputsReq,
    #[serde(default = "ten")]
    top: usize,
}
fn ten() -> usize {
    10
}

async fn score(State(app): State<AppState>, Json(req): Json<ScoreReq>) -> ApiResult {
    let v = tokio::task::spawn_blocking(move || -> std::result::Result<Value, ApiError> {
        let r = resolve(&app.store, &req.inputs)?;
        let sc = Scorer::new(&r.geometry, &r.corpus, &r.profile, &r.layout)?;
        let st = sc.identity();
        let chars: String = st.chars(&sc).iter().collect();
        Ok(json!({"schema": SCHEMA, "chars": chars, "rows": r.layout.all_rows(&r.geometry), "analysis": analyze(&sc, &st, req.top)}))
    })
    .await??;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct RunReq {
    #[serde(flatten)]
    inputs: InputsReq,
    #[serde(default)]
    algo: Option<String>,
    #[serde(default)]
    time_ms: Option<u64>,
    #[serde(default)]
    evals: Option<u64>,
    #[serde(default)]
    seed: Option<u64>,
    #[serde(default)]
    threads: Option<usize>,
    #[serde(default)]
    pins: String,
    #[serde(default)]
    sa_steps: Option<u64>,
    #[serde(default)]
    ga_pop: Option<usize>,
    #[serde(default)]
    memetic: Option<gecko_core::optimize::memetic::MemeticParams>,
    /// Extra descent neighbourhoods, e.g. "cycle3,depth2".
    #[serde(default)]
    moves: Option<String>,
    /// Corpus to score the results on without searching on it (L06).
    #[serde(default)]
    holdout: Option<String>,
    /// A board + layout search (G6) on the parametric `geometry` instead of a layout search.
    #[serde(default)]
    geom: Option<GeomReq>,
    /// Build the symbol set and start layout from the board and corpus (`gecko keyset`)
    /// instead of `chars` / `layout`.
    #[serde(default)]
    keyset: Option<KeysetReq>,
}

/// `optimize --keyset` options. `place` searches the fixed keys' places before the run
/// (synchronously, `place_evals` ≤ 5M SA evaluations per settle).
#[derive(Deserialize)]
struct KeysetReq {
    mode: String,
    #[serde(default)]
    symbols: Option<usize>,
    #[serde(default)]
    pairs: String,
    #[serde(default)]
    digits: Option<bool>,
    #[serde(default)]
    keys: Option<String>,
    #[serde(default)]
    layer_mode: String,
    #[serde(default)]
    thumb_letters: bool,
    #[serde(default)]
    place: bool,
    #[serde(default)]
    place_evals: Option<u64>,
}

/// Geometry search settings: `free` as `gecko geom optimize --free`, plus any `GeomParams`
/// field (budget, seed and threads come from the run request).
#[derive(Deserialize)]
struct GeomReq {
    free: String,
    #[serde(flatten)]
    search: GeomParams,
}

async fn start_run(State(app): State<AppState>, Json(mut req): Json<RunReq>) -> ApiResult {
    if let Some(g) = req.geom.take() {
        return start_geom_run(app, req, g);
    }
    let mut r = resolve(&app.store, &req.inputs)?;
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    if let Some(k) = &req.keyset {
        let spec = keyset::spec_from(&k.mode, k.symbols, &k.pairs, k.digits, k.keys.as_deref(), &k.layer_mode, k.thumb_letters)?;
        let ks = keyset::build(&r.geometry, &r.corpus, &spec)?;
        keyset::apply_profile(&r.geometry, &ks.layout, &mut r.profile, k.thumb_letters);
        r.layout = ks.layout;
        if k.place && r.layout.chars.iter().any(|&c| gecko_core::layout::is_fixed_key(c)) {
            let opts = keyset::PlaceOpts {
                evals: k.place_evals.unwrap_or(1_000_000).clamp(1000, 5_000_000),
                seed: req.seed.unwrap_or(1),
                threads: req.threads.unwrap_or(cores).clamp(1, cores.max(1)),
                pins: gecko_core::layout::parse_symbols(&req.pins)?,
                ..Default::default()
            };
            r.layout = keyset::place_fixed(&r.geometry, &r.corpus, &r.profile, &r.layout, &opts)?.layout;
        }
    }
    let algo_name = req.algo.as_deref().unwrap_or("sa");
    let algo = Algo::parse(algo_name).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, format!("unknown algo '{algo_name}'")))?;
    let mut params = Params {
        algo,
        seed: req.seed.unwrap_or(1),
        threads: req.threads.unwrap_or(cores).clamp(1, cores.max(1)),
        time_ms: if req.evals.is_some() { None } else { Some(req.time_ms.unwrap_or(10_000).min(3_600_000)) },
        evals: req.evals,
        top: 10,
        ..Default::default()
    };
    if let Some(s) = req.sa_steps {
        params.sa.steps = if s == 0 { 0 } else { s.max(1000) };
    }
    if let Some(p) = req.ga_pop {
        params.ga.pop_size = p.clamp(4, 10_000);
    }
    if let Some(mut m) = req.memetic {
        m.ga.pop_size = m.ga.pop_size.clamp(4, 10_000);
        m.islands = m.islands.min(cores.max(1));
        m.local_steps = m.local_steps.min(10_000);
        m.migrants = m.migrants.min(m.ga.pop_size);
        params.memetic = m;
    }
    if let Some(m) = &req.moves {
        params.moves = gecko_core::optimize::moves::MoveParams::parse(m)
            .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, format!("unknown moves '{m}'")))?;
    }
    let holdout = req
        .holdout
        .as_deref()
        .filter(|h| !h.is_empty())
        .map(|h| Ok::<_, ApiError>(app.store.corpus(store_name("corpus", h)?)?))
        .transpose()?;
    let spec = RunSpec {
        geometry: &r.geometry,
        corpus: &r.corpus,
        profile: &r.profile,
        start: &r.layout,
        pins: &req.pins,
        params: params.clone(),
        holdout: holdout.as_ref(),
    };
    let prep = run::prepare(&app.store.runs, &spec)?;
    let id = prep.config.id.clone();
    let (tx, _) = broadcast::channel(256);
    let live = Arc::new(LiveRun { tx, history: Mutex::new(Vec::new()), done: AtomicBool::new(false), control: Control::default() });
    app.live.lock().unwrap().insert(id.clone(), live.clone());
    let pins = req.pins.clone();
    std::thread::spawn(move || {
        let spec = RunSpec {
            geometry: &r.geometry,
            corpus: &r.corpus,
            profile: &r.profile,
            start: &r.layout,
            pins: &pins,
            params,
            holdout: holdout.as_ref(),
        };
        let res = run::execute(&prep, &spec, &live.control, |p| live.send("progress", serde_json::to_string(p).unwrap()));
        match res {
            Ok(res) => live.send("done", serde_json::to_string(&res).unwrap()),
            Err(e) => live.send("failed", json!({"error": e.to_string()}).to_string()),
        }
        live.done.store(true, Ordering::SeqCst);
    });
    Ok(Json(json!({"schema": SCHEMA, "id": id, "config": spec_config(&app, &id)})))
}

fn start_geom_run(app: AppState, req: RunReq, g: GeomReq) -> ApiResult {
    let r = resolve(&app.store, &req.inputs)?;
    let name = req.inputs.geometry.clone().unwrap_or_else(|| app.store.defaults.geometry.clone());
    let (params, board) = app.store.param_board(&name)?;
    let start = r.layout.fit(&board)?;
    let vars = geomsearch::parse_free(&params, &g.free)?;
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let mut s = g.search;
    s.seed = req.seed.unwrap_or(1);
    s.threads = req.threads.unwrap_or(cores).clamp(1, cores.max(1));
    s.evals = req.evals.map(|e| e.clamp(1, 1_000_000));
    s.time_ms = if s.evals.is_some() { None } else { Some(req.time_ms.unwrap_or(60_000).clamp(1000, 3_600_000)) };
    s.inner_evals = s.inner_evals.min(10_000_000);
    s.polish_evals = s.polish_evals.min(1_000_000_000);
    s.popsize = s.popsize.map(|p| p.clamp(2, 1000));
    s.rounds = s.rounds.clamp(1, 100);
    let spec = GeomSpec {
        params: &params,
        board: &board,
        corpus: &r.corpus,
        profile: &r.profile,
        start: &start,
        pins: &req.pins,
        vars: &vars,
        search: s.clone(),
    };
    let prep = geomsearch::prepare(&app.store.runs, &spec, &g.free)?;
    let id = prep.id.clone();
    let config = json!(prep.config);
    let (tx, _) = broadcast::channel(256);
    let live = Arc::new(LiveRun { tx, history: Mutex::new(Vec::new()), done: AtomicBool::new(false), control: Control::default() });
    app.live.lock().unwrap().insert(id.clone(), live.clone());
    let pins = req.pins;
    std::thread::spawn(move || {
        let spec = GeomSpec {
            params: &params,
            board: &board,
            corpus: &r.corpus,
            profile: &r.profile,
            start: &start,
            pins: &pins,
            vars: &vars,
            search: s,
        };
        let res = geomsearch::execute(&prep, &spec, &live.control, |p| live.send("progress", serde_json::to_string(p).unwrap()));
        match res {
            Ok(res) => live.send("done", serde_json::to_string(&res).unwrap()),
            Err(e) => live.send("failed", json!({"error": e.to_string()}).to_string()),
        }
        live.done.store(true, Ordering::SeqCst);
    });
    Ok(Json(json!({"schema": SCHEMA, "id": id, "config": config})))
}

fn spec_config(app: &App, id: &str) -> Value {
    run::load_config(&app.store.runs.join(id)).ok().map(|c| json!(c)).unwrap_or(Value::Null)
}

fn check_id(id: &str) -> std::result::Result<(), ApiError> {
    if id.is_empty() || id.contains('/') || id.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "bad run id".into()));
    }
    Ok(())
}

async fn list_runs(State(app): State<AppState>) -> ApiResult {
    let live = app.live.lock().unwrap();
    let v: Vec<Value> = run::list(&app.store.runs)
        .into_iter()
        .map(|s| {
            let mut v = json!(s);
            if let Some(l) = live.get(&s.id) {
                if !l.done.load(Ordering::SeqCst) {
                    v["status"] = json!("running");
                }
            }
            v
        })
        .collect();
    Ok(Json(json!(v)))
}

async fn get_run(State(app): State<AppState>, Path(id): Path<String>) -> ApiResult {
    check_id(&id)?;
    let dir = app.store.runs.join(&id);
    let (running, paused) = match app.live.lock().unwrap().get(&id) {
        Some(l) if !l.done.load(Ordering::SeqCst) => (true, l.control.paused()),
        _ => (false, false),
    };
    if geomsearch::is_run(&dir) {
        let config = geomsearch::load_config(&dir).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
        let param = geomsearch::load_start(&dir)?;
        return Ok(Json(json!({"schema": SCHEMA, "kind": "geometry", "config": config, "param": param,
            "result": geomsearch::load_result(&dir).ok(), "progress": geomsearch::load_progress(&dir), "running": running, "paused": paused})));
    }
    let config = run::load_config(&dir).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    let result = run::load_result(&dir).ok();
    Ok(Json(
        json!({"schema": SCHEMA, "config": config, "result": result, "progress": run::load_progress(&dir), "running": running, "paused": paused}),
    ))
}

async fn cancel_run(State(app): State<AppState>, Path(id): Path<String>) -> ApiResult {
    check_id(&id)?;
    if let Some(l) = app.live.lock().unwrap().get(&id) {
        l.control.cancel();
    }
    Ok(Json(json!({"schema": SCHEMA, "id": id, "cancelled": true})))
}

async fn pause_run(State(app): State<AppState>, Path(id): Path<String>) -> ApiResult {
    set_paused(&app, &id, true)
}

async fn resume_run(State(app): State<AppState>, Path(id): Path<String>) -> ApiResult {
    set_paused(&app, &id, false)
}

fn set_paused(app: &App, id: &str, on: bool) -> ApiResult {
    check_id(id)?;
    let live = app.live.lock().unwrap();
    let l = live
        .get(id)
        .filter(|l| !l.done.load(Ordering::SeqCst))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "run is not running".into()))?;
    l.control.set_paused(on);
    Ok(Json(json!({"schema": SCHEMA, "id": id, "paused": on})))
}

async fn atlas_summary(State(app): State<AppState>, Path(id): Path<String>) -> std::result::Result<Response, ApiError> {
    check_id(&id)?;
    let bytes = std::fs::read(app.store.runs.join(&id).join("atlas.json")).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], bytes).into_response())
}

/// `qd.json` of a MAP-Elites run (the grid and every elite).
async fn qd_summary(State(app): State<AppState>, Path(id): Path<String>) -> std::result::Result<Response, ApiError> {
    check_id(&id)?;
    let bytes = std::fs::read(app.store.runs.join(&id).join("qd.json")).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], bytes).into_response())
}

/// Atlas binaries (`optima`, `edges`, `samples`); parsed in the browser by gecko-wasm.
async fn atlas_file(State(app): State<AppState>, Path((id, file)): Path<(String, String)>) -> std::result::Result<Response, ApiError> {
    check_id(&id)?;
    if !["optima", "edges", "samples"].contains(&file.as_str()) {
        return Err(ApiError(StatusCode::NOT_FOUND, format!("no atlas file '{file}'")));
    }
    let bytes =
        std::fs::read(app.store.runs.join(&id).join(format!("{file}.bin"))).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/octet-stream"), (header::CACHE_CONTROL, "no-store")], bytes).into_response())
}

#[derive(Deserialize)]
struct SwapReq {
    #[serde(flatten)]
    inputs: InputsReq,
}

/// Score change of every slot-pair swap for a layout (fallback for the WASM `swap_matrix`).
async fn swap_matrix(State(app): State<AppState>, Json(req): Json<SwapReq>) -> ApiResult {
    let v = tokio::task::spawn_blocking(move || -> std::result::Result<Value, ApiError> {
        let r = resolve(&app.store, &req.inputs)?;
        let sc = Scorer::new(&r.geometry, &r.corpus, &r.profile, &r.layout)?;
        let st = sc.identity();
        Ok(json!({"schema": SCHEMA, "n": sc.n, "score": st.score, "matrix": gecko_core::report::swap_matrix(&sc, &st)}))
    })
    .await??;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct PathReq {
    #[serde(flatten)]
    inputs: InputsReq,
    /// Target layout, slot-ordered like `chars`.
    to: String,
}

/// Path relinking from `chars` to `to` (fallback for the WASM `Engine.path`).
async fn path(State(app): State<AppState>, Json(req): Json<PathReq>) -> ApiResult {
    let v = tokio::task::spawn_blocking(move || -> std::result::Result<Value, ApiError> {
        let r = resolve(&app.store, &req.inputs)?;
        let sc = Scorer::new(&r.geometry, &r.corpus, &r.profile, &r.layout)?;
        let to = gecko_core::explore::path::target_sym(&sc, &req.to)?;
        let p = gecko_core::explore::path::relink(&sc, &sc.identity().sym, &to)?;
        Ok(json!({"schema": SCHEMA, "start": p.start, "steps": p.steps, "barrier": p.barrier}))
    })
    .await??;
    Ok(Json(v))
}

/// Saved `gecko bench` leaderboards, newest first, without their per-seed traces.
async fn list_bench(State(app): State<AppState>) -> ApiResult {
    let mut v: Vec<Value> = std::fs::read_dir(crate::cmd::bench::bench_dir(&app.store))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice::<Value>(&std::fs::read(e.path()).ok()?).ok())
        .map(|mut b| {
            if let Some(rs) = b["results"].as_array_mut() {
                for r in rs {
                    if let Some(o) = r.as_object_mut() {
                        o.remove("runs");
                    }
                }
            }
            b
        })
        .collect();
    v.sort_by(|a, b| b["id"].as_str().cmp(&a["id"].as_str()));
    Ok(Json(json!(v)))
}

async fn get_bench(State(app): State<AppState>, Path(id): Path<String>) -> std::result::Result<Response, ApiError> {
    check_id(&id)?;
    let bytes = std::fs::read(crate::cmd::bench::bench_dir(&app.store).join(format!("{id}.json")))
        .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/json"), (header::CACHE_CONTROL, "no-store")], bytes).into_response())
}

async fn run_events(
    State(app): State<AppState>,
    Path(id): Path<String>,
) -> std::result::Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>, ApiError> {
    check_id(&id)?;
    let live = app.live.lock().unwrap().get(&id).cloned();
    let (history, rx) = match &live {
        Some(l) => {
            let h = l.history.lock().unwrap();
            (h.clone(), Some(l.tx.subscribe()))
        }
        None => {
            let dir = app.store.runs.join(&id);
            let data = if geomsearch::is_run(&dir) {
                geomsearch::load_result(&dir).map(|r| serde_json::to_string(&r).unwrap())
            } else {
                run::load_result(&dir).map(|r| serde_json::to_string(&r).unwrap())
            };
            let data = data.map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
            (vec![Msg { event: "done", data }], None)
        }
    };
    let finished = history.iter().any(|m| m.event != "progress");
    let to_event = |m: Msg| Ok::<_, Infallible>(Event::default().event(m.event).data(m.data));
    let replay = stream::iter(history);
    let tail = match (rx, finished) {
        // Forward live events and end the stream right after the terminal (done/error) event.
        (Some(rx), false) => BroadcastStream::new(rx)
            .filter_map(|m| async move { m.ok() })
            .scan(false, |ended, m| {
                let out = if *ended { None } else { Some(m.clone()) };
                *ended |= m.event != "progress";
                futures_util::future::ready(out)
            })
            .boxed(),
        _ => stream::empty().boxed(),
    };
    Ok(Sse::new(replay.chain(tail).map(to_event)).keep_alive(KeepAlive::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn api_routes_and_json_404s_take_precedence_over_the_spa() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let app = router(Store::open(root).unwrap(), None);
        let response = app.clone().oneshot(Request::builder().uri("/api/meta").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        for path in ["/api/unknown-route", "/api", "/api/"] {
            let response = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
            assert_eq!(body["schema"], SCHEMA);
        }
    }

    #[tokio::test]
    async fn corpus_sources_list_and_reject_traversal() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let app = router(Store::open(root).unwrap(), None);
        let get = |uri: &str| app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap());
        let response = get("/api/corpora/english/sources").await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert!(body.is_array());
        let response = get("/api/corpora/english/sources/..%2Fenglish.gkc").await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = get("/api/corpora/english/sources/missing.txt").await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mixes_save_and_report_their_composition() {
        let root = std::env::temp_dir().join(format!("gecko-mix-test-{}", std::process::id()));
        let dir = root.join("data/corpora");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(root.join("gecko.toml"), "data = \"data\"\n").unwrap();
        std::fs::write(dir.join("index.toml"), "[keep]\nkind = \"language\"\n").unwrap();
        for (name, cs, text) in [("a", "ab\u{2423}", "ab ba ab"), ("b", "a(", "(a(a")] {
            let mut c = Corpus::new(name, cs);
            c.add_text(text);
            std::fs::write(dir.join(format!("{name}.gkc")), c.to_file_bytes()).unwrap();
        }
        let app = router(Store::open(&root).unwrap(), None);
        let post = |body: Value| {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/corpora/mix")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
        };
        let parts = json!([{"corpus": "a", "weight": 3}, {"corpus": "b", "weight": 1}]);
        assert_eq!(post(json!({"name": "m", "parts": parts, "edits": 0.05})).await.unwrap().status(), StatusCode::OK);
        assert!(dir.join("m.mix.toml").exists() && dir.join("m.gkc").exists());
        for (name, status) in [("a", StatusCode::CONFLICT), ("keep", StatusCode::CONFLICT), ("../x", StatusCode::BAD_REQUEST)] {
            assert_eq!(post(json!({"name": name, "parts": parts})).await.unwrap().status(), status, "{name}");
        }
        assert_eq!(post(json!({"name": "n", "parts": [{"corpus": "../a", "weight": 1}]})).await.unwrap().status(), StatusCode::BAD_REQUEST);
        let r = app.clone().oneshot(Request::builder().uri("/api/corpora/m/composition").body(Body::empty()).unwrap()).await.unwrap();
        let body: Value = serde_json::from_slice(&to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(body["parts"][0]["share"], 0.75);
        assert_eq!(body["edits"], 0.05);
        assert!(body["mono"]["("].as_f64().unwrap() > 0.0);
        let r = app.clone().oneshot(Request::builder().uri("/api/corpora").body(Body::empty()).unwrap()).await.unwrap();
        let list: Value = serde_json::from_slice(&to_bytes(r.into_body(), usize::MAX).await.unwrap()).unwrap();
        let m = list.as_array().unwrap().iter().find(|c| c["id"] == "m").unwrap();
        assert_eq!((m["kind"].as_str(), m["recipe"]["parts"][1]["corpus"].as_str()), (Some("language"), Some("b")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Request inputs are store names: the CLI's file-path fallback must not let the API read
    /// arbitrary files.
    #[tokio::test]
    async fn inputs_reject_paths() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let app = router(Store::open(root).unwrap(), None);
        let score = |body: Value| {
            let req = Request::builder().method("POST").uri("/api/score").header("content-type", "application/json");
            app.clone().oneshot(req.body(Body::from(body.to_string())).unwrap())
        };
        for body in [
            json!({"profile": "/etc/hostname"}),
            json!({"geometry": "../gecko.toml"}),
            json!({"corpus": "../corpora/english"}),
            json!({"layout": "/etc/hostname"}),
            json!({"layout": "run:../x"}),
        ] {
            let response = score(body.clone()).await.unwrap();
            let msg: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
            assert!(msg["error"].as_str().unwrap().starts_with("bad "), "{body}: {msg}");
        }
        assert_eq!(score(json!({"layout": "raw:qwertyuiopasdfghjkl;zxcvbnm,./"})).await.unwrap().status(), StatusCode::OK);
        let free = Request::builder().uri("/api/geometries/..%2Fx/free").body(Body::empty()).unwrap();
        assert_eq!(app.clone().oneshot(free).await.unwrap().status(), StatusCode::BAD_REQUEST);
    }

    /// Calls every JSON endpoint the web UI reads (runs go to a temporary directory). With
    /// `GECKO_API_SAMPLES=<file.ts>` it also writes the responses as TypeScript checked against
    /// the UI's `api` types (`npm run test:contract`), so a renamed or retyped field fails the
    /// typecheck instead of the UI.
    #[tokio::test(flavor = "multi_thread")]
    async fn api_samples() {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let mut store = Store::open(root).unwrap();
        store.runs = std::env::temp_dir().join(format!("gecko-api-samples-{}", std::process::id()));
        std::fs::create_dir_all(&store.runs).unwrap();
        let runs_dir = store.runs.clone();
        let app = router(store, None);
        let call = |method: &str, uri: &str, body: Option<Value>| {
            let req = Request::builder().method(method).uri(uri).header(header::CONTENT_TYPE, "application/json");
            let req = req.body(body.map_or_else(Body::empty, |b| Body::from(b.to_string()))).unwrap();
            let app = app.clone();
            let uri = uri.to_string();
            async move {
                let response = app.oneshot(req).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK, "{uri}");
                serde_json::from_slice::<Value>(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
            }
        };
        let first = |v: Value, n: usize| Value::Array(v.as_array().unwrap().iter().take(n).cloned().collect());
        // Start a run and wait for it to finish.
        let finished = |body: Value| {
            let call = &call;
            async move {
                let started = call("POST", "/api/runs", Some(body)).await;
                let id = started["id"].as_str().unwrap().to_string();
                for _ in 0..1200 {
                    let detail = call("GET", &format!("/api/runs/{id}"), None).await;
                    if detail["running"] == false && !detail["result"].is_null() {
                        return (started, detail, id);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                panic!("run {id} did not finish");
            }
        };
        let mut samples: Vec<(&str, Value)> = vec![
            ("meta", call("GET", "/api/meta", None).await),
            ("layouts", first(call("GET", "/api/layouts", None).await, 3)),
            ("geometries", first(call("GET", "/api/geometries", None).await, 2)),
            ("profiles", call("GET", "/api/profiles", None).await),
            ("corpora", call("GET", "/api/corpora", None).await),
            ("composition", call("GET", "/api/corpora/vim-programmer/composition", None).await),
            ("score", call("POST", "/api/score", Some(json!({"top": 3}))).await),
            ("swapMatrix", call("POST", "/api/swap-matrix", Some(json!({}))).await),
            ("path", call("POST", "/api/path", Some(json!({"to": "qwfpgjluy;arstdhneiozxcvbkm,./"}))).await),
            ("benches", call("GET", "/api/bench", None).await),
        ];
        let (started, detail, _) = finished(json!({"algo": "hc", "evals": 20000, "threads": 1})).await;
        samples.push(("startRun", started));
        assert!(detail["progress"].as_array().unwrap().iter().any(|p| p["workers"].as_array().is_some_and(|w| !w.is_empty())));
        samples.push(("run", detail));
        let (_, _, explore) = finished(json!({"algo": "explore", "evals": 20000, "threads": 1})).await;
        samples.push(("atlas", call("GET", &format!("/api/runs/{explore}/atlas"), None).await));
        let (_, _, qd) = finished(json!({"algo": "qd", "evals": 20000, "threads": 1})).await;
        samples.push(("qd", call("GET", &format!("/api/runs/{qd}/qd"), None).await));
        // A keyset run: Shift pairs travel in the config, and /api/score takes them back.
        let ks = json!({"geometry": "corne-36", "corpus": "writer", "algo": "hc", "evals": 20000, "threads": 1,
            "keyset": {"mode": "shift", "place": true, "place_evals": 2000}});
        let (started, detail, _) = finished(ks).await;
        let shifted = started["config"]["shifted"].clone();
        assert!(shifted.as_array().is_some_and(|p| !p.is_empty()), "{shifted}");
        assert_eq!(started["config"]["profile"]["thumb_zone"], "␣⌫⏎");
        let chars = detail["result"]["best"][0]["chars"].clone();
        let score = detail["result"]["best"][0]["score"].as_f64().unwrap();
        let rescored = call(
            "POST",
            "/api/score",
            Some(json!({"geometry": "corne-36", "corpus": "writer", "chars": chars,
            "shifted": shifted, "profile_override": started["config"]["profile"], "top": 1})),
        )
        .await;
        assert!((rescored["analysis"]["breakdown"]["score"].as_f64().unwrap() - score).abs() < 1e-6, "{rescored}");
        samples.push(("geometryFree", call("GET", "/api/geometries/corne-36/free?spec=3d", None).await));
        let geom = json!({"geometry": "corne-36", "profile": "hand", "evals": 12, "threads": 1,
            "geom": {"free": "fingers.stagger", "inner_evals": 500, "polish_evals": 2000}});
        let (started, detail, geom_id) = finished(geom).await;
        assert_eq!(detail["kind"], "geometry");
        assert!(detail["progress"].as_array().is_some_and(|p| !p.is_empty() && p[0]["values"].as_array().unwrap().len() == 5));
        let boards = call("GET", "/api/geometries", None).await;
        let board =
            boards.as_array().unwrap().iter().find(|g| g["id"] == format!("run:{geom_id}")).expect("the run's board is listed").clone();
        assert!(board["param"].is_object() && board["run"] == geom_id.as_str());
        // The run's board scores like any other.
        call("POST", "/api/score", Some(json!({"geometry": format!("run:{geom_id}"), "top": 1}))).await;
        samples.push(("startGeomRun", started));
        samples.push(("geomRun", detail));
        samples.push(("runs", call("GET", "/api/runs", None).await));
        std::fs::remove_dir_all(&runs_dir).ok();

        let Some(out) = std::env::var_os("GECKO_API_SAMPLES") else { return };
        let mut ts = String::from(
            "// Generated by `npm run test:contract` (gecko-cli serve::tests::api_samples). Do not edit.\n\
             import type { api } from \"../src/api\";\n\
             import type { Loose } from \"./loose\";\n\n\
             type Response<K extends keyof typeof api> = Loose<Awaited<ReturnType<(typeof api)[K]>>>;\n",
        );
        for (name, v) in samples {
            ts += &format!("\nexport const {name} = {} satisfies Response<\"{name}\">;\n", serde_json::to_string_pretty(&v).unwrap());
        }
        std::fs::write(out, ts).unwrap();
    }
}
