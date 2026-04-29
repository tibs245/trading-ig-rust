//! Watchlists domain: full CRUD on watchlists and their markets.
//!
//! Entry point is [`WatchlistsApi`], obtained via [`crate::IgClient::watchlists`].
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! // List all watchlists
//! let watchlists = client.watchlists().list().await?;
//!
//! // List markets in a specific watchlist
//! let markets = client.watchlists().markets("12345678").await?;
//! # Ok(()) }
//! ```

mod api;
pub mod models;

pub use crate::markets::models::MarketSummary;
pub use models::{
    AddMarketResponse, CreateWatchlistRequest, CreateWatchlistResponse, CreateWatchlistStatus,
    DeleteWatchlistResponse, RemoveMarketResponse, WatchlistSummary,
};

use crate::IgClient;

/// Typed accessor for the watchlists REST domain.
///
/// Obtained via [`IgClient::watchlists`].
#[derive(Debug, Clone)]
pub struct WatchlistsApi<'a> {
    pub(crate) client: &'a IgClient,
}
