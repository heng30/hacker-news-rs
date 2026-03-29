pub mod models;

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use crate::db::models::{Config, Episode, Story};
use crate::config::{
    get_api_key_from_env, get_base_url_from_env, get_model_from_env, get_story_count_from_env
};

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS episodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS stories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            episode_id INTEGER NOT NULL,
            hn_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            url TEXT,
            by TEXT NOT NULL,
            score INTEGER NOT NULL,
            time INTEGER NOT NULL,
            summary TEXT,
            summary_zh TEXT,
            fetched_at TEXT NOT NULL,
            FOREIGN KEY (episode_id) REFERENCES episodes(id)
        );
        "#,
    )
    .execute(pool)
    .await?;

    // Insert default config values if they don't exist
    let defaults = [
        ("story_count", "30"),
        ("api_base_url", "https://api.openai.com/v1"),
        ("model", "gpt-4o-mini"),
        ("api_key", ""),
    ];

    for (key, value) in defaults {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO config (key, value, updated_at)
            VALUES (?1, ?2, datetime('now'))
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_config(pool: &SqlitePool) -> Result<Config> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM config")
        .fetch_all(pool)
        .await?;

    let mut config = Config::default();
    for (key, value) in rows {
        match key.as_str() {
            "story_count" => config.story_count = value.parse().unwrap_or(30),
            "api_base_url" => config.api_base_url = value,
            "model" => config.model = value,
            "api_key" => config.api_key = value,
            _ => {}
        }
    }
    Ok(config)
}

/// Get configuration with environment variable overrides.
/// Environment variables take precedence over database values for:
/// - HACKER_NEW_OPENAI_API_KEY (always from env if set, never stored in DB)
/// - HACKER_NEW_OPENAI_BASE_URL (overrides DB if set)
/// - HACKER_NEW_MODEL (overrides DB if set)
/// - HACKER_NEW_STORY_COUNT (overrides DB if set)
pub async fn get_config_with_env_overrides(pool: &SqlitePool) -> Result<Config> {
    let config = get_config(pool).await?;

    // Apply environment variable overrides
    let config = Config {
        // API key from env takes precedence (sensitive, not stored in DB)
        api_key: get_api_key_from_env().unwrap_or(config.api_key),
        // Base URL can be overridden by env
        api_base_url: get_base_url_from_env().unwrap_or(config.api_base_url),
        // Model can be overridden by env
        model: get_model_from_env().unwrap_or(config.model),
        // Story count can be overridden by env
        story_count: get_story_count_from_env().unwrap_or(config.story_count),
    };

    Ok(config)
}

pub async fn update_config(pool: &SqlitePool, config: &Config) -> Result<()> {
    let updates = [
        ("story_count", config.story_count.to_string()),
        ("api_base_url", config.api_base_url.clone()),
        ("model", config.model.clone()),
        ("api_key", config.api_key.clone()),
    ];

    for (key, value) in updates {
        sqlx::query(
            r#"
            INSERT INTO config (key, value, updated_at)
            VALUES (?1, ?2, datetime('now'))
            ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')
            "#,
        )
        .bind(key)
        .bind(&value)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn create_episode(pool: &SqlitePool, date: &str) -> Result<i64> {
    let now = chrono::Utc::now().to_rfc3339();
    let result = sqlx::query(
        r#"
        INSERT INTO episodes (date, created_at, updated_at)
        VALUES (?1, ?2, ?2)
        ON CONFLICT(date) DO UPDATE SET updated_at = ?2
        RETURNING id
        "#,
    )
    .bind(date)
    .bind(&now)
    .fetch_one(pool)
    .await?;

    Ok(result.get::<i64, _>(0))
}

pub async fn get_episode_by_date(pool: &SqlitePool, date: &str) -> Result<Option<Episode>> {
    let episode = sqlx::query_as::<_, Episode>(
        "SELECT id, date, created_at, updated_at FROM episodes WHERE date = ?1",
    )
    .bind(date)
    .fetch_optional(pool)
    .await?;

    Ok(episode)
}

pub async fn get_latest_episode(pool: &SqlitePool) -> Result<Option<Episode>> {
    let episode = sqlx::query_as::<_, Episode>(
        "SELECT id, date, created_at, updated_at FROM episodes ORDER BY date DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(episode)
}

pub async fn get_episodes(pool: &SqlitePool) -> Result<Vec<Episode>> {
    let episodes = sqlx::query_as::<_, Episode>(
        "SELECT id, date, created_at, updated_at FROM episodes ORDER BY date DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(episodes)
}

pub async fn save_story(pool: &SqlitePool, story: &Story) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO stories (episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
    )
    .bind(story.episode_id)
    .bind(story.hn_id)
    .bind(&story.title)
    .bind(&story.url)
    .bind(&story.by)
    .bind(story.score)
    .bind(story.time)
    .bind(&story.summary)
    .bind(&story.summary_zh)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_stories_by_episode(pool: &SqlitePool, episode_id: i64) -> Result<Vec<Story>> {
    let stories = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at FROM stories WHERE episode_id = ?1 ORDER BY score DESC",
    )
    .bind(episode_id)
    .fetch_all(pool)
    .await?;

    Ok(stories)
}

pub async fn get_all_stories(pool: &SqlitePool) -> Result<Vec<Story>> {
    let stories = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at FROM stories ORDER BY fetched_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(stories)
}