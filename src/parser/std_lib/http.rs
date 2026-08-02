use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    Router,
};
use dashmap::DashMap;
use reqwest;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;
use tower_http::services::ServeDir;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Response {
    pub status: i64,
    pub body: String,
    pub content_type: String,
    pub headers: DashMap<String, String>,
}

/// The HTTP method of an outbound request. An enum rather than a string, so an
/// unsupported method is a compile error instead of a runtime one.
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum HTTP_Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

/// One inbound request, handed to the program's `handle_request` function.
/// Routing happens in Nail against these fields rather than in a table here,
/// so a path pattern and its handler stay in the same place.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Request {
    pub method: String,
    pub path: String,
    pub query: DashMap<String, String>,
    pub headers: DashMap<String, String>,
    pub body: String,
}

/// One directory of static files and the URL prefix it is served under.
/// A list of these, rather than a single pair, because a real site serves
/// several trees (`/js`, `/images`, `/fonts`) from different directories.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Static {
    /// URL prefix the files answer on, e.g. `/images`.
    pub prefix: String,
    /// Directory on disk holding them, relative to the working directory.
    pub directory: String,
}

/// How the server runs, plus whatever the program wants its handler to see.
/// The options are typed fields rather than a bag of strings; `state` is the
/// one deliberate hashmap, because it carries application data - page content,
/// file paths - that only the program knows the shape of.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Config {
    /// Directories served as static files. Empty serves none.
    pub static_mounts: Vec<HTTP_Static>,
    /// Request bodies above this get 413. 0 uses the default of 1 MiB.
    pub max_body_bytes: i64,
    /// Handler deadline in seconds, after which the client gets 504. 0 uses 30.
    pub timeout_seconds: i64,
    /// Passed through to handle_request untouched.
    pub state: DashMap<String, String>,
}

/// The defaults, since Nail has no default field values.
pub fn http_default_config() -> HTTP_Config {
    HTTP_Config {
        static_mounts: Vec::new(),
        max_body_bytes: 0,
        timeout_seconds: 0,
        state: DashMap::new(),
    }
}

/// What the transpiler wraps `handle_request` in. Boxed because a Nail
/// function's future is not nameable at this layer.
pub type HandlerFuture = Pin<Box<dyn Future<Output = HTTP_Response> + Send>>;

/// Requests larger than this are rejected with 413 before the handler runs.
/// Overridable with the `max_body_bytes` config key.
const DEFAULT_MAX_BODY_BYTES: usize = 1024 * 1024;
/// A handler that never returns must not hold the connection open forever.
/// Overridable with the `timeout_seconds` config key.
const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// Escapes text for use inside HTML. Anything a client controls must go
/// through this before being embedded in a page - otherwise a crafted URL
/// executes attacker JavaScript on the site's own origin.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

/// Builds a response without panicking: if a header value is somehow
/// invalid, the client gets a 500 instead of the server thread crashing.
fn build_response(response: HTTP_Response) -> Response {
    let status = StatusCode::from_u16(u16::try_from(response.status).unwrap_or(500)).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let content_type = if response.content_type.is_empty() { "text/html; charset=utf-8".to_string() } else { response.content_type.clone() };

    let mut builder = Response::builder().status(status).header(header::CONTENT_TYPE, content_type);
    for entry in response.headers.iter() {
        builder = builder.header(entry.key().as_str(), entry.value().as_str());
    }

    match builder.body(Body::from(response.body)) {
        Ok(built) => built,
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build HTTP response: {}", e)).into_response(),
    }
}

fn header_map_to_nail(headers: &HeaderMap) -> DashMap<String, String> {
    let map = DashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(text) = value.to_str() {
            map.insert(name.as_str().to_string(), text.to_string());
        }
    }
    map
}

/// reqwest links its own version of the `http` crate, so its HeaderMap is a
/// distinct type from axum's and needs its own conversion.
fn reqwest_headers_to_nail(headers: &reqwest::header::HeaderMap) -> DashMap<String, String> {
    let map = DashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(text) = value.to_str() {
            map.insert(name.as_str().to_string(), text.to_string());
        }
    }
    map
}

/// Query strings and form bodies share an encoding, so both go through the
/// same parser rather than a second copy of the decoding rules here.
fn query_to_nail(uri: &Uri) -> DashMap<String, String> {
    match uri.query() {
        Some(query) => super::url::parse_query(query.to_string()),
        None => DashMap::new(),
    }
}

/// Splits a path into its non-empty segments, so `/a/b/` and `/a/b` match.
fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|segment| !segment.is_empty()).collect()
}

/// Matches a path against a pattern, returning the bound `:name` segments.
/// A trailing `*` in the pattern matches the rest of the path.
fn match_path(pattern: &str, path: &str) -> Option<DashMap<String, String>> {
    let pattern_segments = segments(pattern);
    let path_segments = segments(path);
    let params = DashMap::new();

    for (index, pattern_segment) in pattern_segments.iter().enumerate() {
        if *pattern_segment == "*" {
            return Some(params);
        }
        let Some(path_segment) = path_segments.get(index) else {
            return None;
        };
        if let Some(name) = pattern_segment.strip_prefix(':') {
            let decoded = urlencoding::decode(path_segment).map(|value| value.to_string()).unwrap_or_else(|_| (*path_segment).to_string());
            params.insert(name.to_string(), decoded);
        } else if pattern_segment != path_segment {
            return None;
        }
    }

    if path_segments.len() != pattern_segments.len() {
        return None;
    }
    Some(params)
}

/// Whether a request path matches a route pattern such as `/dictionary/:word`.
pub fn http_path_matches(pattern: String, path: String) -> bool {
    match_path(&pattern, &path).is_some()
}

/// The `:name` segments a pattern binds, e.g. `/dictionary/:word` against
/// `/dictionary/cat` yields `{word: cat}`. Empty when the pattern does not
/// match, so callers check with http_path_matches first.
pub fn http_path_params(pattern: String, path: String) -> DashMap<String, String> {
    match_path(&pattern, &path).unwrap_or_default()
}

// THE ONE AND ONLY HTTP SERVER FUNCTION
// Every request, whatever its method or path, is handed to the program's
// `handle_request` function; static files are served from `static_mounts` first.
pub async fn http_server<F>(port: i64, config: HTTP_Config, handler: F) -> Result<(), String>
where
    F: Fn(HTTP_Request, DashMap<String, String>) -> HandlerFuture + Clone + Send + Sync + 'static,
{
    let max_body_bytes = if config.max_body_bytes > 0 { config.max_body_bytes as usize } else { DEFAULT_MAX_BODY_BYTES };
    let timeout = Duration::from_secs(if config.timeout_seconds > 0 { config.timeout_seconds as u64 } else { DEFAULT_TIMEOUT_SECONDS });
    let static_mounts = config.static_mounts.clone();
    let state = config.state.clone();

    let dispatch = move |request: axum::extract::Request| {
        let handler = handler.clone();
        let state = state.clone();
        async move {
            let (parts, body) = request.into_parts();

            // Read the body under a cap: without one, a single request can push
            // megabytes through a handler on a small box.
            let body_bytes = match axum::body::to_bytes(body, max_body_bytes).await {
                Ok(bytes) => bytes,
                Err(_) => {
                    return (StatusCode::PAYLOAD_TOO_LARGE, Html(format!("<pre>413 - request body exceeds {} bytes</pre>", max_body_bytes))).into_response();
                }
            };

            let nail_request = HTTP_Request {
                method: parts.method.as_str().to_string(),
                path: parts.uri.path().to_string(),
                query: query_to_nail(&parts.uri),
                headers: header_map_to_nail(&parts.headers),
                body: String::from_utf8_lossy(&body_bytes).to_string(),
            };
            let requested_path = nail_request.path.clone();

            match tokio::time::timeout(timeout, handler(nail_request, state)).await {
                Ok(response) => build_response(response),
                Err(_) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    // The path is client-controlled, so it is escaped before
                    // being echoed back into HTML.
                    Html(format!("<pre>504 - handler timed out after {}s: {}</pre>", timeout.as_secs(), escape_html(&requested_path))),
                )
                    .into_response(),
            }
        }
    };

    let mut app = Router::new();
    for mount in static_mounts {
        if mount.prefix.is_empty() || mount.directory.is_empty() {
            continue;
        }
        // Serving files from Rust keeps binary assets (fonts, images, audio)
        // working without Nail needing a byte type.
        app = app.nest_service(&mount.prefix, ServeDir::new(mount.directory));
    }
    let app = app.fallback(dispatch);

    // Every interface by default, so `nail run` in a container or on a LAN
    // keeps working. Behind a reverse proxy, set BIND_ADDR=127.0.0.1 so the
    // server is unreachable from outside the machine no matter what the
    // firewall is doing.
    let addr: SocketAddr = match std::env::var("BIND_ADDR") {
        Ok(host) => format!("{}:{}", host, port)
            .parse()
            .map_err(|_| format!("http_server: invalid BIND_ADDR '{}'", host))?,
        Err(_) => SocketAddr::from(([0, 0, 0, 0], port as u16)),
    };
    println!("🔨 Nail HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("http_server: could not bind to port {}: {}", port, e))?;

    axum::serve(listener, app).await.map_err(|e| format!("http_server: server error: {}", e))?;

    Ok(())
}

pub async fn http_request(method: HTTP_Method, url: String, headers: DashMap<String, String>, body: String) -> Result<HTTP_Response, String> {
    let client = reqwest::Client::new();

    let mut request = match method {
        HTTP_Method::Get => client.get(&url),
        HTTP_Method::Post => client.post(&url),
        HTTP_Method::Put => client.put(&url),
        HTTP_Method::Delete => client.delete(&url),
        HTTP_Method::Patch => client.patch(&url),
    };

    // Add headers
    for entry in headers.iter() {
        let key = entry.key();
        let value = entry.value();
        request = request.header(key, value);
    }

    // Add body if not empty
    if !body.is_empty() {
        request = request.body(body);
    }

    let response = request.send().await.map_err(|e| format!("http_request: request to '{}' failed: {}", url, e))?;

    let status = response.status().as_u16() as i64;
    let response_headers = reqwest_headers_to_nail(response.headers());
    let content_type = response_headers.get("content-type").map(|entry| entry.value().clone()).unwrap_or_default();
    let response_body = response.text().await.map_err(|e| format!("http_request: could not read the response body from '{}': {}", url, e))?;

    Ok(HTTP_Response { status, body: response_body, content_type, headers: response_headers })
}
