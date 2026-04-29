//! Lightstreamer streaming client for the IG Markets API.
//!
//! This module is guarded by the `stream` Cargo feature.  Enable it in
//! `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! trading-ig = { version = "...", features = ["stream"] }
//! ```
//!
//! # Quickstart
//!
//! ```no_run
//! # use trading_ig::{IgClient, Environment, Credentials};
//! # async fn run() -> trading_ig::Result<()> {
//! let client = IgClient::builder()
//!     .environment(Environment::Demo)
//!     .api_key("YOUR_KEY")
//!     .credentials(Credentials::password("user", "pass"))
//!     .build()?;
//!
//! // v2 login gives CST/XST which Lightstreamer needs directly.
//! client.session().login_v2().await?;
//!
//! let (stream, _events) = client.streaming().connect().await?;
//! let mut rx = stream.subscribe_market("CS.D.GBPUSD.TODAY.IP").await?;
//!
//! while let Some(update) = rx.recv().await {
//!     println!("{} bid={:?} offer={:?}", update.epic, update.bid, update.offer);
//! }
//!
//! stream.disconnect().await?;
//! # Ok(()) }
//! ```

pub mod client;
pub(crate) mod connection;
pub mod events;
pub mod protocol;
pub mod reconnect;
pub(crate) mod subscription;

pub use client::{StreamingApi, StreamingClient};
pub use events::{
    AccountUpdate, CandleScale, ChartCandleUpdate, ChartTickUpdate, MarketUpdate, TradeUpdate,
};
pub use reconnect::{AutoReconnect, StreamingEvent};
