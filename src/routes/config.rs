use crate::{
    config::{
        get_auto_update_interval_from_env, get_llm_config_from_env, get_search_keywords_from_env,
        get_socks5_proxy_from_env,
    },
    routes::AppState,
};
use axum::{Router, extract::State, http::StatusCode, response::Json, routing::get};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct ConfigResponse {
    api_base_url: String,
    model: String,
    auto_update_interval: u32,
    masked_api_key: String,
    socks5_proxy: Option<String>,
    search_keywords: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

pub fn config_routes() -> Router<Arc<AppState>> {
    Router::new().route("/api/config", get(get_config))
}

async fn get_config(
    _state: State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiResponse<ConfigResponse>>)> {
    let config = get_llm_config_from_env();
    let masked_api_key = mask_sensitive_value(&config.api_key);

    let response = ConfigResponse {
        api_base_url: config.api_base_url,
        model: config.model,
        auto_update_interval: get_auto_update_interval_from_env(),
        masked_api_key,
        socks5_proxy: get_socks5_proxy_from_env(),
        search_keywords: get_search_keywords_from_env().map(|kws| kws.join(",")),
    };

    Ok(Json(ApiResponse::success(response)))
}

// Mask a sensitive string for display.
// Shows first 4 and last 4 characters, with asterisks in between.
// Minimum 8 characters required to show any part; otherwise shows all as asterisks.
fn mask_sensitive_value(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let len = value.len();
    if len <= 8 {
        "*".repeat(len)
    } else {
        let first = &value[..4];
        let last = &value[len - 4..];
        format!("{}****{}", first, last)
    }
}

