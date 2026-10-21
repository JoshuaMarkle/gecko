//! Embedded production UI or an explicitly selected filesystem build. API routing lives
//! outside this fallback, so missing API endpoints always remain JSON errors.

use std::path::{Path, PathBuf};

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn router(root: &Path, web: Option<PathBuf>) -> Router {
    #[cfg(feature = "embed-web")]
    if web.is_none() {
        eprintln!("web: embedded ({} assets)", embedded::ASSETS.len());
        return Router::new().fallback(embedded::serve);
    }
    let web = web.unwrap_or_else(|| root.join("web/dist"));
    if !web.join("index.html").is_file() {
        eprintln!("warning: {} has no index.html; build the UI with `cd web && npm ci && npm run wasm && npm run build`", web.display());
    }
    eprintln!("web: {}", web.display());
    Router::new().fallback_service(ServeDir::new(&web).fallback(ServeFile::new(web.join("index.html"))))
}

#[cfg(feature = "embed-web")]
mod embedded {
    use axum::body::Body;
    use axum::http::{header, Method, StatusCode, Uri};
    use axum::response::{IntoResponse, Response};

    include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));

    pub async fn serve(method: Method, uri: Uri) -> Response {
        if method != Method::GET && method != Method::HEAD {
            return (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "GET, HEAD")]).into_response();
        }
        let Ok(decoded) = percent_encoding::percent_decode_str(uri.path()).decode_utf8() else {
            return StatusCode::BAD_REQUEST.into_response();
        };
        let path = decoded.trim_start_matches('/');
        if path.contains('\\') || path.split('/').any(|s| s == "." || s == "..") {
            return StatusCode::NOT_FOUND.into_response();
        }
        let asset = ASSETS.binary_search_by_key(&path, |(name, _)| *name).ok().map(|i| ASSETS[i]);
        let (name, bytes) = match asset {
            Some(asset) => asset,
            // Extensionless navigation routes load the SPA; missing files retain 404,
            // especially JS/WASM requests whose HTML fallback would hide broken builds.
            None if path.starts_with("assets/") || path.rsplit('/').next().is_some_and(|s| s.contains('.')) => {
                return StatusCode::NOT_FOUND.into_response();
            }
            None => ASSETS.iter().find(|(name, _)| *name == "index.html").copied().expect("build.rs requires index.html"),
        };
        let mime = mime_guess::from_path(name).first_or_octet_stream();
        let content_type = if mime.type_() == "text" { format!("{mime}; charset=utf-8") } else { mime.to_string() };
        let cache = if name.starts_with("assets/") { "public, max-age=31536000, immutable" } else { "no-cache" };
        Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, bytes.len())
            .header(header::CACHE_CONTROL, cache)
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .body(if method == Method::HEAD { Body::empty() } else { Body::from(bytes) })
            .unwrap()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use axum::body::to_bytes;

        #[tokio::test]
        async fn embedded_assets_have_mime_head_and_spa_fallback() {
            for (suffix, mime) in [(".html", "text/html"), (".js", "javascript"), (".css", "text/css"), (".wasm", "application/wasm")] {
                let (name, bytes) = ASSETS.iter().find(|(name, _)| name.ends_with(suffix)).expect("production web asset");
                let response = serve(Method::GET, format!("/{name}").parse().unwrap()).await;
                assert_eq!(response.status(), StatusCode::OK);
                assert!(response.headers()[header::CONTENT_TYPE].to_str().unwrap().contains(mime));
                assert_eq!(to_bytes(response.into_body(), usize::MAX).await.unwrap().as_ref(), *bytes);
                let head = serve(Method::HEAD, format!("/{name}").parse().unwrap()).await;
                assert_eq!(head.headers()[header::CONTENT_LENGTH], bytes.len().to_string());
                assert!(to_bytes(head.into_body(), usize::MAX).await.unwrap().is_empty());
            }
            let index = serve(Method::GET, "/".parse().unwrap()).await;
            let navigation = serve(Method::GET, "/layouts/example".parse().unwrap()).await;
            assert_eq!(to_bytes(index.into_body(), usize::MAX).await.unwrap(), to_bytes(navigation.into_body(), usize::MAX).await.unwrap());
            for path in ["/assets/missing.js", "/missing.wasm", "/%2e%2e/gecko.toml", "/assets/missing"] {
                assert_eq!(serve(Method::GET, path.parse().unwrap()).await.status(), StatusCode::NOT_FOUND, "{path}");
            }
            assert_eq!(serve(Method::POST, "/".parse().unwrap()).await.status(), StatusCode::METHOD_NOT_ALLOWED);
        }
    }
}
