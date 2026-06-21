use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "public/"]
struct StaticAssets;

pub async fn static_handler(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Skip Leptos pkg paths — those are handled by leptos_axum
    if path.starts_with("pkg/") {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            Body::from("Not Found"),
        );
    }

    match StaticAssets::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .as_ref()
                .to_string();
            let body = Body::from(asset.data.into_owned());
            (StatusCode::OK, [(header::CONTENT_TYPE, mime)], body)
        }
        None => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            Body::from("Not Found"),
        ),
    }
}
