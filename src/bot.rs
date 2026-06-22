mod chat;
mod request;
mod response;

pub(crate) use chat::{Chat, ChatConfig};
pub(crate) use request::APIConfig;
pub(crate) use response::StreamTextItem;

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Request Error {0}")]
    Request(#[from] reqwest::Error),
}
