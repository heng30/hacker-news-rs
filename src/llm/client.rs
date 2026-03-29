use anyhow::Result;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::openai;

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

    fn build_client(&self) -> Result<openai::Client> {
        openai::Client::builder()
            .api_key(&self.api_key)
            .base_url(&self.base_url)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))
    }

    /// Generate a summary of the story
    pub async fn summarize(&self, title: &str, url: Option<&str>) -> Result<String> {
        let client = self.build_client()?;
        let agent = client.agent(&self.model)
            .preamble(
                "You are a helpful assistant that summarizes Hacker News stories. \
                 Provide a concise 2-3 sentence summary that captures the key points. \
                 Be informative but brief.",
            )
            .build();

        let context = match url {
            Some(url) => format!("Title: {}\nURL: {}", title, url),
            None => format!("Title: {}", title),
        };

        let summary = agent.prompt(&context).await?;
        Ok(summary)
    }

    /// Translate text to Chinese
    pub async fn translate_to_chinese(&self, text: &str) -> Result<String> {
        let client = self.build_client()?;
        let agent = client.agent(&self.model)
            .preamble(
                "You are a helpful translator. Translate the given English text to Chinese (Simplified). \
                 Keep the translation accurate and natural. Only output the translated text, nothing else.",
            )
            .build();

        let translation = agent.prompt(text).await?;
        Ok(translation)
    }

    /// Generate summary and translate to Chinese in one call
    pub async fn summarize_and_translate(&self, title: &str, url: Option<&str>) -> Result<(String, String)> {
        let client = self.build_client()?;
        let agent = client.agent(&self.model)
            .preamble(
                "You are a helpful assistant that summarizes Hacker News stories. \
                 First, provide a concise 2-3 sentence summary in English. \
                 Then, on a new line after '---', provide the Chinese translation of that summary. \
                 Format your response exactly as:\n<English summary>\n---\n<Chinese translation>",
            )
            .build();

        let context = match url {
            Some(url) => format!("Title: {}\nURL: {}", title, url),
            None => format!("Title: {}", title),
        };

        let response: String = agent.prompt(&context).await?;

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
}