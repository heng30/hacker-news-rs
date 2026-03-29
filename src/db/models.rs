use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub story_count: i32,
    pub api_base_url: String,
    pub model: String,
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            story_count: 30,
            api_base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnStory {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub by: String,
    pub score: i64,
    pub time: i64,
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeWithStories {
    pub episode: Episode,
    pub stories: Vec<Story>,
}