use crate::routes::AppState;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, put},
};
use hacker_news_rs::config::{
    get_auto_update_interval_from_env, get_llm_config_from_env, get_search_keywords_from_env,
    get_socks5_proxy_from_env, get_summary_concurrency_from_env,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
struct ConfigResponse {
    api_base_url: String,
    model: String,
    auto_update_interval: u32,
    masked_api_key: String,
    socks5_proxy: Option<String>,
    search_keywords: Option<String>,
    summary_concurrency: usize,
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

#[derive(Deserialize)]
struct SetLangRequest {
    lang: String,
}

#[derive(Serialize)]
struct LangResponse {
    lang: String,
}

pub fn config_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/config", get(get_config))
        .route("/api/lang", get(get_lang))
        .route("/api/lang", put(set_lang))
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
        summary_concurrency: get_summary_concurrency_from_env(),
    };

    Ok(Json(ApiResponse::success(response)))
}

async fn get_lang(state: State<Arc<AppState>>) -> Json<ApiResponse<LangResponse>> {
    let lang = state.lang.read().unwrap().clone();
    Json(ApiResponse::success(LangResponse { lang }))
}

async fn set_lang(
    state: State<Arc<AppState>>,
    Json(payload): Json<SetLangRequest>,
) -> Json<ApiResponse<LangResponse>> {
    let mut lang = state.lang.write().unwrap();
    *lang = payload.lang.clone();
    tracing::info!("Language setting updated to: {}", payload.lang);
    Json(ApiResponse::success(LangResponse { lang: payload.lang }))
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
