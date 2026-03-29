use anyhow::Result;
use htmd::HtmlToMarkdown;

const MAX_CONTENT_LENGTH: usize = 32000;
const TIMEOUT_SECS: u64 = 30;

/// 从 URL 获取内容并转换为 Markdown
pub async fn fetch_url_content(url: &str) -> Result<Option<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .user_agent("Mozilla/5.0 (compatible; HackerNewsRSS/1.0)")
        .build()?;

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

    let converter = HtmlToMarkdown::new();
    let markdown = match converter.convert(&html) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to convert HTML to Markdown for {}: {}", url, e);
            return Ok(None);
        }
    };

    // 截断过长内容
    let truncated = if markdown.len() > MAX_CONTENT_LENGTH {
        tracing::debug!("Content truncated from {} to {} characters", markdown.len(), MAX_CONTENT_LENGTH);
        &markdown[..MAX_CONTENT_LENGTH]
    } else {
        &markdown
    };

    Ok(Some(truncated.to_string()))
}