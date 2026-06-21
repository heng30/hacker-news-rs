use anyhow::Result;
use crate::bot::{APIConfig, Chat, ChatConfig, StreamTextItem};
use tokio::sync::mpsc;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct LlmClient {
    api_key: String,
    base_url: String,
    model: String,
    no_stream: Option<bool>,
    user_agent: Option<String>,
    request_timeout: u32,
    http_client: reqwest::Client,
    fetch_html_timeout: u32,
    max_content_length: u32,
}

impl LlmClient {
    pub fn new(config: &AppConfig, http_client: reqwest::Client) -> Self {
        Self {
            api_key: config.api_key.clone(),
            base_url: config.api_base_url.clone(),
            model: config.model.clone(),
            no_stream: config.llm_no_stream,
            user_agent: config.llm_user_agent.clone(),
            request_timeout: config.llm_timeout,
            http_client,
            fetch_html_timeout: config.fetch_html_timeout,
            max_content_length: config.max_content_length,
        }
    }

    /// Generate summary based on language preference
    /// lang = "en" -> generates English summary only
    /// lang = "zh" -> generates Chinese summary only
    pub async fn summarize(
        &self,
        title: &str,
        url: Option<&str>,
        lang: &str,
    ) -> Result<(Option<String>, Option<String>)> {
        let content = match url {
            Some(u) => {
                crate::fetcher::fetch_url_content(
                    u,
                    &self.http_client,
                    self.fetch_html_timeout,
                    self.max_content_length,
                )
                .await?
            }
            None => None,
        };

        // Skip LLM call if content fetch failed
        if content.is_none() {
            return Ok((None, None));
        }

        let context = format!("Title: {}\n\nContent:\n{}", title, content.unwrap());

        if lang == "en" {
            let prompt = "You are a helpful assistant that summarizes Hacker News stories in English. \
                          Requirements: \
                          1. MUST be between 200-300 words \
                          2. Provide comprehensive context, technical details, and implications \
                          3. Write in clear, professional English suitable for technical readers \
                          Only output the summary, nothing else.";

            let response = self.call_llm(prompt, &context).await?;
            Ok((Some(response.trim().to_string()), None))
        } else {
            let prompt = "You are a helpful assistant that summarizes Hacker News stories in Chinese. \
                          Requirements: \
                          1. MUST be between 400-500 Chinese characters \
                          2. Provide comprehensive context, technical details, and implications \
                          3. Write in clear, professional Chinese suitable for technical readers \
                          Only output the Chinese summary, nothing else.";

            let response = self.call_llm(prompt, &context).await?;
            Ok((None, Some(response.trim().to_string())))
        }
    }

    async fn call_llm(&self, prompt: &str, context: &str) -> Result<String> {
        let request_config = APIConfig {
            api_base_url: self.base_url.clone(),
            api_model: self.model.clone(),
            api_key: self.api_key.clone(),
            temperature: Some(0.7),
            no_stream: self.no_stream,
            user_agent: self.user_agent.clone(),
            request_timeout: self.request_timeout,
        };

        let (tx, mut rx) = mpsc::channel::<StreamTextItem>(100);
        let chat_config = ChatConfig { tx };
        let chat = Chat::new(prompt, context, chat_config, request_config, vec![]);

        let mut full_response = String::new();
        let chat_handle = tokio::spawn(async move { chat.start().await });

        while let Some(item) = rx.recv().await {
            if let Some(text) = item.text {
                full_response.push_str(&text);
            }
            if item.finished {
                break;
            }
            if let Some(err) = item.etext {
                anyhow::bail!("LLM API error: {}", err);
            }
        }

        chat_handle.await??;
        Ok(full_response)
    }
}
