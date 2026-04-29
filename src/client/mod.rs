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

use crate::client_sentiment::ClientSentimentApi;
use crate::config::IgConfig;
use crate::history::HistoryApi;
use crate::markets::MarketsApi;
use crate::operations::OperationsApi;
use crate::prices::PricesApi;
use crate::repeat_dealing::RepeatDealingApi;
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

    /// Client sentiment API: long/short percentages for IG markets.
    pub fn client_sentiment(&self) -> ClientSentimentApi<'_> {
        ClientSentimentApi { client: self }
    }

    /// History API: activity (v1 + v3) and transactions (v1 + v2).
    pub fn history(&self) -> HistoryApi<'_> {
        HistoryApi { client: self }
    }

    /// Markets API: search, fetch, and navigate IG market instruments.
    pub fn markets(&self) -> MarketsApi<'_> {
        MarketsApi { client: self }
    }

    /// Operations API: manage API application keys.
    pub fn operations(&self) -> OperationsApi<'_> {
        OperationsApi { client: self }
    }

    /// Historical prices API: v1, v2, v3 endpoints and auto-pagination.
    pub fn prices(&self) -> PricesApi<'_> {
        PricesApi { client: self }
    }

    /// Repeat dealing API: windows for re-trading recently dealt instruments.
    pub fn repeat_dealing(&self) -> RepeatDealingApi<'_> {
        RepeatDealingApi { client: self }
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
