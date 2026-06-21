use crate::{config::AppConfig, models::FetchEvent};
use sled::Db;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shared application state available to all handlers and server_fns
#[derive(Clone)]
pub struct AppState {
    /// Sled database handle
    pub db: Arc<Db>,
    /// Application configuration (from clap)
    pub config: Arc<AppConfig>,
    /// HTTP client (with optional SOCKS5 proxy)
    pub http_client: reqwest::Client,
    /// Broadcast channel for SSE fetch events
    pub fetch_events: broadcast::Sender<FetchEvent>,
}
