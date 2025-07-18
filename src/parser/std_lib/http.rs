use axum::{Router, response::Html, routing::get};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

// Simple HTTP route handler type - returns HTML string
type RouteHandler = Arc<dyn Fn() -> String + Send + Sync>;

// Nail callable function: http_server_start
// This function is called from transpiled Nail code which is already async
pub async fn http_server_start(port: i64, html: String) -> Result<(), String> {
    // Create a simple handler that returns the HTML
    let html_clone = html.clone();
    let app = Router::new()
        .route("/", get(move || async move {
            Html(html_clone.clone())
        }));
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port as u16));
    println!("🔨 Nail HTTP server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}

// For more complex routing, we could have:
pub async fn http_server_route(port: i64, routes: HashMap<String, String>) -> Result<(), String> {
    let mut app = Router::new();
    
    let route_count = routes.len();
    
    // Add each route
    for (path, html) in routes {
        let html_clone = html.clone();
        app = app.route(&path, get(move || async move {
            Html(html_clone.clone())
        }));
    }
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port as u16));
    println!("🔨 Nail HTTP server with {} routes listening on http://{}", route_count, addr);
    
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;
    
    Ok(())
}