//! Low-level HTTP transport. **All** outbound requests go through here.
//!
//! Responsibilities:
//! - Build a `reqwest::Client` once, reuse it for the lifetime of `IgClient`.
//! - Inject mandatory headers: `X-IG-API-KEY`, `Accept`, `Content-Type`,
//!   `Version`, plus auth headers (CST/XST or `Authorization: Bearer …`)
//!   pulled from the shared session state.
//! - Surface errors as [`crate::Error`], mapping IG's `errorCode` payload to
//!   `Error::Api`.
//! - Emit a `tracing` span per request, redacting credentials.

use std::sync::Arc;

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tracing::{Instrument, debug, debug_span, info, warn};

use crate::config::IgConfig;
use crate::error::{ApiError, Error, Result};
use crate::session::auth::refresh_oauth_tokens;
use crate::session::tokens::SessionState;
use crate::session::{RestAuth, SharedSession};

const HDR_API_KEY: &str = "X-IG-API-KEY";
const HDR_VERSION: &str = "Version";
const HDR_ACCOUNT_ID: &str = "IG-ACCOUNT-ID";
const HDR_CST: &str = "CST";
const HDR_XST: &str = "X-SECURITY-TOKEN";

/// Raw HTTP response surfaced by [`Transport::request_unauthenticated`].
/// Used for the login flow which needs to read response headers.
#[derive(Debug)]
pub(crate) struct RawResponse {
    #[allow(dead_code)] // kept for symmetry / future logging
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

#[derive(Debug, Clone)]
pub struct Transport {
    inner: reqwest::Client,
    config: Arc<IgConfig>,
    /// Serialises OAuth refreshes — IG rotates the `refresh_token` on
    /// every call, so concurrent POSTs invalidate each other.
    refresh_lock: Arc<Mutex<()>>,
}

impl Transport {
    pub(crate) fn new(config: Arc<IgConfig>) -> Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.request_timeout)
            .build()?;
        Ok(Self {
            inner,
            config,
            refresh_lock: Arc::new(Mutex::new(())),
        })
    }

    fn url(&self, path: &str) -> Result<reqwest::Url> {
        let base = self.config.environment.base_url();
        Ok(base.join(path)?)
    }

    fn base_headers(&self, version: Option<u8>) -> Result<HeaderMap> {
        let mut h = HeaderMap::new();
        h.insert(HDR_API_KEY, HeaderValue::from_str(&self.config.api_key)?);
        h.insert(
            "Accept",
            HeaderValue::from_static("application/json; charset=UTF-8"),
        );
        if let Some(v) = version {
            h.insert(HDR_VERSION, HeaderValue::from_str(&v.to_string())?);
        }
        Ok(h)
    }

    pub(crate) async fn request_unauthenticated<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        version: Option<u8>,
        body: Option<&B>,
    ) -> Result<RawResponse> {
        let url = self.url(path)?;
        let span = debug_span!("ig.http", %method, path = %path, version = ?version);
        let inner = self.inner.clone();
        let headers = self.base_headers(version)?;

        async move {
            let mut req = inner.request(method.clone(), url).headers(headers);
            if let Some(b) = body {
                req = req.json(b);
            }
            debug!("sending request");
            let resp = req.send().await?;
            let status = resp.status();
            let resp_headers = resp.headers().clone();
            let bytes = resp.bytes().await?;
            if !status.is_success() {
                return Err(api_error(status, &bytes));
            }
            Ok(RawResponse {
                status,
                headers: resp_headers,
                body: bytes,
            })
        }
        .instrument(span)
        .await
    }

    /// Proactive refresh before issue + reactive refresh + retry on 401.
    /// v2 sessions skip both paths (no `refresh_token`).
    pub(crate) async fn request_authenticated_raw<B>(
        &self,
        method: Method,
        path: &str,
        version: Option<u8>,
        body: Option<&B>,
        session: &SharedSession,
    ) -> Result<RawResponse>
    where
        B: Serialize + ?Sized,
    {
        let mut state = session.snapshot().await;

        if state.tokens.needs_refresh(self.config.token_refresh_skew) {
            if let Err(e) = self.refresh_if_needed(session).await {
                warn!(error = %e, "proactive token refresh failed; continuing with stale token");
            }
            state = session.snapshot().await;
        }

        let first = self
            .attempt_authenticated_raw(&method, path, version, body, &state)
            .await;

        // Surface the original 401 if the refresh itself fails.
        match first {
            Err(Error::Api { status, source }) if status == StatusCode::UNAUTHORIZED => {
                if state.tokens.refresh.is_none() {
                    return Err(Error::Api { status, source });
                }
                warn!("401 from IG — attempting reactive OAuth refresh + retry");
                match self.refresh_if_needed_forced(session).await {
                    Ok(()) => {
                        let fresh = session.snapshot().await;
                        self.attempt_authenticated_raw(&method, path, version, body, &fresh)
                            .await
                    }
                    Err(refresh_err) => {
                        warn!(refresh_error = %refresh_err, "reactive refresh failed");
                        Err(Error::Api { status, source })
                    }
                }
            }
            other => other,
        }
    }

    async fn attempt_authenticated_raw<B>(
        &self,
        method: &Method,
        path: &str,
        version: Option<u8>,
        body: Option<&B>,
        state: &SessionState,
    ) -> Result<RawResponse>
    where
        B: Serialize + ?Sized,
    {
        let url = self.url(path)?;
        let mut headers = self.base_headers(version)?;
        let Some(rest_auth) = state.tokens.rest.as_ref() else {
            return Err(Error::Auth("no active session — call login() first".into()));
        };
        match rest_auth {
            RestAuth::Cst {
                cst,
                x_security_token,
            } => {
                headers.insert(HDR_CST, HeaderValue::from_str(cst)?);
                headers.insert(HDR_XST, HeaderValue::from_str(x_security_token)?);
            }
            RestAuth::OAuth {
                access_token,
                token_type,
            } => {
                headers.insert(
                    "Authorization",
                    HeaderValue::from_str(&format!("{token_type} {access_token}"))?,
                );
                if let Some(account) = state.account_id.as_deref() {
                    headers.insert(HDR_ACCOUNT_ID, HeaderValue::from_str(account)?);
                }
            }
        }

        // IG drops the body on real DELETE — mirror the Python client and
        // tunnel body-carrying DELETEs through POST + `_method: DELETE`.
        let (effective_method, override_method) = if *method == Method::DELETE && body.is_some() {
            (Method::POST, Some("DELETE"))
        } else {
            (method.clone(), None)
        };
        if let Some(m) = override_method {
            headers.insert("_method", HeaderValue::from_static(m));
        }

        let span = debug_span!(
            "ig.http",
            method = %effective_method,
            path = %path,
            version = ?version,
        );
        let inner = self.inner.clone();

        async move {
            let mut req = inner.request(effective_method, url).headers(headers);
            if let Some(b) = body {
                req = req.json(b);
            }
            debug!("sending request");
            let resp = req.send().await?;
            let status = resp.status();
            let resp_headers = resp.headers().clone();
            let bytes = resp.bytes().await?;
            if !status.is_success() {
                if status == StatusCode::UNAUTHORIZED {
                    warn!("server returned 401 — token may have expired");
                }
                return Err(api_error(status, &bytes));
            }
            Ok(RawResponse {
                status,
                headers: resp_headers,
                body: bytes,
            })
        }
        .instrument(span)
        .await
    }

    /// DCL on `expires_at` identity rather than `needs_refresh` — when
    /// skew ≥ TTL, the rotated token would itself need refresh and
    /// every waiter would still POST.
    async fn refresh_if_needed(&self, session: &SharedSession) -> Result<()> {
        let before = match session.snapshot().await.tokens.refresh.as_ref() {
            Some(r) => r.expires_at,
            None => return Ok(()),
        };
        let _guard = self.refresh_lock.lock().await;
        let after = session
            .snapshot()
            .await
            .tokens
            .refresh
            .as_ref()
            .map(|r| r.expires_at);
        if after != Some(before) {
            debug!("token rotated by another caller — skip");
            return Ok(());
        }
        info!(
            skew_seconds = self.config.token_refresh_skew.as_secs(),
            "OAuth proactive refresh"
        );
        refresh_oauth_tokens(self, session).await?;
        info!("OAuth refresh successful");
        Ok(())
    }

    /// Called after a 401 — same DCL but skips the `needs_refresh` gate
    /// (IG can rotate server-side independently of TTL).
    async fn refresh_if_needed_forced(&self, session: &SharedSession) -> Result<()> {
        let before = session
            .snapshot()
            .await
            .tokens
            .refresh
            .as_ref()
            .map(|r| r.expires_at);
        let _guard = self.refresh_lock.lock().await;
        let after = session
            .snapshot()
            .await
            .tokens
            .refresh
            .as_ref()
            .map(|r| r.expires_at);
        if before != after {
            debug!("token rotated by another caller — skip");
            return Ok(());
        }
        refresh_oauth_tokens(self, session).await?;
        info!("OAuth refresh successful (reactive)");
        Ok(())
    }

    pub(crate) async fn request<B, R>(
        &self,
        method: Method,
        path: &str,
        version: Option<u8>,
        body: Option<&B>,
        session: &SharedSession,
    ) -> Result<R>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let raw = self
            .request_authenticated_raw(method, path, version, body, session)
            .await?;
        if raw.body.is_empty() {
            // DELETE responses may have empty body — let serde deser `()`.
            return Ok(serde_json::from_value(serde_json::Value::Null)?);
        }
        Ok(serde_json::from_slice(&raw.body)?)
    }
}

fn api_error(status: StatusCode, body: &Bytes) -> Error {
    if let Ok(api) = serde_json::from_slice::<ApiError>(body) {
        Error::Api {
            status,
            source: api,
        }
    } else {
        let snippet = String::from_utf8_lossy(body);
        Error::Api {
            status,
            source: ApiError {
                error_code: format!("http-{}", status.as_u16()),
                extra: serde_json::Map::from_iter([(
                    "body".into(),
                    serde_json::Value::String(snippet.into_owned()),
                )]),
            },
        }
    }
}
