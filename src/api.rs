use crate::db::HnStory;
use anyhow::Result;
use serde::Deserialize;

const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const ALGOLIA_API_BASE: &str = "https://hn.algolia.com/api/v1";

#[derive(Debug, Clone, Deserialize)]
struct AlgoliaHit {
    #[serde(rename = "objectID")]
    object_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    author: String,
    #[serde(default)]
    points: i64,
    #[serde(default)]
    created_at_i: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct AlgoliaResponse {
    hits: Vec<AlgoliaHit>,
}

pub struct HnClient {
    client: reqwest::Client,
}

impl Default for HnClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct HnStoryRaw {
    id: i64,
    title: String,
    url: Option<String>,
    by: String,
    score: i64,
    time: i64,
}

impl From<HnStoryRaw> for HnStory {
    fn from(story: HnStoryRaw) -> Self {
        Self {
            id: story.id,
            title: story.title,
            url: story.url,
            by: story.by,
            score: story.score,
            time: story.time,
            tag: "top".to_string(),
        }
    }
}

impl From<AlgoliaHit> for HnStory {
    fn from(h: AlgoliaHit) -> Self {
        Self {
            id: h.object_id.parse().unwrap_or(0),
            title: h.title,
            url: h.url,
            by: h.author,
            score: h.points,
            time: h.created_at_i,
            tag: "".to_string(),
        }
    }
}

impl HnClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    // Fetch top story IDs from Hacker News
    pub async fn fetch_top_stories(&self) -> Result<Vec<i64>> {
        let url = format!("{}/topstories.json", HN_API_BASE);
        let response = self.client.get(&url).send().await?;
        let stories: Vec<i64> = response.json().await?;
        Ok(stories)
    }

    pub async fn fetch_story(&self, id: i64) -> Result<HnStory> {
        let url = format!("{}/item/{}.json", HN_API_BASE, id);
        let response = self.client.get(&url).send().await?;
        let story: HnStoryRaw = response.json().await?;
        Ok(story.into())
    }

    pub async fn fetch_stories(&self, ids: &[i64]) -> Result<Vec<HnStory>> {
        let mut stories = Vec::with_capacity(ids.len());

        for chunk in ids.chunks(10) {
            let futures: Vec<_> = chunk.iter().map(|id| self.fetch_story(*id)).collect();
            let results = futures::future::try_join_all(futures).await?;
            stories.extend(results);
        }

        Ok(stories)
    }

    // Search newest stories by keyword using Algolia API
    // tag parameter is used to mark the search source (the keyword itself)
    pub async fn search_newest(&self, keyword: &str, limit: usize) -> Result<Vec<HnStory>> {
        let url = format!(
            "{}/search_by_date?query={}&tags=story&hitsPerPage={}",
            ALGOLIA_API_BASE, keyword, limit
        );
        tracing::info!(
            "Searching Algolia for keyword '{}' with limit {}",
            keyword,
            limit
        );
        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        tracing::debug!(
            "Algolia response text (first 500 chars): {}",
            text.chars().take(500).collect::<String>()
        );

        let algolia: AlgoliaResponse = serde_json::from_str(&text).map_err(|e| {
            tracing::error!(
                "Failed to parse Algolia response: {}. Response text: {}",
                e,
                text.chars().take(1000).collect::<String>()
            );
            anyhow::anyhow!("Failed to parse Algolia response: {}", e)
        })?;

        tracing::info!(
            "Algolia returned {} hits for keyword '{}'",
            algolia.hits.len(),
            keyword
        );

        Ok(algolia
            .hits
            .into_iter()
            .map(|h| {
                let mut hs: HnStory = h.into();
                hs.tag = keyword.to_string();
                hs
            })
            .collect())
    }
}
