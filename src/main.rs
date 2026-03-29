mod config;
mod db;
mod fetcher;
mod hn;
mod llm;
mod routes;

use axum::Router;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::AppConfig;
use db::init_db;
use routes::AppState;
use routes::episode::episode_routes;
use routes::config::config_routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file first (before anything else)
    // This ensures environment variables are available for all configuration
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let app_config = AppConfig::from_env();
    tracing::info!("Starting server on port {}", app_config.port);

    // Setup database connection
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&app_config.database_url)
        .await?;

    // Initialize database schema
    init_db(&pool).await?;
    tracing::info!("Database initialized");

    // Create app state
    let state = Arc::new(AppState { pool });

    // Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Setup routes
    let app = Router::new()
        .merge(episode_routes())
        .merge(config_routes())
        .fallback_service(ServeDir::new("src/static"))
        .layer(cors)
        .with_state(state);

    // Start server
    let addr = format!("0.0.0.0:{}", app_config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}