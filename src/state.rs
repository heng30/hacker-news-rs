use std::sync::Arc;

use dashmap::DashMap;
use sled::Db;

use crate::config::AppConfig;
use crate::models::FetchProgress;

/// Shared application state available to all handlers and server_fns
#[derive(Clone)]
pub struct AppState {
    /// Sled database handle
    pub db: Arc<Db>,
    /// Application configuration (from clap)
    pub config: Arc<AppConfig>,
    /// HTTP client (with optional SOCKS5 proxy)
    pub http_client: reqwest::Client,
    /// Current language (shared across requests)
    pub lang: Arc<tokio::sync::RwLock<String>>,
    /// Active fetch progress tracking
    pub fetch_progress: Arc<DashMap<String, FetchProgress>>,
}
