//! Login / refresh / switch / logout flows.

use std::time::{Duration, Instant};

use http::Method;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::{Error, Result};
use crate::session::tokens::{AuthTokens, OAuthPayload, SessionState};
use crate::session::{Credentials, SessionHandle, SessionInfo};

/// Public entry point for session management. Obtain via
/// [`crate::IgClient::session`].
#[derive(Debug)]
pub struct SessionApi {
    pub(crate) handle: SessionHandle,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest<'a> {
    identifier: &'a str,
    password: &'a str,
    encrypted_password: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseV3 {
    account_id: String,
    client_id: String,
    timezone_offset: Option<i32>,
    lightstreamer_endpoint: String,
    currency_iso_code: Option<String>,
    locale: Option<String>,
    oauth_token: OAuthPayload,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponseV2 {
    account_id: String,
    client_id: String,
    timezone_offset: Option<i32>,
    lightstreamer_endpoint: String,
    currency_iso_code: Option<String>,
    locale: Option<String>,
}

impl SessionApi {
    /// Log in using the canonical v3 flow (OAuth bearer tokens).
    #[instrument(skip_all)]
    pub async fn login(&self) -> Result<SessionInfo> {
        let creds = self
            .handle
            .credentials
            .as_ref()
            .ok_or_else(|| Error::Auth("no credentials configured on the client".into()))?;

        match creds {
            Credentials::Password { username, password } => {
                self.login_v3(username, password).await
            }
        }
    }

    /// Log in using the legacy v2 flow (CST + X-SECURITY-TOKEN response headers).
    /// Mainly used by the streaming client which still wants CST/XST.
    #[instrument(skip_all)]
    pub async fn login_v2(&self) -> Result<SessionInfo> {
        let creds = self
            .handle
            .credentials
            .as_ref()
            .ok_or_else(|| Error::Auth("no credentials configured on the client".into()))?;

        let Credentials::Password { username, password } = creds;
        let body = LoginRequest {
            identifier: username,
            password,
            encrypted_password: false,
        };

        let resp = self
            .handle
            .transport
            .request_unauthenticated(Method::POST, "session", Some(2), Some(&body))
            .await?;

        let cst = resp
            .headers
            .get("CST")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::Auth("missing CST header in login response".into()))?
            .to_owned();
        let xst = resp
            .headers
            .get("X-SECURITY-TOKEN")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::Auth("missing X-SECURITY-TOKEN header".into()))?
            .to_owned();

        let body: LoginResponseV2 = serde_json::from_slice(&resp.body)?;
        let new_state = SessionState {
            tokens: Some(AuthTokens::Cst {
                cst,
                x_security_token: xst,
            }),
            account_id: Some(body.account_id.clone()),
            client_id: Some(body.client_id.clone()),
            lightstreamer_endpoint: Some(body.lightstreamer_endpoint.clone()),
        };
        self.handle.session.replace(new_state).await;

        Ok(SessionInfo {
            account_id: body.account_id,
            client_id: body.client_id,
            timezone_offset: body.timezone_offset,
            lightstreamer_endpoint: body.lightstreamer_endpoint,
            currency_iso_code: body.currency_iso_code,
            locale: body.locale,
        })
    }

    async fn login_v3(&self, username: &str, password: &str) -> Result<SessionInfo> {
        let body = LoginRequest {
            identifier: username,
            password,
            encrypted_password: false,
        };

        let resp = self
            .handle
            .transport
            .request_unauthenticated(Method::POST, "session", Some(3), Some(&body))
            .await?;

        let body: LoginResponseV3 = serde_json::from_slice(&resp.body)?;

        let expires_in = body
            .oauth_token
            .expires_in
            .parse::<u64>()
            .map_err(|e| Error::Auth(format!("invalid expires_in '{}': {e}", body.oauth_token.expires_in)))?;

        let tokens = AuthTokens::OAuth {
            access_token: body.oauth_token.access_token,
            refresh_token: body.oauth_token.refresh_token,
            token_type: body.oauth_token.token_type,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        };
        let new_state = SessionState {
            tokens: Some(tokens),
            account_id: Some(body.account_id.clone()),
            client_id: Some(body.client_id.clone()),
            lightstreamer_endpoint: Some(body.lightstreamer_endpoint.clone()),
        };
        self.handle.session.replace(new_state).await;

        Ok(SessionInfo {
            account_id: body.account_id,
            client_id: body.client_id,
            timezone_offset: body.timezone_offset,
            lightstreamer_endpoint: body.lightstreamer_endpoint,
            currency_iso_code: body.currency_iso_code,
            locale: body.locale,
        })
    }

    /// Refresh the v3 access token using the stored refresh token.
    #[instrument(skip_all)]
    pub async fn refresh(&self) -> Result<()> {
        let state = self.handle.session.snapshot().await;
        let Some(AuthTokens::OAuth { refresh_token, .. }) = state.tokens else {
            return Err(Error::Auth("no refresh token available".into()));
        };

        #[derive(Serialize)]
        #[serde(rename_all = "snake_case")]
        struct Req<'a> {
            refresh_token: &'a str,
        }

        let resp = self
            .handle
            .transport
            .request_unauthenticated(
                Method::POST,
                "session/refresh-token",
                Some(1),
                Some(&Req { refresh_token: &refresh_token }),
            )
            .await?;

        let payload: OAuthPayload = serde_json::from_slice(&resp.body)?;
        let expires_in = payload
            .expires_in
            .parse::<u64>()
            .map_err(|e| Error::Auth(format!("invalid expires_in: {e}")))?;

        let new_tokens = AuthTokens::OAuth {
            access_token: payload.access_token,
            refresh_token: payload.refresh_token,
            token_type: payload.token_type,
            expires_at: Instant::now() + Duration::from_secs(expires_in),
        };
        self.handle
            .session
            .modify(|s| s.tokens = Some(new_tokens))
            .await;
        Ok(())
    }

    /// Tear down the current session on the server side and locally.
    #[instrument(skip_all)]
    pub async fn logout(&self) -> Result<()> {
        // Best-effort: even if the server call fails (e.g. tokens already
        // expired) we still clear local state.
        let _ = self
            .handle
            .transport
            .request::<(), serde_json::Value>(
                Method::DELETE,
                "session",
                Some(1),
                None::<&()>,
                &self.handle.session,
            )
            .await;
        self.handle.session.replace(SessionState::default()).await;
        Ok(())
    }
}
