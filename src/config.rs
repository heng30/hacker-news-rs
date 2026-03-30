use std::env;

/// Environment variable names for configuration
/// All environment variables are prefixed with HACKER_NEWS_ for consistency
pub const ENV_OPENAI_API_KEY: &str = "HACKER_NEWS_OPENAI_API_KEY";
pub const ENV_OPENAI_BASE_URL: &str = "HACKER_NEWS_OPENAI_BASE_URL";
pub const ENV_MODEL: &str = "HACKER_NEWS_MODEL";
pub const ENV_STORY_COUNT: &str = "HACKER_NEWS_STORY_COUNT";
pub const ENV_DATABASE_URL: &str = "HACKER_NEWS_DATABASE_URL";
pub const ENV_PORT: &str = "HACKER_NEWS_PORT";
pub const ENV_AUTO_UPDATE_INTERVAL: &str = "HACKER_NEWS_AUTO_UPDATE_INTERVAL";
pub const ENV_SOCKS5_PROXY: &str = "HACKER_NEWS_SOCKS5";
pub const ENV_SEARCH_KEYWORDS: &str = "HACKER_NEWS_SEARCH_KEYWORDS";
pub const ENV_LLM_NO_STREAM: &str = "HACKER_NEWS_LLM_NO_STREAM";
pub const ENV_LLM_NO_LLM_PROXY: &str = "HACKER_NEWS_LLM_NO_LLM_PROXY";
pub const ENV_LLM_USER_AGENT: &str = "HACKER_NEWS_LLM_USER_AGENT";

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

/// Get SOCKS5 proxy from environment variable
/// Returns proxy URL if set (e.g., "127.0.0.1:1080" or "socks5://127.0.0.1:1080")
pub fn get_socks5_proxy_from_env() -> Option<String> {
    env::var(ENV_SOCKS5_PROXY).ok().filter(|s| !s.is_empty())
}

/// Get search keywords from environment variable
/// Returns a list of keywords to search for (e.g., ["rust", "go", "linux"])
/// Keywords are comma-separated in the environment variable
pub fn get_search_keywords_from_env() -> Option<Vec<String>> {
    env::var(ENV_SEARCH_KEYWORDS)
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect()
        })
}

/// Get LLM no_stream setting from environment variable
/// When true, disables streaming mode (useful for APIs that don't support it)
pub fn get_llm_no_stream_from_env() -> Option<bool> {
    env::var(ENV_LLM_NO_STREAM)
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Get LLM no_llm_proxy setting from environment variable
/// When true, disables system proxy for LLM requests
pub fn get_llm_no_llm_proxy_from_env() -> Option<bool> {
    env::var(ENV_LLM_NO_LLM_PROXY)
        .ok()
        .and_then(|s| s.parse().ok())
}

/// Get LLM User-Agent from environment variable
/// Some APIs reject the default reqwest User-Agent
pub fn get_llm_user_agent_from_env() -> Option<String> {
    env::var(ENV_LLM_USER_AGENT).ok().filter(|s| !s.is_empty())
}