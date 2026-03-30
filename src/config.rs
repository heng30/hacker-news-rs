use std::env;

pub const ENV_OPENAI_API_KEY: &str = "HACKER_NEWS_OPENAI_API_KEY";
pub const ENV_OPENAI_BASE_URL: &str = "HACKER_NEWS_OPENAI_BASE_URL";
pub const ENV_MODEL: &str = "HACKER_NEWS_MODEL";
pub const ENV_DATABASE_URL: &str = "HACKER_NEWS_DATABASE_URL";
pub const ENV_PORT: &str = "HACKER_NEWS_PORT";
pub const ENV_AUTO_UPDATE_INTERVAL: &str = "HACKER_NEWS_AUTO_UPDATE_INTERVAL";
pub const ENV_SOCKS5_PROXY: &str = "HACKER_NEWS_SOCKS5";
pub const ENV_SEARCH_KEYWORDS: &str = "HACKER_NEWS_SEARCH_KEYWORDS";
pub const ENV_LLM_NO_STREAM: &str = "HACKER_NEWS_LLM_NO_STREAM";
pub const ENV_LLM_USER_AGENT: &str = "HACKER_NEWS_LLM_USER_AGENT";
pub const ENV_LLM_TIMEOUT: &str = "HACKER_NEWS_LLM_TIMEOUT";
pub const ENV_FETCH_HTML_TIMEOUT: &str = "HACKER_NEWS_FETCH_HTML_TIMEOUT";
pub const ENV_MAX_MARKDOWN_CONTENT_LENGTH: &str = "HACKER_NEWS_MARKDOWN_MAX_CONTENT_LENGTH";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub port: u16,
}

impl AppConfig {
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

pub fn get_api_key_from_env() -> Option<String> {
    env::var(ENV_OPENAI_API_KEY).ok().filter(|s| !s.is_empty())
}

pub fn get_base_url_from_env() -> Option<String> {
    env::var(ENV_OPENAI_BASE_URL).ok().filter(|s| !s.is_empty())
}

pub fn get_model_from_env() -> Option<String> {
    env::var(ENV_MODEL).ok().filter(|s| !s.is_empty())
}

pub fn get_llm_config_from_env() -> LlmConfig {
    LlmConfig {
        api_base_url: get_base_url_from_env()
            .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string()),
        model: get_model_from_env().unwrap_or_else(|| "deepseek-chat".to_string()),
        api_key: get_api_key_from_env().unwrap_or_default(),
    }
}

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_base_url: String,
    pub model: String,
    pub api_key: String,
}

pub fn get_auto_update_interval_from_env() -> u32 {
    env::var(ENV_AUTO_UPDATE_INTERVAL)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

pub fn get_socks5_proxy_from_env() -> Option<String> {
    env::var(ENV_SOCKS5_PROXY).ok().filter(|s| !s.is_empty())
}

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

pub fn get_llm_no_stream_from_env() -> Option<bool> {
    env::var(ENV_LLM_NO_STREAM)
        .ok()
        .and_then(|s| s.parse().ok())
}

pub fn get_llm_user_agent_from_env() -> Option<String> {
    env::var(ENV_LLM_USER_AGENT).ok().filter(|s| !s.is_empty())
}

pub fn get_llm_timeout() -> u32 {
    env::var(ENV_LLM_TIMEOUT)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(180)
}

pub fn get_fetch_html_timeout() -> u32 {
    env::var(ENV_FETCH_HTML_TIMEOUT)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

pub fn get_max_markdown_content_length() -> u32 {
    env::var(ENV_MAX_MARKDOWN_CONTENT_LENGTH)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16000)
}
