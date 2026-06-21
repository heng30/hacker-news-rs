pub mod app;
pub mod components;
pub mod models;
pub mod pages;
pub mod server_fns;
pub mod shell;

#[cfg(feature = "ssr")]
pub mod api;
#[cfg(feature = "ssr")]
pub mod bot;
#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod db;
#[cfg(feature = "ssr")]
pub mod error;
#[cfg(feature = "ssr")]
pub mod fetcher;
#[cfg(feature = "ssr")]
pub mod llm;
#[cfg(feature = "ssr")]
pub mod state;
#[cfg(feature = "ssr")]
pub mod static_files;

#[cfg(feature = "ssr")]
pub use api::HnClient;
#[cfg(feature = "ssr")]
pub use models::HnStory;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
