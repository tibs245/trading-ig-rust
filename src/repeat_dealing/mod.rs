//! Repeat dealing domain — windows for re-trading recently dealt instruments.
//!
//! Entry point: [`crate::IgClient::repeat_dealing`].
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! use trading_ig::models::Epic;
//!
//! // All windows
//! let windows = client.repeat_dealing().window().await?;
//!
//! // Windows for a specific epic
//! let epic = Epic::new("CS.D.GBPUSD.TODAY.IP");
//! let windows = client.repeat_dealing().window_for(&epic).await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

pub use api::RepeatDealingApi;
pub use models::RepeatDealingWindow;
