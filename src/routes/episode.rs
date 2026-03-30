use crate::{
    api::HnClient,
    config::{
        get_llm_config_from_env, get_llm_no_stream_from_env, get_llm_timeout,
        get_llm_user_agent_from_env, get_search_keywords_from_env,
    },
    db::{self, EpisodeWithStories},
    llm::LlmClient,
};
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        Json,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, post, put},
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{collections::HashSet, sync::Arc, sync::RwLock};
use tokio_stream::StreamExt as _;

const MAX_TOP_STORIES_PER_FETCH: usize = 20;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub lang: Arc<RwLock<String>>,
}

#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

pub fn episode_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/episode/latest", get(get_latest_episode))
        .route("/api/episode/{date}", get(get_episode_by_date))
        .route("/api/episode/{date}", delete(delete_episode_by_date))
        .route(
            "/api/episode/{date}/stories",
            delete(delete_episode_stories),
        )
        .route("/api/fetch", post(fetch_stories))
        .route("/api/fetch/stream", get(fetch_stories_stream))
        .route("/api/stories", get(get_all_stories))
        .route("/api/stories", delete(delete_all_stories))
        .route("/api/stories/read", delete(delete_read_stories))
        .route(
            "/api/story/{hn_id}/regenerate",
            put(regenerate_story_summary),
        )
        .route("/api/episodes", get(get_episodes_list))
}

async fn get_latest_episode(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<ApiResponse<EpisodeWithStories>>,
    (StatusCode, Json<ApiResponse<EpisodeWithStories>>),
> {
    match db::get_latest_episode(&state.pool).await {
        Ok(Some(episode)) => {
            let stories = db::get_stories_by_episode(&state.pool, episode.id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?;
            Ok(Json(ApiResponse::success(EpisodeWithStories {
                episode,
                stories,
            })))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("No episodes found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )),
    }
}

async fn get_episode_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<
    Json<ApiResponse<EpisodeWithStories>>,
    (StatusCode, Json<ApiResponse<EpisodeWithStories>>),
> {
    match db::get_episode_by_date(&state.pool, &date).await {
        Ok(Some(episode)) => {
            let stories = db::get_stories_by_episode(&state.pool, episode.id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?;
            Ok(Json(ApiResponse::success(EpisodeWithStories {
                episode,
                stories,
            })))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Episode not found")),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )),
    }
}

#[derive(Deserialize)]
struct FetchRequest {
    lang: Option<String>,
}

#[derive(Serialize)]
struct FetchResponse {
    episode: crate::db::Episode,
    stories_count: usize,
}

async fn fetch_stories(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<FetchRequest>,
) -> Result<Json<ApiResponse<FetchResponse>>, (StatusCode, Json<ApiResponse<FetchResponse>>)> {
    let config = get_llm_config_from_env();

    if config.api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("API key not configured")),
        ));
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let episode_id = db::create_episode(&state.pool, &today).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    let existing_hn_ids = db::get_existing_hn_ids(&state.pool, episode_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;
    let existing_ids_set: HashSet<i64> = existing_hn_ids.into_iter().collect();
    tracing::info!(
        "Found {} existing stories for episode {}",
        existing_ids_set.len(),
        episode_id
    );

    let hn_client = HnClient::new();
    let top_ids = hn_client.fetch_top_stories().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "Failed to fetch from HN: {}",
                e
            ))),
        )
    })?;

    let ids: Vec<i64> = top_ids
        .into_iter()
        .filter(|id| !existing_ids_set.contains(id))
        .take(MAX_TOP_STORIES_PER_FETCH)
        .collect();

    tracing::info!(
        "Fetching {} new top stories (filtered out {} duplicates)",
        ids.len(),
        existing_ids_set.len()
    );

    // Fetch top stories details (tag="top")
    let top_stories = if !ids.is_empty() {
        hn_client.fetch_stories(&ids).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&format!(
                    "Failed to fetch stories: {}",
                    e
                ))),
            )
        })?
    } else {
        Vec::new()
    };

    // Fetch keyword search results from Algolia
    let keywords = get_search_keywords_from_env();
    let search_stories = if let Some(kws) = keywords {
        tracing::info!("Searching for keywords: {:?}", kws);
        let mut stories = Vec::new();
        for kw in kws {
            match hn_client.search_newest(&kw, 10).await {
                Ok(s) => {
                    let filtered: Vec<_> = s
                        .into_iter()
                        .filter(|s| !existing_ids_set.contains(&s.id))
                        .collect();
                    tracing::info!(
                        "Found {} stories for keyword '{}' ({} new)",
                        filtered.len(),
                        kw,
                        filtered.len()
                    );
                    stories.extend(filtered);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to search Algolia for '{}': {}. Skipping this keyword.",
                        kw,
                        e
                    );
                    continue;
                }
            }
        }
        stories
    } else {
        Vec::new()
    };

    let mut all_stories: Vec<crate::db::HnStory> = top_stories;
    let top_ids_set: HashSet<i64> = all_stories.iter().map(|s| s.id).collect();
    for story in search_stories {
        if !top_ids_set.contains(&story.id) {
            all_stories.push(story);
        }
    }

    tracing::info!(
        "Total {} unique stories to process ({} top + {} search unique)",
        all_stories.len(),
        top_ids_set.len(),
        all_stories.len() - top_ids_set.len()
    );

    if all_stories.is_empty() {
        let episode = db::get_episode_by_date(&state.pool, &today)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&e.to_string())),
                )
            })?
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("Failed to get created episode")),
                )
            })?;

        return Ok(Json(ApiResponse::success(FetchResponse {
            episode,
            stories_count: 0,
        })));
    }

    tracing::info!(
        "Initializing LLM client with base_url: {}, model: {}",
        config.api_base_url,
        config.model
    );
    let llm_client = LlmClient::new(
        config.api_key.clone(),
        config.api_base_url.clone(),
        config.model.clone(),
        get_llm_no_stream_from_env(),
        get_llm_user_agent_from_env(),
        get_llm_timeout(),
    );

    let lang = payload.lang.as_deref().unwrap_or("zh");
    let mut stories_count = 0;
    let mut first_error: Option<String> = None;

    for hn_story in all_stories {
        let mut story: crate::db::Story = hn_story.into();
        story.episode_id = episode_id;

        tracing::info!(
            "Generating {} summary for story {}: {}",
            lang,
            story.hn_id,
            story.title
        );
        match llm_client
            .summarize(&story.title, story.url.as_deref(), lang)
            .await
        {
            Ok((summary, summary_zh)) => {
                tracing::info!("Successfully generated summary for story {}", story.hn_id);
                story.summary = summary;
                story.summary_zh = summary_zh;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to summarize story {} (title: '{}', url: {:?}): {}",
                    story.hn_id,
                    story.title,
                    story.url,
                    e
                );

                if first_error.is_none() {
                    first_error = Some(e.to_string());
                }
                if lang == "en" {
                    story.summary = Some("Failed to generate summary".to_string());
                } else {
                    story.summary_zh = Some("生成摘要失败".to_string());
                }
            }
        }

        if let Err(e) = db::save_story(&state.pool, &story).await {
            tracing::error!("Failed to save story {}: {}", story.hn_id, e);
            continue;
        }
        stories_count += 1;
    }

    if stories_count == 0 && first_error.is_some() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!(
                "LLM API error: {}",
                first_error.unwrap()
            ))),
        ));
    }

    let episode = db::get_episode_by_date(&state.pool, &today)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to get created episode")),
            )
        })?;

    Ok(Json(ApiResponse::success(FetchResponse {
        episode,
        stories_count,
    })))
}

async fn get_all_stories(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<ApiResponse<Vec<crate::db::Story>>>,
    (StatusCode, Json<ApiResponse<Vec<crate::db::Story>>>),
> {
    let stories = db::get_all_stories(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;
    Ok(Json(ApiResponse::success(stories)))
}

async fn get_episodes_list(
    State(state): State<Arc<AppState>>,
) -> Result<
    Json<ApiResponse<Vec<crate::db::Episode>>>,
    (StatusCode, Json<ApiResponse<Vec<crate::db::Episode>>>),
> {
    let episodes = db::get_episodes(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;
    Ok(Json(ApiResponse::success(episodes)))
}

#[derive(Serialize)]
struct DeleteResponse {
    deleted_count: usize,
}

async fn delete_all_stories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    let deleted_count = db::delete_all_stories(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;
    Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
}

async fn delete_episode_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    let deleted_count = db::delete_episode_by_date(&state.pool, &date)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;
    Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
}

async fn delete_episode_stories(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    let episode = db::get_episode_by_date(&state.pool, &date)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;

    match episode {
        Some(ep) => {
            let deleted_count = db::delete_stories_by_episode(&state.pool, ep.id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?;
            Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Episode not found")),
        )),
    }
}

#[derive(Serialize)]
struct RegenerateResponse {
    story: crate::db::Story,
}

#[derive(Deserialize)]
struct RegenerateRequest {
    lang: Option<String>,
}

async fn regenerate_story_summary(
    State(state): State<Arc<AppState>>,
    Path(hn_id): Path<i64>,
    Json(payload): Json<RegenerateRequest>,
) -> Result<
    Json<ApiResponse<RegenerateResponse>>,
    (StatusCode, Json<ApiResponse<RegenerateResponse>>),
> {
    let config = get_llm_config_from_env();

    if config.api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("API key not configured")),
        ));
    }

    let story = db::get_story_by_hn_id(&state.pool, hn_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;

    let lang = payload.lang.as_deref().unwrap_or("zh");

    match story {
        Some(s) => {
            let llm_client = LlmClient::new(
                config.api_key.clone(),
                config.api_base_url.clone(),
                config.model.clone(),
                get_llm_no_stream_from_env(),
                get_llm_user_agent_from_env(),
                get_llm_timeout(),
            );

            tracing::info!(
                "Regenerating {} summary for story {}: {}",
                lang,
                s.hn_id,
                s.title
            );
            let (summary, summary_zh) = llm_client
                .summarize(&s.title, s.url.as_deref(), lang)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&format!(
                            "Failed to regenerate summary: {e}"
                        ))),
                    )
                })?;

            if lang == "en" {
                db::update_story_summary_by_lang(
                    &state.pool,
                    s.id,
                    lang,
                    &summary.unwrap_or_default(),
                )
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?;
            } else {
                db::update_story_summary_by_lang(
                    &state.pool,
                    s.id,
                    lang,
                    &summary_zh.unwrap_or_default(),
                )
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?;
            }

            let updated_story = db::get_story_by_hn_id(&state.pool, hn_id)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&e.to_string())),
                    )
                })?
                .unwrap();

            Ok(Json(ApiResponse::success(RegenerateResponse {
                story: updated_story,
            })))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("Story not found")),
        )),
    }
}

#[derive(Deserialize)]
struct DeleteReadRequest {
    hn_ids: Vec<i64>,
}

async fn delete_read_stories(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeleteReadRequest>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    if payload.hn_ids.is_empty() {
        return Ok(Json(ApiResponse::success(DeleteResponse {
            deleted_count: 0,
        })));
    }

    let deleted_count = db::delete_stories_by_hn_ids(&state.pool, &payload.hn_ids)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;

    Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum SseEvent {
    #[serde(rename = "story_added")]
    StoryAdded { story: crate::db::Story },
    #[serde(rename = "summary_done")]
    SummaryDone {
        hn_id: i64,
        summary: Option<String>,
        summary_zh: Option<String>,
    },
    #[serde(rename = "summary_error")]
    SummaryError { hn_id: i64, error: String },
    #[serde(rename = "done")]
    Done { stories_count: usize },
}

#[derive(Deserialize)]
struct StreamQuery {
    lang: Option<String>,
}

async fn fetch_stories_stream(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StreamQuery>,
) -> Sse<impl Stream<Item = Result<Event, anyhow::Error>>> {
    let pool = state.pool.clone();
    let lang = query.lang.unwrap_or_else(|| "zh".to_string());

    let (tx, rx) = tokio::sync::mpsc::channel::<SseEvent>(100);

    tokio::spawn(async move {
        if let Err(e) = fetch_stories_stream_task(pool, tx, &lang).await {
            tracing::error!("Error in fetch_stories_stream_task: {}", e);
        }
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|event| {
        let json = serde_json::to_string(&event)?;
        Ok(Event::default().data(json))
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn fetch_stories_background(
    pool: SqlitePool,
    lang: Arc<RwLock<String>>,
) -> anyhow::Result<usize> {
    let lang_value = lang.read().unwrap().clone();
    let config = get_llm_config_from_env();
    if config.api_key.is_empty() {
        tracing::warn!("Auto-update skipped: API key not configured");
        return Ok(0);
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let episode_id = db::create_episode(&pool, &today).await?;

    let existing_hn_ids = db::get_existing_hn_ids(&pool, episode_id).await?;
    let existing_ids_set: HashSet<i64> = existing_hn_ids.into_iter().collect();
    tracing::info!(
        "Auto-update: Found {} existing stories for episode {}",
        existing_ids_set.len(),
        episode_id
    );

    let hn_client = HnClient::new();
    let top_ids = hn_client.fetch_top_stories().await?;
    let ids: Vec<i64> = top_ids
        .into_iter()
        .filter(|id| !existing_ids_set.contains(id))
        .take(MAX_TOP_STORIES_PER_FETCH)
        .collect();

    tracing::info!(
        "Auto-update: Fetching {} new top stories (filtered out {} duplicates)",
        ids.len(),
        existing_ids_set.len()
    );

    let top_stories = if !ids.is_empty() {
        hn_client.fetch_stories(&ids).await?
    } else {
        Vec::new()
    };

    let keywords = crate::config::get_search_keywords_from_env();
    let search_stories = if let Some(kws) = keywords {
        tracing::info!("Auto-update: Searching for keywords: {:?}", kws);
        let mut stories = Vec::new();
        for kw in kws {
            match hn_client.search_newest(&kw, 10).await {
                Ok(s) => {
                    let filtered: Vec<_> = s
                        .into_iter()
                        .filter(|s| !existing_ids_set.contains(&s.id))
                        .collect();
                    tracing::info!(
                        "Auto-update: Found {} stories for keyword '{}'",
                        filtered.len(),
                        kw
                    );
                    stories.extend(filtered);
                }
                Err(e) => {
                    tracing::warn!(
                        "Auto-update: Failed to search Algolia for '{}': {}. Skipping.",
                        kw,
                        e
                    );
                    continue;
                }
            }
        }
        stories
    } else {
        Vec::new()
    };

    let mut all_hn_stories: Vec<crate::db::HnStory> = top_stories;
    let top_ids_set: HashSet<i64> = all_hn_stories.iter().map(|s| s.id).collect();
    for story in search_stories {
        if !top_ids_set.contains(&story.id) {
            all_hn_stories.push(story);
        }
    }

    tracing::info!(
        "Auto-update: Total {} unique stories to process",
        all_hn_stories.len()
    );

    if all_hn_stories.is_empty() {
        tracing::info!("Auto-update: No new stories to fetch");
        return Ok(0);
    }

    tracing::info!(
        "Auto-update: Initializing LLM client with base_url: {}, model: {}",
        config.api_base_url,
        config.model
    );
    let llm_client = LlmClient::new(
        config.api_key.clone(),
        config.api_base_url.clone(),
        config.model.clone(),
        get_llm_no_stream_from_env(),
        get_llm_user_agent_from_env(),
        get_llm_timeout(),
    );

    // Save stories without summaries first
    let mut saved_stories: Vec<(crate::db::Story, i64)> = Vec::new();
    for hn_story in all_hn_stories {
        let mut story: crate::db::Story = hn_story.into();
        story.episode_id = episode_id;
        db::save_story(&pool, &story).await?;

        let saved = db::get_story_by_hn_id(&pool, story.hn_id).await?;
        if let Some(saved_story) = saved {
            saved_stories.push((saved_story.clone(), saved_story.id));
        }
    }

    // Generate summaries in parallel with concurrency control
    let mut stories_count = 0;
    for chunk in saved_stories.chunks(3) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|(story, story_id)| {
                let pool = pool.clone();
                let llm_client = llm_client.clone();
                let lang = lang_value.clone();
                let story_clone = story.clone();
                let story_id_clone = *story_id;

                async move {
                    tracing::info!(
                        "Auto-update: Generating {} summary for story {}: {}",
                        lang,
                        story_clone.hn_id,
                        story_clone.title
                    );
                    match llm_client
                        .summarize(&story_clone.title, story_clone.url.as_deref(), &lang)
                        .await
                    {
                        Ok((summary, summary_zh)) => {
                            tracing::info!(
                                "Auto-update: Successfully generated summary for story {}",
                                story_clone.hn_id
                            );
                            if let Err(e) = db::update_story_summary(
                                &pool,
                                story_id_clone,
                                summary.as_deref(),
                                summary_zh.as_deref(),
                            )
                            .await
                            {
                                tracing::error!(
                                    "Auto-update: Failed to update summary for story {}: {}",
                                    story_clone.hn_id,
                                    e
                                );
                            }
                            true
                        }
                        Err(e) => {
                            tracing::error!(
                                "Auto-update: Failed to summarize story {}: {}",
                                story_clone.hn_id,
                                e
                            );
                            false
                        }
                    }
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        stories_count += results.iter().filter(|&r| *r).count();
    }

    tracing::info!(
        "Auto-update: Completed, {} stories processed",
        stories_count
    );
    Ok(stories_count)
}

async fn fetch_stories_stream_task(
    pool: SqlitePool,
    tx: tokio::sync::mpsc::Sender<SseEvent>,
    lang: &str,
) -> anyhow::Result<()> {
    let config = get_llm_config_from_env();
    if config.api_key.is_empty() {
        return Err(anyhow::anyhow!("API key not configured"));
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let episode_id = db::create_episode(&pool, &today).await?;

    let existing_hn_ids = db::get_existing_hn_ids(&pool, episode_id).await?;
    let existing_ids_set: HashSet<i64> = existing_hn_ids.into_iter().collect();
    tracing::info!(
        "Found {} existing stories for episode {}",
        existing_ids_set.len(),
        episode_id
    );

    let hn_client = HnClient::new();
    let top_ids = hn_client.fetch_top_stories().await?;
    let ids: Vec<i64> = top_ids
        .into_iter()
        .filter(|id| !existing_ids_set.contains(id))
        .take(MAX_TOP_STORIES_PER_FETCH)
        .collect();

    tracing::info!(
        "Fetching {} new top stories (filtered out {} duplicates)",
        ids.len(),
        existing_ids_set.len()
    );

    // Fetch top stories details (tag="top")
    let top_stories = if !ids.is_empty() {
        hn_client.fetch_stories(&ids).await?
    } else {
        Vec::new()
    };

    // Fetch keyword search results from Algolia
    let keywords = crate::config::get_search_keywords_from_env();
    let search_stories = if let Some(kws) = keywords {
        tracing::info!("Searching for keywords: {:?}", kws);
        let mut stories = Vec::new();
        for kw in kws {
            match hn_client.search_newest(&kw, 10).await {
                Ok(s) => {
                    let filtered: Vec<_> = s
                        .into_iter()
                        .filter(|s| !existing_ids_set.contains(&s.id))
                        .collect();
                    tracing::info!(
                        "Found {} stories for keyword '{}' ({} new)",
                        filtered.len(),
                        kw,
                        filtered.len()
                    );
                    stories.extend(filtered);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to search Algolia for '{}': {}. Skipping this keyword.",
                        kw,
                        e
                    );
                    continue;
                }
            }
        }
        stories
    } else {
        Vec::new()
    };

    // Merge top stories and search results, deduplicate by hn_id
    let mut all_hn_stories: Vec<crate::db::HnStory> = top_stories;
    let top_ids_set: HashSet<i64> = all_hn_stories.iter().map(|s| s.id).collect();
    for story in search_stories {
        if !top_ids_set.contains(&story.id) {
            all_hn_stories.push(story);
        }
    }

    tracing::info!("Total {} unique stories to process", all_hn_stories.len());

    if all_hn_stories.is_empty() {
        tx.send(SseEvent::Done { stories_count: 0 }).await?;
        return Ok(());
    }

    tracing::info!(
        "Initializing LLM client with base_url: {}, model: {}",
        config.api_base_url,
        config.model
    );
    let llm_client = LlmClient::new(
        config.api_key.clone(),
        config.api_base_url.clone(),
        config.model.clone(),
        get_llm_no_stream_from_env(),
        get_llm_user_agent_from_env(),
        get_llm_timeout(),
    );

    // Save stories without summaries first and send story_added events
    let mut saved_stories: Vec<(crate::db::Story, i64)> = Vec::new();
    for hn_story in all_hn_stories {
        let mut story: crate::db::Story = hn_story.into();
        story.episode_id = episode_id;
        db::save_story(&pool, &story).await?;

        let saved = db::get_story_by_hn_id(&pool, story.hn_id).await?;
        if let Some(saved_story) = saved {
            saved_stories.push((saved_story.clone(), saved_story.id));
            tx.send(SseEvent::StoryAdded { story: saved_story }).await?;
        }
    }

    // Generate summaries in parallel with concurrency control (3-5 per batch)
    let mut stories_count = 0;
    for chunk in saved_stories.chunks(3) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|(story, story_id)| {
                let pool = pool.clone();
                let tx = tx.clone();
                let llm_client = llm_client.clone();
                let lang = lang.to_string();
                let story_clone = story.clone();
                let story_id_clone = *story_id;

                async move {
                    tracing::info!(
                        "Generating {} summary for story {}: {}",
                        lang,
                        story_clone.hn_id,
                        story_clone.title
                    );
                    match llm_client
                        .summarize(&story_clone.title, story_clone.url.as_deref(), &lang)
                        .await
                    {
                        Ok((summary, summary_zh)) => {
                            tracing::info!(
                                "Successfully generated summary for story {}",
                                story_clone.hn_id
                            );
                            if let Err(e) = db::update_story_summary(
                                &pool,
                                story_id_clone,
                                summary.as_deref(),
                                summary_zh.as_deref(),
                            )
                            .await
                            {
                                tracing::error!(
                                    "Failed to update summary for story {}: {}",
                                    story_clone.hn_id,
                                    e
                                );
                            }
                            tx.send(SseEvent::SummaryDone {
                                hn_id: story_clone.hn_id,
                                summary,
                                summary_zh,
                            })
                            .await
                            .ok();
                            true
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to summarize story {}: {}",
                                story_clone.hn_id,
                                e
                            );
                            tx.send(SseEvent::SummaryError {
                                hn_id: story_clone.hn_id,
                                error: e.to_string(),
                            })
                            .await
                            .ok();
                            false
                        }
                    }
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        stories_count += results.iter().filter(|&r| *r).count();
    }

    tx.send(SseEvent::Done { stories_count }).await?;

    Ok(())
}
