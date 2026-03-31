mod llm;
mod routes;

use anyhow::Result;
use axum::Router;
use hacker_news_rs::{config::AppConfig, db::init_db};
use routes::{AppState, config::config_routes, episode::episode_routes};
use sqlx::SqlitePool;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_config = AppConfig::from_env();
    tracing::info!("Starting server on port {}", app_config.port);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&app_config.database_url)
        .await?;

    init_db(&pool).await?;
    tracing::info!("Database initialized");

    let lang = Arc::new(RwLock::new("zh".to_string()));
    background_update_thread(pool.clone(), lang.clone());

    let state = Arc::new(AppState { pool, lang });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(episode_routes())
        .merge(config_routes())
        .fallback_service(ServeDir::new("src/static"))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", app_config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

fn background_update_thread(pool: SqlitePool, lang: Arc<RwLock<String>>) {
    let interval_minutes = hacker_news_rs::config::get_auto_update_interval_from_env();
    if interval_minutes > 0 {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_secs(interval_minutes as u64 * 60));
            tracing::info!(
                "Auto-update background task started, interval: {} minutes",
                interval_minutes
            );

            loop {
                interval.tick().await;
                tracing::info!("Auto-update triggered");
                match routes::episode::fetch_stories_background(pool.clone(), lang.clone()).await {
                    Ok(count) => {
                        tracing::info!("Auto-update completed: {} stories processed", count)
                    }
                    Err(e) => tracing::error!("Auto-update failed: {}", e),
                }
            }
        });
    }
}
