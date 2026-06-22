use crate::models::ConfigResponse;
use leptos::prelude::*;
use server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
use super::app_state;

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
