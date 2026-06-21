use std::sync::Arc;

use sled::Db;
use tracing::{debug, info};

use crate::error::AppError;
use crate::models::{Episode, EpisodeWithStories, Story};

/// Key prefix for episodes tree
const EPISODES_TREE: &str = "episodes";
/// Key prefix for stories tree
const STORIES_TREE: &str = "stories";
/// Key prefix for URL hashes tree (dedup)
const URL_HASHES_TREE: &str = "url_hashes";

/// Initialize sled database trees
pub fn init_trees(db: &Db) -> Result<(), AppError> {
    // Open trees to ensure they exist
    db.open_tree(EPISODES_TREE)?;
    db.open_tree(STORIES_TREE)?;
    db.open_tree(URL_HASHES_TREE)?;
    db.flush()?;
    Ok(())
}

// ── Episode operations ──────────────────────────────────────────────

/// Create a new episode for the given date, or return existing one
pub fn create_episode(db: &Arc<Db>, date: &str) -> Result<Episode, AppError> {
    let tree = db.open_tree(EPISODES_TREE)?;

    if let Some(data) = tree.get(date)? {
        let episode: Episode = serde_json::from_slice(&data)?;
        debug!("Episode already exists for date: {}", date);
        return Ok(episode);
    }

    let now = chrono::Utc::now().to_rfc3339();
    let episode = Episode {
        date: date.to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    let data = serde_json::to_vec(&episode)?;
    tree.insert(date, data.as_slice())?;
    tree.flush()?;

    info!("Created episode for date: {}", date);
    Ok(episode)
}

/// Get an episode by date
pub fn get_episode_by_date(db: &Arc<Db>, date: &str) -> Result<Option<Episode>, AppError> {
    let tree = db.open_tree(EPISODES_TREE)?;

    if let Some(data) = tree.get(date)? {
        let episode: Episode = serde_json::from_slice(&data)?;
        Ok(Some(episode))
    } else {
        Ok(None)
    }
}

/// Get the latest episode
pub fn get_latest_episode(db: &Arc<Db>) -> Result<Option<Episode>, AppError> {
    let tree = db.open_tree(EPISODES_TREE)?;

    let mut latest: Option<Episode> = None;
    for item in tree.iter().rev() {
        let (_, data) = item?;
        let episode: Episode = serde_json::from_slice(&data)?;
        latest = Some(episode);
        break;
    }

    Ok(latest)
}

/// Get all episodes sorted by date (newest first)
pub fn get_episodes(db: &Arc<Db>) -> Result<Vec<Episode>, AppError> {
    let tree = db.open_tree(EPISODES_TREE)?;

    let mut episodes: Vec<Episode> = Vec::new();
    for item in tree.iter() {
        let (_, data) = item?;
        let episode: Episode = serde_json::from_slice(&data)?;
        episodes.push(episode);
    }

    // Sort by date descending
    episodes.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(episodes)
}

// ── Story operations ────────────────────────────────────────────────

/// Generate a composite key for a story in the stories tree
fn story_key(episode_date: &str, hn_id: i64) -> String {
    format!("{}:{}", episode_date, hn_id)
}

/// Save a story to the database
pub fn save_story(db: &Arc<Db>, story: &Story) -> Result<(), AppError> {
    let tree = db.open_tree(STORIES_TREE)?;
    let key = story_key(&story.episode_date, story.hn_id);
    let data = serde_json::to_vec(story)?;
    tree.insert(key.as_bytes(), data.as_slice())?;
    tree.flush()?;

    debug!("Saved story: {} (hn_id={})", story.title, story.hn_id);
    Ok(())
}

/// Get all stories for an episode
pub fn get_stories_by_episode(db: &Arc<Db>, episode_date: &str) -> Result<Vec<Story>, AppError> {
    let tree = db.open_tree(STORIES_TREE)?;
    let prefix = format!("{}:", episode_date);

    let mut stories: Vec<Story> = Vec::new();
    for item in tree.scan_prefix(prefix.as_bytes()) {
        let (_, data) = item?;
        let story: Story = serde_json::from_slice(&data)?;
        stories.push(story);
    }

    // Sort by score descending
    stories.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(stories)
}

/// Get a story by hn_id
pub fn get_story_by_hn_id(db: &Arc<Db>, hn_id: i64) -> Result<Option<Story>, AppError> {
    let tree = db.open_tree(STORIES_TREE)?;

    // Scan all stories to find by hn_id
    for item in tree.iter() {
        let (_, data) = item?;
        let story: Story = serde_json::from_slice(&data)?;
        if story.hn_id == hn_id {
            return Ok(Some(story));
        }
    }

    Ok(None)
}

/// Update a story's summary
pub fn update_story_summary(
    db: &Arc<Db>,
    episode_date: &str,
    hn_id: i64,
    summary: Option<&str>,
    summary_zh: Option<&str>,
) -> Result<Story, AppError> {
    let tree = db.open_tree(STORIES_TREE)?;
    let key = story_key(episode_date, hn_id);

    let mut story: Story = if let Some(data) = tree.get(&key)? {
        serde_json::from_slice(&data)?
    } else {
        return Err(AppError::NotFound(format!(
            "Story not found: hn_id={}",
            hn_id
        )));
    };

    if let Some(s) = summary {
        story.summary = Some(s.to_string());
    }
    if let Some(s) = summary_zh {
        story.summary_zh = Some(s.to_string());
    }

    let data = serde_json::to_vec(&story)?;
    tree.insert(key.as_bytes(), data.as_slice())?;
    tree.flush()?;

    Ok(story)
}

/// Delete all stories
pub fn delete_all_stories(db: &Arc<Db>) -> Result<usize, AppError> {
    let tree = db.open_tree(STORIES_TREE)?;
    let count = tree.len();
    tree.clear()?;
    tree.flush()?;

    // Also clear URL hashes
    let url_tree = db.open_tree(URL_HASHES_TREE)?;
    url_tree.clear()?;
    url_tree.flush()?;

    info!("Deleted all {} stories", count);
    Ok(count)
}

/// Delete stories by their hn_ids
pub fn delete_stories_by_hn_ids(db: &Arc<Db>, hn_ids: &[i64]) -> Result<usize, AppError> {
    let tree = db.open_tree(STORIES_TREE)?;
    let hn_id_set: std::collections::HashSet<i64> = hn_ids.iter().copied().collect();

    let mut deleted = 0;
    let mut keys_to_delete: Vec<Vec<u8>> = Vec::new();

    for item in tree.iter() {
        let (key, data) = item?;
        let story: Story = serde_json::from_slice(&data)?;
        if hn_id_set.contains(&story.hn_id) {
            keys_to_delete.push(key.to_vec());
            deleted += 1;
        }
    }

    for key in keys_to_delete {
        tree.remove(&key)?;
    }
    tree.flush()?;

    info!("Deleted {} stories by hn_ids", deleted);
    Ok(deleted)
}

/// Get an episode with its stories
pub fn get_episode_with_stories(db: &Arc<Db>, date: &str) -> Result<Option<EpisodeWithStories>, AppError> {
    let episode = get_episode_by_date(db, date)?;

    match episode {
        Some(episode) => {
            let stories = get_stories_by_episode(db, date)?;
            // Deduplicate stories by hn_id
            let mut seen = std::collections::HashSet::new();
            let stories = stories
                .into_iter()
                .filter(|s| {
                    let key = if s.hn_id != 0 {
                        format!("hn:{}", s.hn_id)
                    } else {
                        format!("url:{}", s.url.as_deref().unwrap_or(&s.title))
                    };
                    if seen.contains(&key) {
                        false
                    } else {
                        seen.insert(key);
                        true
                    }
                })
                .collect();

            Ok(Some(EpisodeWithStories { episode, stories }))
        }
        None => Ok(None),
    }
}

// ── URL hash operations ─────────────────────────────────────────────

/// Check if a URL has already been fetched
pub fn is_url_seen(db: &Arc<Db>, url: &str) -> Result<bool, AppError> {
    let tree = db.open_tree(URL_HASHES_TREE)?;
    let hash = blake3_hash(url);
    Ok(tree.contains_key(&hash)?)
}

/// Mark a URL as seen
pub fn mark_url_seen(db: &Arc<Db>, url: &str) -> Result<(), AppError> {
    let tree = db.open_tree(URL_HASHES_TREE)?;
    let hash = blake3_hash(url);
    tree.insert(&hash, b"1")?;
    tree.flush()?;
    Ok(())
}

/// Simple hash function for URLs
pub fn blake3_hash(url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
