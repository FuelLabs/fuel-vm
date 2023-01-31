use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum TestError {
    #[error("Test failed {0}")]
    Failed(String),
    #[error("Unsupported action {0}")]
    UnsupportedAction(String),
    #[error("Failed to decode encoded value")]
    DecodingError,
}
