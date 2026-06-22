pub mod config;
pub mod episodes;
pub mod fetch;
pub mod preferences;
pub mod stories;

#[cfg(feature = "ssr")]
pub(crate) fn app_state()
-> Result<std::sync::Arc<crate::state::AppState>, server_fn::error::ServerFnError> {
    leptos::prelude::use_context::<std::sync::Arc<crate::state::AppState>>()
        .ok_or_else(|| server_fn::error::ServerFnError::new("AppState not found"))
}
