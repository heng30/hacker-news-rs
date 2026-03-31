use super::fetcher::USER_AGENT;
use crate::db::HnStory;
use anyhow::Result;
use serde::Deserialize;

const HN_RSS_BASE: &str = "https://hnrss.org";
const HN_API_BASE: &str = "https://hacker-news.firebaseio.com/v0";

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

impl HnClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

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

    // Search newest stories by keyword using hnrss.org RSS API
    // tag parameter is used to mark the search source (the keyword itself)
    pub async fn search_newest(&self, keyword: &str) -> Result<Vec<HnStory>> {
        let url = format!("{}/newest?q={}", HN_RSS_BASE, keyword);
        tracing::info!("Searching hnrss for keyword '{}'", keyword);

        let response = self.client.get(&url).send().await?;
        let text = response.text().await?;
        tracing::debug!(
            "hnrss response text (first 500 chars): {}",
            text.chars().take(500).collect::<String>()
        );

        let feed = feed_rs::parser::parse(text.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse RSS feed: {}", e))?;

        let stories: Vec<HnStory> = feed
            .entries
            .iter()
            .map(|entry| {
                let id = extract_hn_id_from_entry(entry);
                let score = extract_score_from_entry(entry);
                let time = entry.published.map(|dt| dt.timestamp()).unwrap_or(0);

                HnStory {
                    id,
                    title: entry
                        .title
                        .as_ref()
                        .map(|t| t.content.clone())
                        .unwrap_or_default(),
                    url: entry.links.first().map(|l| l.href.clone()),
                    by: entry
                        .authors
                        .first()
                        .map(|a| a.name.clone())
                        .unwrap_or_default(),
                    score,
                    time,
                    tag: keyword.to_string(),
                }
            })
            .collect();

        tracing::info!(
            "hnrss returned {} hits for keyword '{}'",
            stories.len(),
            keyword
        );
        Ok(stories)
    }
}

fn extract_hn_id_from_entry(entry: &feed_rs::model::Entry) -> i64 {
    if entry.id.contains("news.ycombinator.com/item?id=")
        && let Some(id_str) = entry.id.split("id=").last()
    {
        return id_str.parse().unwrap_or(0);
    }
    0
}

fn extract_score_from_entry(entry: &feed_rs::model::Entry) -> i64 {
    if let Some(content) = &entry.content
        && let Some(body) = &content.body
    {
        if let Some(points_start) = body.find("Points: ") {
            let rest = &body[points_start + 8..];
            if let Some(points_end) = rest.find('<') {
                return rest[..points_end].parse().unwrap_or(0);
            }
        }
    }
    0
}
