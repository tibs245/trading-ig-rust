//! History domain — activity (v1 + v3) and transactions (v1 + v2).
//!
//! Access via [`crate::IgClient::history`]:
//!
//! ```no_run
//! use trading_ig::history::ActivityRequest;
//!
//! # async fn run(client: &trading_ig::IgClient) -> trading_ig::Result<()> {
//! // Activity v3 (paginated, all pages returned):
//! let acts = client.history().activity_v3(ActivityRequest::default()).await?;
//!
//! // Activity v1 (last 24 h):
//! let acts_v1 = client.history().activity_by_period_v1(86_400_000).await?;
//!
//! // Transactions v2:
//! use trading_ig::history::TransactionsRequest;
//! let txs = client.history().transactions_v2(TransactionsRequest::default()).await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

pub use models::{
    Activity, ActivityAction, ActivityChannel, ActivityDetails, ActivityRequest, ActivityStatus,
    ActivityType, ActivityV1, PageData, Transaction, TransactionType, TransactionsMetadata,
    TransactionsRequest, TransactionsResponse,
};

use crate::IgClient;

/// Typed accessor for the history domain.
///
/// Obtain one via [`IgClient::history`].
#[derive(Debug, Clone)]
pub struct HistoryApi<'a> {
    pub(crate) client: &'a IgClient,
}
