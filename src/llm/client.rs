use anyhow::Result;
use bot::{APIConfig, Chat, ChatConfig, StreamTextItem};
use tokio::sync::mpsc;

pub struct LlmClient {
    api_key: String,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn new(api_key: String, base_url: String, model: String) -> Self {
        Self {
            api_key,
            base_url,
            model,
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
        // 获取 URL 内容
        let content = match url {
            Some(u) => crate::fetcher::content::fetch_url_content(u).await?,
            None => None,
        };

        let context = match content {
            Some(c) => format!("Title: {}\n\nContent:\n{}", title, c),
            None => match url {
                Some(u) => format!("Title: {}\nURL: {}", title, u),
                None => format!("Title: {}", title),
            },
        };

        if lang == "en" {
            // Generate English summary
            let prompt = "You are a helpful assistant that summarizes Hacker News stories in English. \
                          Requirements: \
                          1. MUST be between 200-300 words \
                          2. Provide comprehensive context, technical details, and implications \
                          3. Write in clear, professional English suitable for technical readers \
                          Only output the summary, nothing else.";

            let response = self.call_llm(prompt, &context).await?;
            Ok((Some(response.trim().to_string()), None))
        } else {
            // Generate Chinese summary
            let prompt = "You are a helpful assistant that summarizes Hacker News stories in Chinese. \
                          Requirements: \
                          1. MUST be between 500-600 Chinese characters (字数必须在500-600字之间) \
                          2. Provide comprehensive context, technical details, and implications \
                          3. Write in clear, professional Chinese suitable for technical readers \
                          Only output the Chinese summary, nothing else.";

            let response = self.call_llm(prompt, &context).await?;
            Ok((None, Some(response.trim().to_string())))
        }
    }

    /// Internal helper to call the LLM API using the bot library
    async fn call_llm(&self, prompt: &str, context: &str) -> Result<String> {
        let request_config = APIConfig {
            api_base_url: self.base_url.clone(),
            api_model: self.model.clone(),
            api_key: self.api_key.clone(),
            temperature: Some(0.7),
        };

        let (tx, mut rx) = mpsc::channel::<StreamTextItem>(100);
        let chat_config = ChatConfig { tx };
        let chat = Chat::new(prompt, context, chat_config, request_config, vec![]);

        // Spawn the chat task
        let chat_handle = tokio::spawn(async move { chat.start().await });

        // Collect the response
        let mut full_response = String::new();
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

        // Wait for the chat task to complete
        chat_handle.await??;

        Ok(full_response)
    }
}

