use anyhow::Result;
use bot::{APIConfig, Chat, ChatConfig, StreamTextItem};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct LlmClient {
    api_key: String,
    base_url: String,
    model: String,
    no_stream: Option<bool>,
    user_agent: Option<String>,
}

impl LlmClient {
    pub fn new(
        api_key: String,
        base_url: String,
        model: String,
        no_stream: Option<bool>,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            api_key,
            base_url,
            model,
            no_stream,
            user_agent,
        }
    }

    // Generate summary based on language preference
    // lang = "en" -> generates English summary only
    // lang = "zh" -> generates Chinese summary only
    pub async fn summarize(
        &self,
        title: &str,
        url: Option<&str>,
        lang: &str,
    ) -> Result<(Option<String>, Option<String>)> {
        let content = match url {
            Some(u) => crate::fetcher::fetch_url_content(u).await?,
            None => None,
        };

        // 内容抓取失败时返回空摘要，不调用 LLM
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
