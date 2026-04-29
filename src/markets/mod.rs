//! Markets domain — search, fetch, and navigate IG market instruments.
//!
//! Access via [`crate::IgClient::markets`]:
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! use trading_ig::models::common::Epic;
//!
//! // Search for markets
//! let results = client.markets().search("EUR/USD").await?;
//!
//! // Fetch a single market
//! let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
//! let details = client.markets().get(&epic).await?;
//!
//! // Navigate the market hierarchy
//! let root = client.markets().navigation().await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

use crate::client::IgClient;

/// Typed accessor for all markets endpoints.
///
/// Obtained via [`IgClient::markets`].
#[derive(Debug)]
pub struct MarketsApi<'a> {
    pub(crate) client: &'a IgClient,
}
