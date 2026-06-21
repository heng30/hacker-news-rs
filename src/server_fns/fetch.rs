use leptos::prelude::*;
use server_fn::error::ServerFnError;

use crate::models::FetchProgress;

#[cfg(feature = "ssr")]
use std::collections::HashSet;
#[cfg(feature = "ssr")]
use std::sync::Arc;

#[cfg(feature = "ssr")]
use crate::models::{HnStory, Story};
#[cfg(feature = "ssr")]
use crate::state::AppState;

#[cfg(feature = "ssr")]
fn app_state() -> Result<Arc<AppState>, ServerFnError> {
    use_context::<Arc<AppState>>()
        .ok_or_else(|| ServerFnError::new("AppState not found"))
}

/// Start a background fetch and return a fetch_id for polling
#[server]
pub async fn start_fetch(lang: String) -> Result<String, ServerFnError> {
    let state = app_state()?;

    if state.config.api_key.is_empty() {
        return Err(ServerFnError::new("API key not configured"));
    }

    let fetch_id = uuid::Uuid::new_v4().to_string();
    let fetch_id_clone = fetch_id.clone();

    // Initialize progress
    state.fetch_progress.insert(
        fetch_id.clone(),
        FetchProgress {
            fetch_id: fetch_id.clone(),
            ..Default::default()
        },
    );

    // Spawn background task
    let db = state.db.clone();
    let config = state.config.clone();
    let http_client = state.http_client.clone();
    let fetch_progress = state.fetch_progress.clone();

    tokio::spawn(async move {
        if let Err(e) = fetch_stories_task(db, config, http_client, &lang, &fetch_id_clone, fetch_progress.clone()).await {
            tracing::error!("Fetch task error: {}", e);
            if let Some(mut progress) = fetch_progress.get_mut(&fetch_id_clone) {
                progress.finished = true;
            }
        }
    });

    Ok(fetch_id)
}

/// Poll fetch progress
#[server]
pub async fn get_fetch_status(fetch_id: String) -> Result<FetchProgress, ServerFnError> {
    let state = app_state()?;

    let result = match state.fetch_progress.get(&fetch_id) {
        Some(progress) => Ok(progress.clone()),
        None => Err(ServerFnError::new("Fetch not found")),
    };
    result
}

#[cfg(feature = "ssr")]
async fn fetch_stories_task(
    db: Arc<sled::Db>,
    config: Arc<crate::config::AppConfig>,
    http_client: reqwest::Client,
    lang: &str,
    fetch_id: &str,
    fetch_progress: Arc<dashmap::DashMap<String, FetchProgress>>,
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

    let hn_client = crate::api::HnClient::new(http_client.clone());
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

    tracing::info!("Fetching {} top stories with score >= {}", top_stories.len(), min_score);

    // Fetch keyword search results
    let search_stories = if let Some(keywords) = &config.search_keywords {
        let mut stories = Vec::new();
        for kw in keywords {
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
                    tracing::warn!("Failed to search for '{}': {}. Skipping.", kw, e);
                    continue;
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

    tracing::info!("Total {} unique stories to process", all_hn_stories.len());

    if let Some(mut progress) = fetch_progress.get_mut(fetch_id) {
        progress.total_stories = all_hn_stories.len();
    }

    if all_hn_stories.is_empty() {
        if let Some(mut progress) = fetch_progress.get_mut(fetch_id) {
            progress.finished = true;
        }
        return Ok(());
    }

    // Save stories first (without summaries)
    let llm_client = crate::llm::LlmClient::new(&config, http_client);
    let mut saved_stories: Vec<Story> = Vec::new();

    for hn_story in &all_hn_stories {
        let mut story: Story = hn_story.clone().into();
        story.episode_date = episode.date.clone();
        crate::db::save_story(&db, &story)?;

        let url_key = story.url.as_deref().unwrap_or(&story.title);
        crate::db::mark_url_seen(&db, url_key)?;

        if let Some(saved) = crate::db::get_story_by_hn_id(&db, story.hn_id)? {
            if let Some(mut progress) = fetch_progress.get_mut(fetch_id) {
                progress.stories_added += 1;
                progress.stories.push(saved.clone());
            }
            saved_stories.push(saved);
        }
    }

    // Generate summaries in batches
    let concurrency = config.summary_concurrency;
    let mut stories_count = 0;

    for chunk in saved_stories.chunks(concurrency) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|story| {
                let db = db.clone();
                let llm_client = llm_client.clone();
                let lang = lang.to_string();
                let story_clone = story.clone();

                async move {
                    match llm_client
                        .summarize(&story_clone.title, story_clone.url.as_deref(), &lang)
                        .await
                    {
                        Ok((summary, summary_zh)) => {
                            let _ = crate::db::update_story_summary(
                                &db,
                                &story_clone.episode_date,
                                story_clone.hn_id,
                                summary.as_deref(),
                                summary_zh.as_deref(),
                            );
                            Ok(story_clone.hn_id)
                        }
                        Err(e) => {
                            tracing::error!("Failed to summarize {}: {}", story_clone.hn_id, e);
                            Err(e)
                        }
                    }
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let error_count = results.iter().filter(|r| r.is_err()).count();
        stories_count += success_count;

        if let Some(mut progress) = fetch_progress.get_mut(fetch_id) {
            progress.summaries_done += success_count;
            progress.summaries_error += error_count;
        }
    }

    // Refresh stories list from DB
    if let Some(mut progress) = fetch_progress.get_mut(fetch_id) {
        progress.stories = crate::db::get_stories_by_episode(&db, &episode.date).unwrap_or_default();
        progress.finished = true;
    }

    tracing::info!("Fetch completed: {} stories processed", stories_count);
    Ok(())
}
