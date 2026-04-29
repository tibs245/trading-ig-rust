//! The top-level [`IgClient`] — entry point for all REST domain APIs.
//!
//! Construction goes through [`IgClientBuilder`]:
//!
//! ```no_run
//! use trading_ig::{IgClient, Environment, Credentials};
//!
//! # async fn run() -> trading_ig::Result<()> {
//! let client = IgClient::builder()
//!     .environment(Environment::Demo)
//!     .api_key("YOUR_API_KEY")
//!     .credentials(Credentials::password("user", "pass"))
//!     .build()?;
//!
//! client.session().login().await?;
//! # Ok(()) }
//! ```

mod builder;
pub(crate) mod http;

pub use builder::IgClientBuilder;

use std::sync::Arc;

use crate::config::IgConfig;
use crate::session::{Credentials, SessionApi, SessionHandle, SharedSession};

use http::Transport;

/// Cheap-to-clone handle to an authenticated (or about-to-be-authenticated)
/// IG client. Internally a shared transport plus shared session state.
#[derive(Debug, Clone)]
pub struct IgClient {
    pub(crate) transport: Transport,
    pub(crate) session: SharedSession,
    pub(crate) credentials: Option<Credentials>,
    pub(crate) config: Arc<IgConfig>,
}

impl IgClient {
    pub fn builder() -> IgClientBuilder {
        IgClientBuilder::default()
    }

    /// Read-only access to the resolved configuration.
    pub fn config(&self) -> &IgConfig {
        &self.config
    }

    /// Session API: login, refresh, switch account, logout.
    pub fn session(&self) -> SessionApi {
        SessionApi {
            handle: SessionHandle {
                transport: self.transport.clone(),
                session: self.session.clone(),
                credentials: self.credentials.clone(),
            },
        }
    }
}
