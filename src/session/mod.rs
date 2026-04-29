//! Session lifecycle: login (v2/v3), refresh, switch account, logout.
//!
//! This module owns the *state* that the HTTP transport reads on every
//! request: the active auth tokens and the bound account. Domain modules
//! never touch session state directly.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::client::http::Transport;
use crate::error::{Error, Result};

mod auth;
#[cfg(feature = "encryption")]
pub mod encryption;
mod tokens;

pub use auth::{SessionApi, SessionDetails, SwitchAccountResponse};
pub use tokens::{AuthTokens, SessionState};

/// User-supplied login credentials.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Plain username + password (v2/v3 login).
    Password { username: String, password: String },
}

impl Credentials {
    pub fn password(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Password {
            username: username.into(),
            password: password.into(),
        }
    }
}

/// Subset of the `POST /session` response surfaced to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub account_id: String,
    pub client_id: String,
    pub timezone_offset: Option<i32>,
    pub lightstreamer_endpoint: String,
    pub currency_iso_code: Option<String>,
    pub locale: Option<String>,
}

/// Shared, mutable session state. Cheap to clone (Arc).
#[derive(Debug, Clone, Default)]
pub struct SharedSession {
    inner: Arc<RwLock<SessionState>>,
}

impl SharedSession {
    pub async fn snapshot(&self) -> SessionState {
        self.inner.read().await.clone()
    }

    pub(crate) async fn replace(&self, new: SessionState) {
        *self.inner.write().await = new;
    }

    pub(crate) async fn modify<F>(&self, f: F)
    where
        F: FnOnce(&mut SessionState),
    {
        let mut guard = self.inner.write().await;
        f(&mut guard);
    }

    pub async fn require_authenticated(&self) -> Result<SessionState> {
        let s = self.snapshot().await;
        if s.tokens.is_some() {
            Ok(s)
        } else {
            Err(Error::Auth(
                "no active session — call session().login() first".into(),
            ))
        }
    }
}

/// Internal handle: a `Transport` plus a `SharedSession`. Used by [`SessionApi`].
#[derive(Debug, Clone)]
pub(crate) struct SessionHandle {
    pub(crate) transport: Transport,
    pub(crate) session: SharedSession,
    pub(crate) credentials: Option<Credentials>,
}
