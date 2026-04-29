//! Builder for [`IgClient`].

use std::sync::Arc;
use std::time::Duration;

use crate::client::IgClient;
use crate::client::http::Transport;
use crate::config::{Environment, IgConfig};
use crate::error::{Error, Result};
use crate::session::{Credentials, SharedSession};

#[derive(Debug, Default)]
pub struct IgClientBuilder {
    environment: Option<Environment>,
    api_key: Option<String>,
    credentials: Option<Credentials>,
    user_agent: Option<String>,
    request_timeout: Option<Duration>,
    token_refresh_skew: Option<Duration>,
}

impl IgClientBuilder {
    pub fn environment(mut self, env: Environment) -> Self {
        self.environment = Some(env);
        self
    }
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
    pub fn credentials(mut self, creds: Credentials) -> Self {
        self.credentials = Some(creds);
        self
    }
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }
    pub fn request_timeout(mut self, d: Duration) -> Self {
        self.request_timeout = Some(d);
        self
    }
    pub fn token_refresh_skew(mut self, d: Duration) -> Self {
        self.token_refresh_skew = Some(d);
        self
    }

    pub fn build(self) -> Result<IgClient> {
        let environment = self
            .environment
            .ok_or_else(|| Error::Config("environment is required".into()))?;
        let api_key = self
            .api_key
            .ok_or_else(|| Error::Config("api_key is required".into()))?;

        let mut cfg = IgConfig::new(environment, api_key);
        if let Some(ua) = self.user_agent {
            cfg.user_agent = ua;
        }
        if let Some(t) = self.request_timeout {
            cfg.request_timeout = t;
        }
        if let Some(s) = self.token_refresh_skew {
            cfg.token_refresh_skew = s;
        }

        let cfg = Arc::new(cfg);
        let transport = Transport::new(cfg.clone())?;
        Ok(IgClient {
            transport,
            session: SharedSession::default(),
            credentials: self.credentials,
            config: cfg,
        })
    }
}
