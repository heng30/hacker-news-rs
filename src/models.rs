use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// An episode groups stories by date
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub date: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A single HN story with AI-generated summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Story {
    pub id: u64,
    pub episode_date: String,
    pub hn_id: i64,
    pub title: String,
    pub url: Option<String>,
    pub by: String,
    pub score: i64,
    pub time: i64,
    pub summary: Option<String>,
    pub fetched_at: String,
    pub tag: String,
}

/// A story fetched from HN API (before saving to DB)
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
            episode_date: String::new(),
            hn_id: hn.id,
            title: hn.title,
            url: hn.url,
            by: hn.by,
            score: hn.score,
            time: hn.time,
            summary: None,
            fetched_at: now,
            tag: hn.tag,
        }
    }
}

/// Episode with its stories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeWithStories {
    pub episode: Episode,
    pub stories: Vec<Story>,
}

/// Fetch event for SSE push updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FetchEvent {
    /// A story has been saved to DB
    StoryAdded { hn_id: i64 },
    /// A summary has been generated
    SummaryDone { hn_id: i64 },
    /// A summary generation failed
    SummaryError { hn_id: i64 },
    /// All fetch and summary work is complete
    Finished {
        total: usize,
        summaries: usize,
        errors: usize,
    },
}

/// Config response for the settings modal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub api_base_url: String,
    pub model: String,
    pub auto_update_interval: u32,
    pub masked_api_key: String,
    pub socks5_proxy: Option<String>,
    pub search_keywords: Option<String>,
    pub summary_concurrency: usize,
}

/// User preferences stored server-side
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub theme: String,
    pub show_unread: bool,
    pub read_stories: HashSet<i64>,
    pub favorite_stories: HashSet<i64>,
}
