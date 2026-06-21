#[cfg(feature = "ssr")]
use std::{
    net::{IpAddr, SocketAddr},
    process,
    sync::Arc,
    time::Duration,
};

#[cfg(feature = "ssr")]
use axum::Router;
#[cfg(feature = "ssr")]
use axum::routing::post;
#[cfg(feature = "ssr")]
use clap::Parser;
#[cfg(feature = "ssr")]
use hns::config::AppConfig;
#[cfg(feature = "ssr")]
use hns::config::Args;
#[cfg(feature = "ssr")]
use hns::db;
#[cfg(feature = "ssr")]
use hns::state::AppState;
#[cfg(feature = "ssr")]
use leptos::{config::LeptosOptions, prelude::*};
#[cfg(feature = "ssr")]
use leptos_axum::{LeptosRoutes, generate_route_list};

/// Wrapper state that holds both LeptosOptions and AppState
#[cfg(feature = "ssr")]
#[derive(Clone)]
struct AppStateWrapper {
    leptos_options: LeptosOptions,
    app_state: Arc<AppState>,
}

#[cfg(feature = "ssr")]
impl axum::extract::FromRef<AppStateWrapper> for LeptosOptions {
    fn from_ref(state: &AppStateWrapper) -> Self {
        state.leptos_options.clone()
    }
}

#[cfg(feature = "ssr")]
impl axum::extract::FromRef<AppStateWrapper> for Arc<AppState> {
    fn from_ref(state: &AppStateWrapper) -> Self {
        state.app_state.clone()
    }
}

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let addr = SocketAddr::from((
        args.host
            .parse::<IpAddr>()
            .unwrap_or_else(|e| {
                eprintln!("Invalid host address: {}", e);
                process::exit(1);
            }),
        args.port,
    ));

    // Resolve database path
    let db_path = AppConfig::resolve_db_path(&args.db, "hns");

    // Ensure database directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            eprintln!("Failed to create database directory: {}", e);
            process::exit(1);
        });
    }

    // Build config from args
    let config = AppConfig::from_args(&args, db_path);

    tracing::info!(
        "Configuration: host={}, port={}, model={}, api_base_url={}",
        config.host,
        config.port,
        config.model,
        config.api_base_url
    );

    // Install ring as the rustls crypto provider before creating any HTTP client
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls ring crypto provider");

    // Open sled database
    let sled_db = sled::open(&config.db_path).unwrap_or_else(|e| {
        eprintln!("Failed to open database {}: {}", config.db_path, e);
        process::exit(1);
    });
    db::init_trees(&sled_db).expect("Failed to initialize database trees");

    // Build HTTP client with optional SOCKS5 proxy
    let mut client_builder = reqwest::Client::builder()
        .user_agent(hns::fetcher::USER_AGENT);

    if let Some(proxy) = &config.socks5_proxy {
        let proxy_url = if proxy.starts_with("socks5://") || proxy.starts_with("socks5h://") {
            proxy.clone()
        } else {
            format!("socks5://{}", proxy)
        };
        client_builder = client_builder
            .proxy(reqwest::Proxy::all(&proxy_url).expect("Failed to create SOCKS5 proxy"));
        tracing::info!("Using SOCKS5 proxy: {}", proxy_url);
    }

    let http_client = client_builder
        .build()
        .expect("Failed to create HTTP client");

    let app_state = Arc::new(AppState {
        db: Arc::new(sled_db),
        config: Arc::new(config.clone()),
        http_client: http_client.clone(),
        fetch_progress: Arc::new(dashmap::DashMap::new()),
    });

    // Background auto-update thread
    if config.auto_update_interval > 0 {
        let interval_minutes = config.auto_update_interval;
        let state = app_state.clone();
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

                match background_fetch(state.clone()).await {
                    Ok(count) => {
                        tracing::info!("Auto-update completed: {} stories processed", count)
                    }
                    Err(e) => tracing::error!("Auto-update failed: {}", e),
                }
            }
        });
    }

    let leptos_options = LeptosOptions::builder()
        .site_addr(addr)
        .output_name("hns".to_string())
        .site_root("target/site".to_string())
        .site_pkg_dir("pkg".to_string())
        .build();

    let wrapper = AppStateWrapper {
        leptos_options: leptos_options.clone(),
        app_state: app_state.clone(),
    };

    let routes = generate_route_list(hns::app::App);

    let app = Router::new()
        .route(
            "/api/{*fn_name}",
            post({
                let app_state = app_state.clone();
                move |req| leptos_axum::handle_server_fns_with_context(
                    {
                        let app_state = app_state.clone();
                        move || {
                            provide_context(app_state.clone());
                        }
                    },
                    req,
                )
            }),
        )
        .leptos_routes_with_context(
            &wrapper,
            routes,
            {
                let app_state = app_state.clone();
                move || {
                    provide_context(app_state.clone());
                }
            },
            {
                let leptos_options = leptos_options.clone();
                move || hns::shell::shell(leptos_options.clone())
            },
        )
        .fallback(hns::static_files::static_handler)
        .with_state(wrapper);

    tracing::info!("Hacker News RS starting on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

/// Background fetch for auto-update
#[cfg(feature = "ssr")]
async fn background_fetch(state: Arc<AppState>) -> anyhow::Result<usize> {
    if state.config.api_key.is_empty() {
        tracing::warn!("Auto-update skipped: API key not configured");
        return Ok(0);
    }

    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _episode = db::create_episode(&state.db, &today)?;

    let hn_client = hns::api::HnClient::new(state.http_client.clone());
    let top_ids = hn_client.fetch_top_stories().await?;
    let all_top_stories = hn_client.fetch_stories(&top_ids).await?;

    // Filter by score and dedup
    let min_score = state.config.top_story_min_score;
    let stories: Vec<hns::models::HnStory> = all_top_stories
        .into_iter()
        .filter(|s| {
            let url_key = s.url.as_deref().unwrap_or(&s.title);
            !db::is_url_seen(&state.db, url_key).unwrap_or(false)
        })
        .filter(|s| s.score >= min_score)
        .collect();

    tracing::info!("Auto-update: {} new stories to process", stories.len());

    if stories.is_empty() {
        return Ok(0);
    }

    // Save and summarize
    let llm_client = hns::llm::LlmClient::new(state.config.clone(), state.http_client.clone());
    let mut saved: Vec<hns::models::Story> = Vec::new();

    for hn_story in stories {
        let mut story: hns::models::Story = hn_story.into();
        story.episode_date = today.clone();
        db::save_story(&state.db, &story)?;

        if let Some(url) = &story.url {
            let _ = db::mark_url_seen(&state.db, url);
        }

        if let Some(s) = db::get_story_by_hn_id(&state.db, story.hn_id)? {
            saved.push(s);
        }
    }

    let concurrency = state.config.summary_concurrency;
    let mut count = 0;

    for chunk in saved.chunks(concurrency) {
        let futures: Vec<_> = chunk
            .iter()
            .map(|story| {
                let db = state.db.clone();
                let llm = llm_client.clone();
                let story_clone = story.clone();

                async move {
                    match llm
                        .summarize(&story_clone.title, story_clone.url.as_deref())
                        .await
                    {
                        Ok(summary) => {
                            let _ = db::update_story_summary(
                                &db,
                                &story_clone.episode_date,
                                story_clone.hn_id,
                                summary.as_deref(),
                            );
                            true
                        }
                        Err(e) => {
                            tracing::error!(
                                "Auto-update: Failed to summarize {}: {}",
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
        count += results.iter().filter(|&&r| r).count();
    }

    Ok(count)
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // no client-side main function
    // see lib.rs for hydration function instead
}
