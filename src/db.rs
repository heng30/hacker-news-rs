use anyhow::Result;
use blake3::Hash;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Episode {
    pub id: i64,
    pub date: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Story {
    pub id: i64,
    pub episode_id: i64,
    pub hn_id: i64,
    pub title: String,
    pub url: Option<String>,
    pub by: String,
    pub score: i64,
    pub time: i64,
    pub summary: Option<String>,
    pub summary_zh: Option<String>,
    pub fetched_at: String,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnStory {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub by: String,
    pub score: i64,
    pub time: i64,
    pub tag: String,
}

impl From<HnStory> for Story {
    fn from(hn: HnStory) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: 0,
            episode_id: 0,
            hn_id: hn.id,
            title: hn.title,
            url: hn.url,
            by: hn.by,
            score: hn.score,
            time: hn.time,
            summary: None,
            summary_zh: None,
            fetched_at: now,
            tag: hn.tag,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeWithStories {
    pub episode: Episode,
    pub stories: Vec<Story>,
}

pub async fn init_db(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
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
            tag TEXT NOT NULL DEFAULT 'top',
            FOREIGN KEY (episode_id) REFERENCES episodes(id)
        );

        CREATE TABLE IF NOT EXISTS url_hashes (
            url_hash TEXT PRIMARY KEY,
            hn_id INTEGER NOT NULL,
            first_seen_at TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

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
        INSERT INTO stories (episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at, tag)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
    .bind(&story.tag)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_stories_by_episode(pool: &SqlitePool, episode_id: i64) -> Result<Vec<Story>> {
    let stories = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at, tag FROM stories WHERE episode_id = ?1 ORDER BY score DESC",
    )
    .bind(episode_id)
    .fetch_all(pool)
    .await?;

    Ok(stories)
}

pub async fn get_all_stories(pool: &SqlitePool) -> Result<Vec<Story>> {
    let stories = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at, tag FROM stories ORDER BY fetched_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(stories)
}

pub async fn get_story_by_hn_id(pool: &SqlitePool, hn_id: i64) -> Result<Option<Story>> {
    let story = sqlx::query_as::<_, Story>(
        "SELECT id, episode_id, hn_id, title, url, by, score, time, summary, summary_zh, fetched_at, tag FROM stories WHERE hn_id = ?1",
    )
    .bind(hn_id)
    .fetch_optional(pool)
    .await?;

    Ok(story)
}

pub async fn update_story_summary_by_lang(
    pool: &SqlitePool,
    story_id: i64,
    lang: &str,
    summary: &str,
) -> Result<()> {
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

pub async fn update_story_summary(
    pool: &SqlitePool,
    story_id: i64,
    summary: Option<&str>,
    summary_zh: Option<&str>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE stories SET summary = ?1, summary_zh = ?2, fetched_at = datetime('now')
        WHERE id = ?3
        "#,
    )
    .bind(summary)
    .bind(summary_zh)
    .bind(story_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_all_stories(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query("DELETE FROM stories").execute(pool).await?;
    sqlx::query("DELETE FROM episodes").execute(pool).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn delete_episode_by_date(pool: &SqlitePool, date: &str) -> Result<usize> {
    let episode = get_episode_by_date(pool, date).await?;

    match episode {
        Some(ep) => {
            let stories_result = sqlx::query("DELETE FROM stories WHERE episode_id = ?1")
                .bind(ep.id)
                .execute(pool)
                .await?;

            sqlx::query("DELETE FROM episodes WHERE id = ?1")
                .bind(ep.id)
                .execute(pool)
                .await?;

            Ok(stories_result.rows_affected() as usize)
        }
        None => Ok(0),
    }
}

pub async fn delete_stories_by_episode(pool: &SqlitePool, episode_id: i64) -> Result<usize> {
    let result = sqlx::query("DELETE FROM stories WHERE episode_id = ?1")
        .bind(episode_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() as usize)
}

pub async fn delete_stories_by_hn_ids(pool: &SqlitePool, hn_ids: &[i64]) -> Result<usize> {
    if hn_ids.is_empty() {
        return Ok(0);
    }

    let placeholders: Vec<String> = hn_ids.iter().map(|_| "?".to_string()).collect();
    let query = format!(
        "DELETE FROM stories WHERE hn_id IN ({})",
        placeholders.join(",")
    );

    let mut query_builder = sqlx::query(&query);
    for hn_id in hn_ids {
        query_builder = query_builder.bind(hn_id);
    }

    let result = query_builder.execute(pool).await?;

    Ok(result.rows_affected() as usize)
}

pub fn compute_dedup_hash(url: Option<&str>, title: &str) -> String {
    let content = url.unwrap_or(title);
    Hash::from(blake3::hash(content.as_bytes()))
        .to_hex()
        .to_string()
}

pub async fn get_existing_url_hashes(pool: &SqlitePool) -> Result<HashSet<String>> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT url_hash FROM url_hashes")
        .fetch_all(pool)
        .await?;

    Ok(rows.into_iter().map(|(hash,)| hash).collect())
}

pub async fn insert_url_hash(pool: &SqlitePool, url_hash: &str, hn_id: i64) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO url_hashes (url_hash, hn_id, first_seen_at)
        VALUES (?1, ?2, ?3)
        "#,
    )
    .bind(url_hash)
    .bind(hn_id)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(())
}
