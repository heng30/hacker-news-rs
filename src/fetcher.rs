use anyhow::Result;
use scraper::Html;

pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0";

const SKIP_TAGS: &[&str] = &[
    "script", "style", "noscript", "svg", "canvas", "iframe", "embed", "object", "img", "video",
    "audio", "nav", "footer", "aside", "form",
];

/// Fetch content from a URL and extract text
pub async fn fetch_url_content(
    url: &str,
    client: &reqwest::Client,
    _fetch_html_timeout: u32,
    max_content_length: u32,
) -> Result<Option<String>> {
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

    let text = extract_text(&html);

    let truncated = if text.len() > max_content_length as usize {
        tracing::debug!(
            "Content truncated from {} to {} characters",
            text.len(),
            max_content_length
        );
        &text[..max_content_length as usize]
    } else {
        &text
    };

    Ok(Some(truncated.to_string()))
}

fn extract_text(html: &str) -> String {
    let mut document = Html::parse_document(html);
    let root = document.tree.root();
    let mut to_detach = Vec::new();

    for node in root.descendants() {
        if let Some(element) = node.value().as_element()
            && SKIP_TAGS.contains(&element.name.local.as_ref())
        {
            to_detach.push(node.id());
        }
    }

    for node_id in to_detach {
        if let Some(mut node) = document.tree.get_mut(node_id) {
            node.detach();
        }
    }

    let text: String = document
        .tree
        .root()
        .descendants()
        .filter_map(|n| n.value().as_text())
        .map(|t| t.text.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
