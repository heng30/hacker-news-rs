use leptos::prelude::*;
use server_fn::error::ServerFnError;

use crate::models::Story;

#[cfg(feature = "ssr")]
use crate::state::AppState;

#[cfg(feature = "ssr")]
fn app_state() -> Result<std::sync::Arc<AppState>, ServerFnError> {
    use_context::<std::sync::Arc<AppState>>()
        .ok_or_else(|| ServerFnError::new("AppState not found"))
}

/// Fetch a single story by its HN ID (for incremental SSE updates)
#[server]
pub async fn get_story(hn_id: i64) -> Result<Option<Story>, ServerFnError> {
    let state = app_state()?;
    crate::db::get_story_by_hn_id(&state.db, hn_id)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn regenerate_summary(hn_id: i64) -> Result<Story, ServerFnError> {
    let state = app_state()?;

    if state.config.api_key.is_empty() {
        return Err(ServerFnError::new("API key not configured"));
    }

    let story = crate::db::get_story_by_hn_id(&state.db, hn_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("Story not found"))?;

    let llm_client = crate::llm::LlmClient::new(state.config.clone(), state.http_client.clone());

    tracing::info!("Regenerating summary for story {}: {}", story.hn_id, story.title);
    let summary = llm_client
        .summarize(&story.title, story.url.as_deref())
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to regenerate summary: {e}")))?;

    let updated = crate::db::update_story_summary(
        &state.db,
        &story.episode_date,
        story.hn_id,
        summary.as_deref(),
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(updated)
}

#[server]
pub async fn delete_all_stories() -> Result<usize, ServerFnError> {
    let state = app_state()?;
    crate::db::delete_all_stories(&state.db)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn delete_read_stories(hn_ids: Vec<i64>) -> Result<usize, ServerFnError> {
    let state = app_state()?;
    crate::db::delete_stories_by_hn_ids(&state.db, &hn_ids)
        .map_err(|e| ServerFnError::new(e.to_string()))
}
