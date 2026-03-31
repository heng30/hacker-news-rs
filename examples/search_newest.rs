use hacker_news_rs::HnClient;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let client = HnClient::new();
    let keyword = std::env::args().nth(1).unwrap_or_else(|| "rust".to_string());

    println!("Searching HN for keyword: '{}'", keyword);

    match client.search_newest(&keyword).await {
        Ok(stories) => {
            println!("Found {} stories:", stories.len());
            for story in stories {
                println!(
                    "  [{}] {} - by {} (score: {})",
                    story.id, story.title, story.by, story.score
                );
                if let Some(url) = &story.url {
                    println!("    URL: {}", url);
                }
            }
        }
        Err(e) => {
            eprintln!("Error searching: {}", e);
        }
    }
}