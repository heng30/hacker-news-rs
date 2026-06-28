use crate::models::{Story, UserPreferences};
use leptos::prelude::*;
use server_fn::error::ServerFnError;
use std::collections::HashSet;

#[cfg(feature = "ssr")]
use super::app_state;

#[server]
pub async fn get_user_preferences() -> Result<UserPreferences, ServerFnError> {
    let state = app_state()?;
    let theme = crate::db::get_theme(&state.db).map_err(|e| ServerFnError::new(e.to_string()))?;
    let show_unread =
        crate::db::get_show_unread(&state.db).map_err(|e| ServerFnError::new(e.to_string()))?;
    let read_stories =
        crate::db::get_read_stories(&state.db).map_err(|e| ServerFnError::new(e.to_string()))?;
    let favorite_stories = crate::db::get_favorite_stories(&state.db)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(UserPreferences {
        theme,
        show_unread,
        read_stories,
        favorite_stories,
    })
}

#[server]
pub async fn set_theme(theme: String) -> Result<(), ServerFnError> {
    let state = app_state()?;
    crate::db::set_theme(&state.db, &theme).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn set_show_unread(show: bool) -> Result<(), ServerFnError> {
    let state = app_state()?;
    crate::db::set_show_unread(&state.db, show).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn mark_story_read(hn_id: i64) -> Result<HashSet<i64>, ServerFnError> {
    let state = app_state()?;
    crate::db::mark_story_read(&state.db, hn_id).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn set_read_stories(reads: HashSet<i64>) -> Result<(), ServerFnError> {
    let state = app_state()?;
    crate::db::set_read_stories(&state.db, &reads).map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn toggle_story_favorite(hn_id: i64) -> Result<HashSet<i64>, ServerFnError> {
    let state = app_state()?;
    let favs = crate::db::get_favorite_stories(&state.db)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    if favs.contains(&hn_id) {
        crate::db::unmark_story_favorite(&state.db, hn_id)
            .map_err(|e| ServerFnError::new(e.to_string()))
    } else {
        crate::db::mark_story_favorite(&state.db, hn_id)
            .map_err(|e| ServerFnError::new(e.to_string()))
    }
}

#[server]
pub async fn get_favorite_stories() -> Result<Vec<Story>, ServerFnError> {
    let state = app_state()?;
    crate::db::get_favorite_stories_with_details(&state.db)
        .map_err(|e| ServerFnError::new(e.to_string()))
}
