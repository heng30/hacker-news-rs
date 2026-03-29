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

    /// Generate a summary of the story
    pub async fn summarize(&self, title: &str, url: Option<&str>) -> Result<String> {
        let context = match url {
            Some(u) => format!("Title: {}\nURL: {}", title, u),
            None => format!("Title: {}", title),
        };

        let prompt = "You are a helpful assistant that summarizes Hacker News stories. \
                      Provide a concise 2-3 sentence summary that captures the key points. \
                      Be informative but brief.";

        let response = self.call_llm(prompt, &context).await?;
        Ok(response)
    }

    /// Translate text to Chinese
    pub async fn translate_to_chinese(&self, text: &str) -> Result<String> {
        let prompt = "You are a helpful translator. Translate the given English text to Chinese (Simplified). \
                      Keep the translation accurate and natural. Only output the translated text, nothing else.";

        let response = self.call_llm(prompt, text).await?;
        Ok(response)
    }

    /// Generate summary and translate to Chinese in one call
    pub async fn summarize_and_translate(
        &self,
        title: &str,
        url: Option<&str>,
    ) -> Result<(String, String)> {
        let context = match url {
            Some(u) => format!("Title: {}\nURL: {}", title, u),
            None => format!("Title: {}", title),
        };

        let prompt = "You are a helpful assistant that summarizes Hacker News stories. \
                      First, provide a concise 2-3 sentence summary in English. \
                      Then, on a new line after '---', provide the Chinese translation of that summary. \
                      Requirements for Chinese translation: \
                      1. MUST be between 300-400 Chinese characters (字数必须在300-400字之间) \
                      2. Expand on the English summary with relevant context, technical details, and implications \
                      3. Do NOT use any italic text or formatting - use plain text only (不使用斜体，只用纯文本) \
                      4. Write in clear, professional Chinese suitable for technical readers \
                      Format your response exactly as:\n<English summary>\n---\n<Chinese translation (300-400 chars, plain text)>";

        let response = self.call_llm(prompt, &context).await?;

        // Parse the response
        let parts: Vec<&str> = response.splitn(2, "---").collect();
        let summary = parts[0].trim().to_string();
        let summary_zh = if parts.len() > 1 {
            parts[1].trim().to_string()
        } else {
            String::new()
        };

        Ok((summary, summary_zh))
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