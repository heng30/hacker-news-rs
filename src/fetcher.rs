use crate::config;
use anyhow::Result;
use htmd::HtmlToMarkdown;
use std::time::Duration;

pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";

pub async fn fetch_url_content(url: &str) -> Result<Option<String>> {
    let mut client_builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(config::get_fetch_html_timeout() as u64))
        .user_agent(USER_AGENT);

    if let Some(proxy) = config::get_socks5_proxy_from_env() {
        let proxy_url = if proxy.starts_with("socks5://") || proxy.starts_with("socks5h://") {
            proxy
        } else {
            format!("socks5://{}", proxy)
        };
        client_builder = client_builder.proxy(reqwest::Proxy::all(&proxy_url)?);
    }

    let client = client_builder.build()?;
    let response = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to fetch URL {}: {}", url, e);
            return Ok(None);
        }
    };

    if !response.status().is_success() {
        tracing::warn!("URL {} returned status {}", url, response.status());
        return Ok(None);
    }

    let html = match response.text().await {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!("Failed to read response from {}: {}", url, e);
            return Ok(None);
        }
    };

    let markdown = match HtmlToMarkdown::new().convert(&html) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to convert HTML to Markdown for {}: {}", url, e);
            return Ok(None);
        }
    };

    let truncated = if markdown.len() > config::get_max_markdown_content_length() as usize {
        tracing::debug!(
            "Content truncated from {} to {} characters",
            markdown.len(),
            config::get_max_markdown_content_length()
        );
        &markdown[..config::get_max_markdown_content_length() as usize]
    } else {
        &markdown
    };

    Ok(Some(truncated.to_string()))
}
