//! Token storage. REST, streaming and refresh surfaces are independent
//! so a v3 OAuth session can also hold CST/XST for Lightstreamer
//! without losing its refresh capability (the earlier tagged-enum
//! port forced callers to choose).

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum RestAuth {
    Cst {
        cst: String,
        x_security_token: String,
    },
    OAuth {
        access_token: String,
        token_type: String,
    },
}

#[derive(Debug, Clone)]
pub struct StreamingAuth {
    pub cst: String,
    pub x_security_token: String,
}

#[derive(Debug, Clone)]
pub struct RefreshState {
    pub refresh_token: String,
    pub expires_at: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct AuthTokens {
    pub rest: Option<RestAuth>,
    pub streaming: Option<StreamingAuth>,
    pub refresh: Option<RefreshState>,
}

impl AuthTokens {
    pub fn needs_refresh(&self, skew: Duration) -> bool {
        match &self.refresh {
            Some(r) => Instant::now() + skew >= r.expires_at,
            None => false,
        }
    }

    pub fn has_rest_auth(&self) -> bool {
        self.rest.is_some()
    }

    pub fn has_streaming_auth(&self) -> bool {
        self.streaming.is_some()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct OAuthPayload {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    /// IG returns `expires_in` as a string of seconds, not a number.
    pub expires_in: String,
}

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
