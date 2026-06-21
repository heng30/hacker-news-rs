use crate::{
    bot::{APIConfig, Chat, ChatConfig, StreamTextItem},
    config::AppConfig,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct LlmClient {
    config: Arc<AppConfig>,
    fetch_client: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: Arc<AppConfig>, fetch_client: reqwest::Client) -> Self {
        Self {
            config,
            fetch_client,
        }
    }

    /// Generate Chinese summary
    pub async fn summarize(&self, title: &str, url: Option<&str>) -> Result<Option<String>> {
        let content = match url {
            Some(u) => {
                crate::fetcher::fetch_url_content(
                    u,
                    &self.fetch_client,
                    self.config.fetch_html_timeout,
                    self.config.max_content_length,
                )
                .await?
            }
            None => None,
        };

        // Skip LLM call if content fetch failed
        if content.is_none() {
            return Ok(None);
        }

        let context = format!("Title: {}\n\nContent:\n{}", title, content.unwrap());

        let prompt = "你是一个帮助总结 Hacker News 故事的助手。\
                      要求：\
                      1. 必须在400-500个中文字符之间 \
                      2. 提供全面的上下文、技术细节和影响 \
                      3. 用清晰、专业的中文撰写，适合技术读者阅读 \
                      只输出中文摘要，不要输出其他内容。";

        let response = self.call_llm(prompt, &context).await?;
        Ok(Some(response.trim().to_string()))
    }

    async fn call_llm(&self, prompt: &str, context: &str) -> Result<String> {
        let request_config = APIConfig {
            api_base_url: self.config.api_base_url.clone(),
            api_model: self.config.model.clone(),
            api_key: self.config.api_key.clone(),
            temperature: Some(0.7),
            no_stream: if self.config.llm_no_stream {
                Some(true)
            } else {
                None
            },
            user_agent: self.config.llm_user_agent.clone(),
            request_timeout: self.config.llm_timeout,
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
