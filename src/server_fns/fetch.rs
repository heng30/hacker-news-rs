use leptos::prelude::*;
use server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
pub use ssr::fetch_stories_task;

#[cfg(feature = "ssr")]
use super::app_state;

/// Start a background fetch and return a fetch_id for SSE subscription
#[server]
pub async fn start_fetch() -> Result<String, ServerFnError> {
    let state = app_state()?;

    if state.config.api_key.is_empty() {
        return Err(ServerFnError::new("API key not configured"));
    }

    let fetch_id = uuid::Uuid::new_v4().to_string();

    // Spawn background task
    let db = state.db.clone();
    let config = state.config.clone();
    let http_client = state.http_client.clone();
    let event_tx = state.fetch_events.clone();

    tokio::spawn(async move {
        if let Err(e) = ssr::fetch_stories_task(db, config, http_client, event_tx).await {
            tracing::error!("Fetch task error: {}", e);
        }
    });

    Ok(fetch_id)
}

#[cfg(feature = "ssr")]
mod ssr {
    use crate::{
        api::HnClient,
        config::AppConfig,
        llm::LlmClient,
        models::{FetchEvent, HnStory, Story},
    };
    use std::{collections::HashSet, sync::Arc};
    use tokio::sync::broadcast::Sender;

    pub async fn fetch_stories_task(
        db: Arc<sled::Db>,
        config: Arc<AppConfig>,
        http_client: reqwest::Client,
        event_tx: Sender<FetchEvent>,
    ) -> anyhow::Result<()> {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let episode = crate::db::create_episode(&db, &today)?;

        // Get existing URL hashes for dedup
        let url_hashes_tree = db.open_tree("url_hashes")?;
        let existing_urls: HashSet<String> = url_hashes_tree
            .iter()
            .filter_map(|item| {
                let (key, _) = item.ok()?;
                Some(String::from_utf8_lossy(&key).to_string())
            })
            .collect();

        let hn_client = HnClient::new(http_client.clone());
        let top_ids = hn_client.fetch_top_stories().await?;
        let all_top_stories = hn_client.fetch_stories(&top_ids).await?;

        let min_score = config.top_story_min_score;
        let top_stories: Vec<HnStory> = all_top_stories
            .into_iter()
            .filter(|s| {
                let url_key = s.url.as_deref().unwrap_or(&s.title);
                let hash = crate::db::blake3_hash(url_key);
                !existing_urls.contains(&hash)
            })
            .filter(|s| s.score >= min_score)
            .collect();

        tracing::info!(
            "Fetching {} top stories with score >= {}",
            top_stories.len(),
            min_score
        );

        // Fetch keyword search results
        let search_stories = if let Some(keywords) = &config.search_keywords {
            let mut stories = Vec::new();
            for kw in keywords {
                // hnrss.org RSS/Atom search
                match hn_client.search_by_rss(kw).await {
                    Ok(s) => {
                        let filtered: Vec<_> = s
                            .into_iter()
                            .filter(|s| {
                                let url_key = s.url.as_deref().unwrap_or(&s.title);
                                let hash = crate::db::blake3_hash(url_key);
                                !existing_urls.contains(&hash)
                            })
                            .collect();
                        stories.extend(filtered);
                    }
                    Err(e) => {
                        tracing::warn!("hnrss.org search failed for '{}': {}", kw, e);
                    }
                }

                // Algolia search
                match hn_client.search_by_keyword(kw).await {
                    Ok(s) => {
                        let filtered: Vec<_> = s
                            .into_iter()
                            .filter(|s| {
                                let url_key = s.url.as_deref().unwrap_or(&s.title);
                                let hash = crate::db::blake3_hash(url_key);
                                !existing_urls.contains(&hash)
                            })
                            .collect();
                        stories.extend(filtered);
                    }
                    Err(e) => {
                        tracing::warn!("Algolia search failed for '{}': {}", kw, e);
                    }
                }
            }
            stories
        } else {
            Vec::new()
        };

        // Merge and deduplicate
        let mut all_hn_stories: Vec<HnStory> = Vec::new();
        let mut seen_urls: HashSet<String> = HashSet::new();
        for story in top_stories.into_iter().chain(search_stories.into_iter()) {
            let url_key = story.url.as_deref().unwrap_or(&story.title);
            let hash = crate::db::blake3_hash(url_key);
            if !seen_urls.contains(&hash) {
                seen_urls.insert(hash);
                all_hn_stories.push(story);
            }
        }

        let total_stories = all_hn_stories.len();
        tracing::info!("Total {} unique stories to process", total_stories);

        if all_hn_stories.is_empty() {
            _ = event_tx.send(FetchEvent::Finished {
                total: 0,
                summaries: 0,
                errors: 0,
            });
            return Ok(());
        }

        // Save stories one by one, emit event for each
        let llm_client = LlmClient::new(config.clone(), http_client);
        let mut saved_stories: Vec<Story> = Vec::new();

        for hn_story in &all_hn_stories {
            let mut story: Story = hn_story.clone().into();
            story.episode_date = episode.date.clone();
            crate::db::save_story(&db, &story)?;

            let url_key = story.url.as_deref().unwrap_or(&story.title);
            crate::db::mark_url_seen(&db, url_key)?;

            if let Some(saved) = crate::db::get_story_by_hn_id(&db, story.hn_id)? {
                saved_stories.push(saved);
                _ = event_tx.send(FetchEvent::StoryAdded { hn_id: story.hn_id });
            }
        }

        // Generate summaries for new stories
        let concurrency = config.summary_concurrency;
        let mut summaries_count = 0usize;
        let mut errors_count = 0usize;

        let (s, e) =
            generate_summaries(&db, &llm_client, &saved_stories, concurrency, &event_tx).await;
        summaries_count += s;
        errors_count += e;

        // Regenerate summaries for unread stories that are missing summaries
        let (s, e) =
            backfill_missing_summaries(&db, &llm_client, &episode.date, concurrency, &event_tx)
                .await;
        summaries_count += s;
        errors_count += e;

        _ = event_tx.send(FetchEvent::Finished {
            total: total_stories,
            summaries: summaries_count,
            errors: errors_count,
        });

        tracing::info!(
            "Fetch completed: {} summaries, {} errors",
            summaries_count,
            errors_count
        );
        Ok(())
    }

    /// Generate summaries for a list of stories in batches
    async fn generate_summaries(
        db: &Arc<sled::Db>,
        llm_client: &LlmClient,
        stories: &[Story],
        concurrency: usize,
        event_tx: &Sender<FetchEvent>,
    ) -> (usize, usize) {
        let mut summaries_count = 0usize;
        let mut errors_count = 0usize;

        for chunk in stories.chunks(concurrency) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|story| {
                    let db = db.clone();
                    let llm_client = llm_client.clone();
                    let story_clone = story.clone();

                    async move {
                        match llm_client
                            .summarize(&story_clone.title, story_clone.url.as_deref())
                            .await
                        {
                            Ok(summary) => {
                                _ = crate::db::update_story_summary(
                                    &db,
                                    &story_clone.episode_date,
                                    story_clone.hn_id,
                                    summary.as_deref(),
                                );
                                Ok(story_clone.hn_id)
                            }
                            Err(e) => {
                                tracing::error!("Failed to summarize {}: {}", story_clone.hn_id, e);
                                Err(story_clone.hn_id)
                            }
                        }
                    }
                })
                .collect();

            let results = futures::future::join_all(futures).await;

            for result in &results {
                match result {
                    Ok(hn_id) => {
                        summaries_count += 1;
                        _ = event_tx.send(FetchEvent::SummaryDone { hn_id: *hn_id });
                    }
                    Err(hn_id) => {
                        errors_count += 1;
                        _ = event_tx.send(FetchEvent::SummaryError { hn_id: *hn_id });
                    }
                }
            }
        }

        (summaries_count, errors_count)
    }

    /// Generate summaries for unread stories that are missing summaries
    async fn backfill_missing_summaries(
        db: &Arc<sled::Db>,
        llm_client: &LlmClient,
        episode_date: &str,
        concurrency: usize,
        event_tx: &Sender<FetchEvent>,
    ) -> (usize, usize) {
        let read_stories = match crate::db::get_read_stories(db) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Failed to get read stories: {}", e);
                return (0, 0);
            }
        };

        let all_stories = match crate::db::get_stories_by_episode(db, episode_date) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to get episode stories: {}", e);
                return (0, 0);
            }
        };

        let missing: Vec<Story> = all_stories
            .into_iter()
            .filter(|s| !read_stories.contains(&s.hn_id))
            .filter(|s| s.summary.is_none() || s.summary.as_deref().unwrap_or_default().is_empty())
            .collect();

        if missing.is_empty() {
            return (0, 0);
        }

        tracing::info!(
            "Backfilling summaries for {} unread stories missing summaries",
            missing.len()
        );

        generate_summaries(db, llm_client, &missing, concurrency, event_tx).await
    }
}
