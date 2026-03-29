use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::db::{self, models::EpisodeWithStories};
use crate::hn::api::HnClient;
use crate::llm::client::LlmClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
}

pub fn episode_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/episode/latest", get(get_latest_episode))
        .route("/api/episode/{date}", get(get_episode_by_date))
        .route("/api/episode/{date}", delete(delete_episode_by_date))
        .route("/api/episode/{date}/stories", delete(delete_episode_stories))
        .route("/api/fetch", post(fetch_stories))
        .route("/api/stories", get(get_all_stories))
        .route("/api/stories", delete(delete_all_stories))
        .route("/api/stories/read", delete(delete_read_stories))
        .route("/api/story/{hn_id}/regenerate", put(regenerate_story_summary))
        .route("/api/episodes", get(get_episodes_list))
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

async fn get_latest_episode(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<EpisodeWithStories>>, (StatusCode, Json<ApiResponse<EpisodeWithStories>>)> {
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
            Ok(Json(ApiResponse::success(EpisodeWithStories { episode, stories })))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("No episodes found")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(&e.to_string())))),
    }
}

async fn get_episode_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<Json<ApiResponse<EpisodeWithStories>>, (StatusCode, Json<ApiResponse<EpisodeWithStories>>)> {
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
            Ok(Json(ApiResponse::success(EpisodeWithStories { episode, stories })))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Episode not found")))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(&e.to_string())))),
    }
}

#[derive(Serialize)]
struct FetchResponse {
    episode: crate::db::models::Episode,
    stories_count: usize,
}

async fn fetch_stories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<FetchResponse>>, (StatusCode, Json<ApiResponse<FetchResponse>>)> {
    // Get configuration with environment variable overrides
    let config = db::get_config_with_env_overrides(&state.pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;

    if config.api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("API key not configured")),
        ));
    }

    // Get today's date
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Create or get episode
    let episode_id = db::create_episode(&state.pool, &today)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;

    // Get existing hn_ids for this episode to avoid duplicates
    let existing_hn_ids = db::get_existing_hn_ids(&state.pool, episode_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(&e.to_string())),
            )
        })?;
    let existing_ids_set: std::collections::HashSet<i64> = existing_hn_ids.into_iter().collect();
    tracing::info!("Found {} existing stories for episode {}", existing_ids_set.len(), episode_id);

    // Fetch top stories from HN
    let hn_client = HnClient::new();
    let top_ids = hn_client.fetch_top_stories().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!("Failed to fetch from HN: {}", e))),
        )
    })?;

    // Filter out already existing stories and take top N based on config
    let ids: Vec<i64> = top_ids
        .into_iter()
        .filter(|id| !existing_ids_set.contains(id))
        .take(config.story_count as usize)
        .collect();

    tracing::info!("Fetching {} new stories (filtered out {} duplicates)", ids.len(), existing_ids_set.len());

    if ids.is_empty() {
        // No new stories to fetch
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

    let hn_stories = hn_client.fetch_stories(&ids).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!("Failed to fetch stories: {}", e))),
        )
    })?;

    // Initialize LLM client
    tracing::info!("Initializing LLM client with base_url: {}, model: {}", config.api_base_url, config.model);
    let llm_client = LlmClient::new(config.api_key.clone(), config.api_base_url.clone(), config.model.clone());

    // Process each story
    let mut stories_count = 0;
    let mut first_error: Option<String> = None;
    for hn_story in hn_stories {
        let mut story: crate::db::models::Story = hn_story.into();
        story.episode_id = episode_id;

        // Generate summary and translate
        tracing::info!("Generating summary for story {}: {}", story.hn_id, story.title);
        match llm_client.summarize_and_translate(&story.title, story.url.as_deref()).await {
            Ok((summary, summary_zh)) => {
                tracing::info!("Successfully generated summary for story {}", story.hn_id);
                story.summary = Some(summary);
                story.summary_zh = Some(summary_zh);
            }
            Err(e) => {
                tracing::error!(
                    "Failed to summarize story {} (title: '{}', url: {:?}): {}",
                    story.hn_id,
                    story.title,
                    story.url,
                    e
                );
                // Record first error to return later
                if first_error.is_none() {
                    first_error = Some(e.to_string());
                }
                story.summary = Some("Failed to generate summary".to_string());
                story.summary_zh = Some("生成摘要失败".to_string());
            }
        }

        if let Err(e) = db::save_story(&state.pool, &story).await {
            tracing::error!("Failed to save story {}: {}", story.hn_id, e);
            continue;
        }
        stories_count += 1;
    }

    // If all stories failed due to LLM error, return error
    if stories_count == 0 && first_error.is_some() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&format!("LLM API error: {}", first_error.unwrap()))),
        ));
    }

    // Get the episode
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
) -> Result<Json<ApiResponse<Vec<crate::db::models::Story>>>, (StatusCode, Json<ApiResponse<Vec<crate::db::models::Story>>>)> {
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
) -> Result<Json<ApiResponse<Vec<crate::db::models::Episode>>>, (StatusCode, Json<ApiResponse<Vec<crate::db::models::Episode>>>)> {
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

/// Delete all stories from database
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

/// Delete an episode and all its stories by date
async fn delete_episode_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    let deleted_count = db::delete_episode_by_date(&state.pool, &date).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;
    Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
}

/// Delete all stories for a specific episode (but keep the episode)
async fn delete_episode_stories(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    let episode = db::get_episode_by_date(&state.pool, &date).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    match episode {
        Some(ep) => {
            let deleted_count = db::delete_stories_by_episode(&state.pool, ep.id).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&e.to_string())),
                )
            })?;
            Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
        }
        None => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Episode not found"))))
    }
}

#[derive(Serialize)]
struct RegenerateResponse {
    story: crate::db::models::Story,
}

/// Regenerate summary for a specific story
async fn regenerate_story_summary(
    State(state): State<Arc<AppState>>,
    Path(hn_id): Path<i64>,
) -> Result<Json<ApiResponse<RegenerateResponse>>, (StatusCode, Json<ApiResponse<RegenerateResponse>>)> {
    // Get configuration
    let config = db::get_config_with_env_overrides(&state.pool).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    if config.api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("API key not configured")),
        ));
    }

    // Get story by hn_id
    let story = db::get_story_by_hn_id(&state.pool, hn_id).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    match story {
        Some(s) => {
            // Initialize LLM client
            let llm_client = LlmClient::new(config.api_key.clone(), config.api_base_url.clone(), config.model.clone());

            // Regenerate summary
            tracing::info!("Regenerating summary for story {}: {}", s.hn_id, s.title);
            let (summary, summary_zh) = llm_client
                .summarize_and_translate(&s.title, s.url.as_deref())
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(&format!("Failed to regenerate summary: {}", e))),
                    )
                })?;

            // Update story in database
            db::update_story_summary(&state.pool, s.id, &summary, &summary_zh).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&e.to_string())),
                )
            })?;

            // Return updated story
            let updated_story = db::get_story_by_hn_id(&state.pool, hn_id).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(&e.to_string())),
                )
            })?.unwrap();

            Ok(Json(ApiResponse::success(RegenerateResponse { story: updated_story })))
        }
        None => Err((StatusCode::NOT_FOUND, Json(ApiResponse::error("Story not found"))))
    }
}

#[derive(Deserialize)]
struct DeleteReadRequest {
    hn_ids: Vec<i64>,
}

/// Delete stories by hn_ids (for removing read stories)
async fn delete_read_stories(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeleteReadRequest>,
) -> Result<Json<ApiResponse<DeleteResponse>>, (StatusCode, Json<ApiResponse<DeleteResponse>>)> {
    if payload.hn_ids.is_empty() {
        return Ok(Json(ApiResponse::success(DeleteResponse { deleted_count: 0 })));
    }

    let deleted_count = db::delete_stories_by_hn_ids(&state.pool, &payload.hn_ids).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(&e.to_string())),
        )
    })?;

    Ok(Json(ApiResponse::success(DeleteResponse { deleted_count })))
}