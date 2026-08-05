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

/// One cookie on its way out to the browser. Every field is spelled out
/// because the defaults a cookie gets when a field is left off are the unsafe
/// ones: no expiry rule, readable by scripts, sent over plain HTTP, attached
/// to requests other sites make. `http_default_cookie` fills them in the safe
/// way, leaving name and value to the caller.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Cookie {
    pub name: String,
    pub value: String,
    /// The path the cookie is sent back on. `/` for a whole site.
    pub path: String,
    /// Lifetime in seconds. 0 makes a session cookie the browser drops when it
    /// closes; a negative value deletes a cookie already set.
    pub max_age: i64,
    /// Keeps the cookie out of reach of JavaScript, so a script injected into
    /// the page cannot read the session id.
    pub http_only: bool,
    /// Sends it over HTTPS only.
    pub secure: bool,
    /// `Strict`, `Lax` or `None` - how much of a cross-site request carries
    /// the cookie. Lax is the usual answer for a login session: it survives a
    /// normal link into the site but not a form another site submits.
    pub same_site: String,
}

/// A cookie with the safe answers already filled in: site-wide path, session
/// lifetime, hidden from scripts, HTTPS only, Lax.
pub fn http_default_cookie(name: String, value: String) -> HTTP_Cookie {
    return HTTP_Cookie { name, value, path: "/".to_string(), max_age: 0, http_only: true, secure: true, same_site: "Lax".to_string() };
}

/// A cookie name may not contain separators or spaces, and a value may not
/// contain the characters that end it. Letting either through would not make a
/// broken cookie so much as an extra header of the caller's choosing.
fn cookie_name_is_valid(name: &str) -> bool {
    return !name.is_empty() && name.chars().all(|ch| ch.is_ascii_graphic() && !"()<>@,;:\\\"/[]?={}".contains(ch));
}

fn cookie_value_is_valid(value: &str) -> bool {
    return value.chars().all(|ch| ch.is_ascii_graphic() && ch != ';' && ch != ',' && ch != '"' && ch != '\\');
}

/// Build the `Set-Cookie` header value for a cookie.
pub fn http_build_cookie(cookie: HTTP_Cookie) -> Result<String, String> {
    if !cookie_name_is_valid(&cookie.name) {
        return Err(format!("http_build_cookie: '{}' is not a usable cookie name", cookie.name));
    }
    if !cookie_value_is_valid(&cookie.value) {
        return Err(format!("http_build_cookie: the value of cookie '{}' contains a character a cookie cannot carry", cookie.name));
    }

    let same_site = match cookie.same_site.to_lowercase().as_str() {
        "strict" => "Strict",
        "lax" => "Lax",
        "none" => "None",
        other => return Err(format!("http_build_cookie: same_site is '{}', and a cookie understands only Strict, Lax or None", other)),
    };
    // SameSite=None is only honoured on a cookie that is also Secure, and
    // browsers drop the pair outright otherwise - better to say so here than
    // to have the cookie silently vanish.
    if same_site == "None" && !cookie.secure {
        return Err(format!("http_build_cookie: cookie '{}' asks for SameSite=None without Secure, which browsers reject", cookie.name));
    }

    let mut parts = vec![format!("{}={}", cookie.name, cookie.value), format!("Path={}", cookie.path)];
    if cookie.max_age != 0 {
        parts.push(format!("Max-Age={}", cookie.max_age));
    }
    if cookie.http_only {
        parts.push("HttpOnly".to_string());
    }
    if cookie.secure {
        parts.push("Secure".to_string());
    }
    parts.push(format!("SameSite={}", same_site));

    return Ok(parts.join("; "));
}

/// Parse the `Cookie` header a browser sends into name/value pairs. Cookies
/// arrive as one header holding every cookie for the site, separated by `; `,
/// which is why reading one out of it by hand goes wrong so often.
pub fn http_parse_cookies(header: String) -> DashMap<String, String> {
    let cookies = DashMap::new();

    for pair in header.split(';') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        // Only the first `=` separates: a value may contain more of them.
        match pair.split_once('=') {
            Some((name, value)) => {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                // A quoted value is the same value; the quotes are transport.
                let value = value.trim();
                let value = value.strip_prefix('"').and_then(|rest| rest.strip_suffix('"')).unwrap_or(value);
                cookies.insert(name.to_string(), value.to_string());
            }
            // A bare name with no value is not a cookie anyone can read.
            None => continue,
        }
    }

    return cookies;
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
    /// Where the raw body was written, when it was not text.
    ///
    /// Nail's strings are UTF-8, and most of a PNG is not, so a body that is not
    /// text cannot be handed over as `body` - reading it as text replaces every
    /// byte that is not valid UTF-8 and the file is ruined before the handler
    /// sees it. So the server writes those bytes straight to a file and passes
    /// the path instead, and the program moves or copies the file wherever it
    /// belongs. That is how an upload works without Nail needing a bytes type.
    ///
    /// Empty for a text body, where `body` holds it as usual. The file is in the
    /// machine's temporary directory and nothing cleans it up but the program:
    /// move it somewhere permanent or remove it.
    pub body_path: String,
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
    /// Origins browser pages may call this server from. Empty sends no CORS
    /// headers; a list of ["*"] allows any origin.
    pub cors_origins: Vec<String>,
    /// Adds the standard protective headers - nosniff, no framing, a tight
    /// referrer policy - to every response.
    pub security_headers: bool,
    /// Requests per client per minute before 429. 0 turns limiting off. The
    /// client is told apart by x-forwarded-for, so behind a reverse proxy this
    /// is per visitor, not per proxy.
    pub rate_limit_per_minute: i64,
    /// The page a rate-limited client sees, as HTML. Empty uses a plain one.
    pub rate_limit_message: String,
}

/// The defaults, since Nail has no default field values.
pub fn http_default_config() -> HTTP_Config {
    HTTP_Config {
        static_mounts: Vec::new(),
        max_body_bytes: 0,
        timeout_seconds: 0,
        state: DashMap::new(),
        cors_origins: Vec::new(),
        security_headers: false,
        rate_limit_per_minute: 0,
        rate_limit_message: String::new(),
    }
}

/// The page a rate-limited client sees.
fn rate_limit_page(message: &str) -> axum::response::Response {
    let body = if message.is_empty() { "<pre>429 - too many requests, slow down</pre>".to_string() } else { message.to_string() };
    return (StatusCode::TOO_MANY_REQUESTS, Html(body)).into_response();
}

lazy_static::lazy_static! {
    /// One counting window per client for the rate limit.
    static ref RATE_WINDOWS: DashMap<String, (i64, i64)> = DashMap::new();
}

/// The client's address as the reverse proxy reports it, or `direct` when
/// nothing is in front of the server.
fn client_key(headers: &axum::http::HeaderMap) -> String {
    if let Some(forwarded) = headers.get("x-forwarded-for").and_then(|value| value.to_str().ok()) {
        if let Some(first) = forwarded.split(',').next() {
            let first = first.trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    if let Some(real) = headers.get("x-real-ip").and_then(|value| value.to_str().ok()) {
        let real = real.trim();
        if !real.is_empty() {
            return real.to_string();
        }
    }
    return "direct".to_string();
}

/// Count a request against its client's minute window; true means over the cap.
fn over_rate_limit(limit_per_minute: i64, headers: &axum::http::HeaderMap) -> bool {
    if limit_per_minute <= 0 {
        return false;
    }
    let minute = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64 / 60).unwrap_or(0);
    let mut window = RATE_WINDOWS.entry(client_key(headers)).or_insert((minute, 0));
    if window.0 != minute {
        *window = (minute, 0);
    }
    window.1 += 1;
    let over = window.1 > limit_per_minute;
    drop(window);
    // The table would otherwise grow one entry per client forever.
    if RATE_WINDOWS.len() > 100_000 {
        RATE_WINDOWS.retain(|_, (window_minute, _)| *window_minute == minute);
    }
    return over;
}

/// The Access-Control-Allow-Origin value this request has earned, if any.
fn cors_origin_for(request_origin: Option<&str>, allowed: &[String]) -> Option<String> {
    if allowed.is_empty() {
        return None;
    }
    if allowed.iter().any(|origin| origin == "*") {
        return Some("*".to_string());
    }
    let origin = request_origin?;
    if allowed.iter().any(|candidate| candidate == origin) {
        return Some(origin.to_string());
    }
    return None;
}

/// Stamp the config-driven headers on a finished response.
fn apply_response_headers(response: &mut axum::response::Response, cors_origin: Option<String>, security_headers: bool) {
    let headers = response.headers_mut();
    if let Some(origin) = cors_origin {
        if let Ok(value) = axum::http::HeaderValue::from_str(&origin) {
            if origin != "*" {
                headers.insert("vary", axum::http::HeaderValue::from_static("Origin"));
            }
            headers.insert("access-control-allow-origin", value);
        }
    }
    if security_headers {
        headers.insert("x-content-type-options", axum::http::HeaderValue::from_static("nosniff"));
        headers.insert("x-frame-options", axum::http::HeaderValue::from_static("DENY"));
        headers.insert("referrer-policy", axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"));
    }
}

/// The whole answer to a CORS preflight, headers stamped.
fn preflight_response(cors_origin: Option<String>, security_headers: bool) -> axum::response::Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    {
        let headers = response.headers_mut();
        headers.insert("access-control-allow-methods", axum::http::HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, OPTIONS"));
        headers.insert("access-control-allow-headers", axum::http::HeaderValue::from_static("*"));
        headers.insert("access-control-max-age", axum::http::HeaderValue::from_static("86400"));
    }
    apply_response_headers(&mut response, cors_origin, security_headers);
    return response;
}

/// What reading a request body produced: text for the handler, or a file it was
/// written to. Exactly one is ever set.
enum ReceivedBody {
    Text(String),
    File(String),
    Empty,
}

/// Why a body could not be accepted, which the caller turns into a status.
#[derive(Debug)]
enum BodyRefused {
    TooLarge,
    CouldNotWrite(String),
    Unreadable,
}

/// A fresh path for one upload, in the directory bodies are spooled to. Taken as
/// an argument rather than read from the environment so a test can point it at a
/// directory of its own and see exactly which files were left behind.
fn upload_path(spool_directory: &std::path::Path) -> std::path::PathBuf {
    return spool_directory.join(format!("nail_upload_{}.bin", uuid::Uuid::new_v4()));
}

/// Reads the body, deciding as it goes whether the handler gets text or a path.
///
/// Small bodies are assembled in memory and handed over as text when they are
/// valid UTF-8 - a form post or a JSON call never touches the disk. A body that
/// grows past `SPOOL_ABOVE_BYTES` is written to a file from that point on, so a
/// hundred-megabyte upload costs a megabyte of memory rather than a hundred, and
/// the handler is given the path.
///
/// The text-or-file decision is made on the bytes rather than on `Content-Type`:
/// a client that sends a PNG labelled `text/plain` is not a reason to corrupt it,
/// and a form that posts UTF-8 without saying so is not a reason to write a file
/// nobody asked for. The one exception is size, above which everything goes to a
/// file - a body that large is not something a handler wants as a string anyway.
async fn receive_body(body: Body, max_body_bytes: usize, spool_directory: &std::path::Path) -> Result<ReceivedBody, BodyRefused> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let mut stream = body.into_data_stream();
    let mut buffered: Vec<u8> = Vec::new();
    let mut spooled: Option<(std::path::PathBuf, tokio::fs::File)> = None;
    let mut received = 0usize;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| BodyRefused::Unreadable)?;
        received += chunk.len();
        if received > max_body_bytes {
            // Whatever was written so far is of no use to anybody.
            if let Some((path, _)) = spooled.take() {
                let _ = tokio::fs::remove_file(&path).await;
            }
            return Err(BodyRefused::TooLarge);
        }

        match spooled.as_mut() {
            Some((path, file)) => {
                file.write_all(&chunk).await.map_err(|failure| BodyRefused::CouldNotWrite(format!("could not save the uploaded body to {}: {}", path.display(), failure)))?;
            }
            None => {
                buffered.extend_from_slice(&chunk);
                // Past the threshold: open the file, move what is in memory into
                // it, and keep going from there.
                if buffered.len() > SPOOL_ABOVE_BYTES {
                    let path = upload_path(spool_directory);
                    let mut file = tokio::fs::File::create(&path).await.map_err(|failure| BodyRefused::CouldNotWrite(format!("could not save the uploaded body to {}: {}", path.display(), failure)))?;
                    file.write_all(&buffered).await.map_err(|failure| BodyRefused::CouldNotWrite(format!("could not save the uploaded body to {}: {}", path.display(), failure)))?;
                    buffered = Vec::new();
                    spooled = Some((path, file));
                }
            }
        }
    }

    if let Some((path, mut file)) = spooled {
        file.flush().await.map_err(|failure| BodyRefused::CouldNotWrite(format!("could not save the uploaded body to {}: {}", path.display(), failure)))?;
        return Ok(ReceivedBody::File(path.to_string_lossy().to_string()));
    }
    if buffered.is_empty() {
        return Ok(ReceivedBody::Empty);
    }
    match String::from_utf8(buffered) {
        Ok(text) => return Ok(ReceivedBody::Text(text)),
        Err(not_text) => {
            // Not text, and small enough that it was still in memory.
            let bytes = not_text.into_bytes();
            let path = upload_path(spool_directory);
            tokio::fs::write(&path, bytes).await.map_err(|failure| BodyRefused::CouldNotWrite(format!("could not save the uploaded body to {}: {}", path.display(), failure)))?;
            return Ok(ReceivedBody::File(path.to_string_lossy().to_string()));
        }
    }
}

/// What the transpiler wraps `handle_request` in. Boxed because a Nail
/// function's future is not nameable at this layer.
pub type HandlerFuture = Pin<Box<dyn Future<Output = HTTP_Response> + Send>>;

/// Requests larger than this are rejected with 413 before the handler runs.
/// Overridable with the `max_body_bytes` config key.
///
/// Eight mebibytes rather than one: a photograph off a phone is three to five,
/// and a default that rejects a profile picture is a default nobody wants. The
/// cap is not what bounds memory - see `SPOOL_ABOVE_BYTES` for that - it is what
/// stops a client sending a hundred megabytes at a small box.
const DEFAULT_MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// A body bigger than this goes to a file as it arrives rather than being
/// assembled in memory, so what a request costs in memory does not depend on the
/// cap. Bodies under it - a form post, a JSON call - are handled without ever
/// touching the disk.
const SPOOL_ABOVE_BYTES: usize = 1024 * 1024;
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
    let cors_origins = config.cors_origins.clone();
    let security_headers = config.security_headers;
    let rate_limit_per_minute = config.rate_limit_per_minute;
    let rate_limit_message = config.rate_limit_message.clone();

    let dispatch = move |request: axum::extract::Request| {
        let handler = handler.clone();
        let state = state.clone();
        let cors_origins = cors_origins.clone();
        let rate_limit_message = rate_limit_message.clone();
        async move {
            let (parts, body) = request.into_parts();

            if over_rate_limit(rate_limit_per_minute, &parts.headers) {
                return rate_limit_page(&rate_limit_message);
            }
            let request_origin = parts.headers.get("origin").and_then(|value| value.to_str().ok()).map(|origin| origin.to_string());
            let cors_origin = cors_origin_for(request_origin.as_deref(), &cors_origins);
            if parts.method == axum::http::Method::OPTIONS && cors_origin.is_some() {
                return preflight_response(cors_origin, security_headers);
            }

            // Read the body under a cap: without one, a single request can push
            // megabytes through a handler on a small box.
            let (body_text, body_path) = match receive_body(body, max_body_bytes, &std::env::temp_dir()).await {
                Ok(ReceivedBody::Text(text)) => (text, String::new()),
                Ok(ReceivedBody::File(path)) => (String::new(), path),
                Ok(ReceivedBody::Empty) => (String::new(), String::new()),
                Err(BodyRefused::TooLarge) => {
                    return (StatusCode::PAYLOAD_TOO_LARGE, Html(format!("<pre>413 - request body exceeds {} bytes</pre>", max_body_bytes))).into_response();
                }
                Err(BodyRefused::CouldNotWrite(detail)) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("<pre>500 - {}</pre>", escape_html(&detail)))).into_response();
                }
                Err(BodyRefused::Unreadable) => {
                    return (StatusCode::BAD_REQUEST, Html("<pre>400 - the request body could not be read to the end</pre>".to_string())).into_response();
                }
            };

            // Kept so the file can be cleaned up after the handler, whatever the
            // handler does or fails to do with it.
            let spooled_body = body_path.clone();

            let nail_request = HTTP_Request {
                method: parts.method.as_str().to_string(),
                path: parts.uri.path().to_string(),
                query: query_to_nail(&parts.uri),
                headers: header_map_to_nail(&parts.headers),
                body: body_text,
                body_path,
            };
            let requested_path = nail_request.path.clone();

            let answer = tokio::time::timeout(timeout, handler(nail_request, state)).await;

            // An upload the handler kept has been moved or copied by now, so
            // what is left here is a file nobody wants. Removing it is the
            // server's job: a handler that returned early, panicked or timed out
            // would otherwise leave one behind on every request until the disk
            // filled.
            if !spooled_body.is_empty() {
                let _ = tokio::fs::remove_file(&spooled_body).await;
            }

            let mut finished = match answer {
                Ok(response) => build_response(response),
                Err(_) => (
                    StatusCode::GATEWAY_TIMEOUT,
                    // The path is client-controlled, so it is escaped before
                    // being echoed back into HTML.
                    Html(format!("<pre>504 - handler timed out after {}s: {}</pre>", timeout.as_secs(), escape_html(&requested_path))),
                )
                    .into_response(),
            };
            apply_response_headers(&mut finished, cors_origin, security_headers);
            finished
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

/// The name a method goes by in an error message.
fn method_name(method: HTTP_Method) -> &'static str {
    return match method {
        HTTP_Method::Get => "Get",
        HTTP_Method::Post => "Post",
        HTTP_Method::Put => "Put",
        HTTP_Method::Delete => "Delete",
        HTTP_Method::Patch => "Patch",
    };
}

/// The builder for one outbound request, with the caller's headers already on
/// it. Shared by every requesting function so they cannot drift apart in what
/// a method or a header means.
fn start_request(client: &reqwest::Client, method: HTTP_Method, url: &str, headers: &DashMap<String, String>) -> reqwest::RequestBuilder {
    let mut request = match method {
        HTTP_Method::Get => client.get(url),
        HTTP_Method::Post => client.post(url),
        HTTP_Method::Put => client.put(url),
        HTTP_Method::Delete => client.delete(url),
        HTTP_Method::Patch => client.patch(url),
    };

    for entry in headers.iter() {
        request = request.header(entry.key(), entry.value());
    }

    return request;
}

/// A finished reqwest response read into the shape Nail sees.
async fn read_response(response: reqwest::Response, url: &str, caller: &str) -> Result<HTTP_Response, String> {
    let status = response.status().as_u16() as i64;
    let response_headers = reqwest_headers_to_nail(response.headers());
    let content_type = response_headers.get("content-type").map(|entry| entry.value().clone()).unwrap_or_default();
    let response_body = response.text().await.map_err(|e| format!("{}: could not read the response body from '{}': {}", caller, url, e))?;

    return Ok(HTTP_Response { status, body: response_body, content_type, headers: response_headers });
}

pub async fn http_request(method: HTTP_Method, url: String, headers: DashMap<String, String>, body: String) -> Result<HTTP_Response, String> {
    let client = reqwest::Client::new();
    let mut request = start_request(&client, method, &url, &headers);

    // Add body if not empty
    if !body.is_empty() {
        request = request.body(body);
    }

    let response = request.send().await.map_err(|e| format!("http_request: request to '{}' failed: {}", url, e))?;

    return read_response(response, &url, "http_request").await;
}

/// Downloads a URL straight into a file, answering how many bytes were
/// written. The body is streamed to disk piece by piece as it arrives, so a
/// download costs the same memory whatever its size - this is the function for
/// fetching a release archive or a dataset, where `http_request` would read
/// the whole body into a string.
///
/// A response outside the 2xx range is an error naming the status, and any
/// failure once writing has begun removes the partial file rather than leaving
/// a half-download that looks whole.
pub async fn http_download_file(url: String, path: String) -> Result<i64, String> {
    use tokio::io::AsyncWriteExt;

    let client = reqwest::Client::new();
    let mut response = client.get(&url).send().await.map_err(|e| format!("http_download_file: request to '{}' failed: {}", url, e))?;

    let status = response.status().as_u16() as i64;
    if !(200..300).contains(&status) {
        return Err(format!("http_download_file: '{}' answered {} rather than success", url, status));
    }

    let mut file = tokio::fs::File::create(&path).await.map_err(|e| format!("http_download_file: could not write '{}': {}", path, e))?;
    let mut written: i64 = 0;

    loop {
        let piece = match response.chunk().await {
            Ok(Some(piece)) => piece,
            Ok(None) => break,
            Err(e) => {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(format!("http_download_file: the download from '{}' broke off partway: {}", url, e));
            }
        };
        if let Err(e) = file.write_all(&piece).await {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(format!("http_download_file: could not write '{}': {}", path, e));
        }
        written += piece.len() as i64;
    }

    if let Err(e) = file.flush().await {
        drop(file);
        let _ = tokio::fs::remove_file(&path).await;
        return Err(format!("http_download_file: could not write '{}': {}", path, e));
    }

    return Ok(written);
}

/// One part of a `multipart/form-data` body (RFC 7578) - the encoding a form
/// with a file in it uses, and the one file-upload APIs expect.
///
/// A part is either a field or a file, decided by `file_path`: leave it empty
/// and the part carries `value` as text, set it and the part carries that
/// file's bytes. The bytes never pass through Nail, so uploading a PNG needs no
/// byte type in the language. `http_part_text` and `http_part_file` fill in the
/// fields a caller has no opinion about.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Part {
    /// The form field name, e.g. `file` or `purpose`.
    pub name: String,
    /// The text this part carries. Ignored when `file_path` is set.
    pub value: String,
    /// A file on disk whose bytes are the part's content. Empty for a text part.
    pub file_path: String,
    /// The filename the receiving end is told. Empty takes the name from
    /// `file_path`, which is what a browser would send.
    pub file_name: String,
    /// The part's media type. Empty guesses it from the file's extension for a
    /// file part, and leaves it off for a text part.
    pub content_type: String,
}

/// A text field: one form value, no file.
pub fn http_part_text(name: String, value: String) -> HTTP_Part {
    return HTTP_Part { name, value, file_path: String::new(), file_name: String::new(), content_type: String::new() };
}

/// A file field: the file's bytes, its name taken from the path and its media
/// type guessed from the extension.
pub fn http_part_file(name: String, file_path: String) -> HTTP_Part {
    return HTTP_Part { name, value: String::new(), file_path, file_name: String::new(), content_type: String::new() };
}

/// Sends a `multipart/form-data` request built from the given parts.
///
/// The boundary belongs to the body, so this function sets Content-Type
/// itself - a caller who sets it too is rejected rather than quietly sending a
/// body no server can parse.
pub async fn http_request_multipart(method: HTTP_Method, url: String, headers: DashMap<String, String>, parts: &Vec<HTTP_Part>) -> Result<HTTP_Response, String> {
    if matches!(method, HTTP_Method::Get | HTTP_Method::Delete) {
        return Err(format!("http_request_multipart: {} carries no body; a multipart upload needs Post, Put or Patch", method_name(method)));
    }
    if parts.is_empty() {
        return Err("http_request_multipart: a multipart body needs at least one part; build them with http_part_text and http_part_file".to_string());
    }
    if headers.iter().any(|entry| entry.key().eq_ignore_ascii_case("content-type")) {
        return Err("http_request_multipart: the Content-Type header is set from the multipart boundary, so it cannot be passed in headers; remove it".to_string());
    }

    let mut form = reqwest::multipart::Form::new();
    for part in parts.iter() {
        if part.name.is_empty() {
            return Err("http_request_multipart: every part needs a form field name".to_string());
        }

        let built = if part.file_path.is_empty() {
            let mut text = reqwest::multipart::Part::text(part.value.clone());
            if !part.content_type.is_empty() {
                text = text.mime_str(&part.content_type).map_err(|e| format!("http_request_multipart: part '{}' has an unusable content type '{}': {}", part.name, part.content_type, e))?;
            }
            text
        } else {
            let bytes = tokio::fs::read(&part.file_path).await.map_err(|e| format!("http_request_multipart: could not read '{}' for part '{}': {}", part.file_path, part.name, e))?;

            let file_name = if part.file_name.is_empty() {
                std::path::Path::new(&part.file_path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .ok_or_else(|| format!("http_request_multipart: '{}' does not name a file, so part '{}' has no filename to send", part.file_path, part.name))?
            } else {
                part.file_name.clone()
            };

            let content_type = if part.content_type.is_empty() { super::mime::for_path(&part.file_path) } else { part.content_type.clone() };

            reqwest::multipart::Part::bytes(bytes)
                .file_name(file_name)
                .mime_str(&content_type)
                .map_err(|e| format!("http_request_multipart: part '{}' has an unusable content type '{}': {}", part.name, content_type, e))?
        };

        form = form.part(part.name.clone(), built);
    }

    let client = reqwest::Client::new();
    let request = start_request(&client, method, &url, &headers).multipart(form);
    let response = request.send().await.map_err(|e| format!("http_request_multipart: request to '{}' failed: {}", url, e))?;

    return read_response(response, &url, "http_request_multipart").await;
}

/// How hard to try when a request does not go through the first time.
///
/// Only failures the server describes as temporary are retried: a request that
/// never reached it, and the statuses that mean "not now" - 408, 429, 500, 502,
/// 503, 504. A 4xx the server understood and refused is returned as it is,
/// because sending it again would only be refused again.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HTTP_Retry {
    /// Total tries, counting the first. 1 disables retrying.
    pub attempts: i64,
    /// How long to wait after the first failure. Each further wait doubles it.
    pub initial_delay_ms: i64,
    /// The longest any single wait may grow to.
    pub max_delay_ms: i64,
    /// Deadline for one attempt, after which it counts as a failure and the
    /// next attempt starts. 0 waits forever.
    pub timeout_ms: i64,
}

/// Defaults worth having: three tries, a quarter second growing to five, and a
/// thirty second deadline per attempt.
pub fn http_default_retry() -> HTTP_Retry {
    return HTTP_Retry { attempts: 3, initial_delay_ms: 250, max_delay_ms: 5000, timeout_ms: 30000 };
}

/// The statuses worth sending the same request again for.
fn status_is_temporary(status: i64) -> bool {
    return matches!(status, 408 | 429 | 500 | 502 | 503 | 504);
}

/// A `Retry-After` of so many seconds, when the server sent one. The HTTP-date
/// form is ignored rather than guessed at, and the backoff is used instead.
fn retry_after_ms(headers: &DashMap<String, String>) -> Option<u64> {
    let value = headers.iter().find(|entry| entry.key().eq_ignore_ascii_case("retry-after"))?.value().trim().to_string();
    return value.parse::<u64>().ok().map(|seconds| seconds * 1000);
}

/// How long to wait before attempt number `failures + 1`, doubling each time up
/// to the ceiling. Half of the wait is randomised so that a thousand clients
/// knocked off by the same outage do not all come back in the same instant.
fn backoff_ms(retry: &HTTP_Retry, failures: u32) -> u64 {
    let ceiling = retry.max_delay_ms.max(0) as u64;
    let base = (retry.initial_delay_ms.max(0) as u64).saturating_mul(2u64.saturating_pow(failures.saturating_sub(1))).min(ceiling);
    let jitter = (base as f64 / 2.0 * rand::random::<f64>()) as u64;
    return base / 2 + jitter;
}

/// Makes a request, sending it again while it keeps failing in a way that
/// might not fail next time.
///
/// A response is returned whatever its status once the attempts run out, so a
/// program still sees the server's own 500 rather than an error from here. The
/// request is sent again as it was: an API that must not act twice on the same
/// call wants an idempotency key in the headers.
pub async fn http_request_retry(method: HTTP_Method, url: String, headers: DashMap<String, String>, body: String, retry: HTTP_Retry) -> Result<HTTP_Response, String> {
    if retry.attempts < 1 {
        return Err(format!("http_request_retry: attempts must be at least 1, got {}", retry.attempts));
    }
    if retry.initial_delay_ms < 0 || retry.max_delay_ms < 0 || retry.timeout_ms < 0 {
        return Err("http_request_retry: delays and the timeout are measured in milliseconds and cannot be negative".to_string());
    }

    let mut builder = reqwest::Client::builder();
    if retry.timeout_ms > 0 {
        builder = builder.timeout(Duration::from_millis(retry.timeout_ms as u64));
    }
    let client = builder.build().map_err(|e| format!("http_request_retry: could not build the HTTP client: {}", e))?;

    let mut last_error = String::new();
    for attempt in 1..=retry.attempts {
        let last_attempt = attempt == retry.attempts;

        let mut request = start_request(&client, method, &url, &headers);
        if !body.is_empty() {
            request = request.body(body.clone());
        }

        let wait_ms = match request.send().await {
            Ok(response) => {
                let read = read_response(response, &url, "http_request_retry").await?;
                if last_attempt || !status_is_temporary(read.status) {
                    return Ok(read);
                }
                last_error = format!("the server answered {}", read.status);
                retry_after_ms(&read.headers).map(|after| after.min(retry.max_delay_ms.max(0) as u64)).unwrap_or_else(|| backoff_ms(&retry, attempt as u32))
            }
            Err(e) => {
                if last_attempt {
                    return Err(format!("http_request_retry: request to '{}' failed after {} attempts: {}", url, retry.attempts, e));
                }
                last_error = e.to_string();
                backoff_ms(&retry, attempt as u32)
            }
        };

        tokio::time::sleep(Duration::from_millis(wait_ms)).await;
    }

    // Every path above either returns or sleeps and tries again, so the loop
    // running out means the attempt count was not what it claimed to be.
    return Err(format!("http_request_retry: request to '{}' made no attempt ({})", url, last_error));
}

#[cfg(test)]
mod multipart_tests {
    use super::*;

    fn no_headers() -> DashMap<String, String> {
        return DashMap::new();
    }

    #[tokio::test]
    async fn a_method_with_no_body_is_refused_before_anything_is_sent() {
        let parts = vec![http_part_text("field".to_string(), "value".to_string())];
        let error = http_request_multipart(HTTP_Method::Get, "http://127.0.0.1:1/".to_string(), no_headers(), &parts).await.unwrap_err();
        assert!(error.contains("Get carries no body"), "{}", error);
    }

    #[tokio::test]
    async fn a_caller_set_content_type_is_refused_rather_than_overwritten() {
        let headers = no_headers();
        headers.insert("Content-Type".to_string(), "multipart/form-data".to_string());
        let parts = vec![http_part_text("field".to_string(), "value".to_string())];
        let error = http_request_multipart(HTTP_Method::Post, "http://127.0.0.1:1/".to_string(), headers, &parts).await.unwrap_err();
        assert!(error.contains("Content-Type"), "{}", error);
    }

    #[tokio::test]
    async fn a_body_with_no_parts_is_an_error_worth_naming() {
        let error = http_request_multipart(HTTP_Method::Post, "http://127.0.0.1:1/".to_string(), no_headers(), &Vec::new()).await.unwrap_err();
        assert!(error.contains("at least one part"), "{}", error);
    }

    #[tokio::test]
    async fn a_file_part_names_the_file_it_could_not_read() {
        let parts = vec![http_part_file("upload".to_string(), "/nowhere/at/all.png".to_string())];
        let error = http_request_multipart(HTTP_Method::Post, "http://127.0.0.1:1/".to_string(), no_headers(), &parts).await.unwrap_err();
        assert!(error.contains("/nowhere/at/all.png"), "{}", error);
        assert!(error.contains("upload"), "{}", error);
    }

    #[test]
    fn a_text_part_carries_its_value_and_a_file_part_carries_its_path() {
        let text = http_part_text("purpose".to_string(), "avatar".to_string());
        assert_eq!(text.value, "avatar");
        assert!(text.file_path.is_empty());

        let file = http_part_file("file".to_string(), "report.pdf".to_string());
        assert_eq!(file.file_path, "report.pdf");
        assert!(file.value.is_empty());
    }
}

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[test]
    fn only_the_failures_that_might_pass_next_time_are_retried() {
        for temporary in [408, 429, 500, 502, 503, 504] {
            assert!(status_is_temporary(temporary), "{} should be retried", temporary);
        }
        // A request the server understood and refused would be refused again.
        for permanent in [200, 201, 301, 400, 401, 403, 404, 409, 422, 501] {
            assert!(!status_is_temporary(permanent), "{} should not be retried", permanent);
        }
    }

    #[test]
    fn the_wait_doubles_until_it_reaches_the_ceiling() {
        let retry = HTTP_Retry { attempts: 6, initial_delay_ms: 100, max_delay_ms: 400, timeout_ms: 0 };
        // Half of each wait is randomised, so the bound is what is checked.
        for (failures, base) in [(1u32, 100u64), (2, 200), (3, 400), (4, 400), (5, 400)] {
            let waited = backoff_ms(&retry, failures);
            assert!(waited >= base / 2 && waited <= base, "wait {} outside {}..={} for failure {}", waited, base / 2, base, failures);
        }
    }

    #[test]
    fn a_retry_after_in_seconds_is_read_and_a_date_is_left_alone() {
        let headers = DashMap::new();
        headers.insert("Retry-After".to_string(), "2".to_string());
        assert_eq!(retry_after_ms(&headers), Some(2000));

        let dated = DashMap::new();
        dated.insert("retry-after".to_string(), "Wed, 21 Oct 2026 07:28:00 GMT".to_string());
        assert_eq!(retry_after_ms(&dated), None);

        assert_eq!(retry_after_ms(&DashMap::new()), None);
    }

    #[tokio::test]
    async fn settings_that_could_never_work_are_refused_before_the_first_attempt() {
        let zero = HTTP_Retry { attempts: 0, initial_delay_ms: 10, max_delay_ms: 10, timeout_ms: 0 };
        let error = http_request_retry(HTTP_Method::Get, "http://127.0.0.1:1/".to_string(), DashMap::new(), String::new(), zero).await.unwrap_err();
        assert!(error.contains("attempts must be at least 1"), "{}", error);

        let negative = HTTP_Retry { attempts: 2, initial_delay_ms: -1, max_delay_ms: 10, timeout_ms: 0 };
        let error = http_request_retry(HTTP_Method::Get, "http://127.0.0.1:1/".to_string(), DashMap::new(), String::new(), negative).await.unwrap_err();
        assert!(error.contains("cannot be negative"), "{}", error);
    }

    #[tokio::test]
    async fn a_connection_that_never_opens_is_tried_the_agreed_number_of_times() {
        let retry = HTTP_Retry { attempts: 3, initial_delay_ms: 1, max_delay_ms: 2, timeout_ms: 200 };
        // Port 1 on loopback refuses at once, so this exercises the transport
        // failure path without waiting on a real network.
        let error = http_request_retry(HTTP_Method::Get, "http://127.0.0.1:1/".to_string(), DashMap::new(), String::new(), retry).await.unwrap_err();
        assert!(error.contains("after 3 attempts"), "{}", error);
    }
}

#[cfg(test)]
mod download_tests {
    use super::*;

    /// The smallest server a download can be tested against: a loopback
    /// listener answering every connection with the same canned response.
    async fn a_server_answering(canned: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a free loopback port");
        let address = listener.local_addr().expect("the port that was bound");
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut request = [0u8; 2048];
                    let _ = socket.read(&mut request).await;
                    let _ = socket.write_all(canned.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });
        return format!("http://{}/file", address);
    }

    fn a_download_path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("nail_download_{}", name));
        let _ = std::fs::remove_file(&path);
        return path;
    }

    #[tokio::test]
    async fn a_body_arrives_on_disk_with_its_byte_count() {
        let url = a_server_answering("HTTP/1.1 200 OK\r\nContent-Length: 20\r\nConnection: close\r\n\r\nthe downloaded bytes").await;
        let path = a_download_path("whole_body");
        let written = http_download_file(url, path.to_string_lossy().to_string()).await.expect("a downloadable file");
        assert_eq!(written, 20);
        assert_eq!(std::fs::read_to_string(&path).expect("the downloaded file"), "the downloaded bytes");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_status_outside_success_is_an_error_naming_it_and_writes_nothing() {
        let url = a_server_answering("HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found").await;
        let path = a_download_path("not_found");
        let error = http_download_file(url, path.to_string_lossy().to_string()).await.unwrap_err();
        assert!(error.contains("404"), "{}", error);
        assert!(!path.exists(), "a refused download must not create the file");
    }

    #[tokio::test]
    async fn a_path_that_cannot_be_written_names_itself() {
        let url = a_server_answering("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok").await;
        let error = http_download_file(url, "/nowhere/at/all/download.bin".to_string()).await.unwrap_err();
        assert!(error.contains("could not write"), "{}", error);
        assert!(error.contains("/nowhere/at/all/download.bin"), "{}", error);
    }

    /// A server that promises more bytes than it sends breaks the download
    /// partway, and the partial file must not be left looking whole.
    #[tokio::test]
    async fn a_download_that_breaks_off_leaves_no_partial_file() {
        let url = a_server_answering("HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nonly this much").await;
        let path = a_download_path("broken_off");
        let error = http_download_file(url, path.to_string_lossy().to_string()).await.unwrap_err();
        assert!(error.contains("broke off"), "{}", error);
        assert!(!path.exists(), "the partial file must be removed");
    }

    #[tokio::test]
    async fn a_url_that_is_not_a_url_fails_before_anything_is_written() {
        let path = a_download_path("bad_url");
        let error = http_download_file("not a url at all".to_string(), path.to_string_lossy().to_string()).await.unwrap_err();
        assert!(error.contains("failed"), "{}", error);
        assert!(!path.exists(), "a request that never went out must not create the file");
    }

    #[tokio::test]
    async fn a_connection_nobody_answers_is_an_error_not_a_file() {
        let path = a_download_path("refused");
        // Port 1 on loopback refuses at once, the same trick the retry tests use.
        let error = http_download_file("http://127.0.0.1:1/file".to_string(), path.to_string_lossy().to_string()).await.unwrap_err();
        assert!(error.contains("failed"), "{}", error);
        assert!(!path.exists());
    }
}

#[cfg(test)]
mod upload_tests {
    use super::*;

    /// A directory of this test's own, so what is left in it is only ever this
    /// test's doing - the tests run at the same time and share the machine's
    /// temporary directory otherwise.
    fn a_spool_directory(name: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("nail_spool_{}", name));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a writable temporary directory");
        return directory;
    }

    fn files_left_in(directory: &std::path::Path) -> usize {
        return std::fs::read_dir(directory).map(|entries| entries.count()).unwrap_or(0);
    }

    /// Reads a body the way the server does, from bytes a test supplies.
    async fn receive(bytes: Vec<u8>) -> ReceivedBody {
        return receive_body(Body::from(bytes), 16 * 1024 * 1024, &std::env::temp_dir()).await.expect("a body the server can accept");
    }

    #[tokio::test]
    async fn a_text_body_is_handed_over_as_text() {
        match receive(b"name=alex&city=calgary".to_vec()).await {
            ReceivedBody::Text(text) => assert_eq!(text, "name=alex&city=calgary"),
            _ => panic!("a small text body should not touch the disk"),
        }
    }

    #[tokio::test]
    async fn text_in_any_alphabet_is_still_text() {
        match receive("héllo, 世界".as_bytes().to_vec()).await {
            ReceivedBody::Text(text) => assert_eq!(text, "héllo, 世界"),
            _ => panic!("UTF-8 is text whatever alphabet it is in"),
        }
    }

    #[tokio::test]
    async fn an_empty_body_is_neither() {
        assert!(matches!(receive(Vec::new()).await, ReceivedBody::Empty));
    }

    /// The whole point: a PNG arrives byte for byte, where reading it as text
    /// would have replaced every byte that is not valid UTF-8.
    #[tokio::test]
    async fn a_binary_body_goes_to_a_file_unharmed() {
        let png_header: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0xff, 0xd8, 0x00, 0x01];
        let path = match receive(png_header.clone()).await {
            ReceivedBody::File(path) => path,
            _ => panic!("a body that is not text must go to a file"),
        };
        assert_eq!(std::fs::read(&path).expect("the file the server wrote"), png_header);
        // Lossy reading would have destroyed it, which is what this avoids.
        assert_ne!(String::from_utf8_lossy(&png_header).as_bytes(), png_header.as_slice());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn two_uploads_do_not_share_a_file() {
        let bytes: Vec<u8> = vec![0xff, 0xfe, 0x00];
        let first = match receive(bytes.clone()).await {
            ReceivedBody::File(path) => path,
            _ => panic!("binary goes to a file"),
        };
        let second = match receive(bytes).await {
            ReceivedBody::File(path) => path,
            _ => panic!("binary goes to a file"),
        };
        assert_ne!(first, second);
        let _ = std::fs::remove_file(&first);
        let _ = std::fs::remove_file(&second);
    }

    /// Past the threshold everything goes to a file, text or not, so what a
    /// request costs in memory does not depend on the cap.
    #[tokio::test]
    async fn a_body_past_the_threshold_is_written_as_it_arrives() {
        let large: Vec<u8> = vec![b'a'; SPOOL_ABOVE_BYTES + 4096];
        let path = match receive(large.clone()).await {
            ReceivedBody::File(path) => path,
            _ => panic!("a body over the threshold belongs in a file"),
        };
        assert_eq!(std::fs::metadata(&path).expect("the file the server wrote").len() as usize, large.len());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn a_body_over_the_cap_is_refused() {
        let directory = a_spool_directory("over_cap");
        let too_big: Vec<u8> = vec![b'a'; 3 * 1024 * 1024];
        let refused = receive_body(Body::from(too_big), 1024, &directory).await;
        assert!(matches!(refused, Err(BodyRefused::TooLarge)));
        assert_eq!(files_left_in(&directory), 0, "a refused body must not leave a file behind");
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A body that got as far as being written and was then refused for going
    /// over the cap must not leave the part-written file behind.
    #[tokio::test]
    async fn a_body_refused_after_it_started_spooling_leaves_nothing_behind() {
        let directory = a_spool_directory("refused_mid_spool");
        let over_threshold: Vec<u8> = vec![b'a'; SPOOL_ABOVE_BYTES + 4096];
        let cap = SPOOL_ABOVE_BYTES + 1024;

        // Arrives in pieces, so the threshold is crossed - a file is opened -
        // before the cap is reached on a later piece.
        let pieces: Vec<Result<Vec<u8>, std::io::Error>> = over_threshold.chunks(64 * 1024).map(|piece| Ok(piece.to_vec())).collect();
        let stream = futures::stream::iter(pieces);
        let refused = receive_body(Body::from_stream(stream), cap, &directory).await;

        assert!(matches!(refused, Err(BodyRefused::TooLarge)));
        assert_eq!(files_left_in(&directory), 0, "the part-written file must be removed when the body is refused");
        let _ = std::fs::remove_dir_all(&directory);
    }
}

/// The boundary out of a `multipart/form-data` content type. The client chooses
/// it, and without it the body cannot be split at all.
fn multipart_boundary(content_type: &str) -> Result<String, String> {
    let lowered = content_type.to_lowercase();
    if !lowered.starts_with("multipart/") {
        return Err(format!("the content type is `{}`, not multipart/form-data, so there are no parts to take out of it", content_type));
    }
    for parameter in content_type.split(';').skip(1) {
        let (name, value) = match parameter.split_once('=') {
            Some(both) => both,
            None => continue,
        };
        if name.trim().eq_ignore_ascii_case("boundary") {
            let boundary = value.trim().trim_matches('"');
            if boundary.is_empty() {
                return Err("the content type gives an empty boundary, so the body cannot be split".to_string());
            }
            return Ok(boundary.to_string());
        }
    }
    return Err("the content type has no boundary, so the body cannot be split".to_string());
}

/// A file name a client supplied, reduced to something safe to write.
///
/// The name comes from whoever made the request, so it is treated as a suggestion
/// and nothing more: any directory part is dropped, so `../../etc/cron.d/anything`
/// becomes `anything`, and what is left is stripped to letters, digits, dots,
/// hyphens and underscores. The written name is prefixed with a fresh id anyway,
/// so two people uploading `photo.jpg` do not overwrite each other.
fn safe_file_name(supplied: &str) -> String {
    let base = supplied.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base.chars().filter(|character| character.is_ascii_alphanumeric() || *character == '.' || *character == '-' || *character == '_').collect();
    let cleaned = cleaned.trim_matches('.').to_string();
    if cleaned.is_empty() {
        return "upload".to_string();
    }
    return cleaned;
}

/// One part's headers, as far as this needs them.
struct PartHeaders {
    name: String,
    file_name: Option<String>,
    content_type: String,
}

/// Reads `Content-Disposition` and `Content-Type` out of a part's header block.
fn parse_part_headers(block: &str) -> Result<PartHeaders, String> {
    let mut name: Option<String> = None;
    let mut file_name: Option<String> = None;
    let mut content_type = String::new();

    for line in block.split('\n').map(|line| line.trim_end_matches('\r')) {
        let (header, value) = match line.split_once(':') {
            Some(both) => both,
            None => continue,
        };
        if header.trim().eq_ignore_ascii_case("content-type") {
            content_type = value.trim().to_string();
            continue;
        }
        if !header.trim().eq_ignore_ascii_case("content-disposition") {
            continue;
        }
        for parameter in value.split(';').skip(1) {
            let (parameter_name, parameter_value) = match parameter.split_once('=') {
                Some(both) => both,
                None => continue,
            };
            let parameter_value = parameter_value.trim().trim_matches('"').to_string();
            match parameter_name.trim().to_lowercase().as_str() {
                "name" => name = Some(parameter_value),
                "filename" => file_name = Some(parameter_value),
                _ => {}
            }
        }
    }

    return match name {
        Some(name) => Ok(PartHeaders { name, file_name, content_type }),
        None => Err("a part of the body has no name, so there is nowhere to put its value".to_string()),
    };
}

/// Whichever of two finds came first in the buffer. Both forms of line ending are
/// looked for at once, and the earlier one is the real delimiter - a CRLF body
/// also matches the LF pattern one byte later, so taking the earliest keeps the
/// `\r` out of the contents.
fn earliest(first: Option<(usize, usize)>, second: Option<(usize, usize)>) -> Option<(usize, usize)> {
    return match (first, second) {
        (Some(first), Some(second)) => Some(if first.0 <= second.0 { first } else { second }),
        (Some(only), None) | (None, Some(only)) => Some(only),
        (None, None) => None,
    };
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    return haystack.windows(needle.len()).position(|window| window == needle);
}

/// Takes the parts of a `multipart/form-data` body apart: the encoding an HTML
/// form uses when it has a file in it.
///
/// The body is the file the server spooled - `request.body_path` - and the content
/// type is `request.headers`' own, since the boundary that splits the body lives
/// in it. File parts are written into the given directory and text parts come
/// back as values, all in one hashmap:
///
///   `name`             a text field's value, or a file part's written path
///   `name.filename`    the name the client gave the file, cleaned up
///   `name.type`        the content type the client declared for the part
///
/// The body is read in blocks and each file part is written as it is found, so a
/// twenty-megabyte upload costs no more memory than a small one. Text parts are
/// held in memory, because a form field is not where anybody puts twenty
/// megabytes - one over a megabyte is refused rather than kept.
pub async fn multipart_extract(body_path: String, content_type: String, into_directory: String) -> Result<DashMap<String, String>, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A form field bigger than this is not a form field.
    const LARGEST_TEXT_FIELD: usize = 1024 * 1024;
    const READ_BLOCK: usize = 64 * 1024;

    let boundary = multipart_boundary(&content_type).map_err(|detail| format!("http_multipart_extract: {}", detail))?;
    let delimiter = format!("--{}", boundary).into_bytes();
    // The format says CRLF, and most clients send it, but some send bare LF -
    // and a body is not worth refusing over a line ending. Both are accepted
    // everywhere a line ending is looked for.
    let part_end_crlf = [b"\r\n".as_slice(), delimiter.as_slice()].concat();
    let part_end_lf = [b"\n".as_slice(), delimiter.as_slice()].concat();

    let mut file = tokio::fs::File::open(&body_path).await.map_err(|failure| format!("http_multipart_extract: could not read '{}': {}", body_path, failure))?;
    tokio::fs::create_dir_all(&into_directory).await.map_err(|failure| format!("http_multipart_extract: could not create '{}': {}", into_directory, failure))?;

    let found: DashMap<String, String> = DashMap::new();
    let mut buffer: Vec<u8> = Vec::new();
    let mut at_end_of_file = false;

    // Reads another block, and says whether anything was left to read.
    macro_rules! read_more {
        () => {{
            let mut block = vec![0u8; READ_BLOCK];
            let read = file.read(&mut block).await.map_err(|failure| format!("http_multipart_extract: could not read '{}': {}", body_path, failure))?;
            if read == 0 {
                at_end_of_file = true;
                false
            } else {
                buffer.extend_from_slice(&block[..read]);
                true
            }
        }};
    }

    // Everything before the first delimiter is preamble and belongs to nobody.
    loop {
        match find_bytes(&buffer, &delimiter) {
            Some(position) => {
                buffer.drain(..position + delimiter.len());
                break;
            }
            None => {
                if !read_more!() {
                    return Err(format!("http_multipart_extract: '{}' holds no part beginning with the boundary the content type names", body_path));
                }
            }
        }
    }

    loop {
        // After a delimiter comes either `--`, ending the body, or a line break
        // and then the part's headers.
        while buffer.len() < 2 && !at_end_of_file {
            read_more!();
        }
        if buffer.starts_with(b"--") || buffer.is_empty() {
            return Ok(found);
        }
        while buffer.starts_with(b"\r") || buffer.starts_with(b"\n") {
            buffer.remove(0);
            if buffer.is_empty() && !at_end_of_file {
                read_more!();
            }
        }

        let (header_end, blank_line_length) = loop {
            let with_crlf = find_bytes(&buffer, b"\r\n\r\n").map(|position| (position, 4));
            let with_lf = find_bytes(&buffer, b"\n\n").map(|position| (position, 2));
            // Whichever blank line comes first is the one that ends the headers.
            match earliest(with_crlf, with_lf) {
                Some(found) => break found,
                None => {
                    if !read_more!() {
                        return Err("http_multipart_extract: a part of the body has no headers".to_string());
                    }
                }
            }
        };
        let header_block = String::from_utf8_lossy(&buffer[..header_end]).to_string();
        buffer.drain(..header_end + blank_line_length);
        let headers = parse_part_headers(&header_block).map_err(|detail| format!("http_multipart_extract: {}", detail))?;

        // Where this part's contents go: a file of its own, or memory.
        let mut destination: Option<(String, tokio::fs::File)> = match &headers.file_name {
            Some(supplied) => {
                let written_name = format!("{}_{}", uuid::Uuid::new_v4(), safe_file_name(supplied));
                let path = std::path::Path::new(&into_directory).join(&written_name);
                let file = tokio::fs::File::create(&path).await.map_err(|failure| format!("http_multipart_extract: could not write '{}': {}", path.display(), failure))?;
                Some((path.to_string_lossy().to_string(), file))
            }
            None => None,
        };
        let mut text_value: Vec<u8> = Vec::new();

        // The part runs to the next delimiter. Everything up to it is contents;
        // what is kept back each round is the longest run that could still turn
        // out to be the start of one.
        loop {
            let next_delimiter = earliest(
                find_bytes(&buffer, &part_end_crlf).map(|position| (position, part_end_crlf.len())),
                find_bytes(&buffer, &part_end_lf).map(|position| (position, part_end_lf.len())),
            );
            if let Some((position, delimiter_length)) = next_delimiter {
                let contents: Vec<u8> = buffer.drain(..position).collect();
                buffer.drain(..delimiter_length);
                match destination.as_mut() {
                    Some((path, file)) => file.write_all(&contents).await.map_err(|failure| format!("http_multipart_extract: could not write '{}': {}", path, failure))?,
                    None => text_value.extend_from_slice(&contents),
                }
                break;
            }

            if at_end_of_file {
                return Err(format!("http_multipart_extract: the part named '{}' is never closed by the boundary", headers.name));
            }

            let keep_back = part_end_crlf.len().saturating_sub(1);
            if buffer.len() > keep_back {
                let take = buffer.len() - keep_back;
                let contents: Vec<u8> = buffer.drain(..take).collect();
                match destination.as_mut() {
                    Some((path, file)) => file.write_all(&contents).await.map_err(|failure| format!("http_multipart_extract: could not write '{}': {}", path, failure))?,
                    None => {
                        text_value.extend_from_slice(&contents);
                        if text_value.len() > LARGEST_TEXT_FIELD {
                            return Err(format!("http_multipart_extract: the field named '{}' is larger than {} bytes, which is more than a form field should be - a file part needs a filename", headers.name, LARGEST_TEXT_FIELD));
                        }
                    }
                }
            }
            read_more!();
        }

        match destination {
            Some((path, mut file)) => {
                file.flush().await.map_err(|failure| format!("http_multipart_extract: could not finish writing '{}': {}", path, failure))?;
                found.insert(headers.name.clone(), path);
                found.insert(format!("{}.filename", headers.name), safe_file_name(headers.file_name.as_deref().unwrap_or("upload")));
            }
            None => {
                let value = String::from_utf8(text_value).map_err(|_| format!("http_multipart_extract: the field named '{}' is not text; a part that is not text needs a filename so it can be written to a file", headers.name))?;
                found.insert(headers.name.clone(), value);
            }
        }
        if !headers.content_type.is_empty() {
            found.insert(format!("{}.type", headers.name), headers.content_type);
        }
    }
}

#[cfg(test)]
mod inbound_multipart_tests {
    use super::*;

    const BOUNDARY: &str = "----NailBoundary123";

    fn a_directory(name: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!("nail_multipart_{}", name));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("a writable temporary directory");
        return directory;
    }

    /// Builds a body the way a browser does, from parts given as
    /// (name, optional filename, content type, contents).
    fn a_body(directory: &std::path::Path, parts: Vec<(&str, Option<&str>, &str, Vec<u8>)>) -> String {
        let mut body: Vec<u8> = Vec::new();
        for (name, file_name, content_type, contents) in parts {
            body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
            match file_name {
                Some(file_name) => body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n", name, file_name).as_bytes()),
                None => body.extend_from_slice(format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes()),
            }
            if !content_type.is_empty() {
                body.extend_from_slice(format!("Content-Type: {}\r\n", content_type).as_bytes());
            }
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(&contents);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", BOUNDARY).as_bytes());

        let path = directory.join("body.bin");
        std::fs::write(&path, body).expect("a writable body");
        return path.to_string_lossy().to_string();
    }

    fn content_type() -> String {
        return format!("multipart/form-data; boundary={}", BOUNDARY);
    }

    #[tokio::test]
    async fn text_fields_come_back_as_values() {
        let directory = a_directory("text_fields");
        let body = a_body(&directory, vec![("name", None, "", b"alex".to_vec()), ("city", None, "", b"calgary".to_vec())]);

        let found = multipart_extract(body, content_type(), directory.join("files").to_string_lossy().to_string()).await.expect("a readable body");
        assert_eq!(found.get("name").expect("the name field").value(), "alex");
        assert_eq!(found.get("city").expect("the city field").value(), "calgary");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn a_file_part_is_written_and_its_path_returned() {
        let directory = a_directory("file_part");
        let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0xff, 0x00, 0x01];
        let body = a_body(&directory, vec![("caption", None, "", b"my photo".to_vec()), ("photo", Some("holiday.png"), "image/png", png.clone())]);

        let into = directory.join("files");
        let found = multipart_extract(body, content_type(), into.to_string_lossy().to_string()).await.expect("a readable body");

        assert_eq!(found.get("caption").expect("the caption").value(), "my photo");
        let written = found.get("photo").expect("the photo").value().clone();
        assert_eq!(std::fs::read(&written).expect("the written file"), png, "the bytes must arrive unharmed");
        assert_eq!(found.get("photo.filename").expect("the file name").value(), "holiday.png");
        assert_eq!(found.get("photo.type").expect("the type").value(), "image/png");
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A part larger than one read block exercises the refilling, which is where
    /// a naive parser loses bytes.
    #[tokio::test]
    async fn a_part_larger_than_one_block_arrives_whole() {
        let directory = a_directory("large_part");
        let contents: Vec<u8> = (0..300_000u32).map(|index| (index % 251) as u8).collect();
        let body = a_body(&directory, vec![("upload", Some("big.bin"), "application/octet-stream", contents.clone()), ("after", None, "", b"still here".to_vec())]);

        let found = multipart_extract(body, content_type(), directory.join("files").to_string_lossy().to_string()).await.expect("a readable body");
        let written = found.get("upload").expect("the upload").value().clone();
        assert_eq!(std::fs::read(&written).expect("the written file"), contents);
        // The part after a large one is still found, so nothing was lost.
        assert_eq!(found.get("after").expect("the field after").value(), "still here");
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The file name comes from whoever made the request, so it never decides
    /// where anything is written.
    #[tokio::test]
    async fn a_file_name_cannot_point_outside_the_directory() {
        let directory = a_directory("escaping_name");
        let body = a_body(&directory, vec![("photo", Some("../../etc/cron.d/anything"), "image/png", b"x".to_vec())]);

        let into = directory.join("files");
        let found = multipart_extract(body, content_type(), into.to_string_lossy().to_string()).await.expect("a readable body");
        let written = found.get("photo").expect("the photo").value().clone();
        assert!(written.starts_with(&into.to_string_lossy().to_string()), "written outside the directory: {}", written);
        assert!(!written.contains(".."), "the path still holds a parent step: {}", written);
        assert_eq!(found.get("photo.filename").expect("the file name").value(), "anything");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn two_uploads_of_the_same_name_do_not_overwrite_each_other() {
        let directory = a_directory("same_name");
        let into = directory.join("files");
        let first_body = a_body(&directory, vec![("photo", Some("photo.jpg"), "image/jpeg", b"first".to_vec())]);
        let first = multipart_extract(first_body, content_type(), into.to_string_lossy().to_string()).await.expect("a readable body");
        let second_body = a_body(&directory, vec![("photo", Some("photo.jpg"), "image/jpeg", b"second".to_vec())]);
        let second = multipart_extract(second_body, content_type(), into.to_string_lossy().to_string()).await.expect("a readable body");

        let first_path = first.get("photo").expect("the first photo").value().clone();
        let second_path = second.get("photo").expect("the second photo").value().clone();
        assert_ne!(first_path, second_path);
        assert_eq!(std::fs::read_to_string(&first_path).expect("the first file"), "first");
        assert_eq!(std::fs::read_to_string(&second_path).expect("the second file"), "second");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn a_content_type_without_a_boundary_says_so() {
        let directory = a_directory("no_boundary");
        let body = a_body(&directory, vec![("name", None, "", b"alex".to_vec())]);

        let failure = multipart_extract(body.clone(), "multipart/form-data".to_string(), directory.to_string_lossy().to_string()).await.unwrap_err();
        assert!(failure.contains("no boundary"), "got: {}", failure);

        let wrong_kind = multipart_extract(body, "application/json".to_string(), directory.to_string_lossy().to_string()).await.unwrap_err();
        assert!(wrong_kind.contains("not multipart/form-data"), "got: {}", wrong_kind);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn a_body_that_is_not_multipart_at_all_says_so() {
        let directory = a_directory("not_multipart");
        let path = directory.join("body.bin");
        std::fs::write(&path, "name=alex&city=calgary").expect("a writable file");

        let failure = multipart_extract(path.to_string_lossy().to_string(), content_type(), directory.to_string_lossy().to_string()).await.unwrap_err();
        assert!(failure.contains("holds no part beginning with the boundary"), "got: {}", failure);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[tokio::test]
    async fn a_part_that_is_never_closed_says_so() {
        let directory = a_directory("unclosed");
        let path = directory.join("body.bin");
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"name\"\r\n\r\n");
        body.extend_from_slice(b"alex with no closing boundary");
        std::fs::write(&path, body).expect("a writable file");

        let failure = multipart_extract(path.to_string_lossy().to_string(), content_type(), directory.to_string_lossy().to_string()).await.unwrap_err();
        assert!(failure.contains("never closed"), "got: {}", failure);
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The format says CRLF; some clients send bare LF, and a body is not worth
    /// refusing over a line ending.
    #[tokio::test]
    async fn a_body_with_plain_line_endings_is_read_too() {
        let directory = a_directory("lf_endings");
        let path = directory.join("body.bin");
        let body = format!("--{boundary}\nContent-Disposition: form-data; name=\"name\"\n\nalex\n--{boundary}\nContent-Disposition: form-data; name=\"photo\"; filename=\"a.txt\"\n\nfile contents\n--{boundary}--\n", boundary = BOUNDARY);
        std::fs::write(&path, body).expect("a writable file");

        let found = multipart_extract(path.to_string_lossy().to_string(), content_type(), directory.join("files").to_string_lossy().to_string()).await.expect("a readable body");
        assert_eq!(found.get("name").expect("the name field").value(), "alex");
        let written = found.get("photo").expect("the photo").value().clone();
        assert_eq!(std::fs::read_to_string(&written).expect("the written file"), "file contents");
        let _ = std::fs::remove_dir_all(&directory);
    }
}

// ---------------------------------------------------------------------------
// Live updates: server-sent events and websockets
// ---------------------------------------------------------------------------

/// What the transpiler wraps `handle_message` in, the same way `HandlerFuture`
/// wraps `handle_request`.
pub type MessageFuture = Pin<Box<dyn Future<Output = String> + Send>>;

lazy_static::lazy_static! {
    /// One broadcast channel per name, created the first time anything touches
    /// it. Held here rather than in the program because a channel has waiting
    /// tasks in it, which is not something a Nail value can hold.
    static ref LIVE_CHANNELS: DashMap<String, tokio::sync::broadcast::Sender<String>> = DashMap::new();
}

/// A channel holds this many unread messages per subscriber before the oldest
/// are dropped. A subscriber that lags further behind than this skips ahead
/// rather than stalling the sender.
const LIVE_CHANNEL_CAPACITY: usize = 256;

fn live_channel(name: &str) -> tokio::sync::broadcast::Sender<String> {
    return LIVE_CHANNELS.entry(name.to_string()).or_insert_with(|| tokio::sync::broadcast::channel(LIVE_CHANNEL_CAPACITY).0).clone();
}

/// Sends a message to everyone subscribed to a channel - every SSE stream and
/// websocket connected to it - and returns how many subscribers there were to
/// receive it. Nobody listening is 0, not an error: a chat room being empty is
/// an ordinary state of a chat room.
pub async fn http_live_send(channel: String, message: String) -> i64 {
    return live_channel(&channel).send(message).map(|received| received as i64).unwrap_or(0);
}

/// How many subscribers a channel has right now.
pub async fn http_live_count(channel: String) -> i64 {
    return live_channel(&channel).receiver_count() as i64;
}

/// The channel a live request asked for: `?channel=name`, or `main` when it
/// did not say.
fn requested_channel(params: &std::collections::HashMap<String, String>) -> String {
    return params.get("channel").filter(|name| !name.is_empty()).cloned().unwrap_or_else(|| "main".to_string());
}

/// `http_server` with a live endpoint beside the ordinary routes.
///
/// A GET to `live_path` is a server-sent-event stream: everything the program
/// passes to `http_live_send` on that channel arrives as an SSE `data:` event,
/// which htmx's sse extension and the browser's own EventSource both consume. A
/// websocket upgrade on the same path joins the same channel, and every text
/// frame the client sends is answered by the program's `handle_message`
/// function - its return value goes back to that one client, with the empty
/// string meaning no reply. `?channel=name` picks the channel either way.
///
/// SSE and websockets share one path and one channel space on purpose: the
/// broadcast is the same broadcast, and which transport a client uses is the
/// client's business.
pub async fn http_server_realtime<F, M>(port: i64, config: HTTP_Config, live_path: String, handler: F, message_handler: M) -> Result<(), String>
where
    F: Fn(HTTP_Request, DashMap<String, String>) -> HandlerFuture + Clone + Send + Sync + 'static,
    M: Fn(String, DashMap<String, String>) -> MessageFuture + Clone + Send + Sync + 'static,
{
    use axum::extract::ws::{Message as WsMessage, WebSocketUpgrade};
    use futures::{SinkExt, StreamExt};

    if live_path.is_empty() || !live_path.starts_with('/') {
        return Err(format!("http_server_realtime: `{}` is not a path for the live endpoint - it needs to start with /", live_path));
    }

    let max_body_bytes = if config.max_body_bytes > 0 { config.max_body_bytes as usize } else { DEFAULT_MAX_BODY_BYTES };
    let timeout = Duration::from_secs(if config.timeout_seconds > 0 { config.timeout_seconds as u64 } else { DEFAULT_TIMEOUT_SECONDS });
    let static_mounts = config.static_mounts.clone();
    let state = config.state.clone();
    let cors_origins = config.cors_origins.clone();
    let security_headers = config.security_headers;
    let rate_limit_per_minute = config.rate_limit_per_minute;
    let rate_limit_message = config.rate_limit_message.clone();

    // The same dispatch `http_server` uses, wrapped the same way.
    let dispatch_state = state.clone();
    let dispatch = move |request: axum::extract::Request| {
        let handler = handler.clone();
        let state = dispatch_state.clone();
        let cors_origins = cors_origins.clone();
        let rate_limit_message = rate_limit_message.clone();
        async move {
            let (parts, body) = request.into_parts();
            if over_rate_limit(rate_limit_per_minute, &parts.headers) {
                return rate_limit_page(&rate_limit_message);
            }
            let request_origin = parts.headers.get("origin").and_then(|value| value.to_str().ok()).map(|origin| origin.to_string());
            let cors_origin = cors_origin_for(request_origin.as_deref(), &cors_origins);
            if parts.method == axum::http::Method::OPTIONS && cors_origin.is_some() {
                return preflight_response(cors_origin, security_headers);
            }
            let (body_text, body_path) = match receive_body(body, max_body_bytes, &std::env::temp_dir()).await {
                Ok(ReceivedBody::Text(text)) => (text, String::new()),
                Ok(ReceivedBody::File(path)) => (String::new(), path),
                Ok(ReceivedBody::Empty) => (String::new(), String::new()),
                Err(BodyRefused::TooLarge) => {
                    return (StatusCode::PAYLOAD_TOO_LARGE, Html(format!("<pre>413 - request body exceeds {} bytes</pre>", max_body_bytes))).into_response();
                }
                Err(BodyRefused::CouldNotWrite(detail)) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, Html(format!("<pre>500 - {}</pre>", escape_html(&detail)))).into_response();
                }
                Err(BodyRefused::Unreadable) => {
                    return (StatusCode::BAD_REQUEST, Html("<pre>400 - the request body could not be read to the end</pre>".to_string())).into_response();
                }
            };
            let spooled_body = body_path.clone();
            let nail_request = HTTP_Request {
                method: parts.method.as_str().to_string(),
                path: parts.uri.path().to_string(),
                query: query_to_nail(&parts.uri),
                headers: header_map_to_nail(&parts.headers),
                body: body_text,
                body_path,
            };
            let requested_path = nail_request.path.clone();
            let answer = tokio::time::timeout(timeout, handler(nail_request, state)).await;
            if !spooled_body.is_empty() {
                let _ = tokio::fs::remove_file(&spooled_body).await;
            }
            let mut finished = match answer {
                Ok(response) => build_response(response),
                Err(_) => (StatusCode::GATEWAY_TIMEOUT, Html(format!("<pre>504 - handler timed out after {}s: {}</pre>", timeout.as_secs(), escape_html(&requested_path)))).into_response(),
            };
            apply_response_headers(&mut finished, cors_origin, security_headers);
            finished
        }
    };

    // The live endpoint: an SSE stream for a plain GET, a websocket for an
    // upgrade, both on the channel the query names.
    let message_handler = std::sync::Arc::new(message_handler);
    let live_state = state.clone();
    let live = move |upgrade: Option<WebSocketUpgrade>, axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>| {
        let message_handler = message_handler.clone();
        let state = live_state.clone();
        async move {
            let channel = requested_channel(&params);
            let receiver = live_channel(&channel).subscribe();

            match upgrade {
                Some(upgrade) => {
                    return upgrade.on_upgrade(move |socket| async move {
                        let (mut to_client, mut from_client) = socket.split();
                        let mut broadcast = receiver;
                        // Replies to this client's own messages travel beside
                        // the broadcast, through a lane of their own.
                        let (reply_lane, mut replies) = tokio::sync::mpsc::unbounded_channel::<String>();

                        let sending = tokio::spawn(async move {
                            loop {
                                tokio::select! {
                                    broadcast_message = broadcast.recv() => match broadcast_message {
                                        Ok(message) => {
                                            if to_client.send(WsMessage::Text(message)).await.is_err() {
                                                break;
                                            }
                                        }
                                        // A subscriber that fell too far behind
                                        // skips ahead rather than dying.
                                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                                    },
                                    reply = replies.recv() => match reply {
                                        Some(message) => {
                                            if to_client.send(WsMessage::Text(message)).await.is_err() {
                                                break;
                                            }
                                        }
                                        None => break,
                                    },
                                }
                            }
                        });

                        while let Some(Ok(frame)) = from_client.next().await {
                            match frame {
                                WsMessage::Text(text) => {
                                    let reply = message_handler(text.to_string(), state.clone()).await;
                                    if !reply.is_empty() {
                                        let _ = reply_lane.send(reply);
                                    }
                                }
                                WsMessage::Close(_) => break,
                                // Binary frames have no place a Nail program
                                // could put them; pings are answered by axum.
                                _ => {}
                            }
                        }
                        sending.abort();
                    });
                }
                None => {
                    let stream = futures::stream::unfold(receiver, |mut receiver| async move {
                        loop {
                            match receiver.recv().await {
                                Ok(message) => {
                                    return Some((Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(message)), receiver));
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                            }
                        }
                    });
                    return axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
                }
            }
        }
    };

    let mut app = Router::new().route(&live_path, axum::routing::get(live));
    for mount in static_mounts {
        if mount.prefix.is_empty() || mount.directory.is_empty() {
            continue;
        }
        app = app.nest_service(&mount.prefix, ServeDir::new(mount.directory));
    }
    let app = app.fallback(dispatch);

    let addr: SocketAddr = match std::env::var("BIND_ADDR") {
        Ok(host) => format!("{}:{}", host, port).parse().map_err(|_| format!("http_server_realtime: invalid BIND_ADDR '{}'", host))?,
        Err(_) => SocketAddr::from(([0, 0, 0, 0], port as u16)),
    };
    println!("🔨 Nail HTTP server listening on http://{} (live updates at {})", addr, live_path);
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("http_server_realtime: could not bind to port {}: {}", port, e))?;
    axum::serve(listener, app).await.map_err(|e| format!("http_server_realtime: server error: {}", e))?;
    return Ok(());
}

#[cfg(test)]
mod live_tests {
    use super::*;

    #[tokio::test]
    async fn sending_to_nobody_is_zero_not_an_error() {
        assert_eq!(http_live_send("empty_room".to_string(), "hello?".to_string()).await, 0);
        assert_eq!(http_live_count("empty_room".to_string()).await, 0);
    }

    #[tokio::test]
    async fn every_subscriber_on_a_channel_hears_a_send() {
        let mut first = live_channel("live_test_room").subscribe();
        let mut second = live_channel("live_test_room").subscribe();
        assert_eq!(http_live_count("live_test_room".to_string()).await, 2);
        assert_eq!(http_live_send("live_test_room".to_string(), "to everyone".to_string()).await, 2);
        assert_eq!(first.recv().await.expect("the broadcast"), "to everyone");
        assert_eq!(second.recv().await.expect("the broadcast"), "to everyone");
    }

    #[tokio::test]
    async fn channels_do_not_hear_each_other() {
        let mut listener = live_channel("live_test_alpha").subscribe();
        let _other = live_channel("live_test_beta").subscribe();
        assert_eq!(http_live_send("live_test_beta".to_string(), "only beta".to_string()).await, 1);
        assert_eq!(http_live_send("live_test_alpha".to_string(), "only alpha".to_string()).await, 1);
        assert_eq!(listener.recv().await.expect("alpha's own message"), "only alpha");
    }

    #[test]
    fn the_channel_comes_from_the_query_or_defaults_to_main() {
        let mut params = std::collections::HashMap::new();
        assert_eq!(requested_channel(&params), "main");
        params.insert("channel".to_string(), "chat".to_string());
        assert_eq!(requested_channel(&params), "chat");
        params.insert("channel".to_string(), String::new());
        assert_eq!(requested_channel(&params), "main");
    }
}

/// An outbound websocket connection, held by handle like an open file: the
/// other half of http_server_realtime. This is how a program consumes a
/// streaming API - an exchange feed, a chat bridge, another Nail program.
#[cfg(feature = "websocket")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HTTP_Websocket {
    pub handle: String,
    pub url: String,
}

#[cfg(feature = "websocket")]
lazy_static::lazy_static! {
    static ref OPEN_WEBSOCKETS: DashMap<String, tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>> = DashMap::new();
}

/// Open a websocket to a ws:// or wss:// URL.
#[cfg(feature = "websocket")]
pub async fn ws_connect(url: String) -> Result<HTTP_Websocket, String> {
    let trimmed = url.trim().to_string();
    if !(trimmed.starts_with("ws://") || trimmed.starts_with("wss://")) {
        return Err(format!("http_ws_connect: `{}` is not a ws:// or wss:// URL", trimmed));
    }
    let (stream, _) = tokio_tungstenite::connect_async(&trimmed).await.map_err(|e| format!("http_ws_connect: could not connect to `{}`: {}", trimmed, e))?;
    let handle = format!("websocket_{}", uuid::Uuid::new_v4());
    OPEN_WEBSOCKETS.insert(handle.clone(), stream);
    return Ok(HTTP_Websocket { handle, url: trimmed });
}

/// Send one text frame.
#[cfg(feature = "websocket")]
pub async fn ws_send(socket: &HTTP_Websocket, text: String) -> Result<(), String> {
    use futures::SinkExt;
    let mut stream = OPEN_WEBSOCKETS.get_mut(&socket.handle).ok_or_else(|| format!("http_ws_send: the connection to `{}` is closed", socket.url))?;
    return stream
        .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
        .await
        .map_err(|e| format!("http_ws_send: the connection to `{}` failed: {}", socket.url, e));
}

/// The next text frame the other side sends. Waits up to the timeout, or
/// forever when the timeout is 0. Pings are answered quietly; binary frames
/// are skipped; a closed connection is an error and forgets the handle.
#[cfg(feature = "websocket")]
pub async fn ws_receive(socket: &HTTP_Websocket, timeout_milliseconds: i64) -> Result<String, String> {
    use futures::StreamExt;
    let mut stream = OPEN_WEBSOCKETS.get_mut(&socket.handle).ok_or_else(|| format!("http_ws_receive: the connection to `{}` is closed", socket.url))?;
    loop {
        let frame = if timeout_milliseconds > 0 {
            match tokio::time::timeout(Duration::from_millis(timeout_milliseconds as u64), stream.next()).await {
                Ok(frame) => frame,
                Err(_) => return Err(format!("http_ws_receive: nothing arrived from `{}` within {}ms", socket.url, timeout_milliseconds)),
            }
        } else {
            stream.next().await
        };
        match frame {
            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => return Ok(text.to_string()),
            Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) | None => {
                drop(stream);
                OPEN_WEBSOCKETS.remove(&socket.handle);
                return Err(format!("http_ws_receive: `{}` closed the connection", socket.url));
            }
            Some(Ok(_)) => continue,
            Some(Err(e)) => {
                drop(stream);
                OPEN_WEBSOCKETS.remove(&socket.handle);
                return Err(format!("http_ws_receive: the connection to `{}` failed: {}", socket.url, e));
            }
        }
    }
}

/// Say goodbye properly and forget the handle. Closing twice is not an error.
#[cfg(feature = "websocket")]
pub async fn ws_close(socket: &HTTP_Websocket) -> Result<(), String> {
    use futures::SinkExt;
    if let Some((_, mut stream)) = OPEN_WEBSOCKETS.remove(&socket.handle) {
        let _ = stream.send(tokio_tungstenite::tungstenite::Message::Close(None)).await;
    }
    return Ok(());
}

#[cfg(all(test, feature = "websocket"))]
mod ws_client_tests {
    use super::*;

    /// The client talks to Nail's own realtime server: the two halves prove
    /// each other.
    #[tokio::test]
    async fn the_client_and_the_realtime_server_shake_hands() {
        let port = 41895;
        let config = http_default_config();
        tokio::spawn(async move {
            let _ = http_server_realtime(
                port,
                config,
                "/live".to_string(),
                |_request, _state| {
                    Box::pin(async {
                        HTTP_Response { status: 200, body: "ok".to_string(), content_type: "text/plain".to_string(), headers: DashMap::new() }
                    }) as HandlerFuture
                },
                |message, _state| Box::pin(async move { format!("echo {}", message) }) as MessageFuture,
            )
            .await;
        });
        tokio::time::sleep(Duration::from_millis(300)).await;

        let socket = ws_connect(format!("ws://127.0.0.1:{}/live", port)).await.expect("the server is up");
        ws_send(&socket, "hello".to_string()).await.expect("a frame goes out");
        let answer = ws_receive(&socket, 2000).await.expect("a frame comes back");
        assert_eq!(answer, "echo hello");
        ws_close(&socket).await.expect("goodbye is easy");
        assert!(ws_send(&socket, "again".to_string()).await.unwrap_err().contains("closed"));
        assert!(ws_connect("http://not-a-ws-url".to_string()).await.unwrap_err().contains("not a ws://"));
    }
}
