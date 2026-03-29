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

/// Get existing hn_ids for a specific episode to avoid duplicates
pub async fn get_existing_hn_ids(pool: &SqlitePool, episode_id: i64) -> Result<Vec<i64>> {
    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT hn_id FROM stories WHERE episode_id = ?1",
    )
    .bind(episode_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Get story by hn_id
pub async fn get_story_by_hn_id(pool: &SqlitePool, hn_id: i64) -> Result<Option<Story>> {
    let story = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at FROM stories WHERE hn_id = ?1",
    )
    .bind(hn_id)
    .fetch_optional(pool)
    .await?;

    Ok(story)
}

/// Update summary for a specific language only, preserving the other language's summary
pub async fn update_story_summary_by_lang(pool: &SqlitePool, story_id: i64, lang: &str, summary: &str) -> Result<()> {
    if lang == "en" {
        sqlx::query(
            r#"
            UPDATE stories SET summary = ?1, fetched_at = datetime('now')
            WHERE id = ?2
            "#,
        )
        .bind(summary)
        .bind(story_id)
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            r#"
            UPDATE stories SET summary_zh = ?1, fetched_at = datetime('now')
            WHERE id = ?2
            "#,
        )
        .bind(summary)
        .bind(story_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Delete all stories from database and return count of deleted rows
pub async fn delete_all_stories(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query("DELETE FROM stories")
        .execute(pool)
        .await?;

    // Also delete all episodes
    sqlx::query("DELETE FROM episodes")
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as usize)
}

/// Delete episode by date and all its stories, return count of deleted stories
pub async fn delete_episode_by_date(pool: &SqlitePool, date: &str) -> Result<usize> {
    // First get the episode id
    let episode = get_episode_by_date(pool, date).await?;

    match episode {
        Some(ep) => {
            // Delete stories first
            let stories_result = sqlx::query("DELETE FROM stories WHERE episode_id = ?1")
                .bind(ep.id)
                .execute(pool)
                .await?;

            // Delete episode
            sqlx::query("DELETE FROM episodes WHERE id = ?1")
                .bind(ep.id)
                .execute(pool)
                .await?;

            Ok(stories_result.rows_affected() as usize)
        }
        None => Ok(0)
    }
}

/// Delete all stories for a specific episode (keep the episode)
pub async fn delete_stories_by_episode(pool: &SqlitePool, episode_id: i64) -> Result<usize> {
    let result = sqlx::query("DELETE FROM stories WHERE episode_id = ?1")
        .bind(episode_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as usize)
}

/// Delete stories by hn_ids and return count of deleted rows
pub async fn delete_stories_by_hn_ids(pool: &SqlitePool, hn_ids: &[i64]) -> Result<usize> {
    if hn_ids.is_empty() {
        return Ok(0);
    }

    // Build the query with placeholders
    let placeholders: Vec<String> = hn_ids.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        "DELETE FROM stories WHERE hn_id IN ({})",
        placeholders.join(",")
    );

    // Build the query dynamically
    let mut query_builder = sqlx::query(&query);
    for hn_id in hn_ids {
        query_builder = query_builder.bind(hn_id);
    }

    let result = query_builder.execute(pool).await?;

    Ok(result.rows_affected() as usize)
}