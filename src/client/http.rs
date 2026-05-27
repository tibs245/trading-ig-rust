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
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::{Instrument, debug, debug_span, info, warn};

use crate::config::IgConfig;
use crate::error::{ApiError, Error, Result};
use crate::session::tokens::OAuthPayload;
use crate::session::{RefreshState, RestAuth, SharedSession};

const HDR_API_KEY: &str = "X-IG-API-KEY";
const HDR_VERSION: &str = "Version";
const HDR_ACCOUNT_ID: &str = "IG-ACCOUNT-ID";
const HDR_CST: &str = "CST";
const HDR_XST: &str = "X-SECURITY-TOKEN";

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct RefreshTokenRequest<'a> {
    refresh_token: &'a str,
}

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
}

impl Transport {
    pub(crate) fn new(config: Arc<IgConfig>) -> Result<Self> {
        let inner = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .timeout(config.request_timeout)
            .build()?;
        Ok(Self { inner, config })
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

    /// Used by the login/refresh flow before any session exists.
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

    /// Authenticated request returning the raw response (status + headers +
    /// body bytes). Used by endpoints that need to read response headers,
    /// e.g. `GET /session?fetchSessionTokens=true` which puts the CST and
    /// X-SECURITY-TOKEN values into headers, not the body.
    ///
    /// **Auto-refresh behavior** : before every request, the active OAuth
    /// token (if any) is checked against
    /// [`IgConfig::token_refresh_skew`] and proactively refreshed if it
    /// would expire within that window. If the request itself returns
    /// 401, a refresh + retry is attempted once before surfacing the
    /// error. Sessions on the v2 (CST) flow have no refresh mechanism,
    /// so 401s on that path propagate to the caller (who can restart
    /// `login_v2()` if needed).
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
        // 1. Proactive refresh : if the access token is about to expire,
        //    refresh it before issuing the request. Failures here are
        //    logged but non-fatal — the request will still attempt with
        //    the (possibly stale) token, and the reactive path below
        //    will catch the resulting 401.
        if let Err(e) = self.ensure_token_valid(session).await {
            warn!(error = %e, "proactive token refresh failed; continuing with stale token");
        }

        // 2. First attempt with current tokens.
        let first = self
            .attempt_authenticated_raw(&method, path, version, body, session)
            .await;

        // 3. Reactive : on 401 (likely client-token-invalid), try
        //    refresh + retry once. Only do this if we actually have a
        //    refresh state (OAuth session). v2 sessions surface the
        //    401 unchanged.
        //
        //    If the refresh itself fails we propagate the *original*
        //    401 — it's the more meaningful error for the caller
        //    (refresh failure is incidental to the actual call).
        match first {
            Err(Error::Api { status, source }) if status == StatusCode::UNAUTHORIZED => {
                if !self.has_refresh_state(session).await {
                    return Err(Error::Api { status, source });
                }
                warn!("401 from IG — attempting reactive OAuth refresh + retry");
                match self.do_refresh_oauth(session).await {
                    Ok(()) => {
                        self.attempt_authenticated_raw(&method, path, version, body, session)
                            .await
                    }
                    Err(refresh_err) => {
                        warn!(
                            refresh_error = %refresh_err,
                            "reactive refresh failed — surfacing original 401"
                        );
                        Err(Error::Api { status, source })
                    }
                }
            }
            other => other,
        }
    }

    /// Inject auth headers from the current session state and execute
    /// the request. Does not refresh tokens — call sites that need
    /// auto-refresh go through [`request_authenticated_raw`].
    async fn attempt_authenticated_raw<B>(
        &self,
        method: &Method,
        path: &str,
        version: Option<u8>,
        body: Option<&B>,
        session: &SharedSession,
    ) -> Result<RawResponse>
    where
        B: Serialize + ?Sized,
    {
        let url = self.url(path)?;
        let mut headers = self.base_headers(version)?;
        let state = session.snapshot().await;
        let Some(rest_auth) = state.tokens.rest.as_ref() else {
            return Err(Error::Auth(
                "no active session — call session().login() first".into(),
            ));
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
        let span = debug_span!("ig.http", %method, path = %path, version = ?version);
        let inner = self.inner.clone();
        let method_owned = method.clone();

        async move {
            let mut req = inner.request(method_owned, url).headers(headers);
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

    /// True when the session has a refresh token (v3 OAuth flow). v2
    /// (CST) sessions return false and cannot self-refresh.
    async fn has_refresh_state(&self, session: &SharedSession) -> bool {
        session.snapshot().await.tokens.refresh.is_some()
    }

    /// Trigger a proactive OAuth refresh if the access token is within
    /// [`IgConfig::token_refresh_skew`] of its expiry. No-op for v2
    /// sessions (no refresh state) and for OAuth sessions that still
    /// have plenty of TTL.
    async fn ensure_token_valid(&self, session: &SharedSession) -> Result<()> {
        let snapshot = session.snapshot().await;
        if !snapshot
            .tokens
            .needs_refresh(self.config.token_refresh_skew)
        {
            return Ok(());
        }
        info!(
            skew_seconds = self.config.token_refresh_skew.as_secs(),
            "OAuth token within refresh skew — proactive refresh"
        );
        self.do_refresh_oauth(session).await
    }

    /// POST `/session/refresh-token` with the stored `refresh_token`
    /// and update the session's REST + refresh state with the new
    /// payload. The streaming surface is intentionally untouched so an
    /// active Lightstreamer subscription survives the refresh.
    async fn do_refresh_oauth(&self, session: &SharedSession) -> Result<()> {
        let Some(refresh_token) = session
            .snapshot()
            .await
            .tokens
            .refresh
            .as_ref()
            .map(|r| r.refresh_token.clone())
        else {
            return Err(Error::Auth(
                "no refresh token available — session was authenticated via v2".into(),
            ));
        };

        let resp = self
            .request_unauthenticated(
                Method::POST,
                "session/refresh-token",
                Some(1),
                Some(&RefreshTokenRequest {
                    refresh_token: &refresh_token,
                }),
            )
            .await?;

        let payload: OAuthPayload = serde_json::from_slice(&resp.body)?;
        let expires_in = payload.expires_in.parse::<u64>().map_err(|e| {
            Error::Auth(format!("invalid expires_in '{}': {e}", payload.expires_in))
        })?;

        let new_rest = RestAuth::OAuth {
            access_token: payload.access_token,
            token_type: payload.token_type,
        };
        let new_refresh = RefreshState {
            refresh_token: payload.refresh_token,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        };
        session
            .modify(|s| {
                s.tokens.rest = Some(new_rest);
                s.tokens.refresh = Some(new_refresh);
            })
            .await;
        info!(
            ttl_seconds = expires_in,
            "OAuth refresh successful — REST + refresh tokens rotated"
        );
        Ok(())
    }

    /// Authenticated request that deserialises the JSON body. Reads the
    /// active tokens from `session` and injects the appropriate auth headers.
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
            // Endpoints that legitimately return an empty body (DELETE) need
            // a `()` response type; let serde handle that case.
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
