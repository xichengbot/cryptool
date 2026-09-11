use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable error categories exposed across the native/web boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    SymbolNotFound,
    RateLimited,
    CredentialsNotConfigured,
    UpstreamError,
    QueryBudgetExceeded,
    Cancelled,
    Configuration,
}

/// Serializable error returned by query parsing and exchange services.
#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[error("{message}")]
pub struct QueryError {
    pub code: ErrorCode,
    pub message: String,
    pub retry_after_secs: Option<u64>,
    pub retryable: bool,
}

impl QueryError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let retryable = matches!(
            code,
            ErrorCode::RateLimited | ErrorCode::UpstreamError | ErrorCode::QueryBudgetExceeded
        );
        Self {
            code,
            message: message.into(),
            retry_after_secs: None,
            retryable,
        }
    }
}
