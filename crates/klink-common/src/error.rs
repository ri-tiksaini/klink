use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, KlinkError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParseFailure {
    pub path: String,
    pub reason: String,
    pub severity: Severity,
}

#[derive(Debug, Error)]
pub enum KlinkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("invalid binary format: {0}")]
    InvalidFormat(String),
    #[error("scan cancelled")]
    Cancelled,
}
