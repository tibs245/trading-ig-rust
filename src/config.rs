//! Configuration types: environments, base URLs, defaults.

use std::time::Duration;

use url::Url;

/// IG API environment.
///
/// Demo accounts are free to create at <https://www.ig.com/uk/demo-account>.
/// Use `Environment::Custom` only for testing against a mock server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Environment {
    Demo,
    Live,
    /// Override the base URL (e.g. a wiremock instance during tests).
    Custom(Url),
}

impl Environment {
    /// The canonical REST base URL for this environment.
    pub fn base_url(&self) -> Url {
        match self {
            Environment::Demo => {
                Url::parse("https://demo-api.ig.com/gateway/deal/").expect("static URL is valid")
            }
            Environment::Live => {
                Url::parse("https://api.ig.com/gateway/deal/").expect("static URL is valid")
            }
            Environment::Custom(u) => {
                let mut u = u.clone();
                // Ensure trailing slash so `Url::join` behaves correctly.
                if !u.path().ends_with('/') {
                    let new_path = format!("{}/", u.path());
                    u.set_path(&new_path);
                }
                u
            }
        }
    }
}

/// Tunable client parameters. Most users won't need to touch this directly —
/// configure via [`crate::IgClientBuilder`].
#[derive(Debug, Clone)]
pub struct IgConfig {
    pub environment: Environment,
    pub api_key: String,
    pub user_agent: String,
    pub request_timeout: Duration,
    /// How early before the OAuth `expires_in` to proactively refresh.
    pub token_refresh_skew: Duration,
}

impl IgConfig {
    pub fn new(environment: Environment, api_key: impl Into<String>) -> Self {
        Self {
            environment,
            api_key: api_key.into(),
            user_agent: format!("trading-ig-rust/{}", env!("CARGO_PKG_VERSION")),
            request_timeout: Duration::from_secs(30),
            token_refresh_skew: Duration::from_secs(10),
        }
    }
}
