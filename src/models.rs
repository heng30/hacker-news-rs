use serde::{Deserialize, Serialize};

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
    pub summary_zh: Option<String>,
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
            summary_zh: None,
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

/// Fetch progress for tracking SSE-like updates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FetchProgress {
    pub fetch_id: String,
    pub total_stories: usize,
    pub stories_added: usize,
    pub summaries_done: usize,
    pub summaries_error: usize,
    pub finished: bool,
    pub stories: Vec<Story>,
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
