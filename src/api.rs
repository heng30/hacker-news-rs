use crate::models::HnStory;
use anyhow::Result;
use serde::Deserialize;
use std::io::BufReader;

const HNRSS_BASE: &str = "https://hnrss.org/newest";
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

#[derive(Debug, Deserialize)]
struct AlgoliaSearchResponse {
    hits: Vec<AlgoliaHit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct AlgoliaHit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: String,
    url: Option<String>,
    author: String,
    points: i64,
    created_at_i: i64,
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
            "https://hn.algolia.com/api/v1/search?query={}&tags=story&hitsPerPage=50",
            keyword
        );
        tracing::info!("Searching Algolia for keyword '{}'", keyword);

        let response = self.client.get(&url).send().await?;
        let search_result: AlgoliaSearchResponse = response.json().await?;

        tracing::trace!("{:?}", search_result);

        let stories: Vec<HnStory> = search_result
            .hits
            .into_iter()
            .filter_map(|hit| {
                let id: i64 = match hit.object_id.parse() {
                    Ok(id) => id,
                    Err(e) => {
                        tracing::warn!(
                            "Skipping hit with non-numeric objectID '{}': {}",
                            hit.object_id,
                            e
                        );
                        return None;
                    }
                };
                Some(HnStory {
                    id,
                    title: hit.title,
                    url: hit.url,
                    by: hit.author,
                    score: hit.points,
                    time: hit.created_at_i,
                    tag: keyword.to_string(),
                })
            })
            .collect();

        tracing::info!("Algolia returned {} hits for '{}'", stories.len(), keyword);
        Ok(stories)
    }

    /// Search stories by keyword using hnrss.org RSS feed
    pub async fn search_by_rss(&self, keyword: &str) -> Result<Vec<HnStory>> {
        let url = format!("{}?q={}", HNRSS_BASE, keyword);
        tracing::info!("Searching hnrss.org for keyword '{}'", keyword);

        let response = self.client.get(&url).send().await?;
        let body = response.text().await?;

        let stories = Self::parse_rss(&body, keyword)?;
        tracing::info!(
            "hnrss.org returned {} RSS items for '{}'",
            stories.len(),
            keyword
        );

        Ok(stories)
    }

    fn parse_rss(body: &str, keyword: &str) -> Result<Vec<HnStory>> {
        let channel = rss::Channel::read_from(BufReader::new(body.as_bytes()))?;
        let stories = channel
            .items
            .into_iter()
            .filter_map(|item| {
                // <comments> contains the HN link with id, e.g. "https://news.ycombinator.com/item?id=48654862"
                let comments = item.comments.as_deref()?;
                let hn_id: i64 = comments
                    .rsplit_once('=')
                    .and_then(|(_, id)| id.parse().ok())?;
                // <link> is the external article URL
                let url = item.link;
                // <pubDate> is RFC 2822 format
                let time = item
                    .pub_date
                    .as_deref()
                    .and_then(|d| chrono::DateTime::parse_from_rfc2822(d).ok())
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0);
                Some(HnStory {
                    id: hn_id,
                    title: item.title.unwrap_or_default(),
                    url,
                    by: item
                        .dublin_core_ext
                        .as_ref()
                        .and_then(|dc| dc.creators.first().cloned())
                        .unwrap_or_default(),
                    score: 0,
                    time,
                    tag: keyword.to_string(),
                })
            })
            .collect();
        Ok(stories)
    }
}
