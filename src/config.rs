use std::env;

/// Environment variable names for configuration
/// All environment variables are prefixed with HACKER_NEW_ for consistency
pub const ENV_OPENAI_API_KEY: &str = "HACKER_NEW_OPENAI_API_KEY";
pub const ENV_OPENAI_BASE_URL: &str = "HACKER_NEW_OPENAI_BASE_URL";
pub const ENV_MODEL: &str = "HACKER_NEW_MODEL";
pub const ENV_STORY_COUNT: &str = "HACKER_NEW_STORY_COUNT";
pub const ENV_DATABASE_URL: &str = "HACKER_NEW_DATABASE_URL";
pub const ENV_PORT: &str = "HACKER_NEW_PORT";
pub const ENV_AUTO_UPDATE_INTERVAL: &str = "HACKER_NEW_AUTO_UPDATE_INTERVAL";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
}

impl AppConfig {
    /// Load configuration from environment variables.
    /// Note: .env file should be loaded in main.rs before calling this.
    pub fn from_env() -> Self {
        let database_url = env::var(ENV_DATABASE_URL)
            .unwrap_or_else(|_| "sqlite:data/hacker_news.db?mode=rwc".to_string());
        let port = env::var(ENV_PORT)
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self { database_url, port }
    }
}

/// Get API key from environment variable (sensitive, not stored in database)
pub fn get_api_key_from_env() -> Option<String> {
    env::var(ENV_OPENAI_API_KEY).ok().filter(|s| !s.is_empty())
}

/// Get API base URL from environment variable (can override database)
pub fn get_base_url_from_env() -> Option<String> {
    env::var(ENV_OPENAI_BASE_URL).ok().filter(|s| !s.is_empty())
}

/// Get model from environment variable (can override database)
pub fn get_model_from_env() -> Option<String> {
    env::var(ENV_MODEL).ok().filter(|s| !s.is_empty())
}

/// Get story count from environment variable (can override database)
pub fn get_story_count_from_env() -> Option<i32> {
    env::var(ENV_STORY_COUNT)
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
}

/// Get auto_update_interval from environment variable
/// Returns minutes between auto-updates. 0 means disabled.
pub fn get_auto_update_interval_from_env() -> u32 {
    env::var(ENV_AUTO_UPDATE_INTERVAL)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}