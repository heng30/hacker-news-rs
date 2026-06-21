use crate::models::HnStory;
use anyhow::Result;
use serde::Deserialize;

const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";

pub struct HnClient {
    client: reqwest::Client,
}

impl HnClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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

impl HnClient {
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

    /// Search stories by keyword using the Algolia HN Search API
    pub async fn search_by_keyword(&self, keyword: &str) -> Result<Vec<HnStory>> {
        let url = format!(
            "https://hn.algolia.com/api/v1/search?query={}&tags=story",
            keyword
        );
        tracing::info!("Searching Algolia for keyword '{}'", keyword);

        let response = self.client.get(&url).send().await?;
        let search_result: AlgoliaSearchResponse = response.json().await?;

        let stories: Vec<HnStory> = search_result
            .hits
            .into_iter()
            .map(|hit| HnStory {
                id: hit.object_id,
                title: hit.title,
                url: hit.url,
                by: hit.author,
                score: hit.points,
                time: hit.created_at_i,
                tag: keyword.to_string(),
            })
            .collect();

        tracing::info!("Algolia returned {} hits for '{}'", stories.len(), keyword);
        Ok(stories)
    }
}

#[derive(Debug, Deserialize)]
struct AlgoliaSearchResponse {
    hits: Vec<AlgoliaHit>,
}

#[derive(Debug, Deserialize)]
struct AlgoliaHit {
    object_id: i64,
    title: String,
    url: Option<String>,
    author: String,
    points: i64,
    created_at_i: i64,
}
