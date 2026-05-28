//! Crate-wide error type.
//!
//! Domain modules **must not** define their own error enum — extend [`Error`]
//! instead. Variants are intentionally coarse-grained; the `Api` variant
//! carries IG's machine-readable `errorCode` for fine matching by callers.

use http::StatusCode;
use serde::Deserialize;

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IG API error ({status}): {0}", .source.error_code)]
    Api {
        status: StatusCode,
        #[source]
        source: ApiError,
    },

    #[error("authentication error: {0}")]
    Auth(String),

    #[error("rate limited by IG ({0})")]
    RateLimited(String),

    #[error("failed to deserialise response: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    #[error("invalid HTTP header value: {0}")]
    HeaderValue(#[from] http::header::InvalidHeaderValue),
}

/// Wire-level error payload returned by IG.
///
/// Most endpoints return `{ "errorCode": "…" }` on failure. Some include
/// additional context fields, surfaced via the `extra` map.
#[derive(Debug, Clone, Deserialize, thiserror::Error)]
#[error("{error_code}")]
pub struct ApiError {
    #[serde(rename = "errorCode")]
    pub error_code: String,
    #[serde(flatten, default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Error {
    /// True for `Error::Auth` or for `Error::Api` whose IG `errorCode`
    /// matches one of: `oauth-token-invalid`, `client-token-invalid`,
    /// `client-token-missing`, `security-token-*`.
    pub fn is_auth(&self) -> bool {
        match self {
            Error::Auth(_) => true,
            Error::Api { source, .. } => {
                let c = source.error_code.as_str();
                c.contains("oauth-token-invalid")
                    || c.contains("client-token-invalid")
                    || c.contains("client-token-missing")
                    || c.contains("security-token")
            }
            _ => false,
        }
    }

    /// True if the error indicates the rate limit has been hit.
    pub fn is_rate_limited(&self) -> bool {
        match self {
            Error::RateLimited(_) => true,
            Error::Api { source, .. } => source.error_code.contains("api-rate-exceeded"),
            _ => false,
        }
    }
}
