use anyhow::Result;
use crate::db::models::HnStory;

const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";

pub struct HnClient {
    client: reqwest::Client,
}

impl HnClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Fetch top story IDs from Hacker News
    pub async fn fetch_top_stories(&self) -> Result<Vec<i64>> {
        let url = format!("{}/topstories.json", HN_API_BASE);
        let response = self.client.get(&url).send().await?;
        let stories: Vec<i64> = response.json().await?;
        Ok(stories)
    }

    /// Fetch story details by ID
    pub async fn fetch_story(&self, id: i64) -> Result<HnStory> {
        let url = format!("{}/item/{}.json", HN_API_BASE, id);
        let response = self.client.get(&url).send().await?;
        let story: HnStory = response.json().await?;
        Ok(story)
    }

    /// Fetch multiple stories concurrently
    pub async fn fetch_stories(&self, ids: &[i64]) -> Result<Vec<HnStory>> {
        let mut stories = Vec::with_capacity(ids.len());

        // Fetch stories in batches to avoid overwhelming the API
        for chunk in ids.chunks(10) {
            let futures: Vec<_> = chunk.iter().map(|id| self.fetch_story(*id)).collect();
            let results = futures::future::try_join_all(futures).await?;
            stories.extend(results);
        }

        Ok(stories)
    }
}

impl Default for HnClient {
    fn default() -> Self {
        Self::new()
    }
}