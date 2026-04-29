//! Token storage and lifecycle.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Active session tokens, by login flavour.
#[derive(Debug, Clone)]
pub enum AuthTokens {
    /// v1/v2 session tokens, returned as response **headers** by `POST /session`.
    Cst {
        cst: String,
        x_security_token: String,
    },
    /// v3 OAuth tokens.
    OAuth {
        access_token: String,
        refresh_token: String,
        token_type: String,
        /// When the access token will expire. Used to pre-emptively refresh.
        expires_at: Instant,
    },
}

impl AuthTokens {
    pub fn needs_refresh(&self, skew: Duration) -> bool {
        match self {
            Self::Cst { .. } => false,
            Self::OAuth { expires_at, .. } => Instant::now() + skew >= *expires_at,
        }
    }
}

/// Wire-level OAuth payload returned in the v3 login body.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct OAuthPayload {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    /// `expires_in` is a **string** of seconds in IG's response. Quirky.
    pub expires_in: String,
}

/// Snapshot of session-level state. Inexpensive to clone.
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub tokens: Option<AuthTokens>,
    pub account_id: Option<String>,
    pub client_id: Option<String>,
    pub lightstreamer_endpoint: Option<String>,
}
