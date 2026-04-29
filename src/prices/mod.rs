//! Historical prices domain.
//!
//! Access via [`crate::client::IgClient::prices`]:
//!
//! ```no_run
//! # use trading_ig::{IgClient, prices::models::{HistoricalPricesRequest, Resolution}};
//! # async fn run(client: IgClient) -> trading_ig::Result<()> {
//! use trading_ig::models::common::Epic;
//!
//! let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
//!
//! // Single-page flexible query (v3).
//! let prices = client.prices().history_v3(&epic, HistoricalPricesRequest::default()).await?;
//!
//! // All pages combined (v3 auto-paginate).
//! let all = client.prices().history_v3_all(&epic, HistoricalPricesRequest::default()).await?;
//!
//! // Fixed number of bars (v2).
//! let h = client.prices().history_by_num_points_v2(&epic, Resolution::Hour, 100).await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

use crate::client::IgClient;

/// Typed accessor for the historical prices endpoints.
///
/// Obtain one via [`IgClient::prices`].
#[derive(Debug)]
pub struct PricesApi<'a> {
    pub(crate) client: &'a IgClient,
}
