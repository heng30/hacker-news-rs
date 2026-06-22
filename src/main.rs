#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    ssr::main().await
}

#[cfg(not(feature = "ssr"))]
fn main() {
    // no client-side main function
    // see lib.rs for hydration function instead
}

#[cfg(feature = "ssr")]
mod ssr {
    use axum::{
        Router,
        response::sse::{Event, KeepAlive, Sse},
        routing::{get, post},
    };
    use clap::Parser;
    use futures::stream::Stream;
    use hns::{
        app::App,
        config::{AppConfig, Args},
        db,
        models::FetchEvent,
        server_fns::fetch::fetch_stories_task,
        shell::shell,
        state::AppState,
        static_files::static_handler,
    };
    use leptos::{config::LeptosOptions, prelude::*};
    use leptos_axum::{LeptosRoutes, generate_route_list};
    use std::{
        convert::Infallible,
        fs,
        net::{IpAddr, SocketAddr},
        path::Path,
        process,
        sync::Arc,
        time::Duration,
    };
    use tokio_stream::StreamExt;

    #[derive(Clone)]
    struct AppStateWrapper {
        leptos_options: LeptosOptions,
        app_state: Arc<AppState>,
    }

    impl axum::extract::FromRef<AppStateWrapper> for LeptosOptions {
        fn from_ref(state: &AppStateWrapper) -> Self {
            state.leptos_options.clone()
        }
    }

    impl axum::extract::FromRef<AppStateWrapper> for Arc<AppState> {
        fn from_ref(state: &AppStateWrapper) -> Self {
            state.app_state.clone()
        }
    }

    pub async fn main() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();

        let args = Args::parse();

        let addr = SocketAddr::from((
            args.host.parse::<IpAddr>().unwrap_or_else(|e| {
                tracing::warn!("Invalid host address: {}", e);
                process::exit(1);
            }),
            args.port,
        ));

        let db_path = AppConfig::resolve_db_path(&args.db, "hns");
        if let Some(parent) = Path::new(&db_path).parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                tracing::warn!("Failed to create database directory: {}", e);
                process::exit(1);
            });
        }

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

        let sled_db = sled::open(&config.db_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to open database {}: {}", config.db_path, e);
            process::exit(1);
        });
        db::init_trees(&sled_db).expect("Failed to initialize database trees");

        // Build HTTP client with optional SOCKS5 proxy
        let mut client_builder = reqwest::Client::builder().user_agent(hns::fetcher::USER_AGENT);

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

        // Create broadcast channel for SSE fetch events
        let (event_tx, _) = tokio::sync::broadcast::channel::<FetchEvent>(256);

        let app_state = Arc::new(AppState {
            db: Arc::new(sled_db),
            config: Arc::new(config.clone()),
            http_client: http_client.clone(),
            fetch_events: event_tx,
        });

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
                    tracing::debug!("Auto-update triggered");

                    match fetch_stories_task(
                        state.db.clone(),
                        state.config.clone(),
                        state.http_client.clone(),
                        state.fetch_events.clone(),
                    )
                    .await
                    {
                        Ok(()) => tracing::debug!("Auto-update completed"),
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

        let routes = generate_route_list(App);

        let app = Router::new()
            .route("/api/fetch-events", get(fetch_events_sse))
            .route(
                "/api/{*fn_name}",
                post({
                    let app_state = app_state.clone();
                    move |req| {
                        leptos_axum::handle_server_fns_with_context(
                            {
                                let app_state = app_state.clone();
                                move || {
                                    provide_context(app_state.clone());
                                }
                            },
                            req,
                        )
                    }
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
                    move || shell(leptos_options.clone())
                },
            )
            .fallback(static_handler)
            .with_state(wrapper);

        tracing::info!("Hacker News RS starting on http://{}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    }

    /// SSE endpoint: `/api/fetch-events`
    ///
    /// 客户端通过 EventSource 连接此端点，实时接收后台抓取事件（如新 story 到达、摘要完成等）。
    ///
    /// 工作流程：
    /// 1. 从 AppState 中的 broadcast channel 订阅接收端（rx）
    /// 2. 将 broadcast receiver 包装为 Stream，过滤掉因客户端消费慢导致的 lagged 错误
    /// 3. 将每个 FetchEvent 序列化为 JSON，封装为 SSE Event 推送给客户端
    /// 4. 启用 keep-alive 防止连接因空闲被中间代理/负载均衡器断开
    ///
    /// 数据流：fetch_stories_task → event_tx.send() → broadcast channel → rx → SSE → 客户端
    async fn fetch_events_sse(
        axum::extract::State(app_state): axum::extract::State<Arc<AppState>>,
    ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        // 订阅 broadcast channel，每个客户端连接获得独立的接收端
        let rx = app_state.fetch_events.subscribe();
        // BroadcastStream 将 tokio broadcast receiver 转为 Stream；
        // 当客户端消费速度跟不上生产速度时，broadcast 会返回 Lagged 错误，
        // filter_map 将其过滤掉，只保留有效事件
        let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|msg| match msg {
            Ok(event) => {
                // 将 FetchEvent 序列化为 JSON 字符串，封装为 SSE data 帧
                let data = serde_json::to_string(&event).ok()?;
                Some(Ok(Event::default().data(data)))
            }
            Err(_) => None, // 丢弃 lagged 错误，继续推送后续事件
        });
        // keep_alive 定期发送心跳注释帧（`: comment\n\n`），保持连接活跃
        Sse::new(stream).keep_alive(KeepAlive::default())
    }
}
