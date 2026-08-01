use crate::bot::{Error, request, response};
use reqwest::header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE, HeaderMap};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[derive(Debug)]
pub(crate) struct ChatConfig {
    pub tx: mpsc::Sender<response::StreamTextItem>,
}

#[derive(Debug)]
pub(crate) struct Chat {
    pub config: request::APIConfig,
    messages: Vec<request::Message>,
    chat_tx: mpsc::Sender<response::StreamTextItem>,
}

impl Chat {
    pub fn new(
        prompt: impl ToString,
        question: impl ToString,
        config: ChatConfig,
        request_config: request::APIConfig,
        chats: Vec<request::HistoryChat>,
    ) -> Chat {
        let mut messages = vec![];

        for item in chats.into_iter() {
            messages.push(request::Message {
                role: "user".to_string(),
                content: item.utext,
            });

            messages.push(request::Message {
                role: "assistant".to_string(),
                content: item.btext,
            })
        }

        let merged_content = format!("{}\n\n{}", prompt.to_string(), question.to_string());
        messages.push(request::Message {
            role: "user".to_string(),
            content: merged_content,
        });

        Chat {
            messages,
            config: request_config,
            chat_tx: config.tx,
        }
    }

    fn headers(&self, for_stream: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", self.config.api_key).parse().unwrap(),
        );
        if for_stream {
            headers.insert(ACCEPT, "text/event-stream".parse().unwrap());
            headers.insert(CACHE_CONTROL, "no-cache".parse().unwrap());
        } else {
            headers.insert(ACCEPT, "application/json".parse().unwrap());
        }

        headers
    }

    pub async fn start(self) -> Result<(), Error> {
        let mut client_builder = reqwest::Client::builder();

        if let Some(ref ua) = self.config.user_agent {
            client_builder = client_builder.user_agent(ua);
        }

        let client = client_builder.build()?;

        let url = if self.config.api_base_url.ends_with("/chat/completions") {
            self.config.api_base_url.clone()
        } else {
            let base = self.config.api_base_url.trim_end_matches('/');
            format!("{}{}", base, "/chat/completions")
        };

        let use_stream = !self.config.no_stream.unwrap_or(false);
        let headers = self.headers(use_stream);
        let chat_tx = self.chat_tx;

        let request_body = request::ChatCompletion {
            messages: self.messages,
            model: self.config.api_model,
            temperature: self.config.temperature,
            stream: use_stream,
        };

        log::debug!("LLM request URL: {}", url);
        log::debug!(
            "LLM request body: {}",
            serde_json::to_string(&request_body).unwrap_or_default()
        );
        log::debug!("LLM use_stream: {}", use_stream);

        let response = client
            .post(&url)
            .headers(headers)
            .json(&request_body)
            .timeout(Duration::from_secs(self.config.request_timeout as u64))
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await?;
            log::error!("API error: status={}, body={}", status, error_body);
            let item = response::StreamTextItem {
                etext: Some(format!("API error: {}", error_body)),
                ..Default::default()
            };
            if chat_tx.send(item).await.is_err() {
                log::info!("receiver dropped");
            }
            return Ok(());
        }

        if use_stream {
            handle_stream_response(response, chat_tx).await?;
        } else {
            handle_non_stream_response(response, chat_tx).await?;
        }

        Ok(())
    }
}

async fn handle_stream_response(
    response: reqwest::Response,
    chat_tx: mpsc::Sender<response::StreamTextItem>,
) -> Result<(), Error> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();

    loop {
        match stream.next().await {
            Some(Ok(chunk)) => {
                buffer.extend_from_slice(&chunk);

                while let Some((sep, sep_len)) = find_event_separator(&buffer) {
                    // A UTF-8 char split across network chunks can only be
                    // incomplete at the buffer tail (after the last event
                    // separator), so a complete event always decodes intact.
                    let event = match std::str::from_utf8(&buffer[..sep]) {
                        Ok(e) => e.to_string(),
                        Err(_) => {
                            log::error!("SSE event is not valid UTF-8, skipping");
                            buffer.drain(..sep + sep_len);
                            continue;
                        }
                    };
                    buffer.drain(..sep + sep_len);

                    if handle_event(&chat_tx, &event).await? {
                        return Ok(());
                    }
                }
            }
            Some(Err(e)) => log::error!("Stream error: {:?}", e),
            None => {
                // Provider closed the stream without a [DONE] marker: flush a
                // trailing event that had no blank-line terminator.
                if !buffer.is_empty()
                    && let Ok(event) = std::str::from_utf8(&buffer)
                    && handle_event(&chat_tx, event).await?
                {
                    return Ok(());
                }
                break;
            }
        }
    }
    Ok(())
}

/// Locate the next SSE event separator (`\n\n` or `\r\n\r\n`) and return
/// (end of the event content, length of the separator). `\n` never occurs
/// inside a multi-byte UTF-8 sequence, so searching raw bytes is safe.
fn find_event_separator(buf: &[u8]) -> Option<(usize, usize)> {
    if let Some(pos) = buf.windows(2).position(|w| w == b"\n\n") {
        Some((pos, 2))
    } else if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        Some((pos, 4))
    } else {
        None
    }
}

/// Parse and forward a single SSE event. Returns `Ok(true)` when the stream
/// should stop after this event (`[DONE]`, API error, finish reason, or the
/// receiver being dropped).
async fn handle_event(
    tx: &mpsc::Sender<response::StreamTextItem>,
    raw_event: &str,
) -> Result<bool, Error> {
    // A trailing newline can survive on the no-blank-line flush path; strip
    // both `\r` and `\n` so LF/CRLF framings behave identically (e.g. [DONE]).
    let event = raw_event.trim_end_matches(['\r', '\n']);

    if event.is_empty() {
        return Ok(false);
    }

    if event == "data: [DONE]" {
        return Ok(true);
    }

    if !event.starts_with("data:") {
        return Ok(false);
    }

    let json_str = &event[5..];

    if let Ok(err) = serde_json::from_str::<response::Error>(json_str) {
        if let Some(estr) = err.error.get("message") {
            let item = response::StreamTextItem {
                etext: Some(estr.clone()),
                ..Default::default()
            };
            if tx.send(item).await.is_err() {
                log::info!("receiver dropped");
                return Ok(true);
            }
            log::error!("API error: {}", estr);
        }
        return Ok(true);
    }

    match serde_json::from_str::<response::ChatCompletionChunk>(json_str) {
        Ok(chunk) => {
            if chunk.choices.is_empty() {
                return Ok(false);
            }
            let choice = &chunk.choices[0];
            if choice.finish_reason.is_some() {
                let item = response::StreamTextItem {
                    finished: true,
                    ..Default::default()
                };
                if tx.send(item).await.is_err() {
                    log::info!("receiver dropped");
                    return Ok(true);
                }
                return Ok(true);
            }

            let item = if choice.delta.contains_key("content") && choice.delta["content"].is_some()
            {
                Some(response::StreamTextItem {
                    text: choice.delta["content"].clone(),
                    ..Default::default()
                })
            } else if choice.delta.contains_key("reasoning_content")
                && choice.delta["reasoning_content"].is_some()
            {
                Some(response::StreamTextItem {
                    reasoning_text: choice.delta["reasoning_content"].clone(),
                    ..Default::default()
                })
            } else {
                None
            };

            if let Some(item) = item
                && tx.send(item).await.is_err()
            {
                log::info!("receiver dropped");
                return Ok(true);
            }
            Ok(false)
        }
        Err(e) => {
            log::error!("Parse error: {:?} event={}", e, event);
            Ok(false)
        }
    }
}

async fn handle_non_stream_response(
    response: reqwest::Response,
    chat_tx: mpsc::Sender<response::StreamTextItem>,
) -> Result<(), Error> {
    let body = response.text().await?;

    match serde_json::from_str::<response::ChatCompletionResponse>(&body) {
        Ok(resp) => {
            if resp.choices.is_empty() {
                log::error!("Empty choices in response");
                return Ok(());
            }

            let choice = &resp.choices[0];
            let content = choice.message.content.clone();

            let item = response::StreamTextItem {
                text: content,
                ..Default::default()
            };
            if chat_tx.send(item).await.is_err() {
                log::info!("receiver dropped");
                return Ok(());
            }

            let finished_item = response::StreamTextItem {
                finished: true,
                ..Default::default()
            };
            if chat_tx.send(finished_item).await.is_err() {
                log::info!("receiver dropped");
            }
        }
        Err(e) => {
            log::error!("Parse error for non-stream response: {e:?} body={body}");
            if let Ok(err) = serde_json::from_str::<response::Error>(&body)
                && let Some(estr) = err.error.get("message")
            {
                let item = response::StreamTextItem {
                    etext: Some(estr.clone()),
                    ..Default::default()
                };

                if chat_tx.send(item).await.is_err() {
                    log::info!("receiver dropped");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_separator_lf_and_crlf() {
        assert_eq!(find_event_separator(b"data: x\n\nmore"), Some((7, 2)));
        assert_eq!(find_event_separator(b"data: x\r\n\r\nmore"), Some((7, 4)));
        // Mid-event bytes (e.g. a split multi-byte char) are not a separator
        assert_eq!(find_event_separator(b"data: \xE4\xBD"), None);
        assert_eq!(find_event_separator(b"data: x"), None);
    }

    #[tokio::test]
    async fn split_utf8_char_across_chunks_reassembles() {
        let (tx, mut rx) = mpsc::channel(16);
        let mut buffer: Vec<u8> = Vec::new();

        // 你 = U+4F60 → E4 BD A0 in UTF-8. Cut the byte stream between the
        // second and third byte of 你, as a TCP segment boundary would.
        let event = format!(
            "data: {}\n\n",
            r#"{"choices":[{"delta":{"content":"你好"}}]}"#
        );
        let bytes = event.as_bytes();
        let split = event.find("你").unwrap() + 2;

        // First chunk leaves a partial multi-byte char in the buffer; it must
        // stay raw bytes — no separator found, nothing lossy-decoded yet.
        buffer.extend_from_slice(&bytes[..split]);
        assert_eq!(find_event_separator(&buffer), None);

        // Second chunk completes the char and the event framing.
        buffer.extend_from_slice(&bytes[split..]);

        while let Some((sep, sep_len)) = find_event_separator(&buffer) {
            let event = std::str::from_utf8(&buffer[..sep]).unwrap().to_string();
            buffer.drain(..sep + sep_len);
            assert!(!handle_event(&tx, &event).await.unwrap());
        }

        let item = rx.recv().await.unwrap();
        assert_eq!(item.text.as_deref(), Some("你好"));
        assert!(!item.finished);
        assert!(buffer.is_empty());
    }
}
