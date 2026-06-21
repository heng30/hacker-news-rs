use serde::{Deserialize, Serialize};
use std::env;

use clap::Parser;

/// Hacker News RSS Generator with AI Summaries
#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub struct Args {
    /// Server bind address
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Server bind port
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,

    /// Path to sled database directory (overrides default platform data dir)
    #[arg(long)]
    pub db: Option<String>,

    /// OpenAI API key (can also set HACKER_NEWS_OPENAI_API_KEY env var)
    #[arg(short = 'k', long, env = "HACKER_NEWS_OPENAI_API_KEY")]
    pub api_key: Option<String>,

    /// OpenAI API base URL
    #[arg(
        long,
        default_value = "https://api.deepseek.com/v1",
        env = "HACKER_NEWS_OPENAI_BASE_URL"
    )]
    pub api_base_url: String,

    /// LLM model name
    #[arg(long, default_value = "deepseek-chat", env = "HACKER_NEWS_MODEL")]
    pub model: String,

    /// SOCKS5 proxy URL (e.g. socks5://127.0.0.1:1080)
    #[arg(long, env = "HACKER_NEWS_SOCKS5")]
    pub socks5_proxy: Option<String>,

    /// Search keywords (comma-separated, e.g. "rust,linux")
    #[arg(long, env = "HACKER_NEWS_SEARCH_KEYWORDS")]
    pub search_keywords: Option<String>,

    /// Auto update interval in minutes (0 = disabled)
    #[arg(
        long,
        default_value_t = 0,
        env = "HACKER_NEWS_AUTO_UPDATE_INTERVAL"
    )]
    pub auto_update_interval: u32,

    /// Minimum score for top stories
    #[arg(
        long,
        default_value_t = 500,
        env = "HACKER_NEWS_TOP_STORY_MIN_SCORE"
    )]
    pub top_story_min_score: i64,

    /// Summary generation concurrency
    #[arg(
        long,
        default_value_t = 3,
        env = "HACKER_NEWS_SUMMARY_CONCURRENCY"
    )]
    pub summary_concurrency: usize,

    /// Disable LLM streaming (use non-streaming API)
    #[arg(long, env = "HACKER_NEWS_LLM_NO_STREAM")]
    pub llm_no_stream: Option<bool>,

    /// Custom User-Agent for LLM API requests
    #[arg(long, env = "HACKER_NEWS_LLM_USER_AGENT")]
    pub llm_user_agent: Option<String>,

    /// LLM request timeout in seconds
    #[arg(long, default_value_t = 180, env = "HACKER_NEWS_LLM_TIMEOUT")]
    pub llm_timeout: u32,

    /// HTML fetch timeout in seconds
    #[arg(long, default_value_t = 30, env = "HACKER_NEWS_FETCH_HTML_TIMEOUT")]
    pub fetch_html_timeout: u32,

    /// Maximum content length for text extraction
    #[arg(
        long,
        default_value_t = 16000,
        env = "HACKER_NEWS_MAX_CONTENT_LENGTH"
    )]
    pub max_content_length: u32,
}

/// Resolved application configuration (after clap parsing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    pub api_key: String,
    pub api_base_url: String,
    pub model: String,
    pub socks5_proxy: Option<String>,
    pub search_keywords: Option<Vec<String>>,
    pub auto_update_interval: u32,
    pub top_story_min_score: i64,
    pub summary_concurrency: usize,
    pub llm_no_stream: Option<bool>,
    pub llm_user_agent: Option<String>,
    pub llm_timeout: u32,
    pub fetch_html_timeout: u32,
    pub max_content_length: u32,
}

impl AppConfig {
    /// Build AppConfig from parsed clap Args, resolving db_path to platform default if not specified
    pub fn from_args(args: &Args, db_path: String) -> Self {
        let search_keywords = args
            .search_keywords
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty())
                    .collect::<Vec<_>>()
            });

        Self {
            host: args.host.clone(),
            port: args.port,
            db_path,
            api_key: args.api_key.clone().unwrap_or_default(),
            api_base_url: args.api_base_url.clone(),
            model: args.model.clone(),
            socks5_proxy: args
                .socks5_proxy
                .as_ref()
                .filter(|s| !s.is_empty())
                .cloned(),
            search_keywords,
            auto_update_interval: args.auto_update_interval,
            top_story_min_score: args.top_story_min_score,
            summary_concurrency: args.summary_concurrency,
            llm_no_stream: args.llm_no_stream,
            llm_user_agent: args
                .llm_user_agent
                .as_ref()
                .filter(|s| !s.is_empty())
                .cloned(),
            llm_timeout: args.llm_timeout,
            fetch_html_timeout: args.fetch_html_timeout,
            max_content_length: args.max_content_length,
        }
    }

    /// Resolve database path: use --db arg, or HACKER_NEWS_DATABASE_URL env, or platform default
    pub fn resolve_db_path(args_db: &Option<String>, app_name: &str) -> String {
        // 1. CLI arg takes precedence
        if let Some(path) = args_db {
            return path.clone();
        }

        // 2. Environment variable fallback
        if let Ok(path) = env::var("HACKER_NEWS_DATABASE_URL") {
            if !path.is_empty() {
                return path;
            }
        }

        // 3. Platform data directory
        let app_dirs =
            platform_dirs::AppDirs::new(Some(app_name), true).expect("Failed to resolve data dir");
        app_dirs
            .data_dir
            .join("hacker_news_db")
            .to_string_lossy()
            .to_string()
    }

    /// Mask API key for display
    pub fn masked_api_key(&self) -> String {
        mask_sensitive_value(&self.api_key)
    }
}

/// Mask a sensitive string for display.
/// Shows first 4 and last 4 characters, with asterisks in between.
fn mask_sensitive_value(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let len = value.len();
    if len <= 8 {
        "*".repeat(len)
    } else {
        let first = &value[..4];
        let last = &value[len - 4..];
        format!("{}****{}", first, last)
    }
}
