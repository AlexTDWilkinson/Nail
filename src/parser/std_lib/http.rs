use axum::{Router, response::Html, routing::get};
use dashmap::DashMap;
use std::net::SocketAddr;
use reqwest;

// Stdlib struct that matches what Nail expects
#[derive(Debug, Clone)]
pub struct HTTP_Response {
    pub status: i64,
    pub body: String,
    pub success: bool,
}


// Nail callable function: http_server_start
// This function is called from transpiled Nail code which is already async
pub async fn http_server_start(port: i64, html: String) -> Result<(), String> {
    // Create a simple handler that returns the HTML
    let html_clone = html.clone();
    let app = Router::new()
        .route("/", get(move || async move {
            Html(html_clone.clone())
        }));
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port as u16));
    println!("🔨 Nail HTTP server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}

// For more complex routing with DashMap
pub async fn http_server_route(port: i64, routes: &DashMap<String, String>) -> Result<(), String> {
    let mut app = Router::new();
    
    let route_count = routes.len();
    
    // Add each route
    for entry in routes.iter() {
        let path = entry.key().clone();
        let html = entry.value().clone();
        app = app.route(&path, get(move || {
            let html_clone = html.clone();
            async move {
                Html(html_clone)
            }
        }));
    }
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port as u16));
    println!("🔨 Nail HTTP server with {} routes listening on http://{}", route_count, addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}

// HTTP client functions for making requests
pub async fn request_get(url: String) -> Result<HTTP_Response, String> {
    let response = reqwest::get(&url)
        .await
        .map_err(|e| format!("HTTP GET request failed: {}", e))?;
    
    let status = response.status();
    let status_code = status.as_u16() as i64;
    let is_success = status.is_success();
    
    let body = response.text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    
    Ok(HTTP_Response {
        status: status_code,
        body,
        success: is_success,
    })
}

pub async fn request_post(url: String, body: String) -> Result<HTTP_Response, String> {
    let client = reqwest::Client::new();
    let response = client.post(&url)
        .body(body)
        .send()
        .await
        .map_err(|e| format!("HTTP POST request failed: {}", e))?;
    
    let status = response.status();
    let status_code = status.as_u16() as i64;
    let is_success = status.is_success();
    
    let response_body = response.text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    
    Ok(HTTP_Response {
        status: status_code,
        body: response_body,
        success: is_success,
    })
}