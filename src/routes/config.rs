use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db;
use crate::config::{
    get_base_url_from_env, get_model_from_env, get_story_count_from_env, get_api_key_from_env,
    get_auto_update_interval_from_env, get_socks5_proxy_from_env, get_search_keywords_from_env,
};
use crate::routes::AppState;

/// Mask a sensitive string for display.
/// Shows first 4 and last 4 characters, with asterisks in between.
/// Minimum 8 characters required to show any part; otherwise shows all as asterisks.
fn mask_sensitive_value(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let len = value.len();
    if len <= 8 {
        // Too short to safely show any part
        "*".repeat(len)
    } else {
        let first = &value[..4];
        let last = &value[len - 4..];
        format!("{}****{}", first, last)
    }
}

pub fn config_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/config", get(get_config))
        .route("/api/config", put(update_config))
}

#[derive(Serialize)]
struct ConfigResponse {
    story_count: i32,
    api_base_url: String,
    model: String,
    auto_update_interval: u32,
    // Indicate which values are overridden by environment variables
    story_count_from_env: bool,
    api_base_url_from_env: bool,
    model_from_env: bool,
    api_key_from_env: bool,
    auto_update_interval_from_env: bool,
    // Masked API key for display (shows first 4 and last 4 characters)
    masked_api_key: String,
    // SOCKS5 proxy (always from env)
    socks5_proxy: Option<String>,
    socks5_proxy_from_env: bool,
    // Search keywords (always from env)
    search_keywords: Option<String>,
    search_keywords_from_env: bool,
}

#[derive(Deserialize)]
struct UpdateConfigRequest {
    story_count: Option<i32>,
    api_base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
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

    fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiResponse<ConfigResponse>>)> {
    let db_config = db::get_config(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    // Apply environment variable overrides and track which are from env
    let story_count_from_env = get_story_count_from_env().is_some();
    let api_base_url_from_env = get_base_url_from_env().is_some();
    let model_from_env = get_model_from_env().is_some();
    let api_key_from_env = get_api_key_from_env().is_some();
    let auto_update_interval_from_env = true; // auto_update_interval always comes from env

    // Get the actual API key (from env or db) for masking
    let actual_api_key = get_api_key_from_env().unwrap_or(db_config.api_key);
    let masked_api_key = mask_sensitive_value(&actual_api_key);

    let response = ConfigResponse {
        story_count: get_story_count_from_env().unwrap_or(db_config.story_count),
        api_base_url: get_base_url_from_env().unwrap_or(db_config.api_base_url),
        model: get_model_from_env().unwrap_or(db_config.model),
        auto_update_interval: get_auto_update_interval_from_env(),
        story_count_from_env,
        api_base_url_from_env,
        model_from_env,
        api_key_from_env,
        auto_update_interval_from_env,
        masked_api_key,
        socks5_proxy: get_socks5_proxy_from_env(),
        socks5_proxy_from_env: true,
        search_keywords: get_search_keywords_from_env().map(|kws| kws.join(",")),
        search_keywords_from_env: true,
    };

    Ok(Json(ApiResponse::success(response)))
}

async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<Json<ApiResponse<ConfigResponse>>, (StatusCode, Json<ApiResponse<ConfigResponse>>)> {
    // Get current config from database
    let mut config = db::get_config(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    // Check if environment variables are set for sensitive fields
    // If so, don't update those fields in database (they're controlled by env)
    let api_key_from_env = get_api_key_from_env().is_some();
    let api_base_url_from_env = get_base_url_from_env().is_some();
    let model_from_env = get_model_from_env().is_some();
    let story_count_from_env = get_story_count_from_env().is_some();

    // Update only provided fields that are NOT controlled by environment variables
    if !story_count_from_env && payload.story_count.is_some() {
        config.story_count = payload.story_count.unwrap();
    }
    if !api_base_url_from_env && payload.api_base_url.is_some() {
        config.api_base_url = payload.api_base_url.unwrap();
    }
    if !model_from_env && payload.model.is_some() {
        config.model = payload.model.unwrap();
    }
    // API key should never be stored in database if it comes from environment
    if !api_key_from_env {
        if let Some(api_key) = payload.api_key {
            if !api_key.is_empty() {
                config.api_key = api_key;
            }
        }
    }

    db::update_config(&state.pool, &config).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    // Get the actual API key (from env or db) for masking
    let actual_api_key = get_api_key_from_env().unwrap_or(config.api_key);
    let masked_api_key = mask_sensitive_value(&actual_api_key);

    // Return config with env overrides applied
    let response = ConfigResponse {
        story_count: get_story_count_from_env().unwrap_or(config.story_count),
        api_base_url: get_base_url_from_env().unwrap_or(config.api_base_url),
        model: get_model_from_env().unwrap_or(config.model),
        auto_update_interval: get_auto_update_interval_from_env(),
        story_count_from_env,
        api_base_url_from_env,
        model_from_env,
        api_key_from_env,
        auto_update_interval_from_env: true,
        masked_api_key,
        socks5_proxy: get_socks5_proxy_from_env(),
        socks5_proxy_from_env: true,
        search_keywords: get_search_keywords_from_env().map(|kws| kws.join(",")),
        search_keywords_from_env: true,
    };

    Ok(Json(ApiResponse::success(response)))
}