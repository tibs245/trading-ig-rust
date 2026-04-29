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
use tracing::{Instrument, debug, debug_span, warn};

use crate::config::IgConfig;
use crate::error::{ApiError, Error, Result};
use crate::session::{AuthTokens, SharedSession};

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
        let url = self.url(path)?;
        let mut headers = self.base_headers(version)?;
        let state = session.snapshot().await;
        let Some(tokens) = state.tokens else {
            return Err(Error::Auth(
                "no active session — call session().login() first".into(),
            ));
        };
        match &tokens {
            AuthTokens::Cst {
                cst,
                x_security_token,
            } => {
                headers.insert(HDR_CST, HeaderValue::from_str(cst)?);
                headers.insert(HDR_XST, HeaderValue::from_str(x_security_token)?);
            }
            AuthTokens::OAuth {
                access_token,
                token_type,
                ..
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
