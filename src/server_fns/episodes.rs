use crate::models::{Episode, EpisodeWithStories};
use leptos::prelude::*;
use server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
use super::app_state;

#[server]
pub async fn get_latest_episode() -> Result<Option<EpisodeWithStories>, ServerFnError> {
    let state = app_state()?;
    let result =
        crate::db::get_latest_episode(&state.db).map_err(|e| ServerFnError::new(e.to_string()))?;

    match result {
        Some(episode) => {
            let stories = crate::db::get_stories_by_episode(&state.db, &episode.date)
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            Ok(Some(EpisodeWithStories { episode, stories }))
        }
        None => Ok(None),
    }
}

#[server]
pub async fn get_episode_by_date(
    date: String,
) -> Result<Option<EpisodeWithStories>, ServerFnError> {
    let state = app_state()?;
    crate::db::get_episode_with_stories(&state.db, &date)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn get_episodes() -> Result<Vec<Episode>, ServerFnError> {
    let state = app_state()?;
    crate::db::get_episodes(&state.db).map_err(|e| ServerFnError::new(e.to_string()))
}
