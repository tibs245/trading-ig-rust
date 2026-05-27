//! Token storage and lifecycle.
//!
//! The three auth surfaces (REST, Lightstreamer streaming, OAuth refresh)
//! are kept **independent** in [`AuthTokens`] so an OAuth-authenticated
//! session can also hold CST/X-SECURITY-TOKEN for Lightstreamer without
//! losing its refresh capability. This mirrors the Python `trading-ig`
//! design where Bearer + CST + X-SECURITY-TOKEN cohabit in
//! `session.headers` simultaneously.
//!
//! The earlier port modelled tokens as a single tagged enum
//! (`Cst | OAuth`), which forced callers to choose between
//! OAuth-with-refresh OR CST-for-streaming. Long-running v3 sessions
//! that needed Lightstreamer had to call `session().read(true)`, which
//! overwrote the OAuth state with CST and lost the refresh token —
//! making the session unrefreshable and producing
//! `error.security.client-token-invalid` 401s once IG rotated the
//! tokens server-side.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// REST-side authentication state.
///
/// Either OAuth Bearer (v3 login) or CST + X-SECURITY-TOKEN header
/// pair (v2 login). The HTTP layer injects the corresponding headers
/// per request from this single source.
#[derive(Debug, Clone)]
pub enum RestAuth {
    /// v1/v2 session tokens, injected as `CST` and `X-SECURITY-TOKEN`
    /// request headers.
    Cst {
        cst: String,
        x_security_token: String,
    },
    /// v3 OAuth bearer token, injected as `Authorization: <type> <token>`.
    OAuth {
        access_token: String,
        token_type: String,
    },
}

/// Streaming-side tokens.
///
/// Lightstreamer always needs CST/X-SECURITY-TOKEN regardless of the
/// REST flow. Populated either by `login_v2()` (which returns the
/// headers natively) or by `session().read(true)` for sessions that
/// authenticated via v3 OAuth.
#[derive(Debug, Clone)]
pub struct StreamingAuth {
    pub cst: String,
    pub x_security_token: String,
}

/// Refresh state for OAuth (v3) sessions.
///
/// Used by the proactive refresh path in
/// [`crate::client::http`] (refresh just before `expires_at`) and by
/// reactive 401 recovery. Absent for v2 sessions.
#[derive(Debug, Clone)]
pub struct RefreshState {
    pub refresh_token: String,
    /// Instant after which the OAuth access token will be rejected by
    /// IG. The proactive refresh compares this against
    /// `Instant::now() + skew`.
    pub expires_at: Instant,
}

/// Container for all active session tokens.
///
/// The three fields are independent : a v3 session that has called
/// `read(true)` carries all three (rest=OAuth, streaming=Cst,
/// refresh=Some) ; a pure v2 session carries `rest=Cst` + `streaming`
/// (no refresh). The default (`Default::default()`) represents an
/// unauthenticated session.
#[derive(Debug, Clone, Default)]
pub struct AuthTokens {
    pub rest: Option<RestAuth>,
    pub streaming: Option<StreamingAuth>,
    pub refresh: Option<RefreshState>,
}

impl AuthTokens {
    /// True when the OAuth access token will expire within `skew`.
    /// v2 (CST) sessions always return false — they have no refresh
    /// mechanism and rely on reactive recovery if IG rotates the
    /// tokens server-side.
    pub fn needs_refresh(&self, skew: Duration) -> bool {
        match &self.refresh {
            Some(r) => Instant::now() + skew >= r.expires_at,
            None => false,
        }
    }

    /// True when the REST surface has an active auth header source.
    pub fn has_rest_auth(&self) -> bool {
        self.rest.is_some()
    }

    /// True when the streaming surface (Lightstreamer) has CST/XST.
    pub fn has_streaming_auth(&self) -> bool {
        self.streaming.is_some()
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
    pub tokens: AuthTokens,
    pub account_id: Option<String>,
    pub client_id: Option<String>,
    pub lightstreamer_endpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_refresh_false_when_no_refresh_state() {
        let t = AuthTokens {
            rest: Some(RestAuth::Cst {
                cst: "c".into(),
                x_security_token: "x".into(),
            }),
            streaming: None,
            refresh: None,
        };
        assert!(!t.needs_refresh(Duration::from_secs(10)));
    }

    #[test]
    fn needs_refresh_true_when_within_skew() {
        let t = AuthTokens {
            rest: None,
            streaming: None,
            refresh: Some(RefreshState {
                refresh_token: "r".into(),
                expires_at: Instant::now() + Duration::from_secs(5),
            }),
        };
        assert!(t.needs_refresh(Duration::from_secs(10)));
    }

    #[test]
    fn needs_refresh_false_when_outside_skew() {
        let t = AuthTokens {
            rest: None,
            streaming: None,
            refresh: Some(RefreshState {
                refresh_token: "r".into(),
                expires_at: Instant::now() + Duration::from_mins(1),
            }),
        };
        assert!(!t.needs_refresh(Duration::from_secs(10)));
    }

    #[test]
    fn three_surfaces_independent() {
        let t = AuthTokens {
            rest: Some(RestAuth::OAuth {
                access_token: "a".into(),
                token_type: "Bearer".into(),
            }),
            streaming: Some(StreamingAuth {
                cst: "c".into(),
                x_security_token: "x".into(),
            }),
            refresh: Some(RefreshState {
                refresh_token: "r".into(),
                expires_at: Instant::now() + Duration::from_mins(1),
            }),
        };
        assert!(t.has_rest_auth());
        assert!(t.has_streaming_auth());
        assert!(!t.needs_refresh(Duration::from_secs(10)));
    }
}
