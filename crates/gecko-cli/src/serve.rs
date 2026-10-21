//! `gecko serve`: the built web app plus a JSON API over `data/` and `runs/`, with live run
//! progress over SSE. JSON shapes match the CLI's `--json` output. See `context/web.md`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use futures_util::stream::{self, Stream, StreamExt};
use gecko_core::optimize::{Algo, Control, Params};
use gecko_core::report::analyze;
use gecko_core::run::{self, RunSpec};
use gecko_core::store::Store;
use gecko_core::{Layout, Profile, Scorer, SCHEMA};
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
        .route("/profiles", get(profiles))
        .route("/corpora", get(corpora))
        .route("/corpora/{name}", get(corpus_bin))
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
        .route("/runs/{id}/atlas/{file}", get(atlas_file))
        .route("/swap-matrix", post(swap_matrix))
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

async fn geometries(State(app): State<AppState>) -> ApiResult {
    let mut out = Vec::new();
    for id in app.store.list("geometries") {
        let g = app.store.geometry(&id)?;
        let mut v = serde_json::to_value(&g)?;
        v["id"] = json!(id);
        v["flat"] = serde_json::to_value(g.flat2d())?;
        out.push(v);
    }
    Ok(Json(json!(out)))
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
    for id in app.store.list("corpora") {
        let c = app.store.corpus(&id)?;
        out.push(json!({"id": id, "name": c.name, "chars": c.total_chars(), "charset": c.charset.iter().collect::<String>()}));
    }
    Ok(Json(json!(out)))
}

async fn corpus_bin(State(app): State<AppState>, Path(name): Path<String>) -> std::result::Result<Response, ApiError> {
    if name.contains('/') || name.contains("..") {
        return Err(ApiError(StatusCode::BAD_REQUEST, "bad corpus name".into()));
    }
    let bytes = std::fs::read(app.store.corpus_path(&name)).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response())
}

/// Source texts of a corpus: `data/corpora/src/<name>/`, or for `english` the loose files in
/// `data/corpora/src/` (the Gutenberg novels it was built from). Sorted by file name.
fn source_files(app: &App, name: &str) -> Vec<PathBuf> {
    let src = app.store.corpus_path(name).with_file_name("src");
    let dir = src.join(name);
    let dir = if dir.is_dir() { dir } else if name == "english" { src } else { return Vec::new() };
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
}

struct Resolved {
    geometry: gecko_core::Geometry,
    corpus: gecko_core::Corpus,
    profile: Profile,
    layout: Layout,
}

fn resolve(store: &Store, r: &InputsReq) -> std::result::Result<Resolved, ApiError> {
    let d = &store.defaults;
    let geometry = store.geometry(r.geometry.as_deref().unwrap_or(&d.geometry))?;
    let corpus = store.corpus(r.corpus.as_deref().unwrap_or(&d.corpus))?;
    let profile = match &r.profile_override {
        Some(p) => p.clone(),
        None => store.profile(r.profile.as_deref().unwrap_or(&d.profile))?,
    };
    let layout = match (&r.chars, &r.layout) {
        (Some(c), _) => Layout::from_str_flat("custom", &geometry.name, c)?,
        (None, Some(spec)) => store.layout(spec, &geometry.name)?,
        (None, None) => store.layout(&d.reference, &geometry.name)?,
    }
    .fit(&geometry)?;
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
}

async fn start_run(State(app): State<AppState>, Json(req): Json<RunReq>) -> ApiResult {
    let r = resolve(&app.store, &req.inputs)?;
    let algo_name = req.algo.as_deref().unwrap_or("sa");
    let algo = Algo::parse(algo_name).ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, format!("unknown algo '{algo_name}'")))?;
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
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
    let spec = RunSpec {
        geometry: &r.geometry,
        corpus: &r.corpus,
        profile: &r.profile,
        start: &r.layout,
        pins: &req.pins,
        params: params.clone(),
    };
    let prep = run::prepare(&app.store.runs, &spec)?;
    let id = prep.config.id.clone();
    let (tx, _) = broadcast::channel(256);
    let live = Arc::new(LiveRun { tx, history: Mutex::new(Vec::new()), done: AtomicBool::new(false), control: Control::default() });
    app.live.lock().unwrap().insert(id.clone(), live.clone());
    let pins = req.pins.clone();
    std::thread::spawn(move || {
        let spec = RunSpec { geometry: &r.geometry, corpus: &r.corpus, profile: &r.profile, start: &r.layout, pins: &pins, params };
        let res = run::execute(&prep, &spec, &live.control, |p| live.send("progress", serde_json::to_string(p).unwrap()));
        match res {
            Ok(res) => live.send("done", serde_json::to_string(&res).unwrap()),
            Err(e) => live.send("failed", json!({"error": e.to_string()}).to_string()),
        }
        live.done.store(true, Ordering::SeqCst);
    });
    Ok(Json(json!({"schema": SCHEMA, "id": id, "config": spec_config(&app, &id)})))
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
    let config = run::load_config(&dir).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
    let result = run::load_result(&dir).ok();
    let (running, paused) = match app.live.lock().unwrap().get(&id) {
        Some(l) if !l.done.load(Ordering::SeqCst) => (true, l.control.paused()),
        _ => (false, false),
    };
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
            let res = run::load_result(&app.store.runs.join(&id)).map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;
            (vec![Msg { event: "done", data: serde_json::to_string(&res).unwrap() }], None)
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
}
