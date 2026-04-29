//! Client sentiment domain — proportion of IG clients long vs short.
//!
//! Entry point: [`crate::IgClient::client_sentiment`].
//!
//! ```no_run
//! # async fn run(client: trading_ig::IgClient) -> trading_ig::Result<()> {
//! // Single market
//! let s = client.client_sentiment().get("CC.D.LCO.UNC.IP").await?;
//! println!("{}% long, {}% short", s.long_position_percentage, s.short_position_percentage);
//!
//! // Multiple markets
//! let many = client.client_sentiment().get_many(&["CC.D.LCO.UNC.IP", "IX.D.DAX.IFD.IP"]).await?;
//!
//! // Related markets
//! let related = client.client_sentiment().related("CC.D.LCO.UNC.IP").await?;
//! # Ok(()) }
//! ```

pub mod api;
pub mod models;

pub use api::ClientSentimentApi;
pub use models::Sentiment;
