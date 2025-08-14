use axum::{
    body::Body,
    extract::Query,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use dashmap::DashMap;
use reqwest;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct HTTP_Response {
    pub status: i64,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HTTP_Route {
    pub path: String,
    pub content: String,
    pub content_type: String, // e.g., "text/html", "application/json"
    pub status_code: u16,     // HTTP status code (200, 404, etc.)
}

// THE ONE AND ONLY HTTP SERVER FUNCTION
// Handles static content, routes, and query parameters
pub async fn http_server(port: i64, routes: DashMap<String, HTTP_Route>) -> Result<(), String> {
    let mut app = Router::new();

    // If there's only one route with key "/" or "", serve it as static
    if routes.len() == 1 {
        if let Some(entry) = routes.iter().next() {
            let key = entry.key();
            if key == "/" || key == "" {
                let route = entry.value().clone();
                let content = route.content.clone();
                let content_type = route.content_type.clone();
                let status = StatusCode::from_u16(route.status_code).unwrap_or(StatusCode::OK);
                app = app.route("/", get(move || async move { Response::builder().status(status).header(header::CONTENT_TYPE, content_type).body(content).unwrap() }));
            }
        }
    } else {
        // Group routes by path for query parameter handling
        let mut path_routes: HashMap<String, DashMap<String, HTTP_Route>> = HashMap::new();

        for entry in routes.iter() {
            let key = entry.key().clone();
            let route = entry.value().clone();

            // Check if this is a query-based route
            if let Some(pos) = key.find('?') {
                let path = key[..pos].to_string();
                let query = key[pos + 1..].to_string();

                path_routes.entry(path.clone()).or_insert_with(DashMap::new).insert(query, route);
            } else {
                // Simple path without query params
                path_routes.entry(key.clone()).or_insert_with(DashMap::new).insert("".to_string(), route);
            }
        }

        // Add routes to the app
        for (path, query_map) in path_routes {
            let query_map_clone = query_map.clone();
            let path_clone = path.clone();
            app = app.route(
                &path,
                get(move |Query(params): Query<HashMap<String, String>>| {
                    let qmap = query_map_clone.clone();
                    let path_for_error = path_clone.clone();
                    async move {
                        // Build query string from params
                        let mut query_parts: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                        query_parts.sort();
                        let query_string = query_parts.join("&");

                        // Try exact match first
                        if let Some(route) = qmap.get(&query_string) {
                            let content = route.content.clone();
                            let content_type = route.content_type.clone();
                            let status = StatusCode::from_u16(route.status_code).unwrap_or(StatusCode::OK);
                            return Response::builder().status(status).header(header::CONTENT_TYPE, content_type).body(Body::from(content)).unwrap();
                        }

                        // Fall back to no-query-param version
                        if let Some(route) = qmap.get("") {
                            let content = route.content.clone();
                            let content_type = route.content_type.clone();
                            let status = StatusCode::from_u16(route.status_code).unwrap_or(StatusCode::OK);
                            return Response::builder().status(status).header(header::CONTENT_TYPE, content_type).body(Body::from(content)).unwrap();
                        }

                        Html(format!("<pre>404 - Route not found: {}?{}</pre>", path_for_error, query_string)).into_response()
                    }
                }),
            );
        }
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], port as u16));
    println!("🔨 Nail HTTP server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    axum::serve(listener, app).await.map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

pub async fn http_request(method: String, url: String, headers: HashMap<String, String>, body: String) -> Result<HTTP_Response, String> {
    let client = reqwest::Client::new();

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        _ => return Err(format!("Unsupported HTTP method: {}", method)),
    };

    // Add headers
    for (key, value) in headers {
        request = request.header(&key, &value);
    }

    // Add body if not empty
    if !body.is_empty() {
        request = request.body(body);
    }

    let response = request.send().await.map_err(|e| e.to_string())?;

    let status = response.status().as_u16() as i64;
    let response_body = response.text().await.map_err(|e| e.to_string())?;

    Ok(HTTP_Response { status, body: response_body })
}
