use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    Router,
};
use tower_http::services::ServeDir;

pub fn static_routes() -> Router {
    // Serve static files from the static directory
    Router::new().nest_service("/", ServeDir::new("src/static"))
}

pub async fn serve_index() -> Result<Response<Body>, (StatusCode, String)> {
    // This is a fallback that serves index.html directly
    let index_html = include_str!("../static/index.html");
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(index_html))
        .unwrap())
}