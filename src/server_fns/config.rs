use leptos::prelude::*;
use server_fn::error::ServerFnError;

use crate::models::ConfigResponse;

#[cfg(feature = "ssr")]
use crate::state::AppState;

#[cfg(feature = "ssr")]
fn app_state() -> Result<std::sync::Arc<AppState>, ServerFnError> {
    use_context::<std::sync::Arc<AppState>>()
        .ok_or_else(|| ServerFnError::new("AppState not found"))
}

#[server]
pub async fn get_config() -> Result<ConfigResponse, ServerFnError> {
    let state = app_state()?;

    Ok(ConfigResponse {
        api_base_url: state.config.api_base_url.clone(),
        model: state.config.model.clone(),
        auto_update_interval: state.config.auto_update_interval,
        masked_api_key: state.config.masked_api_key(),
        socks5_proxy: state.config.socks5_proxy.clone(),
        search_keywords: state
            .config
            .search_keywords
            .as_ref()
            .map(|kws| kws.join(",")),
        summary_concurrency: state.config.summary_concurrency,
    })
}

#[server]
pub async fn set_lang(lang: String) -> Result<String, ServerFnError> {
    let state = app_state()?;
    let mut current = state.lang.write().await;
    *current = lang.clone();
    tracing::info!("Language setting updated to: {}", lang);
    Ok(lang)
}
