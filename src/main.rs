mod api;
mod config;
mod db;
mod fetcher;
mod llm;
mod routes;

use anyhow::Result;
use axum::Router;
use config::AppConfig;
use db::init_db;
use routes::AppState;
use routes::config::config_routes;
use routes::episode::episode_routes;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
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

    let state = Arc::new(AppState { pool });

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
