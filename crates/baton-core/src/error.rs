#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("invalid envelope: {0}")]
    Envelope(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
