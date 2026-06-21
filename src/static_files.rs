use axum::{
    body::Body,
    http::{Request, StatusCode, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "public/"]
struct PublicAssets;

#[derive(RustEmbed)]
#[folder = "target/site/pkg/"]
struct PkgAssets;

pub async fn static_handler(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Try pkg assets first (hns.js, hns.wasm, hns.css)
    if path.starts_with("pkg/") {
        let pkg_path = path.trim_start_matches("pkg/");
        // wasm-bindgen JS references *_bg.wasm but the file is named *.wasm
        let asset = PkgAssets::get(pkg_path).or_else(|| {
            pkg_path.strip_suffix("_bg.wasm")
                .and_then(|name| PkgAssets::get(&format!("{}.wasm", name)))
        });
        match asset {
            Some(asset) => {
                let mime = mime_guess::from_path(pkg_path)
                    .first_or_octet_stream()
                    .as_ref()
                    .to_string();
                let body = Body::from(asset.data.into_owned());
                return (StatusCode::OK, [(header::CONTENT_TYPE, mime)], body).into_response();
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    [(header::CONTENT_TYPE, "text/plain".to_string())],
                    Body::from("Not Found"),
                )
                    .into_response();
            }
        }
    }

    // Try public assets (style.css, etc.)
    match PublicAssets::get(path) {
        Some(asset) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .as_ref()
                .to_string();
            let body = Body::from(asset.data.into_owned());
            (StatusCode::OK, [(header::CONTENT_TYPE, mime)], body).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            Body::from("Not Found"),
        )
            .into_response(),
    }
}
